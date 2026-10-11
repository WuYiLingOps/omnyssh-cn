<script lang="ts">
  // Asks before a quit cuts off live work (stores/quit.ts). Cancel holds the focus, so
  // a stray Enter or a key meant for the terminal never quits.
  import Modal from './Modal.svelte';
  import { t } from 'svelte-i18n';
  import { confirmQuit, quitPrompt, type LiveWorkPart } from '$lib/stores/quit';
  import { traySupport } from '$lib/stores/tray';
  import { activeEntity } from '$lib/stores/activeEntity';
  import { isMac } from '$lib/platform';

  let { work }: { work: LiveWorkPart[] | null } = $props();

  // Name one tally with its count, e.g. "2 terminals" / "1 transfer"; a language with
  // no plural forms simply uses the `other` branch (mirrors the TUI's plain count).
  const partText = (part: LiveWorkPart): string =>
    $t(`quit.work.${part.kind}`, { values: { count: part.count } });

  // "2 terminals, 1 transfer and 1 tunnel" — parts joined with the language's list
  // separator and, before the last, its conjunction (`and` / `和`). Chinese joins every
  // item the same way, so `workListEnd` is empty there.
  const summary = $derived.by(() => {
    const parts = work === null ? [] : work.map(partText);
    if (parts.length === 0) return '';
    if (parts.length === 1) return parts[0];
    const head = parts.slice(0, -1).join($t('quit.workSeparator'));
    return `${head}${$t('quit.workListEnd')}${parts.at(-1)}`;
  });

  let dontAsk = $state(false);
  let quitting = $state(false);
  let cancelButton = $state<HTMLButtonElement>();

  // `autofocus` yields to whatever holds the focus, often a live terminal.
  $effect(() => cancelButton?.focus());

  function cancel(): void {
    quitPrompt.set(null);
  }

  function keepRunning(): void {
    cancel();
    activeEntity.selectSettings();
  }

  async function quit(): Promise<void> {
    quitting = true;
    try {
      if (dontAsk) await confirmQuit.set(false);
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      await getCurrentWindow().destroy();
    } catch {
      quitting = false;
    }
  }

  const button =
    'inline-flex items-center justify-center rounded-full px-7 py-2.5 text-sm font-medium transition ' +
    'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus disabled:opacity-50';
</script>

<Modal label={$t('quit.title')} onClose={cancel}>
  <div class="space-y-3 px-5 py-4">
    <h2 class="text-sm font-semibold">{$t('quit.title')}</h2>
    <p class="text-sm text-muted">{$t('quit.body', { values: { summary } })}</p>
    <label class="flex items-center gap-2 text-sm text-muted">
      <input type="checkbox" class="accent-accent" bind:checked={dontAsk} />
      {$t('quit.dontAsk')}
    </label>
    {#if $traySupport.available}
      <button
        type="button"
        class="text-left text-sm text-fg underline underline-offset-2 hover:text-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus"
        onclick={keepRunning}
      >
        {isMac ? $t('quit.keepMenuBar') : $t('quit.keepTray')}
      </button>
    {/if}
    <div class="flex justify-end gap-2 pt-1">
      <button
        bind:this={cancelButton}
        type="button"
        class="{button} text-muted hover:bg-surface-inset hover:text-fg"
        onclick={cancel}
      >
        {$t('quit.cancel')}
      </button>
      <button
        type="button"
        class="{button} bg-status-crit text-bg hover:opacity-90 active:scale-[0.98]"
        disabled={quitting}
        onclick={quit}
      >
        {$t('quit.quit')}
      </button>
    </div>
  </div>
</Modal>
