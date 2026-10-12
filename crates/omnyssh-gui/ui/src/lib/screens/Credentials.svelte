<script lang="ts">
  // SSH credentials screen (identity-keychain plan §六): the keychain key
  // entries and reusable login identities of `identities.toml`, with their
  // reference badges. Import validates on the backend before anything is
  // stored; materials (PEM, passphrases, passwords) are sent in once and
  // never come back out. Refusal errors arrive localized (`keyInUse` /
  // `identityInUse` carry the count), so a refused delete just surfaces its
  // message under the list.
  import { onMount } from 'svelte';
  import { t } from 'svelte-i18n';
  import { open as dialogOpen } from '@tauri-apps/plugin-dialog';
  import type { IdentityDto, KeyEntryDto, PrivateKeyFileDto } from '$lib/bindings';
  import { Surface, Chip, Icon, Button } from '$lib/theme';
  import Modal from '$lib/components/Modal.svelte';
  import Select from '$lib/components/Select.svelte';
  import {
    listIdentities,
    listKeyEntries,
    deleteIdentity,
    deleteKeyEntry,
    readPrivateKeyFile,
    saveIdentity,
    saveKeyEntry
  } from '$lib/ipc/commands';
  import { lastError } from '$lib/stores/notifications';

  type Dialog =
    | { kind: 'import-key' }
    | { kind: 'edit-key'; entry: KeyEntryDto }
    | { kind: 'add-identity' }
    | { kind: 'edit-identity'; identity: IdentityDto }
    | { kind: 'delete-key'; entry: KeyEntryDto }
    | { kind: 'delete-identity'; identity: IdentityDto };

  let keys = $state<KeyEntryDto[]>([]);
  let identities = $state<IdentityDto[]>([]);
  let dialog = $state<Dialog | null>(null);
  let actionError = $state<string | null>(null);

  // Import-key form state (one dialog at a time, so a single set suffices).
  let importPem = $state('');
  let importFileName = $state('');
  let importEncrypted = $state(false);
  let importFileError = $state<string | null>(null);
  let importLabel = $state('');
  let importPassphrase = $state('');
  let importSaving = $state(false);

  // Edit-key form state.
  let editKeyId = $state<string | null>(null);
  let editKeyLabel = $state('');
  let editKeyPassphrase = $state('');
  let editKeySaving = $state(false);

  // Identity form state. `identityPassword === null` means unchanged (the
  // placeholder says so); `''` deletes; text replaces.
  let identityId = $state<string | null>(null);
  let identityLabel = $state('');
  let identityUsername = $state('');
  let identityPassword = $state<string | null>(null);
  let identityKeyId = $state('');
  let identitySaving = $state(false);

  const message = (e: unknown): string => (e instanceof Error ? e.message : String(e));

  async function refresh(): Promise<void> {
    try {
      const [k, i] = await Promise.all([listKeyEntries(), listIdentities()]);
      keys = k;
      identities = i;
    } catch (e) {
      lastError.set(message(e));
    }
  }

  onMount(refresh);

  const pill =
    'inline-flex items-center gap-1.5 rounded-full border border-default px-2.5 py-1 text-xs ' +
    'font-medium text-muted transition hover:border-strong hover:bg-accent hover:text-accent-fg ' +
    'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus';
  const iconBtn =
    'grid h-8 w-8 place-items-center rounded-lg text-muted transition hover:bg-surface-inset ' +
    'hover:text-fg focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus';
  const input =
    'w-full rounded-lg bg-surface-inset px-3 py-2 text-sm text-fg outline-none ' +
    'focus-visible:ring-2 focus-visible:ring-focus placeholder:text-faint';

  function openImport(): void {
    importPem = '';
    importFileName = '';
    importEncrypted = false;
    importFileError = null;
    importLabel = '';
    importPassphrase = '';
    importSaving = false;
    dialog = { kind: 'import-key' };
  }

  function openEditKey(entry: KeyEntryDto): void {
    editKeyId = entry.id;
    editKeyLabel = entry.label;
    editKeyPassphrase = '';
    editKeySaving = false;
    dialog = { kind: 'edit-key', entry };
  }

  function openAddIdentity(): void {
    identityId = null;
    identityLabel = '';
    identityUsername = '';
    identityPassword = '';
    identityKeyId = '';
    identitySaving = false;
    dialog = { kind: 'add-identity' };
  }

  function openEditIdentity(identity: IdentityDto): void {
    identityId = identity.id;
    identityLabel = identity.label;
    identityUsername = identity.username;
    identityPassword = null; // blank = keep the stored one
    identityKeyId = identity.keyId;
    identitySaving = false;
    dialog = { kind: 'edit-identity', identity };
  }

  async function pickKeyFile(): Promise<void> {
    const picked = await dialogOpen({ multiple: false });
    if (!picked || typeof picked !== 'string') return;
    importFileError = null;
    importFileName = picked.replace(/[\\/]+$/, '').split(/[\\/]/).pop() ?? picked;
    try {
      const read: PrivateKeyFileDto = await readPrivateKeyFile(picked);
      if (read.error) {
        importFileError = read.error;
        importPem = '';
      } else {
        importPem = read.pem;
        importEncrypted = read.encrypted;
        if (importLabel === '') importLabel = importFileName;
      }
    } catch (e) {
      importFileError = message(e);
    }
  }

  async function submitImport(): Promise<void> {
    importSaving = true;
    try {
      await saveKeyEntry(null, importLabel, importPem, importPassphrase);
      dialog = null;
      await refresh();
    } catch (e) {
      // A rejected passphrase keeps the dialog open: the form still holds the
      // picked file and label.
      actionError = message(e);
    } finally {
      importSaving = false;
    }
  }

  async function submitEditKey(): Promise<void> {
    if (!editKeyId) return;
    editKeySaving = true;
    try {
      // No PEM on an edit: the stored material stays. `passphrase` `null`
      // keeps the saved one; the blank field here means exactly that.
      await saveKeyEntry(editKeyId, editKeyLabel, null, null);
      dialog = null;
      await refresh();
    } catch (e) {
      actionError = message(e);
    } finally {
      editKeySaving = false;
    }
  }

  async function submitIdentity(): Promise<void> {
    identitySaving = true;
    try {
      await saveIdentity(
        identityId,
        identityLabel,
        identityUsername,
        identityPassword,
        identityKeyId
      );
      dialog = null;
      await refresh();
    } catch (e) {
      actionError = message(e);
    } finally {
      identitySaving = false;
    }
  }

  async function confirmDeleteKey(id: string): Promise<void> {
    try {
      await deleteKeyEntry(id);
      actionError = null;
    } catch (e) {
      actionError = message(e);
    }
    dialog = null;
    await refresh();
  }

  async function confirmDeleteIdentity(id: string): Promise<void> {
    try {
      await deleteIdentity(id);
      actionError = null;
    } catch (e) {
      actionError = message(e);
    }
    dialog = null;
    await refresh();
  }

  async function copyPublicKey(entry: KeyEntryDto): Promise<void> {
    try {
      await navigator.clipboard.writeText(entry.publicKey);
    } catch (e) {
      lastError.set(message(e));
    }
  }

  const keyLabel = (id: string): string => keys.find((k) => k.id === id)?.label ?? id;
</script>

<section class="flex h-full flex-col px-6 pb-6 pt-3">
  <div class="mb-2 flex items-center gap-3">
    <h1 class="text-lg font-semibold tracking-tight">{$t('credentials.title')}</h1>
    <div class="ml-auto flex items-center gap-2">
      <button type="button" class={pill} onclick={openImport}>
        <Icon name="key" size={13} />
        {$t('credentials.import')}
      </button>
      <button type="button" class={pill} onclick={openAddIdentity}>
        <Icon name="plus" size={13} />
        {$t('credentials.newIdentity')}
      </button>
    </div>
  </div>
  <p class="mb-5 text-sm text-muted">{$t('credentials.intro')}</p>

  {#if actionError}
    <div class="mb-4 rounded-lg border border-default bg-surface-inset px-4 py-3 text-sm text-fg">
      {actionError}
    </div>
  {/if}

  <div class="min-h-0 flex-1 space-y-6 overflow-y-auto">
    <div>
      <h2 class="mb-2 flex items-center gap-2 text-sm font-semibold text-muted">
        <Icon name="key" size={14} />
        {$t('credentials.keys')}
      </h2>
      {#if keys.length === 0}
        <p
          class="rounded-lg border border-dashed border-default px-4 py-6 text-center text-sm text-muted"
        >
          {$t('credentials.keysEmpty')}
        </p>
      {:else}
        <ul class="space-y-2">
          {#each keys as entry (entry.id)}
            <li>
              <Surface class="flex items-center gap-4 p-4">
                <div class="min-w-0 flex-1">
                  <div class="flex flex-wrap items-center gap-2">
                    <span class="truncate font-medium" title={entry.label}>{entry.label}</span>
                    <Chip variant={entry.refCount > 0 ? 'default' : 'outline'}>
                      {entry.refCount > 0
                        ? $t('credentials.refCount', { values: { count: entry.refCount } })
                        : $t('credentials.noRefs')}
                    </Chip>
                  </div>
                  <div
                    class="mt-1 truncate font-mono text-xs text-muted"
                    title={entry.fingerprint}
                  >
                    {entry.fingerprint}
                  </div>
                </div>
                <div class="flex shrink-0 items-center gap-1.5">
                  <button
                    type="button"
                    class={iconBtn}
                    title={$t('credentials.copyPub')}
                    aria-label={$t('credentials.copyPub')}
                    onclick={() => copyPublicKey(entry)}
                  >
                    <Icon name="copy" size={14} />
                  </button>
                  <button
                    type="button"
                    class={iconBtn}
                    title={$t('credentials.edit')}
                    aria-label={$t('credentials.edit')}
                    onclick={() => openEditKey(entry)}
                  >
                    <Icon name="edit" size={15} />
                  </button>
                  <button
                    type="button"
                    class={iconBtn}
                    title={$t('credentials.delete')}
                    aria-label={$t('credentials.delete')}
                    onclick={() => (dialog = { kind: 'delete-key', entry })}
                  >
                    <Icon name="trash" size={15} />
                  </button>
                </div>
              </Surface>
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    <div>
      <h2 class="mb-2 flex items-center gap-2 text-sm font-semibold text-muted">
        <Icon name="terminal" size={14} />
        {$t('credentials.identities')}
      </h2>
      {#if identities.length === 0}
        <p
          class="rounded-lg border border-dashed border-default px-4 py-6 text-center text-sm text-muted"
        >
          {$t('credentials.identitiesEmpty')}
        </p>
      {:else}
        <ul class="space-y-2">
          {#each identities as identity (identity.id)}
            <li>
              <Surface class="flex items-center gap-4 p-4">
                <div class="min-w-0 flex-1">
                  <div class="flex flex-wrap items-center gap-2">
                    <span class="truncate font-medium" title={identity.label}>
                      {identity.label}
                    </span>
                    <Chip variant={identity.refCount > 0 ? 'default' : 'outline'}>
                      {identity.refCount > 0
                        ? $t('credentials.refCount', { values: { count: identity.refCount } })
                        : $t('credentials.noRefs')}
                    </Chip>
                  </div>
                  <div class="mt-1 truncate text-xs text-muted">
                    {identity.username}
                    ·
                    {identity.keyId === ''
                      ? $t('credentials.passAuth')
                      : keyLabel(identity.keyId)}
                  </div>
                </div>
                <div class="flex shrink-0 items-center gap-1.5">
                  <button
                    type="button"
                    class={iconBtn}
                    title={$t('credentials.edit')}
                    aria-label={$t('credentials.edit')}
                    onclick={() => openEditIdentity(identity)}
                  >
                    <Icon name="edit" size={15} />
                  </button>
                  <button
                    type="button"
                    class={iconBtn}
                    title={$t('credentials.delete')}
                    aria-label={$t('credentials.delete')}
                    onclick={() => (dialog = { kind: 'delete-identity', identity })}
                  >
                    <Icon name="trash" size={15} />
                  </button>
                </div>
              </Surface>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  </div>
</section>

{#if dialog?.kind === 'import-key'}
  <Modal label={$t('credentials.importTitle')} onClose={() => (dialog = null)}>
    <form class="flex min-h-0 flex-col" onsubmit={(e) => { e.preventDefault(); void submitImport(); }}>
      <header class="border-b border-default px-5 py-3.5">
        <h2 class="text-sm font-semibold">{$t('credentials.importTitle')}</h2>
      </header>

      <div class="min-h-0 flex-1 space-y-3.5 overflow-y-auto px-5 py-4">
        <div class="space-y-1">
          <span class="block text-xs font-medium text-muted">{$t('credentials.keyFile')}</span>
          <button
            type="button"
            class="flex w-full items-center gap-2.5 rounded-lg border border-dashed border-strong px-3 py-2.5 text-sm text-fg transition hover:bg-surface-inset focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus"
            onclick={() => void pickKeyFile()}
          >
            <span class={importPem === '' ? 'text-faint' : 'text-status-ok'}>
              <Icon name="file" size={15} />
            </span>
            {#if importPem === ''}
              <span class="text-muted">{$t('credentials.pickFile')}</span>
            {:else}
              <span class="min-w-0 flex-1 truncate text-left" title={importFileName}>
                {importFileName}
              </span>
              <span class="shrink-0 text-status-ok">
                <Icon name="check" size={14} />
              </span>
            {/if}
          </button>
          {#if importFileError}
            <p class="text-xs text-status-crit">{$t('credentials.fileError')}: {importFileError}</p>
          {/if}
        </div>
        <label class="block space-y-1">
          <span class="text-xs font-medium text-muted">{$t('credentials.label')}</span>
          <input id="cred-key-label" class={input} bind:value={importLabel} required />
        </label>
        <label class="block space-y-1">
          <span class="text-xs font-medium text-muted">{$t('credentials.passphrase')}</span>
          <input
            id="cred-key-pass"
            class={input}
            type="password"
            bind:value={importPassphrase}
          />
          <p class="text-[11px] text-faint">
            {importEncrypted
              ? $t('credentials.encryptedKeyNeedsPassphrase')
              : $t('credentials.passphraseHint')}
          </p>
        </label>
      </div>

      <footer class="flex justify-end gap-2 border-t border-default px-5 py-3">
        <Button variant="ghost" onclick={() => (dialog = null)}>
          {$t('credentials.cancel')}
        </Button>
        <Button variant="primary" type="submit" disabled={importPem === '' || importSaving}>
          {$t('credentials.save')}
        </Button>
      </footer>
    </form>
  </Modal>
{:else if dialog?.kind === 'edit-key'}
  {@const entry = dialog.entry}
  <Modal label={$t('credentials.edit')} onClose={() => (dialog = null)}>
    <form class="flex min-h-0 flex-col" onsubmit={(e) => { e.preventDefault(); void submitEditKey(); }}>
      <header class="border-b border-default px-5 py-3.5">
        <h2 class="text-sm font-semibold">{entry.label}</h2>
      </header>

      <div class="min-h-0 flex-1 space-y-3.5 overflow-y-auto px-5 py-4">
        <label class="block space-y-1">
          <span class="text-xs font-medium text-muted">{$t('credentials.label')}</span>
          <input id="cred-edit-label" class={input} bind:value={editKeyLabel} required />
        </label>
        <div class="space-y-1">
          <span class="text-xs font-medium text-muted">{$t('credentials.passphrase')}</span>
          <p class="text-[11px] text-faint">{$t('credentials.passphraseHint')}</p>
        </div>
      </div>

      <footer class="flex justify-end gap-2 border-t border-default px-5 py-3">
        <Button variant="ghost" onclick={() => (dialog = null)}>
          {$t('credentials.cancel')}
        </Button>
        <Button variant="primary" type="submit" disabled={editKeySaving}>
          {$t('credentials.save')}
        </Button>
      </footer>
    </form>
  </Modal>
{:else if dialog?.kind === 'add-identity' || dialog?.kind === 'edit-identity'}
  <Modal
    label={identityId === null ? $t('credentials.newIdentity') : $t('credentials.identityTitle')}
    onClose={() => (dialog = null)}
  >
    <form class="flex min-h-0 flex-col" onsubmit={(e) => { e.preventDefault(); void submitIdentity(); }}>
      <header class="border-b border-default px-5 py-3.5">
        <h2 class="text-sm font-semibold">
          {identityId === null ? $t('credentials.newIdentity') : $t('credentials.identityTitle')}
        </h2>
      </header>

      <div class="min-h-0 flex-1 space-y-3.5 overflow-y-auto px-5 py-4">
        <label class="block space-y-1">
          <span class="text-xs font-medium text-muted">{$t('credentials.label')}</span>
          <input id="cred-id-label" class={input} bind:value={identityLabel} required />
        </label>
        <label class="block space-y-1">
          <span class="text-xs font-medium text-muted">{$t('credentials.username')}</span>
          <input id="cred-id-user" class={input} bind:value={identityUsername} required />
        </label>
        <label class="block space-y-1">
          <span class="text-xs font-medium text-muted">{$t('credentials.bindKey')}</span>
          <Select id="cred-id-key" class={input} bind:value={identityKeyId}>
            <option value="">{$t('credentials.bindKeyNone')}</option>
            {#each keys as entry (entry.id)}
              <option value={entry.id}>{entry.label}</option>
            {/each}
          </Select>
        </label>
        <label class="block space-y-1">
          <span class="text-xs font-medium text-muted">{$t('credentials.password')}</span>
          <input
            id="cred-id-pass"
            class={input}
            type="password"
            bind:value={identityPassword}
            placeholder={identityPassword === null ? '••••••••' : ''}
          />
          <p class="text-[11px] text-faint">{$t('credentials.passwordHint')}</p>
        </label>
      </div>

      <footer class="flex justify-end gap-2 border-t border-default px-5 py-3">
        <Button variant="ghost" onclick={() => (dialog = null)}>
          {$t('credentials.cancel')}
        </Button>
        <Button variant="primary" type="submit" disabled={identitySaving}>
          {$t('credentials.save')}
        </Button>
      </footer>
    </form>
  </Modal>
{:else if dialog?.kind === 'delete-key'}
  {@const entry = dialog.entry}
  <Modal
    label={$t('credentials.deleteTitle', { values: { name: entry.label } })}
    onClose={() => (dialog = null)}
  >
    <form
      onsubmit={(e) => {
        e.preventDefault();
        void confirmDeleteKey(entry.id);
      }}
    >
      <header class="border-b border-default px-5 py-3.5">
        <h2 class="text-sm font-semibold">
          {$t('credentials.deleteTitle', { values: { name: entry.label } })}
        </h2>
      </header>
      <p class="px-5 py-4 text-sm text-muted">{$t('credentials.deleteHint')}</p>
      <footer class="flex justify-end gap-2 border-t border-default px-5 py-3">
        <Button variant="ghost" onclick={() => (dialog = null)}>
          {$t('credentials.cancel')}
        </Button>
        <Button variant="primary" type="submit">
          <Icon name="trash" size={13} />
          {$t('credentials.delete')}
        </Button>
      </footer>
    </form>
  </Modal>
{:else if dialog?.kind === 'delete-identity'}
  {@const identity = dialog.identity}
  <Modal
    label={$t('credentials.deleteTitle', { values: { name: identity.label } })}
    onClose={() => (dialog = null)}
  >
    <form
      onsubmit={(e) => {
        e.preventDefault();
        void confirmDeleteIdentity(identity.id);
      }}
    >
      <header class="border-b border-default px-5 py-3.5">
        <h2 class="text-sm font-semibold">
          {$t('credentials.deleteTitle', { values: { name: identity.label } })}
        </h2>
      </header>
      <p class="px-5 py-4 text-sm text-muted">{$t('credentials.deleteHint')}</p>
      <footer class="flex justify-end gap-2 border-t border-default px-5 py-3">
        <Button variant="ghost" onclick={() => (dialog = null)}>
          {$t('credentials.cancel')}
        </Button>
        <Button variant="primary" type="submit">
          <Icon name="trash" size={13} />
          {$t('credentials.delete')}
        </Button>
      </footer>
    </form>
  </Modal>
{/if}
