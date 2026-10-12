import { describe, expect, it } from "vitest";

import type { SyncedHost, SyncPayload } from "./types";
import { detectSuspiciousShrink } from "./guards";

const hosts = (n: number): SyncedHost[] =>
  Array.from({ length: n }, (_, i) => ({
    name: `h${i}`,
    hostname: `10.0.0.${i}`,
    user: "root",
    port: 22,
    tags: [],
    source: "manual" as const,
    identitiesOnly: false,
    keyId: '',
    identityId: '',
    monitoring: "ssh" as const,
    localForwards: [],
    tunnelAutostart: false,
    forwardAgent: false,
  }));

const payload = (hs: SyncedHost[], snippets: Array<{ name: string }> = []): SyncPayload => ({
  hosts: hs,
  snippets: snippets.map((s) => ({ name: s.name, command: "uptime", scope: "global" as const })),
  syncedAt: 0,
});

describe("shrink detection", () => {
  it("normal add/remove does not trigger", () => {
    const base = payload(hosts(10));
    expect(detectSuspiciousShrink(payload(hosts(9)), base)).toEqual({ suspicious: false });
    expect(detectSuspiciousShrink(payload(hosts(12)), base)).toEqual({ suspicious: false });
  });

  it("relative shrink ≥30% and ≥2 records is blocked", () => {
    const base = payload(hosts(10));
    const finding = detectSuspiciousShrink(payload(hosts(6)), base);
    expect(finding.suspicious).toBe(true);
    if (finding.suspicious) {
      expect(finding.reason).toBe("bulk-shrink");
      expect(finding.lost).toBe(4);
    }
    // Fewer than 2 lost records is not blocked (a normal single delete).
    expect(detectSuspiciousShrink(payload(hosts(9)), base)).toEqual({ suspicious: false });
  });

  it("absolute shrink ≥10 records is blocked outright (small ratio on a big vault)", () => {
    const base = payload(hosts(200));
    const finding = detectSuspiciousShrink(payload(hosts(188)), base);
    expect(finding.suspicious).toBe(true);
    if (finding.suspicious) expect(finding.reason).toBe("large-shrink");
  });

  it("snippets are checked independently", () => {
    const base = payload(hosts(10), Array.from({ length: 5 }, (_, i) => ({ name: `s${i}` })));
    const finding = detectSuspiciousShrink(payload(hosts(10), [{ name: "s0" }]), base);
    expect(finding.suspicious).toBe(true);
    if (finding.suspicious) expect(finding.entityType).toBe("snippets");
  });

  it("hosts are checked independently", () => {
    const base = payload(hosts(10));
    const finding = detectSuspiciousShrink(payload(hosts(2)), base);
    expect(finding.suspicious).toBe(true);
    if (finding.suspicious) expect(finding.entityType).toBe("hosts");
  });

  it("uses the remote as reference when base is missing, flagged viaRemote", () => {
    const remote = payload(hosts(10));
    const finding = detectSuspiciousShrink(payload(hosts(3)), null, remote);
    expect(finding.suspicious).toBe(true);
    if (finding.suspicious) {
      expect(finding.viaRemote).toBe(true);
      expect(finding.baseCount).toBe(10);
    }
  });

  it("neither base nor remote present → no block", () => {
    expect(detectSuspiciousShrink(payload([]), null, null)).toEqual({ suspicious: false });
  });

  it("a password-stripped payload is not a shrink (same record count)", () => {
    const base = payload(hosts(10));
    const stripped = payload(hosts(10)).hosts.map((h) => ({ ...h, password: null }));
    expect(detectSuspiciousShrink({ ...payload(hosts(10)), hosts: stripped }, base)).toEqual({
      suspicious: false,
    });
  });
});
