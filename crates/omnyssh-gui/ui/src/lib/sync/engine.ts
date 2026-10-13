/**
 * Sync engine: state machine + anchor + three guards (ported from DockPilot's
 * CloudSyncManager, cut to the single GitHub Gist provider).
 *
 * One sync round (syncNow):
 *   download remote → compare the anchor signature
 *   ├─ remote unchanged & local payload hash unchanged → no-op
 *   ├─ remote unchanged & local changed → upload (version = remote+1, shrink guard)
 *   └─ remote changed (another device pushed) → decrypt remote → three-way
 *       merge(base, local, remote)
 *       ├─ merge ok → upload the merge → apply locally → write base → write anchor
 *       └─ decrypt failed (different password) → CONFLICT; the UI offers remote/local
 *
 * Write order (crash-safe, as upstream): base first, then the anchor. A crash
 * in between → missing/stale anchor forces a remote re-check + merge next
 * time, never a silent overwrite.
 *
 * Guards:
 *   1. Shrink detection (guards.ts) → BLOCKED; the UI offers "restore remote / force push"
 *   2. Empty-vault protection: no base + local empty + remote non-empty → waits
 *      for user confirmation, never silently overwrites
 *   3. Startup latch: automatic sync does not fire until startupChecked is true
 *      (the store layer owns that)
 *
 * The sync password and the GitHub token live in memory only; localStorage
 * holds the config / verificationHash / base / anchor.
 */

import { getVersion } from "@tauri-apps/api/app";

import type {
  ConflictInfo,
  DeviceFlowStart,
  MasterKeyConfig,
  ProviderSyncAnchor,
  SecurityState,
  ShrinkFinding,
  SyncConfig,
  SyncPayload,
  SyncResult,
  SyncState,
  SyncedFile,
} from "./types";
import { DEFAULT_SYNC_CONFIG, SYNC_STORAGE_KEYS, loadDeviceInfo } from "./types";
import {
  changeMasterPassword,
  createMasterKeyConfig,
  decryptPayload,
  encryptPayload,
  verifyPassword,
} from "./encryption";
import { mergeSyncPayloads } from "./merge";
import { detectSuspiciousShrink } from "./guards";
import { createSyncedFileSignature, decideRemoteChanged } from "./anchor";
import { buildSyncPayload, payloadFingerprint } from "./payload";
import * as gist from "./gist";
import * as auth from "./auth";
import type { TokenBackend } from "./auth";

export type { TokenBackend };

/** The pending decision info while the empty-vault guard waits for the user. */
export interface EmptyVaultPending {
  payload: SyncPayload;
  remoteFile: SyncedFile;
}

export interface CloudSyncState {
  securityState: SecurityState;
  syncState: SyncState;
  /** A sync operation is in flight. */
  syncing: boolean;
  /** The GitHub token is loaded (sync is possible). */
  connected: boolean;
  account: SyncConfig["account"];
  autoSync: boolean;
  /** Whether the payload carries login passwords (mirrors the persisted switch). */
  syncPasswords: boolean;
  /** Whether the payload carries SSH credential materials (PEM / passphrases /
   *  identity passwords; mirrors the persisted switch). */
  syncCredentials: boolean;
  tokenBackend: TokenBackend | null;
  gistId: string | null;
  lastSyncAt: number | null;
  lastSyncVersion: number | null;
  remoteVersion: number | null;
  lastError: string | null;
  conflict: ConflictInfo | null;
  shrinkFinding: Extract<ShrinkFinding, { suspicious: true }> | null;
  emptyVaultPending: EmptyVaultPending | null;
  /** The startup remote check has finished (the gate for automatic sync). */
  startupChecked: boolean;
  /** The GitHub token has expired (API 401): reconnect required, sync paused. */
  tokenInvalid: boolean;
  /** Both-sides-modified count from the most recent merge (local wins) — shown
   *  so a remote edit isn't silently swallowed. Cleared on the next sync. */
  lastMergeConflicts: number | null;
}

const initialState: CloudSyncState = {
  securityState: "NO_KEY",
  syncState: "IDLE",
  syncing: false,
  connected: false,
  account: null,
  autoSync: true,
  syncPasswords: false,
  syncCredentials: false,
  tokenBackend: null,
  gistId: null,
  lastSyncAt: null,
  lastSyncVersion: null,
  remoteVersion: null,
  lastError: null,
  conflict: null,
  shrinkFinding: null,
  emptyVaultPending: null,
  startupChecked: false,
  tokenInvalid: false,
  lastMergeConflicts: null,
};

// ---------------------------------------------------------------------------
// localStorage basics
// ---------------------------------------------------------------------------

function loadJSON<T>(key: string): T | null {
  try {
    const raw = localStorage.getItem(key);
    return raw ? (JSON.parse(raw) as T) : null;
  } catch {
    return null;
  }
}

function saveJSON(key: string, value: unknown): void {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // WebView storage unavailable: sync degrades to per-session behaviour, no crash
  }
}

function removeKey(key: string): void {
  try {
    localStorage.removeItem(key);
  } catch {
    // ignore
  }
}

const loadConfig = (): SyncConfig => ({ ...DEFAULT_SYNC_CONFIG, ...(loadJSON<SyncConfig>(SYNC_STORAGE_KEYS.SYNC_CONFIG) ?? {}) });
const saveConfig = (config: SyncConfig) => saveJSON(SYNC_STORAGE_KEYS.SYNC_CONFIG, config);

const loadMasterKeyConfig = (): MasterKeyConfig | null => loadJSON<MasterKeyConfig>(SYNC_STORAGE_KEYS.MASTER_KEY_CONFIG);
const saveMasterKeyConfig = (config: MasterKeyConfig) => saveJSON(SYNC_STORAGE_KEYS.MASTER_KEY_CONFIG, config);

const loadBase = (): SyncPayload | null => loadJSON<SyncPayload>(SYNC_STORAGE_KEYS.BASE_PAYLOAD);
const saveBase = (payload: SyncPayload) => saveJSON(SYNC_STORAGE_KEYS.BASE_PAYLOAD, payload);

const loadAnchor = (): ProviderSyncAnchor | null => loadJSON<ProviderSyncAnchor>(SYNC_STORAGE_KEYS.ANCHOR);

async function saveAnchor(file: SyncedFile, resourceId: string | null): Promise<void> {
  const signature = await createSyncedFileSignature(file);
  saveJSON(SYNC_STORAGE_KEYS.ANCHOR, {
    signature,
    version: file.meta.version,
    updatedAt: file.meta.updatedAt,
    deviceId: file.meta.deviceId,
    resourceId,
    observedAt: Date.now(),
  } satisfies ProviderSyncAnchor);
}

// ---------------------------------------------------------------------------
// Engine state (module-level singleton; the snapshot reference updates only on
// notify, which is what the Svelte store reads)
// ---------------------------------------------------------------------------

const device = loadDeviceInfo();
const state: CloudSyncState = { ...initialState };

let snapshot: Readonly<CloudSyncState> = { ...state };
const listeners = new Set<() => void>();

/** The sync password (memory only). */
let masterPassword: string | null = null;
/** The GitHub token (memory only, loaded from the secret store). */
let accessToken: string | null = null;
/** The app version (for meta). */
let appVersion = "0.0.0";
/** Guards against concurrent syncs. */
let syncSeq = 0;
/** The skip hash after a merge was applied: stops React/Svelte state
 *  round-trips from triggering a redundant sync. */
let skipHash: string | null = null;
/** The pending remote encrypted file during CONFLICT / BLOCKED (restore-remote uses it). */
let conflictRemoteFile: SyncedFile | null = null;
let blockedRemoteFile: SyncedFile | null = null;

export function getState(): Readonly<CloudSyncState> {
  return snapshot;
}

export function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

function notify(): void {
  Object.assign(state, {
    securityState: masterKeyConfigExists() ? (masterPassword ? "UNLOCKED" : "LOCKED") : "NO_KEY",
  });
  snapshot = { ...state };
  for (const listener of listeners) listener();
}

function masterKeyConfigExists(): boolean {
  return loadMasterKeyConfig() !== null;
}

function setError(error: string | null): void {
  state.lastError = error;
  notify();
}

/** Marks the payload fingerprint: the next automatic trigger with the same
 *  fingerprint is skipped. */
export function markSkipHash(payload: SyncPayload): void {
  skipHash = payloadFingerprint(payload);
}

/** Whether the current payload is exactly what a merge just applied back
 *  (this sync round should be skipped). */
export function isPayloadSkipped(payload: SyncPayload): boolean {
  if (skipHash === null) return false;
  if (payloadFingerprint(payload) === skipHash) return true;
  // A different fingerprint means the user changed something again: drop the skip mark.
  skipHash = null;
  return false;
}

// ---------------------------------------------------------------------------
// Initialisation
// ---------------------------------------------------------------------------

/** Called once at app startup: loads config / token / password config, no
 *  network requests. */
export async function initialize(): Promise<void> {
  try {
    appVersion = await getVersion();
  } catch {
    appVersion = "0.0.0";
  }
  const config = loadConfig();
  state.autoSync = config.autoSync;
  state.syncPasswords = config.syncPasswords;
  state.syncCredentials = config.syncCredentials;
  state.gistId = config.gistId;
  state.tokenBackend = config.tokenBackend;
  state.account = config.account;
  state.lastSyncAt = config.lastSyncAt;
  state.lastSyncVersion = config.lastSyncVersion;
  if (config.tokenBackend) {
    try {
      accessToken = await auth.loadToken(config.tokenBackend);
    } catch {
      accessToken = null;
    }
  }
  state.connected = accessToken !== null;
  // Auto-unlock: a remembered sync password (keychain / encrypted file) is
  // restored so no manual unlock is needed at startup — regardless of whether
  // a GitHub connection exists yet (the password can be set before connect).
  // A verification failure (the cloud password config changed) clears the
  // stale memory and locks again.
  if (masterKeyConfigExists() && config.passwordBackend) {
    try {
      const stored = await auth.loadSyncPassword(config.passwordBackend);
      if (stored) {
        const ok = await unlock(stored);
        if (!ok) await forgetStoredPassword(config.passwordBackend);
      }
    } catch {
      // Secret store unavailable: stay locked, do not block startup
    }
  }
  notify();
}

/** Persists the last sync time / version (survives restarts for display). */
function persistSyncState(): void {
  saveConfig({
    ...loadConfig(),
    lastSyncAt: state.lastSyncAt,
    lastSyncVersion: state.lastSyncVersion,
  });
}

/** The startup remote check finished (success / conflict / blocked / backoff
 *  exhausted): releases the automatic-sync latch. */
export function markStartupChecked(): void {
  if (!state.startupChecked) {
    state.startupChecked = true;
    notify();
  }
}

// ---------------------------------------------------------------------------
// GitHub connection management
// ---------------------------------------------------------------------------

export const startDeviceFlow = auth.startDeviceFlow;

/**
 * Finishes the Device Flow: polls for the token → stores it in the keychain →
 * fetches the account → discovers / records the Gist. The UI displays the
 * user code via onUserCode; the returned promise resolves once authorization
 * completes.
 */
export async function completeGitHubAuth(
  start: DeviceFlowStart,
  events: auth.DeviceFlowPollEvents = {},
  signal?: AbortSignal,
): Promise<void> {
  setError(null);
  const tokens = await auth.pollForToken(start, events, signal);
  const backend = await auth.saveToken(tokens.accessToken);
  accessToken = tokens.accessToken;

  let account: SyncConfig["account"] = null;
  try {
    account = await auth.getUserInfo(tokens.accessToken);
  } catch {
    // Account fetch failure does not block connecting
  }

  const gistId = await gist.findSyncGist(tokens.accessToken);
  const config: SyncConfig = { ...loadConfig(), gistId, tokenBackend: backend, account };
  saveConfig(config);

  state.gistId = gistId;
  state.tokenBackend = backend;
  state.account = account;
  state.connected = true;
  notify();
}

/** Disconnects GitHub: deletes the token, clears merge state (base / anchor /
 *  Gist id), keeps the sync password. */
export async function disconnectGitHub(): Promise<void> {
  const backend = state.tokenBackend;
  if (backend) {
    await auth.deleteToken(backend).catch(() => {});
  }
  accessToken = null;
  const config = loadConfig();
  const next: SyncConfig = {
    ...config,
    gistId: null,
    tokenBackend: null,
    account: null,
    lastSyncAt: null,
    lastSyncVersion: null,
  };
  saveConfig(next);
  // Clear merge state: after an account / resource switch the old base and
  // anchor cannot be reused.
  removeKey(SYNC_STORAGE_KEYS.BASE_PAYLOAD);
  removeKey(SYNC_STORAGE_KEYS.ANCHOR);
  state.gistId = null;
  state.tokenBackend = null;
  state.account = null;
  state.connected = false;
  state.lastSyncAt = null;
  state.lastSyncVersion = null;
  state.remoteVersion = null;
  state.startupChecked = false;
  if (state.syncState === "BLOCKED" || state.syncState === "CONFLICT" || state.syncState === "ERROR") {
    state.syncState = "IDLE";
    state.shrinkFinding = null;
    state.conflict = null;
    conflictRemoteFile = null;
    blockedRemoteFile = null;
  }
  notify();
}

// ---------------------------------------------------------------------------
// Sync password management
// ---------------------------------------------------------------------------

/** First-time sync password setup. */
export async function setMasterPassword(password: string): Promise<void> {
  const config = await createMasterKeyConfig(password);
  saveMasterKeyConfig(config);
  masterPassword = password;
  await rememberPassword(password);
  notify();
}

/** Unlock: verifies the password, then holds it in memory only. */
export async function unlock(password: string): Promise<boolean> {
  const config = loadMasterKeyConfig();
  if (!config) return false;
  const ok = await verifyPassword(password, config);
  if (ok) {
    masterPassword = password;
    notify();
  }
  return ok;
}

/** Unlocks and remembers the password in this machine's secret store (auto
 *  unlock on next startup); a remember failure does not affect this unlock. */
export async function unlockAndRemember(password: string): Promise<boolean> {
  const ok = await unlock(password);
  if (ok) await rememberPassword(password);
  return ok;
}

async function rememberPassword(password: string): Promise<void> {
  try {
    const backend = await auth.saveSyncPassword(password);
    saveConfig({ ...loadConfig(), passwordBackend: backend });
  } catch {
    // Secret store unavailable (no keychain and the encrypted file failed):
    // memory-only for this session
  }
}

/** Clears this machine's remembered sync password (backend defaults to the
 *  recorded landing spot). */
async function forgetStoredPassword(backend?: auth.TokenBackend): Promise<void> {
  const target = backend ?? loadConfig().passwordBackend;
  if (!target) return;
  try {
    await auth.deleteSyncPassword(target);
  } catch {
    // Entry absent / secret store unavailable: treat as cleared
  }
  saveConfig({ ...loadConfig(), passwordBackend: null });
}

export function lock(): void {
  masterPassword = null;
  // Locking forgets: the remembered sync password is cleared too, so the next
  // startup starts locked.
  void forgetStoredPassword();
  notify();
}

export function isUnlocked(): boolean {
  return masterPassword !== null;
}

/** Changes the sync password (the next sync re-uploads everything under it). */
export async function changePassword(oldPassword: string, newPassword: string): Promise<boolean> {
  const config = loadMasterKeyConfig();
  if (!config) return false;
  const next = await changeMasterPassword(oldPassword, newPassword, config);
  if (!next) return false;
  saveMasterKeyConfig(next);
  masterPassword = newPassword;
  // Update the remembered password too, or the next startup auto-unlock fails.
  await rememberPassword(newPassword);
  notify();
  return true;
}

/** Checks a cloud file decrypts with the current password (first-use-on-a-new-
 *  device hints). */
export async function canDecrypt(file: SyncedFile): Promise<boolean> {
  if (!masterPassword) return false;
  try {
    await decryptPayload(file, masterPassword);
    return true;
  } catch {
    return false;
  }
}

// ---------------------------------------------------------------------------
// Switches
// ---------------------------------------------------------------------------

export function setAutoSync(enabled: boolean): void {
  state.autoSync = enabled;
  saveConfig({ ...loadConfig(), autoSync: enabled });
  notify();
}

/** Whether the payload carries login passwords (default off). */
export function setSyncPasswords(enabled: boolean): void {
  state.syncPasswords = enabled;
  saveConfig({ ...loadConfig(), syncPasswords: enabled });
  notify();
}

/** Whether the payload carries SSH credential materials (default off). */
export function setSyncCredentials(enabled: boolean): void {
  state.syncCredentials = enabled;
  saveConfig({ ...loadConfig(), syncCredentials: enabled });
  notify();
}

export function syncPasswordsEnabled(): boolean {
  return loadConfig().syncPasswords;
}

export function syncCredentialsEnabled(): boolean {
  return loadConfig().syncCredentials;
}

// ---------------------------------------------------------------------------
// Upload and commit
// ---------------------------------------------------------------------------

interface UploadOpts {
  overrideShrink?: boolean;
  base: SyncPayload | null;
  baseVersion: number;
}

/** Encrypts and uploads the local payload; past the shrink guard; on success
 *  writes base first, then the anchor. */
async function uploadPayload(payload: SyncPayload, opts: UploadOpts): Promise<SyncResult> {
  if (!masterPassword) {
    return { success: false, action: "none", error: "sync password is not unlocked" };
  }

  const shrink = detectSuspiciousShrink(payload, opts.base, null);
  if (shrink.suspicious && !opts.overrideShrink) {
    state.syncState = "BLOCKED";
    state.shrinkFinding = shrink;
    notify();
    return { success: false, action: "none", shrinkBlocked: true, finding: shrink };
  }

  const encrypted = await encryptPayload(payload, masterPassword, device.deviceId, device.deviceName, appVersion, opts.baseVersion);
  let resourceId = state.gistId;
  if (resourceId) {
    await gist.updateSyncGist(accessToken!, resourceId, encrypted);
  } else {
    resourceId = await gist.createSyncGist(accessToken!, encrypted);
    state.gistId = resourceId;
    saveConfig({ ...loadConfig(), gistId: resourceId });
  }

  commitSyncedPayload(payload, encrypted, resourceId);
  return { success: true, action: "upload", version: encrypted.meta.version };
}

/** Local commit after a successful upload: lastSync update → base first →
 *  anchor second (order is not interchangeable). */
function commitSyncedPayload(payload: SyncPayload, file: SyncedFile, resourceId: string): void {
  state.lastSyncAt = Date.now();
  state.lastSyncVersion = file.meta.version;
  state.remoteVersion = file.meta.version;
  persistSyncState();
  saveBase(payload);
  void saveAnchor(file, resourceId);
}

/** Adopts the remote (content identical, or the merge equals the remote): no
 *  upload, align locally and commit the state. */
async function adoptRemote(remotePayload: SyncPayload, remoteFile: SyncedFile, resourceId: string, applyLocal: (p: SyncPayload) => Promise<void>): Promise<void> {
  await applyLocal(remotePayload);
  markSkipHash(remotePayload);
  state.lastSyncAt = Date.now();
  state.lastSyncVersion = remoteFile.meta.version;
  state.remoteVersion = remoteFile.meta.version;
  persistSyncState();
  saveBase(remotePayload);
  await saveAnchor(remoteFile, resourceId);
}

// ---------------------------------------------------------------------------
// One sync round
// ---------------------------------------------------------------------------

export interface SyncOptions {
  reason: "startup" | "auto" | "manual";
  /** Force push (skips the shrink guard and the merge). */
  overrideShrink?: boolean;
  /** Confirms the empty-vault merge (local empty + remote non-empty + no base). */
  confirmedEmptyVault?: boolean;
  /** Applies the payload to this device (injected by the store layer; goes
   *  through the Rust apply + reload commands). */
  applyLocal: (payload: SyncPayload) => Promise<void>;
}

export async function syncNow(opts: SyncOptions): Promise<SyncResult> {
  if (!accessToken) {
    return { success: false, action: "none", error: "GitHub is not connected" };
  }
  if (!masterPassword) {
    return { success: false, action: "none", error: "the sync password is not unlocked" };
  }
  if (state.syncing) {
    return { success: false, action: "none", error: "a sync is already in progress" };
  }

  const seq = ++syncSeq;
  state.syncing = true;
  state.syncState = "SYNCING";
  state.lastError = null;
  notify();

  try {
    const result = await runSync(opts, seq);
    logSyncOutcome(opts.reason, result);
    return result;
  } catch (error) {
    if (seq !== syncSeq) {
      return { success: false, action: "none", error: String(error) };
    }
    const message = error instanceof Error ? error.message : String(error);
    state.syncing = false;
    state.syncState = "ERROR";
    state.lastError = message;
    notify();
    console.error(`cloud sync failed (${opts.reason}): ${message}`);
    return { success: false, action: "none", error: message };
  }
}

/** Sync result logging: success → info; failure → error; user decisions
 *  (conflict, guard, empty vault) → warn. */
function logSyncOutcome(reason: SyncOptions["reason"], r: SyncResult): void {
  const what: Record<SyncResult["action"], string> = {
    upload: "uploaded local data",
    download: "pulled cloud data",
    merge: "merged cloud data",
    none: "cloud and local are identical",
  };
  if (r.success) {
    const conflicts = r.mergeConflicts != null ? `; ${r.mergeConflicts} both-side edit(s) resolved local-first` : "";
    console.info(`cloud sync done (${reason}): ${what[r.action]}${r.version != null ? ` (v${r.version})` : ""}${conflicts}`);
    return;
  }
  if (!r.error) return;
  if (r.conflictDetected || r.shrinkBlocked || r.error === "empty-vault-guard") {
    console.warn(`cloud sync paused (${reason}): ${r.error}`);
  } else {
    console.error(`cloud sync failed (${reason}): ${r.error}`);
  }
}

async function runSync(opts: SyncOptions, seq: number): Promise<SyncResult> {
  const assertCurrent = () => {
    if (seq !== syncSeq) throw new Error("the sync was superseded by a newer operation");
  };

  // 1. Make sure the Gist is usable.
  let resourceId = state.gistId;
  if (!resourceId) {
    resourceId = await gist.findSyncGist(accessToken!);
    if (resourceId) {
      state.gistId = resourceId;
      saveConfig({ ...loadConfig(), gistId: resourceId });
    }
  }
  assertCurrent();

  // 2. Download the remote + anchor decision.
  const remoteFile = resourceId ? await gist.downloadSyncGist(accessToken!, resourceId) : null;
  if (resourceId && !remoteFile) {
    // The Gist was deleted in the cloud: clear the record; the next upload rebuilds it.
    state.gistId = null;
    saveConfig({ ...loadConfig(), gistId: null });
    resourceId = null;
  }
  assertCurrent();

  const signature = await createSyncedFileSignature(remoteFile);
  const anchor = loadAnchor();
  const decision = decideRemoteChanged({
    currentSignature: signature,
    currentResourceId: resourceId,
    anchor,
    hasRemoteFile: remoteFile !== null,
  });

  const localPayload = await buildSyncPayload(
    loadConfig().syncPasswords,
    loadConfig().syncCredentials,
  );
  const base = loadBase();

  const finish = (result: SyncResult): SyncResult => {
    state.syncing = false;
    if (result.success) {
      state.syncState = "IDLE";
      state.shrinkFinding = null;
    }
    // The merge-conflict note belongs to a round that actually landed (applied
    // locally / adopted); a BLOCKED round must not claim conflicts it never wrote.
    state.lastMergeConflicts = result.mergeConflicts ?? null;
    notify();
    return result;
  };

  // 3a. The remote has not changed.
  if (remoteFile && !decision.remoteChanged) {
    if (!opts.overrideShrink && payloadFingerprint(localPayload) === payloadFingerprint(base)) {
      // Both ends identical → no-op (backfill a possibly missing anchor, in
      // case WebView data was cleared). A successful consistency check still
      // counts as a sync: refresh and persist the time so it survives restarts.
      if (!anchor) await saveAnchor(remoteFile, resourceId!);
      state.lastSyncAt = Date.now();
      state.lastSyncVersion = remoteFile.meta.version;
      state.remoteVersion = remoteFile.meta.version;
      persistSyncState();
      return finish({ success: true, action: "none", version: remoteFile.meta.version });
    }
    const result = await uploadPayload(localPayload, {
      overrideShrink: opts.overrideShrink,
      base,
      baseVersion: remoteFile.meta.version,
    });
    return finish(result);
  }

  // 3b. No remote file (first sync or deleted) → upload local directly.
  if (!remoteFile) {
    if (!hasLocalData(localPayload)) {
      return finish({ success: true, action: "none" });
    }
    const result = await uploadPayload(localPayload, {
      overrideShrink: opts.overrideShrink,
      base,
      baseVersion: anchor?.version ?? 0,
    });
    return finish(result);
  }

  // 3c. The remote changed → decrypt → merge.
  let remotePayload: SyncPayload;
  try {
    remotePayload = await decryptPayload(remoteFile, masterPassword!);
  } catch (decryptError) {
    // Different password (or the devices' passwords diverged) → conflict UI;
    // malformed files (missing meta etc.) land here too.
    conflictRemoteFile = remoteFile;
    state.syncing = false;
    state.syncState = "CONFLICT";
    state.conflict = {
      localVersion: anchor?.version ?? 0,
      localUpdatedAt: anchor?.updatedAt ?? 0,
      localDeviceName: device.deviceName,
      remoteVersion: remoteFile.meta?.version ?? 0,
      remoteUpdatedAt: remoteFile.meta?.updatedAt ?? 0,
      remoteDeviceName: remoteFile.meta?.deviceName,
    };
    notify();
    return {
      success: false,
      action: "none",
      conflictDetected: true,
      error: `decryption failed (the sync password may differ): ${decryptError instanceof Error ? decryptError.message : String(decryptError)}`,
    };
  }
  assertCurrent();

  // Content identical: adopt the remote identity (anchor/base) only, no upload.
  if (payloadFingerprint(remotePayload) === payloadFingerprint(localPayload)) {
    await adoptRemote(remotePayload, remoteFile, resourceId!, opts.applyLocal);
    return finish({ success: true, action: "none", version: remoteFile.meta.version });
  }

  // Empty-vault protection: no base + no local entities + remote has data →
  // wait for the user's confirmation.
  if (
    base === null &&
    localPayload.hosts.length === 0 &&
    localPayload.snippets.length === 0 &&
    localPayload.keys.length === 0 &&
    localPayload.identities.length === 0 &&
    !opts.confirmedEmptyVault &&
    (remotePayload.hosts.length > 0 ||
      remotePayload.snippets.length > 0 ||
      remotePayload.keys.length > 0 ||
      remotePayload.identities.length > 0)
  ) {
    state.syncing = false;
    state.syncState = "IDLE";
    state.emptyVaultPending = { payload: remotePayload, remoteFile };
    notify();
    return { success: false, action: "none", error: "empty-vault-guard" };
  }

  // Three-way merge.
  const merged = mergeSyncPayloads(base, localPayload, remotePayload);
  const mergedPayload = merged.payload;

  // Shrink guard: the merge result lost too much vs base → BLOCKED.
  const mergedShrink = detectSuspiciousShrink(mergedPayload, base, remotePayload);
  if (mergedShrink.suspicious && !opts.overrideShrink) {
    blockedRemoteFile = remoteFile;
    state.syncing = false;
    state.syncState = "BLOCKED";
    state.shrinkFinding = mergedShrink;
    notify();
    return { success: false, action: "none", shrinkBlocked: true, finding: mergedShrink };
  }

  // The merge equals the remote (nothing local-only) → adopt the remote and
  // skip a pointless upload.
  if (payloadFingerprint(mergedPayload) === payloadFingerprint(remotePayload)) {
    await adoptRemote(mergedPayload, remoteFile, resourceId!, opts.applyLocal);
    return finish({
      success: true,
      action: "download",
      version: remoteFile.meta.version,
      mergedPayload,
      mergeConflicts: merged.hadConflicts ? merged.summary.modified.conflicts : undefined,
    });
  }

  // Upload the merge (version = remote + 1), apply locally, commit base/anchor.
  const encrypted = await encryptPayload(
    mergedPayload,
    masterPassword!,
    device.deviceId,
    device.deviceName,
    appVersion,
    remoteFile.meta.version,
  );
  await gist.updateSyncGist(accessToken!, resourceId!, encrypted);
  assertCurrent();

  await opts.applyLocal(mergedPayload);
  markSkipHash(mergedPayload);
  commitSyncedPayload(mergedPayload, encrypted, resourceId!);

  return finish({
    success: true,
    action: "merge",
    version: encrypted.meta.version,
    mergedPayload,
    mergeConflicts: merged.hadConflicts ? merged.summary.modified.conflicts : undefined,
  });
}

function hasLocalData(payload: SyncPayload): boolean {
  return (
    payload.hosts.length > 0 ||
    payload.snippets.length > 0 ||
    payload.keys.length > 0 ||
    payload.identities.length > 0
  );
}

// ---------------------------------------------------------------------------
// Conflict / BLOCKED / empty-vault recovery actions
// ---------------------------------------------------------------------------

/**
 * Conflict resolution: use the remote (apply its data). A conflict means the
 * remote was encrypted with another device's password, so the user must type
 * the "cloud password" to decrypt it; on success this device's sync password
 * resets to that password (verificationHash updated) and syncs stay aligned.
 */
export async function resolveConflictUseRemote(
  cloudPassword: string,
  applyLocal: (p: SyncPayload) => Promise<void>,
): Promise<SyncResult> {
  const file = conflictRemoteFile;
  if (!file || !accessToken) {
    return { success: false, action: "none", error: "no conflicting remote data" };
  }
  let payload: SyncPayload;
  try {
    payload = await decryptPayload(file, cloudPassword);
  } catch {
    return { success: false, action: "none", error: "the cloud password is wrong; decryption failed" };
  }
  try {
    // Align with the cloud password: the next upload encrypts with it, so the
    // other device can keep decrypting.
    const config = await createMasterKeyConfig(cloudPassword);
    saveMasterKeyConfig(config);
    masterPassword = cloudPassword;
    // Update the remembered password too, or the next startup auto-unlock fails.
    await rememberPassword(cloudPassword);
    await adoptRemote(payload, file, state.gistId ?? "", applyLocal);
    state.conflict = null;
    conflictRemoteFile = null;
    state.syncState = "IDLE";
    state.lastError = null;
    notify();
    return { success: true, action: "download", version: file.meta.version };
  } catch (error) {
    return { success: false, action: "none", error: error instanceof Error ? error.message : String(error) };
  }
}

/** Conflict resolution: use local (force-push local data over the cloud). */
export async function resolveConflictUseLocal(): Promise<SyncResult> {
  const conflict = state.conflict;
  state.conflict = null;
  conflictRemoteFile = null;
  state.syncState = "IDLE";
  notify();
  const result = await forcePushLocal(conflict?.remoteVersion);
  return result;
}

/** BLOCKED recovery: restore the remote data to this device. */
export async function restoreRemoteFromBlocked(applyLocal: (p: SyncPayload) => Promise<void>): Promise<SyncResult> {
  const file = blockedRemoteFile;
  if (!file || !accessToken) {
    return { success: false, action: "none", error: "no remote data to restore" };
  }
  try {
    const payload = await decryptPayload(file, masterPassword!);
    await adoptRemote(payload, file, state.gistId ?? "", applyLocal);
    state.shrinkFinding = null;
    blockedRemoteFile = null;
    state.syncState = "IDLE";
    state.lastError = null;
    notify();
    return { success: true, action: "download", version: file.meta.version };
  } catch (error) {
    return { success: false, action: "none", error: error instanceof Error ? error.message : String(error) };
  }
}

/** Force-push local (skips the shrink guard; the empty-vault "push local"
 *  path lands here too). With no baseVersion given, prefer the last downloaded
 *  remote version so the cloud version number never regresses. */
export async function forcePushLocal(baseVersion?: number): Promise<SyncResult> {
  if (!accessToken || !masterPassword) {
    return { success: false, action: "none", error: "not connected or not unlocked" };
  }
  if (state.syncing) {
    return { success: false, action: "none", error: "a sync is already in progress" };
  }
  ++syncSeq;
  state.syncing = true;
  state.syncState = "SYNCING";
  notify();
  try {
    const anchor = loadAnchor();
    const resolvedBaseVersion = baseVersion ?? blockedRemoteFile?.meta.version ?? anchor?.version ?? 0;
    const payload = await buildSyncPayload(
      loadConfig().syncPasswords,
      loadConfig().syncCredentials,
    );
    const result = await uploadPayload(payload, {
      overrideShrink: true,
      base: loadBase(),
      baseVersion: resolvedBaseVersion,
    });
    state.syncing = false;
    if (result.success) {
      state.syncState = "IDLE";
      state.shrinkFinding = null;
      blockedRemoteFile = null;
      state.emptyVaultPending = null;
    } else {
      state.syncState = "ERROR";
      state.lastError = result.error ?? "push failed";
    }
    notify();
    return result;
  } catch (error) {
    state.syncing = false;
    state.syncState = "ERROR";
    state.lastError = error instanceof Error ? error.message : String(error);
    notify();
    return { success: false, action: "none", error: String(error) };
  }
}

/** Empty-vault confirmation: adopt the remote data to restore local. */
export async function confirmEmptyVaultRestore(applyLocal: (p: SyncPayload) => Promise<void>): Promise<SyncResult> {
  const pending = state.emptyVaultPending;
  if (!pending) {
    return { success: false, action: "none", error: "nothing to restore" };
  }
  state.emptyVaultPending = null;
  await adoptRemote(pending.payload, pending.remoteFile, state.gistId ?? "", applyLocal);
  state.syncState = "IDLE";
  notify();
  return { success: true, action: "download", version: pending.remoteFile.meta.version };
}

/** Empty-vault protection: push local instead (usually means the user cleared
 *  it on purpose). */
export async function confirmEmptyVaultPush(): Promise<SyncResult> {
  const pending = state.emptyVaultPending;
  state.emptyVaultPending = null;
  notify();
  return forcePushLocal(pending?.remoteFile.meta.version);
}

// ---------------------------------------------------------------------------
// History (browse + restore Gist revisions)
// ---------------------------------------------------------------------------

export interface RevisionPreview {
  sha: string;
  version: number;
  updatedAt: number;
  deviceName?: string;
  appVersion?: string;
  hosts: number;
  snippets: number;
}

/** Makes sure the Gist is usable (the engine's id, or a fresh cloud lookup). */
async function ensureGistId(): Promise<string | null> {
  if (!accessToken) return null;
  if (state.gistId) return state.gistId;
  const found = await gist.findSyncGist(accessToken);
  if (found) {
    state.gistId = found;
    saveConfig({ ...loadConfig(), gistId: found });
  }
  return found;
}

/** The revision history (newest first; the first entry is the cloud's current content). */
export async function fetchHistory(): Promise<Array<{ sha: string; date: number }>> {
  const gistId = await ensureGistId();
  if (!accessToken || !gistId) return [];
  const history = await gist.getGistHistory(accessToken, gistId);
  return history.map((h) => ({ sha: h.version, date: h.date.getTime() }));
}

/** Downloads and decrypts one revision, returning a summary preview (no restore). */
export async function previewRevision(sha: string): Promise<RevisionPreview> {
  if (!accessToken || !masterPassword) {
    throw new Error("connect GitHub and unlock the sync password first");
  }
  const gistId = await ensureGistId();
  if (!gistId) throw new Error("the sync Gist was not found");
  const file = await gist.downloadGistRevision(accessToken, gistId, sha);
  if (!file) throw new Error("this revision does not contain the sync file");
  let payload: SyncPayload;
  try {
    payload = await decryptPayload(file, masterPassword);
  } catch {
    // WebKit reports decryption failures as an opaque OperationError; phrase
    // it as something actionable.
    throw new Error(
      "decryption failed: this revision may have been uploaded with a different sync password (a password change only affects later uploads); it cannot be previewed or restored",
    );
  }
  return {
    sha,
    version: file.meta.version,
    updatedAt: file.meta.updatedAt,
    deviceName: file.meta.deviceName,
    appVersion: file.meta.appVersion,
    hosts: payload.hosts.length,
    snippets: payload.snippets.length,
  };
}

/**
 * Restores a chosen historical revision: applies the old data locally and
 * pushes it to the cloud as a **new version** (append-only restore, the Gist
 * history is never rewritten; other devices pull the restored content
 * automatically). A restore is an intentional overwrite, so the shrink guard
 * is skipped; requires unlocked state and no pending conflict/block.
 */
export async function restoreRevision(
  sha: string,
  applyLocal: (p: SyncPayload) => Promise<void>,
): Promise<SyncResult> {
  if (!accessToken || !masterPassword) {
    return { success: false, action: "none", error: "GitHub is not connected or the sync password is locked" };
  }
  if (state.syncing) {
    return { success: false, action: "none", error: "a sync is already in progress" };
  }
  if (state.syncState === "CONFLICT" || state.syncState === "BLOCKED") {
    return { success: false, action: "none", error: "resolve the current conflict/blocked state first" };
  }
  const gistId = await ensureGistId();
  if (!gistId) {
    return { success: false, action: "none", error: "the sync Gist was not found" };
  }

  ++syncSeq;
  state.syncing = true;
  state.syncState = "SYNCING";
  notify();
  try {
    const file = await gist.downloadGistRevision(accessToken, gistId, sha);
    if (!file) {
      throw new Error("this revision does not contain the sync file");
    }
    let payload: SyncPayload;
    try {
      payload = await decryptPayload(file, masterPassword);
    } catch {
      throw new Error(
        "decryption failed: this revision may have been uploaded with a different sync password (a password change only affects later uploads); it cannot be restored",
      );
    }

    // Push the restore as remote version + 1 (append-only, history untouched).
    // remoteVersion is memory-only: after a restart with a missing anchor
    // (rare), backfill it from the cloud meta so the version never regresses.
    const anchor = loadAnchor();
    let baseVersion = Math.max(state.remoteVersion ?? 0, anchor?.version ?? 0);
    if (baseVersion === 0) {
      const current = await gist.downloadSyncGist(accessToken, gistId);
      baseVersion = current?.meta.version ?? 0;
    }
    const encrypted = await encryptPayload(
      payload,
      masterPassword,
      device.deviceId,
      device.deviceName,
      appVersion,
      baseVersion,
    );
    await gist.updateSyncGist(accessToken, gistId, encrypted);

    await applyLocal(payload);
    markSkipHash(payload);
    commitSyncedPayload(payload, encrypted, gistId);

    state.syncing = false;
    state.syncState = "IDLE";
    state.shrinkFinding = null;
    notify();
    return { success: true, action: "upload", version: encrypted.meta.version };
  } catch (error) {
    state.syncing = false;
    state.syncState = "ERROR";
    state.lastError = error instanceof Error ? error.message : String(error);
    notify();
    return { success: false, action: "none", error: error instanceof Error ? error.message : String(error) };
  }
}

export function dismissEmptyVaultPrompt(): void {
  state.emptyVaultPending = null;
  notify();
}
