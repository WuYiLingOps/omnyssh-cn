/**
 * Anchor decision (ported from DockPilot's sync engine, itself from Netcatty).
 *
 * "Has the remote changed since it was last observed?" — four key verdicts:
 *   1. no anchor + no remote file  → unchanged (nothing to merge on first sync)
 *   2. no anchor + remote has data → changed (three-way merge with an empty base)
 *   3. resourceId drifted          → changed (the Gist was rebuilt; the old anchor is void)
 *   4. signature mismatch          → changed (new ciphertext for the same file)
 * Everything else is unchanged and the caller short-circuits.
 */

import type { ProviderSyncAnchor, SyncedFile } from "./types";
import { arrayBufferToBase64, sha256 } from "./encryption";

export interface RemoteChangedDecision {
  remoteChanged: boolean;
  reason: string;
}

export function decideRemoteChanged(input: {
  currentSignature: string | null;
  currentResourceId: string | null;
  anchor: ProviderSyncAnchor | null;
  hasRemoteFile: boolean;
}): RemoteChangedDecision {
  const { currentSignature, currentResourceId, anchor, hasRemoteFile } = input;

  if (!anchor) {
    if (!hasRemoteFile) {
      return { remoteChanged: false, reason: "no-anchor-no-remote" };
    }
    if (currentSignature === null) {
      // The file exists but no signature can be computed (malformed structure /
      // new schema) → treat as changed, routing through the decrypt-merge path
      // to surface the error to the user rather than silently overwriting.
      return { remoteChanged: true, reason: "unreadable-remote" };
    }
    return { remoteChanged: true, reason: "no-anchor-remote-has-data" };
  }

  // Resource drift: the Gist was rebuilt, the old anchor signature is void.
  if ((anchor.resourceId ?? null) !== currentResourceId) {
    return { remoteChanged: true, reason: "resource-id-changed" };
  }

  // Same resource, different signature → new ciphertext / new meta.
  if ((anchor.signature ?? null) !== currentSignature) {
    return { remoteChanged: true, reason: "signature-mismatch" };
  }

  return { remoteChanged: false, reason: "anchor-matches" };
}

/**
 * Cloud file signature: Base64(SHA-256(JSON(meta + ciphertext))).
 * The meta carries iv/salt/version, so any real upload changes the signature.
 */
export async function createSyncedFileSignature(syncedFile: SyncedFile | null): Promise<string | null> {
  if (!syncedFile) return null;
  const bytes = new TextEncoder().encode(JSON.stringify({ meta: syncedFile.meta, payload: syncedFile.payload }));
  return arrayBufferToBase64(await sha256(bytes));
}
