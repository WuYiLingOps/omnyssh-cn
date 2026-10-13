/**
 * Sync payload build and apply.
 *
 * buildSyncPayload: hosts from the Rust `sync_export_hosts` (manual entries
 * only; passwords included only when the user's `syncPasswords` switch is on),
 * snippets from `list_snippets`, identity metadata from `sync_export_identities`
 * (materials included only when `syncCredentials` is on).
 *
 * applySyncPayload: `sync_apply_identities` first (so hosts' key pins can
 * resolve against the freshly landed identity graph) → `sync_apply_hosts`
 * (atomic hosts.toml write, passwords into the secret store, pins translated)
 * → `sync_apply_snippets` → `reload_hosts`. Order matters: identities before
 * hosts, apply, then reload, so one event carries the final state.
 */

import { invoke } from "@tauri-apps/api/core";

import type { HostSyncDto, SnippetDto } from "$lib/bindings";
import { reloadHosts, listSnippets } from "$lib/ipc/commands";

import type { SyncPayload } from "./types";

/**
 * Builds the local upload payload. `includePasswords` mirrors the
 * `syncPasswords` switch; `includeCredentials` mirrors `syncCredentials` —
 * both gate secret materials only, identity metadata always rides the payload.
 */
export async function buildSyncPayload(
  includePasswords: boolean,
  includeCredentials: boolean,
): Promise<SyncPayload> {
  const [hosts, snippets, identities] = await Promise.all([
    invoke<HostSyncDto[]>("sync_export_hosts", { includePasswords }),
    listSnippets(),
    invoke<{ 0: import("$lib/bindings").KeySyncDto[]; 1: import("$lib/bindings").IdentitySyncDto[] }>(
      "sync_export_identities",
      { includeCredentials },
    ),
  ]);
  return {
    hosts,
    snippets,
    keys: identities[0],
    identities: identities[1],
    syncedAt: Date.now(),
  };
}

/**
 * Applies a merged / downloaded payload to this device, then reloads the host
 * cache and re-reads the snippet list so the UI reflects the new state.
 * `reloadHosts` also restarts the pollers and brings the tunnels in line.
 */
export async function applySyncPayload(payload: SyncPayload): Promise<void> {
  await invoke("sync_apply_identities", {
    keys: payload.keys,
    identitiesPayload: payload.identities,
  });
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
