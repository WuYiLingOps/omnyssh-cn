<script lang="ts">
  // Bottom region (tech-gui.md §2): context + host summary, and where background
  // errors surface (§3.5). The summary counts total / online / alert / offline
  // (§4.1); colour lives only in the status dots, per the brandbook.
  import { lastError } from '$lib/stores/notifications';
  import { hostSummary } from '$lib/stores/hostSummary';
  import { maskText, streamerMode } from '$lib/stores/streamer';
  import { streamerChordLabel } from '$lib/stores/ui';
  import { Icon, StatusDot } from '$lib/theme';
  import { t } from 'svelte-i18n';
</script>

<footer
  class="col-span-2 col-start-1 row-start-2 flex items-center justify-between gap-4 border-t border-default bg-surface px-5 py-2 text-xs text-muted"
>
  {#if $lastError}
    {@const text = $maskText($lastError)}
    <!-- A long error (a hint to act on) is cut to one line; hover shows all of it. -->
    <span class="min-w-0 truncate text-status-crit" title={text}>{text}</span>
  {:else}
    <span class="min-w-0 truncate">{$t('app.ready')}</span>
  {/if}
  <div class="flex shrink-0 items-center gap-3">
    {#if $streamerMode}
      <!-- Always in view, so nobody records believing addresses are hidden when they
           are not, or forgets the mode is on. No box or padding: the bar keeps its height. -->
      <span
        class="flex text-fg"
        role="img"
        title={$t('statusBar.streamerOn') + ' (' + streamerChordLabel + ')'}
        aria-label={$t('statusBar.streamerOn')}
      >
        <Icon name="eye" size={14} />
      </span>
    {/if}
    <span>{$t('statusBar.hostCount', { values: { count: $hostSummary.total } })}</span>
    <span class="flex items-center gap-1.5">
      <StatusDot status="ok" label={$t('statusBar.onlineLabel')} />{$t('statusBar.online', {
        values: { count: $hostSummary.online }
      })}
    </span>
    <span class="flex items-center gap-1.5">
      <StatusDot status="warn" label={$t('statusBar.alertLabel')} />{$t('statusBar.alert', {
        values: { count: $hostSummary.alert }
      })}
    </span>
    <span class="flex items-center gap-1.5">
      <StatusDot status="off" label={$t('statusBar.offlineLabel')} />{$t('statusBar.offline', {
        values: { count: $hostSummary.offline }
      })}
    </span>
  </div>
</footer>
