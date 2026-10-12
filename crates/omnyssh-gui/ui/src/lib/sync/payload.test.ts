import { beforeEach, describe, expect, it, vi } from "vitest";

import type { SyncedHost, SyncPayload } from "./types";

// The payload module talks to the backend via invoke and the ipc wrappers;
// mock both so the tests stay pure and assert the call contract.
const invokeMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));
const reloadHostsMock = vi.fn().mockResolvedValue(undefined);
const listSnippetsMock = vi.fn().mockResolvedValue([]);
vi.mock("$lib/ipc/commands", () => ({
  reloadHosts: (...args: unknown[]) => reloadHostsMock(...args),
  listSnippets: (...args: unknown[]) => listSnippetsMock(...args),
}));

const { buildSyncPayload, applySyncPayload, payloadFingerprint } = await import("./payload");

const host = (name: string, password?: string | null): SyncedHost => ({
  name,
  hostname: `${name}.example.com`,
  user: "root",
  port: 22,
  tags: [],
  source: "manual",
  identitiesOnly: false,
  keyId: '',
  identityId: '',
    monitoring: "ssh" as const,
  localForwards: [],
  tunnelAutostart: false,
  forwardAgent: false,
  ...(password === undefined ? {} : { password }),
});

describe("buildSyncPayload", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    listSnippetsMock.mockReset().mockResolvedValue([]);
  });

  it("passes the syncPasswords switch to sync_export_hosts and assembles the payload", async () => {
    invokeMock.mockResolvedValue([host("a")]);
    listSnippetsMock.mockResolvedValue([{ name: "s", command: "uptime", scope: "global" }]);

    const p = await buildSyncPayload(true);

    expect(invokeMock).toHaveBeenCalledWith("sync_export_hosts", { includePasswords: true });
    expect(p.hosts.map((h) => h.name)).toEqual(["a"]);
    expect(p.snippets.map((s) => s.name)).toEqual(["s"]);
    expect(typeof p.syncedAt).toBe("number");
  });

  it("with passwords off, export runs without secrets (the Rust side strips them)", async () => {
    invokeMock.mockResolvedValue([host("a")]);

    await buildSyncPayload(false);

    expect(invokeMock).toHaveBeenCalledWith("sync_export_hosts", { includePasswords: false });
  });
});

describe("applySyncPayload", () => {
  beforeEach(() => {
    invokeMock.mockReset().mockResolvedValue(null);
    reloadHostsMock.mockClear();
    listSnippetsMock.mockReset().mockResolvedValue([]);
  });

  it("applies hosts, then snippets, then reloads — in that order", async () => {
    const p: SyncPayload = {
      hosts: [host("a")],
      snippets: [{ name: "s", command: "uptime", scope: "global" }],
      syncedAt: 1,
    };
    const order: string[] = [];
    invokeMock.mockImplementation((cmd: string) => {
      order.push(cmd);
      return Promise.resolve(null);
    });
    reloadHostsMock.mockImplementation(() => {
      order.push("reload_hosts");
      return Promise.resolve();
    });
    listSnippetsMock.mockImplementation(() => {
      order.push("list_snippets");
      return Promise.resolve([]);
    });

    await applySyncPayload(p);

    expect(order).toEqual(["sync_apply_hosts", "sync_apply_snippets", "reload_hosts", "list_snippets"]);
    expect(invokeMock).toHaveBeenCalledWith("sync_apply_hosts", { hosts: p.hosts });
    expect(invokeMock).toHaveBeenCalledWith("sync_apply_snippets", { snippets: p.snippets });
  });

  it("a failing host apply aborts before snippets are touched", async () => {
    invokeMock.mockImplementation((cmd: string) =>
      cmd === "sync_apply_hosts" ? Promise.reject(new Error("boom")) : Promise.resolve(null),
    );

    await expect(
      applySyncPayload({ hosts: [host("a")], snippets: [], syncedAt: 1 }),
    ).rejects.toThrow("boom");
    expect(invokeMock).not.toHaveBeenCalledWith("sync_apply_snippets", expect.anything());
    expect(reloadHostsMock).not.toHaveBeenCalled();
  });
});

describe("payloadFingerprint", () => {
  it("ignores syncedAt and key order; distinguishes content", () => {
    const a: SyncPayload = { hosts: [host("a")], snippets: [], syncedAt: 1 };
    const b: SyncPayload = { hosts: [host("a")], snippets: [], syncedAt: 999 };
    const c: SyncPayload = { hosts: [host("a", "secret")], snippets: [], syncedAt: 1 };

    expect(payloadFingerprint(a)).toBe(payloadFingerprint(b));
    expect(payloadFingerprint(a)).not.toBe(payloadFingerprint(c));
  });

  it("null fingerprints to the empty string", () => {
    expect(payloadFingerprint(null)).toBe("");
  });
});
