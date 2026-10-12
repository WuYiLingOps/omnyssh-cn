/**
 * Three-way merge (ported from DockPilot's sync engine, cut down to the two
 * OmnySSH entity kinds).
 *
 * With "the last successful sync's snapshot" as base, changes are detected per
 * entity key:
 *   - present on one side only          → an addition, kept
 *   - in base, deleted on one side      → deleted unless the other side modified it
 *                                         (keeping the modification is safer)
 *   - modified on one side only         → the modified side wins
 *   - modified on both sides            → local wins, counted as a conflict
 * With no base (first sync) it degenerates to a union keyed on the entity
 * key; duplicate keys prefer local.
 *
 * Merge keys: hosts by **name** (hosts.toml has no id field; the upsert
 * contract keys on name), snippets by **name** too (snippets.toml likewise —
 * a rename is delete-old + save-new by contract). There are no scalar
 * settings in Phase 1, so the whole scalar-merge block is gone.
 */

import type { SyncedHost, SyncedSnippet, SyncPayload } from "./types";

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

export interface MergeSummary {
  added: { local: number; remote: number };
  deleted: { local: number; remote: number };
  modified: { local: number; remote: number; conflicts: number };
}

export interface MergeResult {
  payload: SyncPayload;
  /** Whether a both-sides-modified conflict exists (resolved local-first). */
  hadConflicts: boolean;
  summary: MergeSummary;
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

/** Deterministic JSON serialisation (object keys sorted) for content-level
 *  comparison. */
export function fingerprint(value: unknown): string {
  return JSON.stringify(value, (_key, v) => {
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

interface EntityMergeResult<T> {
  merged: T[];
  conflicts: number;
  added: { local: number; remote: number };
  deleted: { local: number; remote: number };
  modified: { local: number; remote: number };
}

// ---------------------------------------------------------------------------
// Entity array merge (hosts by name, snippets by name)
// ---------------------------------------------------------------------------

function mergeEntityArrays<T>(base: T[], local: T[], remote: T[], keyOf: (e: T) => string): EntityMergeResult<T> {
  const baseMap = new Map(base.map((e) => [keyOf(e), e]));
  const localMap = new Map(local.map((e) => [keyOf(e), e]));
  const remoteMap = new Map(remote.map((e) => [keyOf(e), e]));

  const allKeys = new Set([...baseMap.keys(), ...localMap.keys(), ...remoteMap.keys()]);

  const merged: T[] = [];
  let conflicts = 0;
  const added = { local: 0, remote: 0 };
  const deleted = { local: 0, remote: 0 };
  const modified = { local: 0, remote: 0 };

  for (const key of allKeys) {
    const baseItem = baseMap.get(key);
    const localItem = localMap.get(key);
    const remoteItem = remoteMap.get(key);

    const inBase = baseItem !== undefined;
    const inLocal = localItem !== undefined;
    const inRemote = remoteItem !== undefined;

    if (!inBase && inLocal && !inRemote) {
      merged.push(localItem);
      added.local++;
    } else if (!inBase && !inLocal && inRemote) {
      merged.push(remoteItem);
      added.remote++;
    } else if (!inBase && inLocal && inRemote) {
      // Both sides added the same key — local wins.
      merged.push(localItem);
      if (fingerprint(localItem) !== fingerprint(remoteItem)) conflicts++;
    } else if (inBase && inLocal && inRemote) {
      const localChanged = fingerprint(localItem) !== fingerprint(baseItem);
      const remoteChanged = fingerprint(remoteItem) !== fingerprint(baseItem);

      if (!localChanged && !remoteChanged) {
        merged.push(baseItem);
      } else if (localChanged && !remoteChanged) {
        merged.push(localItem);
        modified.local++;
      } else if (!localChanged && remoteChanged) {
        merged.push(remoteItem);
        modified.remote++;
      } else {
        // Both modified — local wins.
        merged.push(localItem);
        if (fingerprint(localItem) !== fingerprint(remoteItem)) conflicts++;
        modified.local++;
        modified.remote++;
      }
    } else if (inBase && !inLocal && inRemote) {
      // Deleted locally: keep the remote modification if there was one
      // (deleting a remote edit would be a silent data loss).
      const remoteChanged = fingerprint(remoteItem) !== fingerprint(baseItem);
      if (remoteChanged) {
        merged.push(remoteItem);
        conflicts++;
      } else {
        deleted.local++;
      }
    } else if (inBase && inLocal && !inRemote) {
      // Deleted remotely: keep the local modification if there was one.
      const localChanged = fingerprint(localItem) !== fingerprint(baseItem);
      if (localChanged) {
        merged.push(localItem);
        conflicts++;
      } else {
        deleted.remote++;
      }
    }
    // inBase && !inLocal && !inRemote → deleted on both sides → dropped.
  }

  return { merged, conflicts, added, deleted, modified };
}

// ---------------------------------------------------------------------------
// Main entry
// ---------------------------------------------------------------------------

/**
 * Three-way merge of sync payloads.
 * @param base   The last successful sync's snapshot (null = first sync / snapshot lost)
 * @param local  This device's current data
 * @param remote The data downloaded from the cloud
 */
export function mergeSyncPayloads(
  base: SyncPayload | null,
  local: SyncPayload,
  remote: SyncPayload,
): MergeResult {
  const emptyBase: SyncPayload = { hosts: [], snippets: [], syncedAt: 0 };
  const b = base ?? emptyBase;

  // Hosts merge by name. A rename is the delete-old + add-new combination the
  // upsert contract implies: the old name merges by the deletion rules, the
  // new name by the addition rules.
  const hosts = mergeEntityArrays<SyncedHost>(
    b.hosts ?? [],
    local.hosts ?? [],
    remote.hosts ?? [],
    (h) => h.name,
  );
  const snippets = mergeEntityArrays<SyncedSnippet>(
    b.snippets ?? [],
    local.snippets ?? [],
    remote.snippets ?? [],
    (s) => s.name,
  );

  const summary: MergeSummary = {
    added: {
      local: hosts.added.local + snippets.added.local,
      remote: hosts.added.remote + snippets.added.remote,
    },
    deleted: {
      local: hosts.deleted.local + snippets.deleted.local,
      remote: hosts.deleted.remote + snippets.deleted.remote,
    },
    modified: {
      local: hosts.modified.local + snippets.modified.local,
      remote: hosts.modified.remote + snippets.modified.remote,
      conflicts: hosts.conflicts + snippets.conflicts,
    },
  };

  const payload: SyncPayload = {
    hosts: hosts.merged,
    snippets: snippets.merged,
    syncedAt: Date.now(),
  };

  return { payload, hadConflicts: summary.modified.conflicts > 0, summary };
}
