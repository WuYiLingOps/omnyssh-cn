<script lang="ts">
  // Per-host snippet results (tech-gui.md §2.2), driven by the `snippetRun` store:
  // each target host shows pending → ok/failed with its output. Colour lives only in
  // the status dot, per the brandbook. Rendered while a run is active and not dismissed.
  import { StatusDot, Button, type Status } from '$lib/theme';
  import { t } from 'svelte-i18n';
  import Modal from '$lib/components/Modal.svelte';
  import { snippetRun, clearRun, type SnippetResultEntry } from '$lib/stores/snippets';
  import { maskText } from '$lib/stores/streamer';

  function dot(entry: SnippetResultEntry): Status {
    if (entry.pending) return 'unknown';
    return entry.ok ? 'ok' : 'crit';
  }

  // The state word is a message key; the status dot's label and the row's badge both
  // resolve it, so a language switch re-strings them together.
  function stateKey(entry: SnippetResultEntry): string {
    if (entry.pending) return 'snippetResults.stateRunning';
    return entry.ok ? 'snippetResults.stateOk' : 'snippetResults.stateFailed';
  }
</script>

{#if $snippetRun && !$snippetRun.dismissed}
  {@const run = $snippetRun}
  <Modal label={$t('snippetResults.title')} onClose={clearRun}>
    <header class="border-b border-default px-5 py-3.5">
      <h2 class="truncate text-sm font-semibold">{$t('snippetResults.heading', { values: { name: run.snippetName } })}</h2>
    </header>

    <div class="min-h-0 flex-1 space-y-3 overflow-y-auto px-5 py-4">
      {#each run.entries as entry (entry.hostName)}
        <div class="rounded-lg border border-default">
          <div class="flex items-center gap-2.5 border-b border-default px-3 py-2">
            <StatusDot
              status={dot(entry)}
              label={$t('snippetResults.hostState', {
                values: { host: entry.hostName, state: $t(stateKey(entry)) }
              })}
            />
            <span class="min-w-0 flex-1 truncate text-sm font-medium">{entry.hostName}</span>
            <span class="shrink-0 text-[11px] uppercase tracking-wider text-faint">{$t(stateKey(entry))}</span>
          </div>
          {#if entry.pending}
            <p class="px-3 py-2 text-xs text-faint">{$t('snippetResults.running')}</p>
          {:else if entry.output.trim()}
            <!-- A failed entry's output is the backend's error (connect or command), so it
                 is masked; a success is the command's own stdout, shown as it came. -->
            <pre
              class="max-h-52 select-text overflow-auto whitespace-pre-wrap break-words px-3 py-2 font-mono text-xs {entry.ok
                ? 'text-muted'
                : 'text-status-crit'}">{entry.ok ? entry.output : $maskText(entry.output)}</pre>
          {:else}
            <p class="px-3 py-2 text-xs text-faint">{$t('snippetResults.noOutput')}</p>
          {/if}
        </div>
      {/each}
    </div>

    <footer class="flex justify-end border-t border-default px-5 py-3">
      <Button variant="secondary" onclick={clearRun}>{$t('snippetResults.close')}</Button>
    </footer>
  </Modal>
{/if}
