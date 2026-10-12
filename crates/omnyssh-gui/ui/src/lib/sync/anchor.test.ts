import { describe, expect, it } from "vitest";

import type { ProviderSyncAnchor, SyncedFile } from "./types";
import { createSyncedFileSignature, decideRemoteChanged } from "./anchor";

const anchor = (over: Partial<ProviderSyncAnchor> = {}): ProviderSyncAnchor => ({
  signature: "sig-1",
  version: 3,
  updatedAt: 100,
  resourceId: "gist-1",
  observedAt: 100,
  ...over,
});

const file = (over: Partial<SyncedFile> = {}): SyncedFile => ({
  meta: {
    version: 3,
    updatedAt: 100,
    deviceId: "d1",
    appVersion: "1.1.5",
    iv: "iv",
    salt: "salt",
    algorithm: "AES-256-GCM",
    kdf: "PBKDF2",
    kdfIterations: 600000,
  },
  payload: "cipher",
  ...over,
});

describe("anchor decision", () => {
  it("no anchor + no remote file → unchanged", () => {
    expect(decideRemoteChanged({ currentSignature: null, currentResourceId: null, anchor: null, hasRemoteFile: false })).toEqual({
      remoteChanged: false,
      reason: "no-anchor-no-remote",
    });
  });

  it("no anchor + remote has data → changed (first sync takes the merge path)", () => {
    expect(decideRemoteChanged({ currentSignature: "x", currentResourceId: "gist-1", anchor: null, hasRemoteFile: true }).remoteChanged).toBe(true);
  });

  it("no anchor + unreadable remote → changed (expose the error, never silent overwrite)", () => {
    expect(decideRemoteChanged({ currentSignature: null, currentResourceId: "gist-1", anchor: null, hasRemoteFile: true }).reason).toBe(
      "unreadable-remote",
    );
  });

  it("resourceId drift → changed", () => {
    const d = decideRemoteChanged({ currentSignature: "sig-1", currentResourceId: "gist-2", anchor: anchor(), hasRemoteFile: true });
    expect(d.remoteChanged).toBe(true);
    expect(d.reason).toBe("resource-id-changed");
  });

  it("signature mismatch → changed", () => {
    const d = decideRemoteChanged({ currentSignature: "sig-2", currentResourceId: "gist-1", anchor: anchor(), hasRemoteFile: true });
    expect(d.remoteChanged).toBe(true);
    expect(d.reason).toBe("signature-mismatch");
  });

  it("signature and resource both match → unchanged (no-op short-circuit)", () => {
    expect(decideRemoteChanged({ currentSignature: "sig-1", currentResourceId: "gist-1", anchor: anchor(), hasRemoteFile: true })).toEqual({
      remoteChanged: false,
      reason: "anchor-matches",
    });
  });
});

describe("cloud file signature", () => {
  it("is reproducible and changes with content", async () => {
    const a = await createSyncedFileSignature(file());
    const b = await createSyncedFileSignature(file());
    expect(a).toBe(b);
    const changed = await createSyncedFileSignature(file({ payload: "cipher-2" }));
    expect(changed).not.toBe(a);
    expect(await createSyncedFileSignature(null)).toBeNull();
  });
});
