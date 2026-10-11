<script lang="ts">
  // Top tab strip (§2): the open terminal/SFTP sessions live here instead of the
  // sidebar, so the left rail stays pure navigation and the footer tools stay clear
  // of a growing list. Same invariants as the old sidebar rows: exactly one filled
  // tab (the active session, the brand's accent inversion), a status dot from the
  // shared `sessionStatusDot` palette, and a per-tab close button. The whole strip
  // is absent while no session is open, so an empty app looks exactly as before.
  import { Icon, StatusDot } from '$lib/theme';
  import { t } from 'svelte-i18n';
  import { sessions, sessionLabel, sessionTitle, sessionStatusDot } from '$lib/stores/sessions';
  import { activeEntity } from '$lib/stores/activeEntity';
  import { closeSession } from '$lib/stores/navigation';

  // A tab is a filled pill, not a document tab: the row-state language is shared
  // with the sidebar rows so the accent inversion reads the same in both rails.
  const tabBase =
    'flex h-8 shrink-0 items-center gap-2 rounded-lg pl-2.5 pr-1 text-sm transition';
  // The ring belongs on the focusable element, so it is applied to buttons only —
  // never the tab wrapper, where :focus-visible can never match.
  const focusRing = 'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus';
  const tabState = (active: boolean): string =>
    active ? 'bg-accent text-accent-fg' : 'text-muted hover:bg-surface-inset hover:text-fg';
</script>

<!-- `pt-[var(--titlebar-h)]` keeps the strip clear of the macOS traffic lights; the
     drag strip above stays clickable (zero-height elsewhere). -->
{#if $sessions.length > 0}
  <div
    class="flex shrink-0 items-center gap-1 overflow-x-auto border-b border-default bg-surface px-2 pt-[var(--titlebar-h)] pb-1.5"
    aria-label={$t('sidebar.sessionTabs')}
  >
    {#each $sessions as s (s.id)}
      {@const active = $activeEntity.kind === 'session' && $activeEntity.id === s.id}
      <div class="{tabBase} {tabState(active)}">
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
      </div>
    {/each}
  </div>
{/if}
