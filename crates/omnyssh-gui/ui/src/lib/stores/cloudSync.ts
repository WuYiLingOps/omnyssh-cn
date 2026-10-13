/**
 * Cloud sync store: wraps the sync engine's subscribe/getState in a Svelte
 * `writable`, drives the automation the React hook used to (startup check
 * with backoff, data-change debounce, visibility re-check), and exposes the
 * user-facing sync actions.
 *
 * Triggers:
 * - Startup: 1s after mount, retrying at 30/60/120/240s until a definitive
 *   outcome; `startupChecked` latches so automatic sync stays off until then.
 * - Data change: hosts / snippets store subscriptions, debounced 3s, with the
 *   engine's skip-hash check so a merge's own write-back doesn't re-trigger.
 *   The TUI edits hosts.toml without passing through these stores — the
 *   startup check and the visibility re-check cover that path.
 * - Visibility: `visibilitychange`, throttled to 30s.
 */

import { get, writable } from "svelte/store";

import { hosts } from "$lib/stores/hosts";
import { snippets } from "$lib/stores/snippets";
import { listSnippets } from "$lib/ipc/commands";
import { applySyncPayload, buildSyncPayload } from "$lib/sync/payload";
import * as engine from "$lib/sync/engine";
import type { CloudSyncState } from "$lib/sync/engine";
import type { RevisionPreview } from "$lib/sync/engine";
import type { SyncPayload, SyncResult } from "$lib/sync/types";

/** The engine state, as a Svelte store. */
export const cloudSync = writable<Readonly<CloudSyncState>>(engine.getState());

engine.subscribe(() => cloudSync.set(engine.getState()));

let initialized = false;

/** Called once from the app shell: loads config / token / password config. */
export async function initCloudSync(): Promise<void> {
  if (initialized) return;
  initialized = true;
  await engine.initialize();
}

// ---------------------------------------------------------------------------
// The local-apply callback the engine invokes after a merge / restore
// ---------------------------------------------------------------------------

/** Applies a merged payload to this device and refreshes both stores. */
const applyLocal = async (payload: SyncPayload): Promise<void> => {
  await applySyncPayload(payload);
  // applySyncPayload already re-read the snippet list into the store; hosts
  // arrive via the `hosts-loaded` broadcast of `reload_hosts`.
};

// ---------------------------------------------------------------------------
// Automatic triggers
// ---------------------------------------------------------------------------

let automationStarted = false;
let debounceTimer: ReturnType<typeof setTimeout> | null = null;
let visibilityLastAt = 0;

const STARTUP_BACKOFF_MS = [30_000, 60_000, 120_000, 240_000];

async function startupCheck(attempt: number): Promise<void> {
  const s = engine.getState();
  if (!s.connected) {
    // Nothing to reconcile without a connection; future connects come from the
    // settings screen, whose flows run their own syncs.
    engine.markStartupChecked();
    return;
  }
  if (s.securityState !== "UNLOCKED") {
    // A locked password cannot decrypt a remote vault; leave the latch closed
    // so unlocking (settings screen) can run the check.
    engine.markStartupChecked();
    return;
  }
  const result = await engine.syncNow({ reason: "startup", applyLocal });
  // Definitive outcomes (including user-decision states) end the startup run;
  // only transient errors retry with backoff.
  if (result.success || result.conflictDetected || result.shrinkBlocked || result.error === "empty-vault-guard") {
    engine.markStartupChecked();
    return;
  }
  if (attempt < STARTUP_BACKOFF_MS.length) {
    setTimeout(() => void startupCheck(attempt + 1), STARTUP_BACKOFF_MS[attempt]);
  } else {
    engine.markStartupChecked();
  }
}

/** The payload fingerprint of the current local data (used to consult the
 *  engine's skip hash before firing an automatic sync). */
async function currentPayload(): Promise<SyncPayload> {
  return buildSyncPayload(engine.syncPasswordsEnabled(), engine.syncCredentialsEnabled());
}

async function debouncedAutoSync(): Promise<void> {
  const s = engine.getState();
  if (!s.startupChecked || !s.autoSync || s.syncing || !s.connected || s.securityState !== "UNLOCKED") return;
  try {
    const payload = await currentPayload();
    if (engine.isPayloadSkipped(payload)) return;
  } catch {
    // Payload build failure (backend busy): skip this round, the next change
    // re-triggers.
    return;
  }
  await engine.syncNow({ reason: "auto", applyLocal });
}

function onDataChanged(): void {
  if (debounceTimer) clearTimeout(debounceTimer);
  debounceTimer = setTimeout(() => {
    debounceTimer = null;
    void debouncedAutoSync().catch((e) => console.warn("automatic sync failed:", e));
  }, 3000);
}

function onVisibility(): void {
  const now = Date.now();
  if (now - visibilityLastAt < 30_000) return;
  visibilityLastAt = now;
  const s = engine.getState();
  if (!s.startupChecked || !s.autoSync || s.syncing || !s.connected || s.securityState !== "UNLOCKED") return;
  // The TUI may have edited hosts.toml while this window was hidden; a sync
  // round reconciles whatever changed on disk.
  void engine
    .syncNow({ reason: "auto", applyLocal })
    .catch((e) => console.warn("visibility sync failed:", e));
}

/** Starts the automation once, from the app shell (after initCloudSync). */
export function startCloudSyncAutomation(): void {
  if (automationStarted) return;
  automationStarted = true;

  setTimeout(() => void startupCheck(0), 1000);

  hosts.subscribe(onDataChanged);
  snippets.subscribe(onDataChanged);

  if (typeof document !== "undefined") {
    document.addEventListener("visibilitychange", () => {
      if (document.visibilityState === "visible") onVisibility();
    });
  }
}

// ---------------------------------------------------------------------------
// Actions (thin wrappers over the engine; the settings screen binds these)
// ---------------------------------------------------------------------------

export const syncActions = {
  /** Manual sync button. */
  manualSync: (): Promise<SyncResult> => engine.syncNow({ reason: "manual", applyLocal }),

  connect: async (): Promise<void> => {
    const start = await engine.startDeviceFlow();
    await engine.completeGitHubAuth(start);
  },

  disconnect: (): Promise<void> => engine.disconnectGitHub(),

  setPassword: (password: string): Promise<void> => engine.setMasterPassword(password),
  unlockAndRemember: (password: string): Promise<boolean> => engine.unlockAndRemember(password),
  lock: (): void => engine.lock(),
  changePassword: (oldPassword: string, newPassword: string): Promise<boolean> =>
    engine.changePassword(oldPassword, newPassword),
  isUnlocked: (): boolean => engine.isUnlocked(),

  setAutoSync: (enabled: boolean): void => engine.setAutoSync(enabled),
  setSyncPasswords: (enabled: boolean): void => engine.setSyncPasswords(enabled),
  setSyncCredentials: (enabled: boolean): void => engine.setSyncCredentials(enabled),

  resolveConflictUseRemote: (cloudPassword: string): Promise<SyncResult> =>
    engine.resolveConflictUseRemote(cloudPassword, applyLocal),
  resolveConflictUseLocal: (): Promise<SyncResult> => engine.resolveConflictUseLocal(),

  restoreRemoteFromBlocked: (): Promise<SyncResult> => engine.restoreRemoteFromBlocked(applyLocal),
  forcePushLocal: (): Promise<SyncResult> => engine.forcePushLocal(),

  confirmEmptyVaultRestore: (): Promise<SyncResult> => engine.confirmEmptyVaultRestore(applyLocal),
  confirmEmptyVaultPush: (): Promise<SyncResult> => engine.confirmEmptyVaultPush(),
  dismissEmptyVaultPrompt: (): void => engine.dismissEmptyVaultPrompt(),

  fetchHistory: (): Promise<Array<{ sha: string; date: number }>> => engine.fetchHistory(),
  previewRevision: (sha: string): Promise<RevisionPreview> => engine.previewRevision(sha),
  restoreRevision: (sha: string): Promise<SyncResult> => engine.restoreRevision(sha, applyLocal),
};

/** Convenience read of the current state for non-reactive contexts. */
export const syncState = (): Readonly<CloudSyncState> => get(cloudSync);
