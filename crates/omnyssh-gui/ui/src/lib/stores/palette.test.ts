import { beforeEach, describe, expect, it } from 'vitest';
import { get } from 'svelte/store';
import type { ConnectionStatusDto, HostDto } from '$lib/bindings';
import type { Session } from './sessions';
import {
  palette,
  paletteItems,
  paletteSignature,
  nextIndex,
  hostStatusDot,
  groupHostsByTag,
  filterByTag,
  tagChips,
  visibleGroups,
  queryHosts
} from './palette';

// The palette filters a command by its *translated* label, so the tests pass a stub that
// maps the one command key to the English text they search for.
const translate = (key: string): string =>
  key === 'palette.commandToggleStreamer' ? 'Toggle streamer mode' : key;


function host(name: string, extra: Partial<HostDto> = {}): HostDto {
  return {
    name,
    hostname: `${name}.example.com`,
    user: 'deploy',
    port: 22,
    tags: [],
    source: 'manual',
    hasKey: false,
    monitoring: 'ssh',
    localForwards: [],
    tunnelAutostart: false,
    forwardAgent: false,
    ...extra
  };
}

function session(id: number, hostName: string, kind: Session['kind'] = 'terminal'): Session {
  return { id, kind, hostName, status: 'connecting' };
}

describe('paletteItems — filter & sections', () => {
  const hosts = [host('web-1', { tags: ['prod'] }), host('db-1', { user: 'root' }), host('web-2')];
  const sessions = [session(1, 'web-1', 'terminal'), session(2, 'db-1', 'sftp')];

  it('navigator lists sessions first, then hosts, then commands', () => {
    const items = paletteItems('navigate', hosts, sessions, '', translate);
    expect(items.map((i) => i.kind)).toEqual(['session', 'session', 'host', 'host', 'host', 'command']);
  });

  it('matches the streamer command by its label, in the navigator only', () => {
    const toggle = { kind: 'command', id: 'toggle-streamer', labelKey: 'palette.commandToggleStreamer' };
    expect(paletteItems('navigate', hosts, sessions, 'streamer', translate)).toEqual([toggle]);
    expect(paletteItems('navigate', hosts, sessions, 'toggle mode', translate)).toEqual([toggle]);
    expect(paletteItems('navigate', hosts, sessions, 'web', translate)).not.toContainEqual(toggle);
    expect(paletteItems('pickHost', hosts, sessions, 'streamer', translate)).toEqual([]);
  });

  it('picker lists only hosts', () => {
    const items = paletteItems('pickHost', hosts, sessions, '', translate);
    expect(items.every((i) => i.kind === 'host')).toBe(true);
    expect(items).toHaveLength(3);
  });

  it('filters hosts by name, hostname, user and tags', () => {
    expect(paletteItems('pickHost', hosts, [], 'web', translate).map((i) => i.kind === 'host' && i.host.name))
      .toEqual(['web-1', 'web-2']);
    expect(paletteItems('pickHost', hosts, [], 'root', translate)).toHaveLength(1); // db-1 by user
    expect(paletteItems('pickHost', hosts, [], 'prod', translate)).toHaveLength(1); // web-1 by tag
    expect(paletteItems('pickHost', hosts, [], 'example.com', translate)).toHaveLength(3); // hostname
  });

  it('requires every whitespace-separated token to match (AND)', () => {
    expect(paletteItems('pickHost', hosts, [], 'web prod', translate).map((i) => i.kind === 'host' && i.host.name))
      .toEqual(['web-1']);
    expect(paletteItems('pickHost', hosts, [], 'web root', translate)).toHaveLength(0);
  });

  it('filters sessions by host name and kind in the navigator', () => {
    expect(paletteItems('navigate', [], sessions, 'sftp', translate).map((i) => i.kind === 'session' && i.session.id))
      .toEqual([2]);
    expect(paletteItems('navigate', [], sessions, 'web', translate)).toHaveLength(1);
  });

  it('an empty query keeps everything', () => {
    expect(paletteItems('navigate', hosts, sessions, '   ', translate)).toHaveLength(6);
  });

  it('lists a shared host name once, as its first host, whatever the query', () => {
    const twins = [host('web-1'), host('web-1', { hostname: 'twin.example.com' })];
    expect(paletteItems('pickHost', twins, [], '', translate)).toEqual([{ kind: 'host', host: twins[0] }]);
    expect(paletteItems('navigate', twins, [], 'twin', translate)).toHaveLength(0);
  });
});

describe('paletteSignature — stable across volatile updates', () => {
  it('is identical when only a session status changes (no spurious highlight reset)', () => {
    const connecting = paletteItems('navigate', [host('web-1')], [session(1, 'web-1')], '', translate);
    const connected = paletteItems(
      'navigate',
      [host('web-1')],
      [{ ...session(1, 'web-1'), status: 'connected' }],
      '',
      translate
    );
    expect(paletteSignature(connecting)).toBe(paletteSignature(connected));
  });

  it('changes when a row is added, removed, or reordered', () => {
    const base = paletteItems('navigate', [host('web-1'), host('web-2')], [], '', translate);
    expect(paletteSignature(base)).not.toBe(
      paletteSignature(paletteItems('navigate', [host('web-1')], [], '', translate))
    );
    expect(paletteSignature(base)).not.toBe(
      paletteSignature(paletteItems('navigate', [host('web-2'), host('web-1')], [], '', translate))
    );
  });

  it('changes when a session row appears', () => {
    const withoutSession = paletteItems('navigate', [host('web-1')], [], '', translate);
    const withSession = paletteItems('navigate', [host('web-1')], [session(1, 'web-1')], '', translate);
    expect(paletteSignature(withoutSession)).not.toBe(paletteSignature(withSession));
  });

  it('changes when the query drops the command row', () => {
    const all = paletteItems('navigate', [host('web-1')], [], '', translate);
    const hostOnly = paletteItems('navigate', [host('web-1')], [], 'web', translate);
    expect(paletteSignature(all)).toBe(`h:web-1\u0000c:toggle-streamer`);
    expect(paletteSignature(hostOnly)).toBe('h:web-1');
  });
});

describe('tag-organized picker — groups, chips, folding', () => {
  const tagged = [
    host('web-1', { tags: ['prod', 'web'] }),
    host('web-2', { tags: ['prod'] }),
    host('db-1', { tags: [' db '] }), // blank-padded tags still count, trimmed
    host('cache-1', { tags: ['prod', 'db'] }),
    host('bare-1'),
    host('bare-2', { tags: [' ', ''] }) // blank-only tags are untagged
  ];

  it('groups hosts under their first non-blank tag, Untagged last, name-sorted', () => {
    expect(groupHostsByTag(tagged).map((g) => [g.tag, g.hosts.map((h) => h.name)])).toEqual([
      ['db', ['db-1']],
      ['prod', ['web-1', 'web-2', 'cache-1']],
      [null, ['bare-1', 'bare-2']]
    ]);
  });

  it('keeps the input order inside a section', () => {
    const two = [host('b', { tags: ['x'] }), host('a', { tags: ['x'] })];
    expect(groupHostsByTag(two)[0].hosts.map((h) => h.name)).toEqual(['b', 'a']);
  });

  it('a tag chip keeps hosts carrying the tag anywhere, untagged keeps the bare ones', () => {
    expect(filterByTag(tagged, 'db').map((h) => h.name)).toEqual(['db-1', 'cache-1']);
    expect(filterByTag(tagged, 'web').map((h) => h.name)).toEqual(['web-1']);
    expect(filterByTag(tagged, null).map((h) => h.name)).toEqual(['bare-1', 'bare-2']);
    expect(filterByTag(tagged, undefined)).toHaveLength(6);
  });

  it('chips count every tag a host carries; untagged trails only when present', () => {
    expect(tagChips(tagged)).toEqual([
      { kind: 'all', count: 6 },
      { kind: 'tag', tag: 'db', count: 2 },
      { kind: 'tag', tag: 'prod', count: 3 },
      { kind: 'tag', tag: 'web', count: 1 },
      { kind: 'untagged', count: 2 }
    ]);
    expect(tagChips([host('solo')])).toEqual([
      { kind: 'all', count: 1 },
      { kind: 'untagged', count: 1 }
    ]);
    expect(tagChips([])).toEqual([{ kind: 'all', count: 0 }]);
  });

  it('visibleGroups empties the folded sections but keeps their headers', () => {
    const groups = groupHostsByTag(tagged);
    const shown = visibleGroups(groups, ['prod', ''], false);
    expect(shown.map((g) => [g.tag, g.hosts.length])).toEqual([
      ['db', 1],
      ['prod', 0],
      [null, 0]
    ]);
  });

  it('a search unfolds every group', () => {
    const groups = groupHostsByTag(tagged);
    expect(visibleGroups(groups, ['prod', ''], true)).toEqual(groups);
  });

  it('the composed list equals the flat palette rows for the same inputs', () => {
    const query = 'web';
    // Nothing folded: folding only removes rows, it never changes what matches.
    const grouped = visibleGroups(
      groupHostsByTag(filterByTag(queryHosts(tagged, query), 'prod')),
      [],
      false
    );
    const groupedRows = grouped.flatMap((g) => g.hosts);
    const flat = paletteItems('pickHost', tagged, [], query, translate);
    expect(groupedRows).toEqual(flat.map((i) => (i.kind === 'host' ? i.host : null)).filter(Boolean));
  });
});

describe('nextIndex — wrapping selection', () => {
  it('moves forward and wraps past the end', () => {
    expect(nextIndex(0, 1, 3)).toBe(1);
    expect(nextIndex(2, 1, 3)).toBe(0);
  });

  it('moves back and wraps past the start', () => {
    expect(nextIndex(0, -1, 3)).toBe(2);
    expect(nextIndex(1, -1, 3)).toBe(0);
  });

  it('stays at 0 for an empty list', () => {
    expect(nextIndex(0, 1, 0)).toBe(0);
    expect(nextIndex(0, -1, 0)).toBe(0);
  });
});

describe('hostStatusDot — connection state → dot', () => {
  const dot = (status: ConnectionStatusDto | undefined) => hostStatusDot(status);
  it('connected is ok, failed is offline, everything else neutral', () => {
    expect(dot({ kind: 'connected' })).toBe('ok');
    expect(dot({ kind: 'failed', message: 'x' })).toBe('off');
    expect(dot({ kind: 'connecting' })).toBe('unknown');
    expect(dot({ kind: 'unknown' })).toBe('unknown');
    expect(dot(undefined)).toBe('unknown');
  });
});

describe('palette store — modes & picker resolution', () => {
  beforeEach(() => palette.close());

  it('open() shows the navigator', () => {
    palette.open();
    expect(get(palette)).toEqual({ open: true, mode: 'navigate' });
  });

  it('pickHost() resolves with the chosen host and closes', async () => {
    const pending = palette.pickHost();
    expect(get(palette)).toEqual({ open: true, mode: 'pickHost' });
    const chosen = host('web-1');
    palette.choose(chosen);
    await expect(pending).resolves.toEqual(chosen);
    expect(get(palette)).toEqual({ open: false, mode: 'navigate' });
  });

  it('closing a pending picker resolves null', async () => {
    const pending = palette.pickHost();
    palette.close();
    await expect(pending).resolves.toBeNull();
  });

  it('opening the navigator cancels a pending picker (resolves null)', async () => {
    const pending = palette.pickHost();
    palette.open();
    await expect(pending).resolves.toBeNull();
    expect(get(palette)).toEqual({ open: true, mode: 'navigate' });
  });
});
