<script lang="ts">
  // Left region (tech-gui.md §2): header (logo + collapse), the four entry points
  // (two selectors that hold a highlight, two spawners that open sessions), and the
  // footer (palette + theme toggle, §5.1). The active highlight is the brand's
  // accent inversion, so exactly one filled row — a selector — is visible here at
  // any moment (the §2 invariant, made legible). Open sessions are a top tab strip
  // above the content (TabBar.svelte), not sidebar rows.
  import Logo from './Logo.svelte';
  import ThemeToggle from './ThemeToggle.svelte';
  import { Icon, type IconName } from '$lib/theme';
  import { t } from 'svelte-i18n';
  import { activeEntity } from '$lib/stores/activeEntity';
  import { type SessionKind } from '$lib/stores/sessions';
  import { sidebarCollapsed } from '$lib/stores/ui';
  import { spawnSession } from '$lib/stores/navigation';
  import { palette } from '$lib/stores/palette';

  // Action-first spawn (tech-gui.md §2): a spawner opens the host-picker, then creates
  // a session of its kind for the chosen host. A dismissed picker spawns nothing.
  async function pickAndSpawn(kind: SessionKind): Promise<void> {
    const host = await palette.pickHost();
    if (host) spawnSession(kind, host.name);
  }

  // Entry-point labels are message keys, resolved through `$t` at render: a plain
  // string captured here would freeze the label at the language the app booted in.
  type Selector = { kind: 'dashboard' | 'credentials' | 'snippets' | 'cloudsync'; label: string; icon: IconName };
  type Spawner = { kind: SessionKind; label: string; icon: IconName };

  const selectors: Selector[] = [
    { kind: 'dashboard', label: 'sidebar.dashboard', icon: 'dashboard' },
    { kind: 'credentials', label: 'sidebar.credentials', icon: 'key' },
    { kind: 'snippets', label: 'sidebar.snippets', icon: 'snippets' },
    { kind: 'cloudsync', label: 'sidebar.cloudSync', icon: 'cloud' }
  ];
  const spawners: Spawner[] = [
    { kind: 'sftp', label: 'sidebar.sftp', icon: 'sftp' },
    { kind: 'terminal', label: 'sidebar.terminal', icon: 'terminal' }
  ];

  const rowBase = 'flex w-full items-center gap-2 rounded-lg px-2 py-1 text-sm transition';
  // The ring belongs on the focusable element, so it is applied to buttons only —
  // never the session-row wrapper div, where :focus-visible can never match.
  const focusRing = 'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus';
  const rowState = (active: boolean): string =>
    active ? 'bg-accent text-accent-fg' : 'text-muted hover:bg-surface-inset hover:text-fg';
</script>

<aside
  class="col-start-1 row-start-1 flex h-full flex-col overflow-hidden border-r border-default bg-surface pt-[var(--titlebar-h)]"
>
  <header
    class="flex items-center gap-2 px-2 py-2 {$sidebarCollapsed ? 'justify-center' : ''}"
  >
    {#if !$sidebarCollapsed}
      <Logo size={18} />
      <span class="flex-1 truncate text-sm font-bold tracking-wide">{$t('app.name')}</span>
    {/if}
    <button
      type="button"
      class="grid h-8 w-8 place-items-center rounded-full text-muted transition hover:bg-surface-inset hover:text-fg focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus"
      title={$sidebarCollapsed ? $t('sidebar.expand') : $t('sidebar.collapse')}
      aria-label={$sidebarCollapsed ? $t('sidebar.expand') : $t('sidebar.collapse')}
      onclick={() => sidebarCollapsed.toggle()}
    >
      <Icon name={$sidebarCollapsed ? 'expand' : 'collapse'} />
    </button>
  </header>

  <!-- Entry points stay pinned; the sidebar holds no scrollable list any more. -->
  <nav class="flex min-h-0 flex-1 flex-col px-1 py-1">
    <ul class="shrink-0 space-y-0.5">
      {#each selectors as sel (sel.kind)}
        <li>
          <button
            type="button"
            class="{rowBase} {focusRing} {rowState($activeEntity.kind === sel.kind)} {$sidebarCollapsed
              ? 'justify-center'
              : ''}"
            title={$t(sel.label)}
            aria-current={$activeEntity.kind === sel.kind ? 'page' : undefined}
            onclick={() =>
              sel.kind === 'dashboard'
                ? activeEntity.selectDashboard()
                : sel.kind === 'credentials'
                  ? activeEntity.selectCredentials()
                  : sel.kind === 'snippets'
                    ? activeEntity.selectSnippets()
                    : activeEntity.selectCloudSync()}
          >
            <Icon name={sel.icon} />
            {#if !$sidebarCollapsed}<span class="truncate">{$t(sel.label)}</span>{/if}
          </button>
        </li>
      {/each}
      {#each spawners as sp (sp.kind)}
        <li>
          <button
            type="button"
            class="{rowBase} {focusRing} {rowState(false)} {$sidebarCollapsed ? 'justify-center' : ''}"
            title={$t(sp.label)}
            onclick={() => pickAndSpawn(sp.kind)}
          >
            <Icon name={sp.icon} />
            {#if !$sidebarCollapsed}<span class="truncate">{$t(sp.label)}</span>{/if}
          </button>
        </li>
      {/each}
    </ul>
  </nav>

  <footer
    class="border-t border-default px-1 py-1.5 {$sidebarCollapsed
      ? 'flex flex-col items-center gap-1'
      : 'flex items-center gap-1'}"
  >
    <button
      type="button"
      class="grid h-8 w-8 place-items-center rounded-full text-muted transition hover:bg-surface-inset hover:text-fg focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus"
      title={$t('sidebar.commandPalette')}
      aria-label={$t('sidebar.commandPalette')}
      onclick={() => palette.open()}
    >
      <Icon name="command" />
    </button>
    <ThemeToggle />
    <!-- Settings is a selector-like screen; the gear holds the active highlight like
         Dashboard/Snippets do, and stays icon-only so it survives collapse (§5.1). -->
    <button
      type="button"
      class="grid h-8 w-8 place-items-center rounded-full transition {focusRing} {$activeEntity.kind ===
      'settings'
        ? 'bg-accent text-accent-fg'
        : 'text-muted hover:bg-surface-inset hover:text-fg'}"
      title={$t('sidebar.settings')}
      aria-label={$t('sidebar.settings')}
      aria-current={$activeEntity.kind === 'settings' ? 'page' : undefined}
      onclick={() => activeEntity.selectSettings()}
    >
      <Icon name="settings" />
    </button>
  </footer>
</aside>
