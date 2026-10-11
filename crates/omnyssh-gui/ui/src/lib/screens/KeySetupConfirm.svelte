<script lang="ts">
  // Asks before a key setup (tech-gui.md §4.3 note, 2026-10-09): it changes this
  // computer and, with sudo, the server's login for everyone. `rerun` is the "Password
  // on" badge's second try, which reuses the key OmnySSH made before.
  import { onMount } from 'svelte';
  import { t } from 'svelte-i18n';
  import type { HostDto } from '$lib/bindings';
  import Modal from '$lib/components/Modal.svelte';
  import { Button } from '$lib/theme';
  import { keyFilePath } from '$lib/stores/keySetup';
  import { streamerMode, displayHostname } from '$lib/stores/streamer';
  import { isWindows } from '$lib/platform';

  let {
    host,
    rerun,
    onConfirm,
    onCancel
  }: { host: HostDto; rerun: boolean; onConfirm: () => void; onCancel: () => void } = $props();

  const keyPath = $derived(keyFilePath(host.name, isWindows));

  // Cancel is first in `actions`. Focused by hand: Modal sets no initial focus, and
  // `autofocus` is skipped while the opening button still holds focus.
  let actions = $state<HTMLElement>();
  onMount(() => actions?.querySelector('button')?.focus());
</script>

<Modal
  label={rerun ? $t('keySetup.confirmRerunModal') : $t('keySetup.confirmModal')}
  onClose={onCancel}
>
  <div class="space-y-3 px-5 py-4">
    <h2 class="text-sm font-semibold">
      {rerun
        ? $t('keySetup.confirmRerunHeading', { values: { host: host.name } })
        : $t('keySetup.confirmHeading', { values: { host: host.name } })}
    </h2>
    <ul class="list-disc space-y-1.5 pl-5 text-sm text-muted">
      {#if rerun}
        <li>
          {$t('keySetup.confirmStepReuse')}
          <span class="break-all font-mono text-xs">{keyPath}</span>
        </li>
      {:else}
        <li>
          {$t('keySetup.confirmStepCreate')}
          <span class="break-all font-mono text-xs">{keyPath}</span>
          {$t('keySetup.confirmStepNoPassphrase')}
        </li>
      {/if}
      <li>
        {$t('keySetup.confirmStepAdd')}
        <span class="break-all font-mono text-xs"
          >{host.user}@{displayHostname(host.hostname, $streamerMode)}</span
        >
        {$t('keySetup.confirmStepAddMid')}
      </li>
      <li>{$t('keySetup.confirmStepDisable')}</li>
    </ul>
    <p class="text-xs text-faint">{$t('keySetup.confirmNote')}</p>
    <div class="flex justify-end gap-2 pt-1" bind:this={actions}>
      <Button variant="ghost" onclick={onCancel}>{$t('keySetup.cancel')}</Button>
      <Button variant="primary" onclick={onConfirm}>
        {rerun ? $t('keySetup.confirmRerun') : $t('keySetup.confirm')}
      </Button>
    </div>
  </div>
</Modal>
