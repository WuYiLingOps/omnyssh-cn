/**
 * Sync payload build and apply.
 *
 * buildSyncPayload: hosts come from the Rust `sync_export_hosts` (manual
 * entries only; passwords included only when the user's `syncPasswords`
 * switch is on), snippets from `list_snippets`.
 *
 * applySyncPayload: `sync_apply_hosts` (atomic hosts.toml write, passwords
 * into the secret store) → `sync_apply_snippets` (atomic snippets.toml write)
 * → `reload_hosts` (refresh cache/pollers/tunnels + broadcast hosts-loaded).
 * Order matters: apply, then reload, so one event carries the final state.
 */

import { invoke } from "@tauri-apps/api/core";

import type { HostSyncDto, SnippetDto } from "$lib/bindings";
import { reloadHosts, listSnippets } from "$lib/ipc/commands";

import type { SyncPayload } from "./types";

/**
 * Builds the local upload payload. `includePasswords` mirrors the
 * `syncPasswords` switch: when on, login passwords ride the payload (the sync
 * password's envelope encryption protects them one layer up); when off, the
 * payload carries no secrets at all and the peer device prompts for passwords
 * as usual.
 */
export async function buildSyncPayload(includePasswords: boolean): Promise<SyncPayload> {
  const [hosts, snippets] = await Promise.all([
    invoke<HostSyncDto[]>("sync_export_hosts", { includePasswords }),
    listSnippets(),
  ]);
  return { hosts, snippets, syncedAt: Date.now() };
}

/**
 * Applies a merged / downloaded payload to this device, then reloads the host
 * cache and re-reads the snippet list so the UI reflects the new state.
 * `reloadHosts` also restarts the pollers and brings the tunnels in line.
 */
export async function applySyncPayload(payload: SyncPayload): Promise<void> {
  await invoke("sync_apply_hosts", { hosts: payload.hosts });
  await invoke("sync_apply_snippets", { snippets: payload.snippets });
  await reloadHosts();
  // The snippets store is event-free (no snippet-loaded broadcast): refresh it
  // directly so the Snippets screen and palette see the applied list.
  await refreshSnippetStore();
}

async function refreshSnippetStore(): Promise<void> {
  const { snippets } = await import("$lib/stores/snippets");
  snippets.set(await listSnippets());
}

/** Canonical payload fingerprint (strips syncedAt; null → ""), used for change
 *  detection and the skip hash. */
export function payloadFingerprint(payload: SyncPayload | null): string {
  if (!payload) return "";
  const { syncedAt: _syncedAt, ...rest } = payload;
  return JSON.stringify(rest, (_key, v) => {
    if (v && typeof v === "object" && !Array.isArray(v)) {
      return Object.keys(v)
        .sort()
        .reduce<Record<string, unknown>>((acc, k) => {
          acc[k] = (v as Record<string, unknown>)[k];
          return acc;
        }, {});
    }
    return v;
  });
}
