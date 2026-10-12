<script lang="ts">
  // Toast renderer, mounted once in the app shell. Bottom-center so it never
  // covers the title bar or the dialogs that spawn from it; messages stack
  // newest-last and each clears itself after TOAST_TTL_MS (or on click).
  import Icon from '$lib/theme/Icon.svelte';
  import type { IconName } from '$lib/theme';
  import { toastStore } from '$lib/stores/toast';

  const row =
    'pointer-events-auto flex max-w-md items-center gap-2 rounded-xl border border-default ' +
    'bg-surface-raised px-4 py-2.5 text-sm text-fg shadow-soft transition';
  const iconFor = (kind: 'ok' | 'error'): IconName => (kind === 'ok' ? 'check' : 'close');
  const colorFor = (kind: 'ok' | 'error'): string =>
    kind === 'ok' ? 'text-status-ok' : 'text-status-crit';
</script>

{#if $toastStore.length > 0}
  <div
    class="pointer-events-none fixed inset-x-0 bottom-16 z-[60] flex flex-col items-center gap-2 px-4"
    role="status"
    aria-live="polite"
  >
    {#each $toastStore as t (t.id)}
      <button
        type="button"
        class="{row} focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus"
        onclick={() => toastStore.dismiss(t.id)}
      >
        <Icon name={iconFor(t.kind)} size={15} class={colorFor(t.kind)} />
        <span class="min-w-0 truncate">{t.message}</span>
      </button>
    {/each}
  </div>
{/if}
