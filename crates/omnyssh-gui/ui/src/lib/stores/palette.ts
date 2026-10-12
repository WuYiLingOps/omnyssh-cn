import { writable } from 'svelte/store';
import type { ConnectionStatusDto, HostDto } from '$lib/bindings';
import type { Status } from '$lib/theme';
import { firstOfEachName } from './hosts';
import type { Session } from './sessions';

// The ⌘K overlay and the host-picker are one component in two modes (tech-gui.md §2):
// `navigate` lists open sessions + hosts (jump to a session, or open a host); `pickHost`
// is scoped to "pick a host for this action" and hands the choice back to its caller.
export type PaletteMode = 'navigate' | 'pickHost';

// A selectable row. Sessions and commands surface only in the navigator; the picker is
// host-only.
export type PaletteItem =
  | { kind: 'session'; session: Session }
  | { kind: 'host'; host: HostDto }
  | CommandItem;

// A command row is a built-in action (there is one today). `labelKey` is a message key,
// not text: the palette translates it to show *and* to match a query against, so a
// localized palette still filters on the words the user actually sees.
type CommandItem = { kind: 'command'; id: 'toggle-streamer'; labelKey: string };

const COMMANDS: CommandItem[] = [
  { kind: 'command', id: 'toggle-streamer', labelKey: 'palette.commandToggleStreamer' }
];

function hostHaystack(h: HostDto): string {
  return `${h.name} ${h.hostname} ${h.user} ${h.tags.join(' ')}`.toLowerCase();
}

function sessionHaystack(s: Session): string {
  return `${s.hostName} ${s.kind}`.toLowerCase();
}

// All whitespace-separated tokens must appear (AND), so "web prod" narrows to a host
// tagged prod named web-* — an empty query keeps everything.
function matches(haystack: string, query: string): boolean {
  return query
    .toLowerCase()
    .split(/\s+/)
    .filter(Boolean)
    .every((token) => haystack.includes(token));
}

/** The picker's host rows for a query: one per name (the first), query-filtered. The
 *  grouped list below composes its rows from this too, so the two never drift. */
export function queryHosts(hosts: HostDto[], query: string): HostDto[] {
  return firstOfEachName(hosts).filter((h) => matches(hostHaystack(h), query));
}

export function querySessions(sessions: Session[], query: string): Session[] {
  return sessions.filter((s) => matches(sessionHaystack(s), query));
}

export function queryCommands(query: string, translate: (key: string) => string): CommandItem[] {
  return COMMANDS.filter((c) => matches(translate(c.labelKey).toLowerCase(), query));
}

/** The filtered, ordered rows for the current mode: sessions, hosts, then commands (the
 *  picker keeps only the hosts). Order mirrors the stores so the list is stable.
 *  `translate` resolves a command's label so its row is filtered on the text the user
 *  sees, not on the underlying key. */
export function paletteItems(
  mode: PaletteMode,
  hosts: HostDto[],
  sessions: Session[],
  query: string,
  translate: (key: string) => string
): PaletteItem[] {
  const hostRows: PaletteItem[] = queryHosts(hosts, query).map((host) => ({ kind: 'host', host }));
  if (mode === 'pickHost') return hostRows;
  const sessionRows: PaletteItem[] = querySessions(sessions, query).map((session) => ({
    kind: 'session',
    session
  }));
  const commandRows: PaletteItem[] = queryCommands(query, translate).map((c) => ({
    kind: 'command',
    id: c.id,
    labelKey: c.labelKey
  }));
  return [...sessionRows, ...hostRows, ...commandRows];
}

/** A stable key for the current result set — its rows' identity and order, but not
 *  volatile fields like a session's live status. The palette resets its highlight only
 *  when this changes, so a background status flip (which mints fresh item objects with
 *  the same ids) never snaps the selection back to the top mid-navigation. */
export function paletteSignature(items: PaletteItem[]): string {
  return items.map(itemKey).join('\u0000');
}

// --- Tag-organized host list (chips + collapsible groups) ---

// Case-insensitive tag order, matching the dashboard's section sort
// (../screens/serverCard.ts `byTagName`): base-letter fold first, then a tiebreak on
// the raw strings.
const byTagName = (a: string, b: string): number => {
  const la = a.toLowerCase();
  const lb = b.toLowerCase();
  if (la !== lb) return la < lb ? -1 : 1;
  return a < b ? -1 : a > b ? 1 : 0;
};

/** A host's tags, trimmed, blanks dropped — the canonical form everywhere below. */
const cleanTags = (h: HostDto): string[] => h.tags.map((t) => t.trim()).filter((t) => t !== '');

/** One host section; `tag` is `null` for the "Untagged" section. */
export type HostGroup = { tag: string | null; hosts: HostDto[] };

// Group-by-tag over hosts, mirroring the dashboard's `groupByTag` (../screens/
// serverCard.ts): a host sits under its first non-blank tag, so it shows once.
// Sections sort case-insensitively, "Untagged" last; input order is kept inside one.
export function groupHostsByTag(hosts: HostDto[]): HostGroup[] {
  const sections = new Map<string | null, HostDto[]>();
  for (const h of hosts) {
    const tag = cleanTags(h)[0] ?? null;
    const section = sections.get(tag);
    if (section) section.push(h);
    else sections.set(tag, [h]);
  }
  return [...sections]
    .map(([tag, hs]) => ({ tag, hosts: hs }))
    .sort((a, b) => (a.tag === null ? 1 : b.tag === null ? -1 : byTagName(a.tag, b.tag)));
}

/** A chip in the picker's tag bar. `all` leads; tags follow by name (a host counts for
 *  every tag it carries, not just its first); `untagged` trails only when it exists. */
export type TagChip =
  | { kind: 'all'; count: number }
  | { kind: 'tag'; tag: string; count: number }
  | { kind: 'untagged'; count: number };

export function tagChips(hosts: HostDto[]): TagChip[] {
  const counts = new Map<string, number>();
  let untagged = 0;
  for (const h of hosts) {
    const tags = cleanTags(h);
    if (tags.length === 0) untagged++;
    else for (const tag of new Set(tags)) counts.set(tag, (counts.get(tag) ?? 0) + 1);
  }
  const chips: TagChip[] = [{ kind: 'all', count: hosts.length }];
  for (const [tag, count] of [...counts].sort((a, b) => byTagName(a[0], b[0])))
    chips.push({ kind: 'tag', tag, count });
  if (untagged > 0) chips.push({ kind: 'untagged', count: untagged });
  return chips;
}

/** Hosts under a tag selection: a tag keeps hosts carrying it (any position), `null`
 *  keeps the untagged ones, `undefined` (the "All" chip) keeps everything. */
export function filterByTag(hosts: HostDto[], tag: string | null | undefined): HostDto[] {
  if (tag === undefined) return hosts;
  if (tag === null) return hosts.filter((h) => cleanTags(h).length === 0);
  return hosts.filter((h) => cleanTags(h).includes(tag));
}

/** Groups with the folded ones emptied (their header row stays, so it can be unfolded
 *  again). A search unfolds everything — no match may hide behind a fold. Fold keys are
 *  the dashboard's (`dashboardView.collapsed`): the tag, `''` for Untagged — shared with
 *  the dashboard, so a section folded there is folded here too. */
export function visibleGroups(
  groups: HostGroup[],
  collapsed: readonly string[],
  searching: boolean
): HostGroup[] {
  if (searching) return groups;
  const folded = new Set(collapsed);
  return groups.map((g) => (folded.has(g.tag ?? '') ? { tag: g.tag, hosts: [] } : g));
}

function itemKey(it: PaletteItem): string {
  switch (it.kind) {
    case 'session':
      return `s:${it.session.id}`;
    case 'host':
      return `h:${it.host.name}`;
    case 'command':
      return `c:${it.id}`;
  }
}

/** Move the selection by `delta`, wrapping at both ends; an empty list stays at 0. */
export function nextIndex(current: number, delta: number, length: number): number {
  if (length === 0) return 0;
  return (((current + delta) % length) + length) % length;
}

// A host reference shows its connection state as a dot (tech-gui.md §2, 1.3), mapped to
// the shared server-state palette. Connecting/not-yet-probed stay neutral; a failed
// host reads offline, matching the status-bar summary's offline bucket.
export function hostStatusDot(status: ConnectionStatusDto | undefined): Status {
  switch (status?.kind) {
    case 'connected':
      return 'ok';
    case 'failed':
      return 'off';
    default:
      return 'unknown';
  }
}

export interface PaletteState {
  open: boolean;
  mode: PaletteMode;
}

function createPalette() {
  const { subscribe, set } = writable<PaletteState>({ open: false, mode: 'navigate' });
  // A pending `pickHost()` resolver. Every open/close settles it exactly once so a
  // caller never hangs when the user switches to the navigator or dismisses the picker.
  let pending: ((host: HostDto | null) => void) | null = null;

  function settle(host: HostDto | null): void {
    const resolve = pending;
    pending = null;
    resolve?.(host);
  }

  return {
    subscribe,
    /** ⌘K navigator: jump to an open session or open a host. */
    open(): void {
      settle(null);
      set({ open: true, mode: 'navigate' });
    },
    /** Action-scoped host picker; resolves with the chosen host, or null if dismissed. */
    pickHost(): Promise<HostDto | null> {
      settle(null);
      set({ open: true, mode: 'pickHost' });
      return new Promise((resolve) => (pending = resolve));
    },
    /** Picker mode: hand the chosen host back to the caller and close. */
    choose(host: HostDto): void {
      settle(host);
      set({ open: false, mode: 'navigate' });
    },
    close(): void {
      settle(null);
      set({ open: false, mode: 'navigate' });
    }
  };
}

export const palette = createPalette();
