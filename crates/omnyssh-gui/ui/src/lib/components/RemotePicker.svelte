<script lang="ts">
  // A remote file picker over the terminal tab's own SFTP session (tech-gui.md §4.2).
  // Browsing drives the same plumbing the SFTP screen uses — `sftpList` out, the
  // `sftp-dir-listed` event into the shared store's remote pane — but scoped to the
  // picker: it only reads the pane it is handed and never marks or transfers from the
  // store, so the terminal's transfer strip is untouched until the owner confirms the
  // download. A row click picks a file (owner saves it) or enters a folder; `..` goes
  // up. Layout copies SftpPane's header/list grammar (path line, `..` first, folders
  // before files) without the checkbox/drag machinery a picker has no use for.
  import { t } from 'svelte-i18n';
  import type { FileEntryDto } from '$lib/bindings';
  import { formatBytes, type Pane } from '$lib/stores/sftp';
  import { maskText } from '$lib/stores/streamer';
  import { manualDownloadReady } from '$lib/terminalDownload';
  import { Icon } from '$lib/theme';

  let {
    pane,
    onBrowse,
    onManual,
    onClose,
    onPick
  }: {
    /** The store pane this picker's session browses (derived by the owner). */
    pane: Pane | undefined;
    /** Enter a directory (a row click on a folder or `..`): keep browsing. */
    onBrowse: (path: string) => void;
    /** The manual-path field's Enter: an absolute path to download sight unseen. */
    onManual: (path: string) => void;
    onClose: () => void;
    onPick: (entry: FileEntryDto) => void;
  } = $props();

  // The manual-path field doubles as the filter for the listing below: a relative or
  // partial fragment narrows the rows to matching names, and an absolute path jumps.
  let filter = $state('');

  const filterTrimmed = $derived(filter.trim());
  // A filter that looks like an absolute path is a jump target, not a name filter:
  // narrowing rows while the user means "go here" reads as the list refusing to move.
  const filtering = $derived(filterTrimmed.length > 0 && !filterTrimmed.startsWith('/'));

  const filtered = $derived.by(() => {
    const entries = pane?.entries ?? [];
    if (!filtering) return entries;
    const q = filterTrimmed.toLowerCase();
    // `..` always stays: it is the way out, not a result.
    return entries.filter((e) => e.name === '..' || e.name.toLowerCase().includes(q));
  });

  const manualReady = $derived(manualDownloadReady(filterTrimmed));

  function onFilterKeydown(e: KeyboardEvent): void {
    if (e.key === 'Enter' && manualReady) {
      e.preventDefault();
      onManual(filterTrimmed);
      filter = '';
    } else if (e.key === 'Escape') {
      // The modal's own Escape handler still fires; clearing the field first keeps a
      // half-typed fragment from reappearing when the picker is reopened.
      if (filter) e.stopPropagation();
      filter = '';
    }
  }

  function pick(entry: FileEntryDto): void {
    filter = '';
    // `..` and folder rows keep browsing; only the manual field's Enter reaches
    // the owner's download-by-path flow.
    if (entry.isDir) onBrowse(entry.path);
    else onPick(entry);
  }

  /** 文件夹行的下载动作：行本体是“进入”，这个按钮才是“下载它”。 */
  function downloadFolder(entry: FileEntryDto, e: MouseEvent): void {
    e.stopPropagation();
    filter = '';
    onPick(entry);
  }
</script>

<div class="flex min-h-0 flex-col">
  <!-- Manual path entry above the browser: an absolute path downloads it directly
       (a path a listing cannot reach — a symlink outside, a hidden mount), anything
       else narrows the listing below. -->
  <div class="shrink-0 px-5 pb-3 pt-4">
    <!-- svelte-ignore a11y_autofocus -->
    <input
      autofocus
      bind:value={filter}
      onkeydown={onFilterKeydown}
      class="w-full rounded-lg bg-surface-inset px-3 py-2 font-mono text-sm text-fg outline-none
        focus-visible:ring-2 focus-visible:ring-focus placeholder:text-faint"
      placeholder={$t('terminal.pickerPathPlaceholder')}
      aria-label={$t('terminal.remotePath')}
      spellcheck="false"
    />
    <p class="mt-2 text-xs text-faint">{$t('terminal.downloadHint')}</p>
  </div>

  <!-- Current path: truncated from the left so the deep end stays visible. -->
  <div class="flex shrink-0 items-center justify-between gap-2 border-y border-default px-5 py-2">
    <span
      class="min-w-0 truncate font-mono text-xs text-faint [direction:rtl] [text-align:left]"
      title={pane?.path || ''}
    >
      {pane?.path || '—'}
    </span>
    {#if pane?.loading && (pane?.entries.length ?? 0) > 0}
      <span class="shrink-0 text-xs text-faint">{$t('sftp.loading')}</span>
    {/if}
  </div>

  <div class="min-h-0 flex-1 overflow-y-auto px-3 py-2" style="height: 40vh">
    {#if pane?.error}
      <p class="px-2 py-6 text-center text-sm text-status-crit">{$maskText(pane.error)}</p>
    {:else if pane?.loading && (pane?.entries.length ?? 0) === 0}
      <p class="px-2 py-6 text-center text-sm text-faint">{$t('sftp.loading')}</p>
    {:else if filtered.length === 0}
      <p class="px-2 py-6 text-center text-sm text-faint">
        {filtering ? $t('terminal.pickerNoMatch') : $t('sftp.emptyDirectory')}
      </p>
    {:else}
      <ul class="space-y-0.5">
        {#each filtered as entry (entry.path)}
          {@const isParent = entry.name === '..'}
          <li>
            <!-- A row is a div with button semantics, not a `<button>`: the folder
                 download action nested inside would be invalid button-in-button HTML. -->
            <div
              role="button"
              tabindex="0"
              class="flex w-full min-w-0 cursor-default items-center gap-2 rounded px-2 py-1.5
                text-left text-sm text-muted transition hover:bg-surface-inset hover:text-fg
                focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus"
              onclick={() => pick(entry)}
              onkeydown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault();
                  pick(entry);
                }
              }}
            >
              <Icon name={entry.isDir ? 'folder' : 'file'} size={15} />
              <span class="min-w-0 flex-1 truncate {entry.isDir ? 'font-medium text-fg' : ''}">
                {entry.name}
              </span>
              {#if !entry.isDir}
                <span class="shrink-0 tabular-nums text-xs text-faint">
                  {formatBytes(entry.size)}
                </span>
              {/if}
              {#if entry.isDir && !isParent}
                <!-- The stock chevron points down; a right turn reads as "enter".
                     The trailing download button picks the folder itself — the row's
                     click enters it — with the same cue the menu entry uses. -->
                <button
                  type="button"
                  class="grid h-6 w-6 shrink-0 place-items-center rounded text-faint transition
                    hover:bg-accent hover:text-accent-fg focus-visible:outline-none
                    focus-visible:ring-2 focus-visible:ring-focus"
                  aria-label={$t('terminal.menuDownload')}
                  title={$t('terminal.menuDownload')}
                  onclick={(e) => downloadFolder(entry, e)}
                >
                  <Icon name="download" size={13} />
                </button>
                <Icon name="chevron" size={13} class="-rotate-90 shrink-0 text-faint" />
              {/if}
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  <footer class="flex shrink-0 justify-end border-t border-default px-5 py-3">
    <button
      type="button"
      class="rounded-full px-4 py-2 text-sm text-muted transition hover:bg-surface-inset hover:text-fg"
      onclick={onClose}
    >
      {$t('sftp.cancel')}
    </button>
  </footer>
</div>
