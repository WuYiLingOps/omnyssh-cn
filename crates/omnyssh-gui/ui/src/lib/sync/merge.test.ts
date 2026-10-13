import { describe, expect, it } from "vitest";

import type { SyncedHost, SyncedKey, SyncedIdentity, SyncPayload } from "./types";
import { fingerprint, mergeSyncPayloads } from "./merge";

const host = (name: string, hostname = `${name}.example.com`): SyncedHost => ({
  name,
  hostname,
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
  keyFp: '',
  identityLabel: '',
});

const key = (fp: string, label = 'key-' + fp.slice(-4)): SyncedKey => ({
  keyFp: fp,
  label,
  publicKey: 'ssh-ed25519 AAA',
  createdAt: 0,
});

const identity = (label: string, keyFp = ''): SyncedIdentity => ({
  label,
  username: 'root',
  keyFp,
  createdAt: 0,
});

const payload = (
  hs: SyncedHost[],
  snippetNames: string[] = [],
  keys: SyncedKey[] = [],
  identities: SyncedIdentity[] = [],
): SyncPayload => ({
  hosts: hs,
  snippets: snippetNames.map((name) => ({ name, command: "uptime", scope: "global" as const })),
  keys,
  identities,
  syncedAt: 0,
});

describe("three-way host merge (by name)", () => {
  it("single-side additions: both sides' additions survive", () => {
    const base = payload([host("a")]);
    const local = payload([host("a"), host("b-local")]);
    const remote = payload([host("a"), host("c-remote")]);

    const r = mergeSyncPayloads(base, local, remote);
    expect(r.payload.hosts.map((h) => h.name).sort()).toEqual(["a", "b-local", "c-remote"]);
    expect(r.summary.added).toEqual({ local: 1, remote: 1 });
    expect(r.hadConflicts).toBe(false);
  });

  it("no base (first sync): union by name, duplicate names prefer local", () => {
    const local = payload([host("a", "local-a.example.com"), host("b")]);
    const remote = payload([host("a", "remote-a.example.com"), host("c")]);

    const r = mergeSyncPayloads(null, local, remote);
    expect(r.payload.hosts.map((h) => h.name).sort()).toEqual(["a", "b", "c"]);
    expect(r.payload.hosts.find((h) => h.name === "a")?.hostname).toBe("local-a.example.com");
    expect(r.hadConflicts).toBe(true); // both sides added "a" with different content
  });

  it("single-side modification: the modified side wins", () => {
    const base = payload([host("a"), host("b")]);
    const local = payload([host("a", "new-a.example.com"), host("b")]);
    const remote = payload([host("a"), host("b", "new-b.example.com")]);

    const r = mergeSyncPayloads(base, local, remote);
    expect(r.payload.hosts.find((h) => h.name === "a")?.hostname).toBe("new-a.example.com");
    expect(r.payload.hosts.find((h) => h.name === "b")?.hostname).toBe("new-b.example.com");
    expect(r.hadConflicts).toBe(false);
    expect(r.summary.modified.local).toBe(1);
    expect(r.summary.modified.remote).toBe(1);
  });

  it("both modified: local wins and the conflict is counted", () => {
    const base = payload([host("a")]);
    const local = payload([host("a", "local-a.example.com")]);
    const remote = payload([host("a", "remote-a.example.com")]);

    const r = mergeSyncPayloads(base, local, remote);
    expect(r.payload.hosts[0].hostname).toBe("local-a.example.com");
    expect(r.hadConflicts).toBe(true);
  });

  it("local delete + untouched remote → deleted; remote edited → the edit survives", () => {
    const base = payload([host("a"), host("b")]);
    // Local deleted b; remote left b untouched.
    const local = payload([host("a")]);
    const remote = payload([host("a"), host("b")]);
    const r1 = mergeSyncPayloads(base, local, remote);
    expect(r1.payload.hosts.map((h) => h.name)).toEqual(["a"]);

    // Local deleted b; remote edited b → keep the edit (safer).
    const remote2 = payload([host("a"), host("b", "edited-b.example.com")]);
    const r2 = mergeSyncPayloads(base, local, remote2);
    expect(r2.payload.hosts.map((h) => h.name).sort()).toEqual(["a", "b"]);
    expect(r2.payload.hosts.find((h) => h.name === "b")?.hostname).toBe("edited-b.example.com");
    expect(r2.hadConflicts).toBe(true);
  });

  it("deleted on both sides → removed", () => {
    const base = payload([host("a"), host("b")]);
    const local = payload([host("a")]);
    const remote = payload([host("a")]);
    const r = mergeSyncPayloads(base, local, remote);
    expect(r.payload.hosts.map((h) => h.name)).toEqual(["a"]);
  });

  it("a rename merges as delete-old + add-new per side", () => {
    // Local renamed a → a2; remote untouched. Expectation: "a" is deleted
    // (untouched remotely), "a2" is added locally, carrying the same content.
    const base = payload([host("a"), host("keep")]);
    const local = payload([host("a2"), host("keep")]);
    const remote = payload([host("a"), host("keep")]);

    const r = mergeSyncPayloads(base, local, remote);
    expect(r.payload.hosts.map((h) => h.name).sort()).toEqual(["a2", "keep"]);
    expect(r.summary.added.local).toBe(1);
    expect(r.summary.deleted.local).toBe(1);
    expect(r.hadConflicts).toBe(false);
    expect(r.payload.hosts.find((h) => h.name === "a2")?.hostname).toBe("a2.example.com");
  });

  it("a rename colliding with a remote edit keeps the remote edit under the old name", () => {
    // Local renamed a → a2; remote edited a in place. The deletion of "a"
    // meets a remote modification → the modified remote entry survives, and
    // the local "a2" is added. Both survive; the conflict is counted.
    const base = payload([host("a")]);
    const local = payload([host("a2")]);
    const remote = payload([host("a", "edited-a.example.com")]);

    const r = mergeSyncPayloads(base, local, remote);
    expect(r.payload.hosts.map((h) => h.name).sort()).toEqual(["a", "a2"]);
    expect(r.payload.hosts.find((h) => h.name === "a")?.hostname).toBe("edited-a.example.com");
    expect(r.hadConflicts).toBe(true);
  });

  it("an imported SSH-config host must never enter the merge payload (guarded upstream)", () => {
    // Imports are filtered out by sync_export_hosts; this pins that a payload
    // entry with source sshConfig would still merge by name if one ever leaked.
    const leaked = { ...host("imp"), source: "sshConfig" as const };
    const base = payload([host("keep")]);
    const local = payload([host("keep"), leaked]);
    const remote = payload([host("keep")]);

    const r = mergeSyncPayloads(base, local, remote);
    expect(r.payload.hosts.map((h) => h.name)).toEqual(["keep", "imp"]);
    expect(r.summary.added.local).toBe(1);
  });
});

describe("three-way snippet merge (by name)", () => {
  it("single-side additions and deletions follow the same rules", () => {
    const base = payload([], ["s1", "s2"]);
    const local = payload([], ["s1", "s2", "s3"]); // added s3
    const remote = payload([], ["s1"]); // deleted s2

    const r = mergeSyncPayloads(base, local, remote);
    expect(r.payload.snippets.map((s) => s.name).sort()).toEqual(["s1", "s3"]);
    expect(r.summary.added.local).toBe(1);
    expect(r.summary.deleted.remote).toBe(1);
  });

  it("both sides modified the same snippet → local wins", () => {
    const base = payload([], ["s1"]);
    const local = payload([], ["s1"]);
    const remote = payload([], ["s1"]);
    local.snippets[0].command = "local-cmd";
    remote.snippets[0].command = "remote-cmd";

    const r = mergeSyncPayloads(base, local, remote);
    expect(r.payload.snippets[0].command).toBe("local-cmd");
    expect(r.hadConflicts).toBe(true);
  });
});

describe("fingerprint", () => {
  it("key order does not affect the fingerprint", () => {
    expect(fingerprint({ a: 1, b: { c: 2, d: 3 } })).toBe(fingerprint({ b: { d: 3, c: 2 }, a: 1 }));
    expect(fingerprint({ a: 1 })).not.toBe(fingerprint({ a: 2 }));
  });
});

describe('three-way key + identity merge', () => {
  it('keys merge by fingerprint; a local-only and a remote-only key both survive', () => {
    const base = payload([], [], [key('fp-a')]);
    const local = payload([], [], [key('fp-a'), key('fp-local')]);
    const remote = payload([], [], [key('fp-a'), key('fp-remote')]);

    const r = mergeSyncPayloads(base, local, remote);
    expect(r.payload.keys.map((k) => k.keyFp).sort()).toEqual(['fp-a', 'fp-local', 'fp-remote']);
  });

  it('a key modified on one side wins; both-side modification resolves local-first', () => {
    const base = payload([], [], [{ keyFp: 'fp', label: 'old', publicKey: 'p', createdAt: 0 }]);
    const local = payload([], [], [{ keyFp: 'fp', label: 'local-label', publicKey: 'p', createdAt: 0 }]);
    const remote = payload([], [], [{ keyFp: 'fp', label: 'remote-label', publicKey: 'p', createdAt: 0 }]);

    const r = mergeSyncPayloads(base, local, remote);
    expect(r.payload.keys[0].label).toBe('local-label');
    expect(r.hadConflicts).toBe(true);
  });

  it('a key deleted locally but unchanged remotely is deleted', () => {
    const base = payload([], [], [key('fp-a'), key('fp-b')]);
    const local = payload([], [], [key('fp-a')]);
    const remote = payload([], [], [key('fp-a'), key('fp-b')]);

    const r = mergeSyncPayloads(base, local, remote);
    expect(r.payload.keys.map((k) => k.keyFp)).toEqual(['fp-a']);
  });

  it('identities merge by label; binding changes propagate', () => {
    const base = payload([], [], [], [identity('vm')]);
    const local = payload([], [], [key('fp-1')], [{ ...identity('vm', 'fp-1') }]);
    const remote = payload([], [], [], [identity('vm')]);

    const r = mergeSyncPayloads(base, local, remote);
    expect(r.payload.identities).toHaveLength(1);
    expect(r.payload.identities[0].keyFp).toBe('fp-1');
  });
});
