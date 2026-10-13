import { describe, expect, it } from "vitest";

import type { SyncPayload, SyncedFile } from "./types";
import {
  createMasterKeyConfig,
  decryptPayload,
  encryptPayload,
  unlockMasterKey,
  verifyPassword,
} from "./encryption";

const makePayload = (): SyncPayload => ({
  hosts: [
    {
      name: "local",
      hostname: "localhost",
      user: "root",
      port: 22,
      tags: ["本地"],
      source: "manual",
      identitiesOnly: false,
      keyId: '',
      identityId: '',
      keyFp: '',
      identityLabel: '',
      monitoring: "ssh" as const,
      localForwards: [],
      tunnelAutostart: false,
      forwardAgent: false,
    },
    {
      name: "srv",
      hostname: "1.2.3.4",
      user: "root",
      port: 2222,
      tags: ["服务器"],
      source: "manual",
      identitiesOnly: true,
      keyId: '',
      identityId: '',
      keyFp: '',
      identityLabel: '',
      monitoring: "ssh" as const,
      identityFile: "~/.ssh/id_ed25519",
      localForwards: [],
      tunnelAutostart: false,
      forwardAgent: false,
    },
  ],
  snippets: [{ name: "uptime", command: "uptime", scope: "global" }],
  keys: [],
  identities: [],
  syncedAt: 12345,
});

describe("encryption roundtrip", () => {
  it("encryptPayload → decryptPayload restores the original", async () => {
    const payload = makePayload();
    const file: SyncedFile = await encryptPayload(payload, "correct-password", "device-1", "test machine", "1.1.5", 5);
    expect(file.meta.version).toBe(6);
    expect(file.meta.algorithm).toBe("AES-256-GCM");
    expect(file.payload).not.toContain("correct-password");

    const decoded = await decryptPayload(file, "correct-password");
    expect(decoded).toEqual(payload);
  });

  it("decryption fails on a wrong password", async () => {
    const file = await encryptPayload(makePayload(), "password-a", "d", "n", "1.1.5");
    await expect(decryptPayload(file, "password-b")).rejects.toThrow();
  });

  it("decryption fails on tampered ciphertext (GCM auth tag)", async () => {
    const file = await encryptPayload(makePayload(), "pw", "d", "n", "1.1.5");
    const bytes = Uint8Array.from(atob(file.payload), (c) => c.charCodeAt(0));
    bytes[0] ^= 0xff;
    const tampered: SyncedFile = { ...file, payload: btoa(String.fromCharCode(...bytes)) };
    await expect(decryptPayload(tampered, "pw")).rejects.toThrow();
  });

  it("every encryption uses a fresh salt + IV (different ciphertext)", async () => {
    const payload = makePayload();
    const a = await encryptPayload(payload, "pw", "d", "n", "1.1.5", 0);
    const b = await encryptPayload(payload, "pw", "d", "n", "1.1.5", 0);
    expect(a.meta.salt).not.toBe(b.meta.salt);
    expect(a.meta.iv).not.toBe(b.meta.iv);
    expect(a.payload).not.toBe(b.payload);
  });
});

describe("master key config", () => {
  it("createMasterKeyConfig + unlockMasterKey roundtrip", async () => {
    const config = await createMasterKeyConfig("my-sync-password");
    expect(config.verificationHash).toBeTruthy();
    expect(JSON.stringify(config)).not.toContain("my-sync-password");

    const unlocked = await unlockMasterKey("my-sync-password", config);
    expect(unlocked).not.toBeNull();

    const wrong = await unlockMasterKey("wrong-password", config);
    expect(wrong).toBeNull();
  });

  it("verifyPassword checks correct / wrong passwords", async () => {
    const config = await createMasterKeyConfig("pw-123456");
    expect(await verifyPassword("pw-123456", config)).toBe(true);
    expect(await verifyPassword("pw-wrong", config)).toBe(false);
  });
});
