<script lang="ts">
  // The ⌘K overlay and the host-picker, one component in two modes (tech-gui.md §2,
  // 1.3). Navigator: jump to an open session, open a host (default: a shell), or run a
  // command. Picker: hand a chosen host back to its caller (the spawner buttons).
  // Keyboard-first — type to filter, ↑/↓ to move, ↵ to select, esc to dismiss.
  // Glass/blur per the brandbook.
  import { tick } from 'svelte';
  import { Icon, StatusDot } from '$lib/theme';
  import { t } from 'svelte-i18n';
  import {
    palette,
    paletteSignature,
    nextIndex,
    hostStatusDot,
    queryHosts,
    querySessions,
    queryCommands,
    groupHostsByTag,
    filterByTag,
    visibleGroups,
    tagChips,
    type PaletteItem,
    type TagChip
  } from '$lib/stores/palette';
  import { hosts, firstOfEachName } from '$lib/stores/hosts';
  import { statuses } from '$lib/stores/statuses';
  import { sessions, sessionLabel, sessionStatusDot } from '$lib/stores/sessions';
  import { activeEntity } from '$lib/stores/activeEntity';
  import { spawnSession } from '$lib/stores/navigation';
  import { dashboardView } from '$lib/stores/dashboardView';
  import { streamerMode, displayHostname } from '$lib/stores/streamer';
  import { isPaletteChord, streamerChordLabel } from '$lib/stores/ui';

  let inputEl = $state<HTMLInputElement>();
  let listEl = $state<HTMLUListElement>();
  let query = $state('');
  let selected = $state(0);
  // The tag chip in effect: `undefined` is "All", `null` the untagged chip, a string a
  // tag. A per-open choice, never persisted — connecting is a momentary intent.
  let tagSel = $state<string | null | undefined>(undefined);

  const searching = $derived(query.trim() !== '');

  // Chips come from every host, not the query's matches, so their counts hold still
  // while typing; the row only earns its place when there is more than "All" to show.
  const chips = $derived(tagChips(firstOfEachName($hosts)));

  // The host run: query first (which dedupes by name), then the tag chip, then
  // grouping, then the folded sections emptied. matched/shown are parallel arrays —
  // the header keeps its match count even while its rows are folded away.
  const matchedGroups = $derived(groupHostsByTag(filterByTag(queryHosts($hosts, query), tagSel)));
  const shownGroups = $derived(visibleGroups(matchedGroups, $dashboardView.collapsed, searching));

  const sessionRows = $derived(querySessions($sessions, query));
  const commandRows = $derived(queryCommands(query, $t));

  // Selectable rows: sessions, the visible host rows in group order, then commands.
  const items = $derived.by<PaletteItem[]>(() => {
    const hostRows: PaletteItem[] = shownGroups.flatMap((g) =>
      g.hosts.map((host) => ({ kind: 'host', host }))
    );
    if ($palette.mode === 'pickHost') return hostRows;
    return [
      ...sessionRows.map((session) => ({ kind: 'session', session }) as PaletteItem),
      ...hostRows,
      ...commandRows.map((c) => ({ kind: 'command', id: c.id, labelKey: c.labelKey }) as PaletteItem)
    ];
  });

  // Where each run starts inside `items`: sessions first, then hosts grouped, then
  // commands. Group headers are not selectable, so they take no index.
  const sessionCount = $derived($palette.mode === 'pickHost' ? 0 : sessionRows.length);
  const commandCount = $derived($palette.mode === 'pickHost' ? 0 : commandRows.length);
  const hostOffsets = $derived.by(() => {
    let acc = 0;
    return shownGroups.map((g) => {
      const start = acc;
      acc += g.hosts.length;
      return start;
    });
  });
  const commandAt = $derived(items.length - commandCount);

  // Folded sections keep their headers, so "no rows" alone is not "no matches".
  const nothingMatches = $derived(
    sessionCount === 0 && commandCount === 0 && matchedGroups.length === 0
  );

  // A value-stable key over the result set: unchanged by a background status flip (same
  // ids, new objects), so the reset effect below can ignore those (see the effect).
  const itemsSignature = $derived(paletteSignature(items));

  const placeholder = $derived(
    $palette.mode === 'pickHost' ? $t('palette.placeholderPick') : $t('palette.placeholderNavigate')
  );
  // The navigator always lists its commands, so only a query can empty it.
  const emptyMessage = $derived(
    $palette.mode === 'pickHost'
      ? query
        ? $t('palette.emptyPickNoMatch')
        : $t('palette.emptyPickNone')
      : $t('palette.emptyNavigate')
  );

  // Focus returns here when the overlay closes, so a keyboard user is not dropped to
  // <body> after picking a host.
  let restoreFocus: HTMLElement | null = null;

  // On open: remember the trigger, clear the query and the tag chip, focus the input.
  // On close: hand focus back. Re-runs on a mode switch too, since the store emits a
  // fresh object.
  $effect(() => {
    if ($palette.open) {
      // Capture once per open session, so a mode switch does not overwrite the trigger
      // with the palette's own input.
      restoreFocus ??= document.activeElement as HTMLElement | null;
      query = '';
      tagSel = undefined;
      selected = 0;
      void tick().then(() => inputEl?.focus());
    } else if (restoreFocus) {
      if (restoreFocus.isConnected) restoreFocus.focus();
      restoreFocus = null;
    }
  });

  // Reset the highlight to the top match whenever the result *set* changes — typing, a
  // mode switch, an added/removed/reordered row — so arrow-nav and Enter never target a
  // stale row. Keyed on the signature, not `items`: a background status flip re-derives
  // `items` (a new array) but not the signature, so it no longer snaps the selection.
  $effect(() => {
    void itemsSignature;
    selected = 0;
  });

  // Keep the highlighted row visible as arrow-nav moves it.
  $effect(() => {
    listEl?.querySelector<HTMLElement>(`[data-index="${selected}"]`)?.scrollIntoView({
      block: 'nearest'
    });
  });

  function selectAt(i: number): void {
    const item = items[i];
    if (!item) return;
    if (item.kind === 'session') {
      activeEntity.activateSession(item.session.id);
      palette.close();
    } else if (item.kind === 'command') {
      streamerMode.toggle();
      palette.close();
    } else if ($palette.mode === 'pickHost') {
      palette.choose(item.host);
    } else {
      // Navigator default action for a host: open a shell (the primary connect path).
      spawnSession('terminal', item.host.name);
      palette.close();
    }
  }

  // Tag chips: `All` clears the selection, a tag keeps hosts carrying it (any
  // position), the untagged chip keeps hosts with no usable tag.
  function chipSelect(chip: TagChip): void {
    tagSel = chip.kind === 'all' ? undefined : chip.kind === 'untagged' ? null : chip.tag;
  }

  const chipOn = (chip: TagChip): boolean =>
    chip.kind === 'all'
      ? tagSel === undefined
      : chip.kind === 'untagged'
        ? tagSel === null
        : tagSel === chip.tag;

  /** ←/→ fold or unfold around the selection. ← folds the group holding the selected
   *  host row (the row then hides, and the signature reset moves the highlight to the
   *  top — folding is a deliberate re-scope); → unfolds the first folded group, whose
   *  rows are hidden and so can never hold the selection themselves. A search unfolds
   *  everything, so both are no-ops while it runs, as on sessions/commands/empty. */
  function foldAroundSelection(unfold: boolean): void {
    if (searching) return;
    if (unfold) {
      const firstFolded = matchedGroups.find((_, gi) => shownGroups[gi].hosts.length === 0);
      if (firstFolded) dashboardView.toggleCollapsed(firstFolded.tag ?? '');
      return;
    }
    const at = selected - sessionCount; // index into the host run of `items`
    let acc = 0;
    for (let gi = 0; gi < shownGroups.length; gi++) {
      const len = shownGroups[gi].hosts.length;
      if (at >= acc && at < acc + len) {
        dashboardView.toggleCollapsed(matchedGroups[gi].tag ?? '');
        return;
      }
      acc += len;
    }
  }

  // Chromium re-fires mousemove when the list scrolls under a stationary pointer, which
  // would snap the highlight back during arrow-nav. Only a real move (changed
  // coordinates) is allowed to drive the selection.
  let pointer = { x: -1, y: -1 };
  function hover(e: MouseEvent, i: number): void {
    if (e.clientX === pointer.x && e.clientY === pointer.y) return;
    pointer = { x: e.clientX, y: e.clientY };
    selected = i;
  }

  function onKeydown(e: KeyboardEvent): void {
    if (isPaletteChord(e)) {
      // ⌘K toggles: open the navigator when closed, dismiss when already open.
      e.preventDefault();
      if ($palette.open) palette.close();
      else palette.open();
      return;
    }
    if (!$palette.open) return;
    // Enter/Escape that commit or cancel an IME composition must reach the input, and
    // Home/End belong to the search field's caret. ←/→ drive the list's fold state only
    // at the text's edges (the caret cannot move further there); inside the query they
    // stay caret keys. ↑/↓ and Enter always drive the list.
    if (e.isComposing) return;
    switch (e.key) {
      case 'Escape':
        e.preventDefault();
        palette.close();
        break;
      case 'ArrowDown':
        e.preventDefault();
        selected = nextIndex(selected, 1, items.length);
        break;
      case 'ArrowUp':
        e.preventDefault();
        selected = nextIndex(selected, -1, items.length);
        break;
      case 'ArrowLeft':
        if (inputEl?.selectionStart === 0) {
          e.preventDefault();
          foldAroundSelection(false);
        }
        break;
      case 'ArrowRight':
        if (caretAtEnd()) {
          e.preventDefault();
          foldAroundSelection(true);
        }
        break;
      case 'Enter':
        e.preventDefault();
        selectAt(selected);
        break;
    }
  }

  /** The caret sits on the last cell (no selection span), so → has nowhere to go. */
  function caretAtEnd(): boolean {
    const el = inputEl;
    return !!el && el.selectionStart === el.value.length && el.selectionEnd === el.value.length;
  }

  const rowBase = 'flex w-full items-center gap-3 rounded-lg px-3 py-2 text-left text-sm transition';
  const rowState = (active: boolean): string =>
    active ? 'bg-accent text-accent-fg' : 'text-muted hover:bg-surface-inset hover:text-fg';
  const sectionHead = 'px-3 pb-1 pt-2 text-[11px] font-medium uppercase tracking-[0.18em] text-faint';
</script>

<svelte:window onkeydown={onKeydown} />

{#if $palette.open}
  <div
    class="fixed inset-0 z-50 flex items-start justify-center px-4 pt-[14vh]"
    role="dialog"
    aria-modal="true"
    aria-label={$palette.mode === 'pickHost' ? $t('palette.pickHost') : $t('palette.navigate')}
  >
    <button
      type="button"
      tabindex="-1"
      aria-label={$t('palette.dismiss')}
      class="absolute inset-0 bg-overlay"
      onclick={() => palette.close()}
    ></button>

    <div
      class="relative w-full max-w-xl overflow-hidden rounded-2xl border border-default bg-glass shadow-soft backdrop-blur-md"
    >
      <div class="flex items-center gap-3 border-b border-default px-4">
        <span class="text-faint"><Icon name="search" /></span>
        <input
          bind:this={inputEl}
          bind:value={query}
          type="text"
          {placeholder}
          aria-label={placeholder}
          autocomplete="off"
          spellcheck="false"
          class="w-full bg-transparent py-3.5 text-sm text-fg outline-none placeholder:text-faint"
        />
      </div>

      <!-- Tag chips: stable counts over every host; hidden while tags add nothing. -->
      {#if chips.length > 1}
        <div
          class="flex items-center gap-1.5 overflow-x-auto border-b border-default px-3 py-2"
          role="group"
          aria-label={$t('palette.tagFilter')}
        >
          {#each chips as chip (chip.kind === 'tag' ? `tag:${chip.tag}` : chip.kind)}
            <button
              type="button"
              class="shrink-0 rounded-full border px-2.5 py-1 text-xs transition focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus {chipOn(
                chip
              )
                ? 'border-accent bg-accent text-accent-fg'
                : 'border-default text-muted hover:border-strong hover:bg-surface-inset hover:text-fg'}"
              aria-pressed={chipOn(chip)}
              onclick={() => chipSelect(chip)}
            >
              {chip.kind === 'all'
                ? $t('palette.tagAll')
                : chip.kind === 'untagged'
                  ? $t('palette.untagged')
                  : chip.tag}
              <span class="tabular-nums opacity-70">{chip.count}</span>
            </button>
          {/each}
        </div>
      {/if}

      <ul bind:this={listEl} class="max-h-[min(24rem,50vh)] overflow-y-auto p-2">
        {#if nothingMatches}
          <li class="px-3 py-6 text-center text-sm text-muted">{emptyMessage}</li>
        {:else}
          {#if $palette.mode === 'navigate' && sessionCount > 0}
            <li class={sectionHead}>{$t('palette.sessions')}</li>
            {#each sessionRows as session, i (session.id)}
              <li>
                <button
                  type="button"
                  data-index={i}
                  aria-current={selected === i ? 'true' : undefined}
                  class="{rowBase} {rowState(selected === i)}"
                  onclick={() => selectAt(i)}
                  onmousemove={(e) => hover(e, i)}
                >
                  <StatusDot status={sessionStatusDot[session.status]} />
                  <Icon name={session.kind} size={16} />
                  <span class="min-w-0 flex-1 truncate">{sessionLabel(session)}</span>
                </button>
              </li>
            {/each}
          {/if}

          {#if matchedGroups.length > 0}
            {#if $palette.mode === 'navigate'}
              <li class={sectionHead}>{$t('palette.hosts')}</li>
            {/if}
            {#each matchedGroups as group, gi (group.tag ?? '\u0000')}
              {@const shown = shownGroups[gi].hosts}
              {@const at = sessionCount + hostOffsets[gi]}
              <!-- Group header: the match count survives folding, so a folded group
                   stays findable. A search unfolds every group (and disables folding). -->
              <li>
                <button
                  type="button"
                  class="flex w-full items-center gap-2 rounded-lg px-3 py-1.5 text-left text-muted transition hover:bg-surface-inset hover:text-fg focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus disabled:opacity-60"
                  aria-expanded={shown.length > 0}
                  title={$t('palette.toggleGroup')}
                  disabled={searching}
                  onclick={() => dashboardView.toggleCollapsed(group.tag ?? '')}
                >
                  <span class="inline-flex text-faint transition-transform {shown.length > 0 ? '' : '-rotate-90'}">
                    <Icon name="chevron" size={12} />
                  </span>
                  <span class="min-w-0 truncate text-xs font-semibold {group.tag === null ? 'italic' : ''}">
                    {group.tag ?? $t('palette.untagged')}
                  </span>
                  <span class="shrink-0 text-[11px] tabular-nums text-faint">{group.hosts.length}</span>
                  <span class="min-w-4 flex-1 border-t border-default"></span>
                </button>
              </li>
              {#each shown as host, hi (host.name)}
                <li>
                  <button
                    type="button"
                    data-index={at + hi}
                    aria-current={selected === at + hi ? 'true' : undefined}
                    class="{rowBase} {rowState(selected === at + hi)} pl-7"
                    onclick={() => selectAt(at + hi)}
                    onmousemove={(e) => hover(e, at + hi)}
                  >
                    <StatusDot status={hostStatusDot($statuses.get(host.name))} />
                    <span class="min-w-0 flex-1 truncate font-medium">{host.name}</span>
                    <span
                      class="shrink-0 truncate font-mono text-xs {selected === at + hi
                        ? ''
                        : 'text-faint'}"
                    >
                      {host.user}@{displayHostname(host.hostname, $streamerMode)}
                    </span>
                  </button>
                </li>
              {/each}
            {/each}
          {/if}

          {#if $palette.mode === 'navigate' && commandCount > 0}
            <li class={sectionHead}>{$t('palette.commands')}</li>
            {#each commandRows as command, j (command.id)}
              {@const i = commandAt + j}
              <li>
                <button
                  type="button"
                  data-index={i}
                  aria-current={selected === i ? 'true' : undefined}
                  class="{rowBase} {rowState(selected === i)}"
                  onclick={() => selectAt(i)}
                  onmousemove={(e) => hover(e, i)}
                >
                  <Icon name="eye" size={16} />
                  <span class="min-w-0 flex-1 truncate">{$t(command.labelKey)}</span>
                  <span class="shrink-0 font-mono text-xs {selected === i ? '' : 'text-faint'}">
                    {streamerChordLabel}
                  </span>
                </button>
              </li>
            {/each}
          {/if}
        {/if}
      </ul>

      <div
        class="flex items-center gap-4 border-t border-default px-4 py-2 font-mono text-[11px] text-faint"
      >
        <span>{$t('palette.hintNavigate')}</span>
        <span>{$t('palette.hintFold')}</span>
        <span>{$t('palette.hintSelect')}</span>
        <span>{$t('palette.hintClose')}</span>
      </div>
    </div>
  </div>
{/if}
