<script lang="ts">
  // Top tab strip (§2): the open terminal/SFTP sessions live here instead of the
  // sidebar, so the left rail stays pure navigation and the footer tools stay clear
  // of a growing list. Same invariants as the old sidebar rows: exactly one filled
  // tab (the active session, the brand's accent inversion), a status dot from the
  // shared `sessionStatusDot` palette, and a per-tab close button. The whole strip
  // is absent while no session is open, so an empty app looks exactly as before.
  //
  // No horizontal scrollbar, ever. Tabs first compress in place (label truncates),
  // and once even the compact floors no longer fit the row flips to overflow mode:
  // inactive tabs become icon-only pills, the active tab keeps a full-width pill,
  // and a pinned `…` button opens a menu listing every session (activate / close).
  import { Icon, StatusDot } from '$lib/theme';
  import { t } from 'svelte-i18n';
  import { sessions, sessionLabel, sessionTitle, sessionStatusDot } from '$lib/stores/sessions';
  import { activeEntity } from '$lib/stores/activeEntity';
  import { closeSession } from '$lib/stores/navigation';

  // Geometry budget (px). Measured against the row's clientWidth, so these only
  // need to track the classes below — flexbox absorbs the slack either way.
  const GAP = 4; // gap-1 between tabs
  const COMPACT_W = 40; // w-10 icon-only inactive tab in overflow mode
  const FLOOR_W = 76; // narrowest a full inactive tab can compress to (dot+icon+close)
  const ACTIVE_MAX = 160; // max-w-40 on the active pill
  const ACTIVE_MIN = 112; // min-w-28 active pill in overflow mode
  const MORE_W = 28; // the pinned `…` trigger

  let stripW = $state(0);
  let menuOpen = $state(false);
  let moreWrap = $state<HTMLDivElement>();

  // Which tabs render, and whether the strip is in overflow mode. Derived purely
  // from widths and counts (never scrollWidth), so hiding tabs cannot feed back
  // into the measurement and oscillate.
  const layout = $derived.by(() => {
    const list = $sessions;
    if (list.length === 0) return { tabs: [], overflow: false };
    const activeId = $activeEntity.kind === 'session' ? $activeEntity.id : null;
    const activeS = list.find((s) => s.id === activeId) ?? list[0];
    // Tightest possible full-tab row: widest active pill + every other tab at its
    // compression floor + the overflow trigger. Below this width, go compact.
    const minRow = ACTIVE_MAX + (list.length - 1) * (FLOOR_W + GAP) + GAP + MORE_W;
    if (stripW === 0 || minRow <= stripW) return { tabs: list, overflow: false };
    // Overflow mode: pack as many leading compact tabs as fit, keeping the active
    // session visible. If it sits beyond the packed prefix, it swaps into the last
    // slot (the menu still lists every session, so nothing becomes unreachable).
    const budget = stripW - MORE_W - ACTIVE_MIN - 2 * GAP;
    let slots = Math.floor((budget + GAP) / (COMPACT_W + GAP));
    slots = Math.max(0, Math.min(slots, list.length - 1));
    const shown = list.slice(0, slots);
    if (!shown.some((s) => s.id === activeS.id)) {
      if (shown.length === slots && slots > 0) shown[slots - 1] = activeS;
      else shown.push(activeS);
    }
    return { tabs: shown, overflow: true };
  });

  // The trigger disappears with the overflow state; never leave a dangling open menu.
  $effect(() => {
    if (!layout.overflow) menuOpen = false;
  });

  function onWindowPointerdown(e: PointerEvent): void {
    if (menuOpen && moreWrap && !moreWrap.contains(e.target as Node)) menuOpen = false;
  }

  function onWindowKeydown(e: KeyboardEvent): void {
    if (menuOpen && e.key === 'Escape') {
      e.preventDefault();
      menuOpen = false;
    }
  }

  // A tab is a filled pill, not a document tab: the row-state language is shared
  // with the sidebar rows so the accent inversion reads the same in both rails.
  // Inactive tabs may compress (the label truncates first); the active pill never
  // shrinks below its label while the strip is in normal mode.
  const tabBase = 'flex h-8 items-center gap-2 rounded-lg pl-2.5 pr-1 text-sm transition';
  // The ring belongs on the focusable element, so it is applied to buttons only —
  // never the tab wrapper, where :focus-visible can never match.
  const focusRing = 'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus';
  const tabState = (active: boolean): string =>
    active ? 'bg-accent text-accent-fg' : 'text-muted hover:bg-surface-inset hover:text-fg';
</script>

<svelte:window onpointerdown={onWindowPointerdown} onkeydown={onWindowKeydown} />

<!-- `pt-[var(--titlebar-h)]` keeps the strip clear of the macOS traffic lights; the
     drag strip above stays clickable (zero-height elsewhere). -->
{#if $sessions.length > 0}
  <div
    class="flex shrink-0 items-center gap-1 border-b border-default bg-surface px-2 pt-[var(--titlebar-h)] pb-1.5"
    aria-label={$t('sidebar.sessionTabs')}
  >
    <!-- The measured row: min-w-0 lets it give up width to the pinned trigger;
         overflow-hidden is a safety net for the frame before stripW settles. -->
    <div class="flex min-w-0 flex-1 items-center gap-1 overflow-hidden" bind:clientWidth={stripW}>
      {#each layout.tabs as s (s.id)}
        {@const active = $activeEntity.kind === 'session' && $activeEntity.id === s.id}
        {@const compact = layout.overflow && !active}
        <div
          class="{tabBase} {tabState(active)}
            {compact
            ? 'w-10 shrink-0 justify-center gap-1.5 px-0'
            : active && layout.overflow
              ? 'min-w-28 max-w-40 flex-1'
              : active
                ? 'shrink-0'
                : 'min-w-0'}"
        >
          {#if compact}
            <!-- Icon-only pill: click to activate; close lives in the menu. -->
            <button
              type="button"
              class="flex min-w-0 items-center gap-1.5 rounded {focusRing}"
              title={sessionTitle(s)}
              aria-label={sessionTitle(s)}
              onclick={() => activeEntity.activateSession(s.id)}
            >
              <StatusDot status={sessionStatusDot[s.status]} />
              <Icon name={s.kind} size={14} />
            </button>
          {:else}
            <button
              type="button"
              class="flex min-w-0 items-center gap-2 rounded {focusRing}"
              title={sessionTitle(s)}
              aria-label={sessionTitle(s)}
              aria-current={active ? 'true' : undefined}
              onclick={() => activeEntity.activateSession(s.id)}
            >
              <StatusDot status={sessionStatusDot[s.status]} />
              <Icon name={s.kind} size={14} />
              <span class="min-w-0 max-w-40 truncate">{sessionLabel(s)}</span>
            </button>
            <button
              type="button"
              class="shrink-0 rounded p-1 opacity-60 transition hover:opacity-100 {focusRing}"
              title={$t('sidebar.closeSession', { values: { name: sessionLabel(s) } })}
              aria-label={$t('sidebar.closeSession', { values: { name: sessionLabel(s) } })}
              onclick={() => closeSession(s.id)}
            >
              <Icon name="close" size={13} />
            </button>
          {/if}
        </div>
      {/each}
    </div>
    {#if layout.overflow}
      <!-- Pinned trigger: outside pointerdown / Escape close it (window handlers). -->
      <div class="relative shrink-0" bind:this={moreWrap}>
        <button
          type="button"
          class="flex h-8 w-7 items-center justify-center rounded-lg text-muted transition
            hover:bg-surface-inset hover:text-fg {focusRing}
            {menuOpen ? 'bg-surface-inset text-fg' : ''}"
          title={$t('sidebar.moreSessions')}
          aria-label={$t('sidebar.moreSessions')}
          aria-haspopup="menu"
          aria-expanded={menuOpen}
          onclick={() => (menuOpen = !menuOpen)}
        >
          <Icon name="more" size={16} />
        </button>
        {#if menuOpen}
          <div
            role="menu"
            aria-label={$t('sidebar.moreSessions')}
            class="absolute top-full right-0 z-50 mt-1 max-h-80 min-w-60 overflow-y-auto
              rounded-xl border border-default bg-surface py-1.5 shadow-soft"
          >
            {#each $sessions as s (s.id)}
              {@const active = $activeEntity.kind === 'session' && $activeEntity.id === s.id}
              <div
                role="menuitem"
                tabindex="-1"
                class="flex w-full cursor-pointer items-center gap-2.5 py-1.5 pr-1.5 pl-3.5 text-left text-sm transition
                  {active ? 'bg-accent text-accent-fg' : 'text-fg hover:bg-surface-inset'}"
                onclick={() => {
                  menuOpen = false;
                  activeEntity.activateSession(s.id);
                }}
                onkeydown={(e) => {
                  if (e.key !== 'Enter' && e.key !== ' ') return;
                  e.preventDefault();
                  menuOpen = false;
                  activeEntity.activateSession(s.id);
                }}
              >
                <StatusDot status={sessionStatusDot[s.status]} />
                <Icon name={s.kind} size={14} />
                <span class="min-w-0 flex-1 truncate">{sessionLabel(s)}</span>
                <button
                  type="button"
                  class="shrink-0 rounded p-1 opacity-60 transition hover:opacity-100 {focusRing}"
                  title={$t('sidebar.closeSession', { values: { name: sessionLabel(s) } })}
                  aria-label={$t('sidebar.closeSession', { values: { name: sessionLabel(s) } })}
                  onclick={(e) => {
                    e.stopPropagation();
                    closeSession(s.id);
                  }}
                >
                  <Icon name="close" size={13} />
                </button>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  </div>
{/if}
