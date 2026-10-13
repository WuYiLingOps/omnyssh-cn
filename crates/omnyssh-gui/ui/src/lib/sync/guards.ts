/**
 * Shrink guard (ported from DockPilot's sync engine, thresholds unchanged).
 *
 * Refuses to push a payload that "silently lost data": local data partially
 * missing (a secret-store failure, a half-load, a hydration race) makes the
 * merged/uploaded result far smaller than the base. With no base it falls
 * back to the current remote as the reference (first sync, snapshot loss).
 */

import type { SyncEntityType, SyncPayload, ShrinkFinding } from "./types";

/** Relative shrink: lost ≥30% and ≥2 entities. */
const BULK_SHRINK_RATIO = 0.3;
const BULK_SHRINK_MIN_ABSOLUTE = 2;
/** Absolute shrink: lost ≥10 blocks outright (catches small ratios on big vaults). */
const LARGE_SHRINK_ABSOLUTE = 10;

/** Passwords being stripped from hosts never counts as a shrink: the guard
 *  counts entity records, and the sync-passwords switch is a user decision.
 *  The same holds for the credential materials on keys/identities (PEM /
 *  passphrases / passwords) — the record count is what matters here. */
const CHECKED_ENTITIES: readonly SyncEntityType[] = ["hosts", "snippets", "keys", "identities"];

const countOf = (p: SyncPayload, key: SyncEntityType): number => {
  const v = p[key];
  return Array.isArray(v) ? v.length : 0;
};

export function detectSuspiciousShrink(
  outgoing: SyncPayload,
  base: SyncPayload | null,
  remote?: SyncPayload | null,
): ShrinkFinding {
  // Only fall back to the remote when the base is missing: a device that
  // legitimately outgrew a stale remote snapshot is unaffected.
  const reference = base ?? remote ?? null;
  const viaRemote = !base && !!remote;
  if (!reference) return { suspicious: false };

  for (const entityType of CHECKED_ENTITIES) {
    const baseCount = countOf(reference, entityType);
    const outgoingCount = countOf(outgoing, entityType);
    const lost = baseCount - outgoingCount;
    if (lost <= 0) continue;

    if (lost >= LARGE_SHRINK_ABSOLUTE) {
      return {
        suspicious: true,
        reason: "large-shrink",
        entityType,
        baseCount,
        outgoingCount,
        lost,
        ...(viaRemote ? { viaRemote: true } : {}),
      };
    }

    if (baseCount > 0 && lost / baseCount >= BULK_SHRINK_RATIO && lost >= BULK_SHRINK_MIN_ABSOLUTE) {
      return {
        suspicious: true,
        reason: "bulk-shrink",
        entityType,
        baseCount,
        outgoingCount,
        lost,
        ...(viaRemote ? { viaRemote: true } : {}),
      };
    }
  }

  return { suspicious: false };
}
