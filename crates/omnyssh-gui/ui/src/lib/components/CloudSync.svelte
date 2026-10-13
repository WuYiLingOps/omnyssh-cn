<script lang="ts">
  // Cloud Sync settings section.
  // GitHub connect via Device Flow, sync password (set / unlock / change),
  // auto-sync and sync-passwords switches, a sync-status row (state badge +
  // version + last error, DockPilot-style) and the Gist revision history. The
  // three decision dialogs (conflict / blocked / empty vault) mount here too:
  // the engine parks those states until the user acts, so presenting them
  // where the controls live keeps one owner.
  import { t } from 'svelte-i18n';
  import { Surface, Icon } from '$lib/theme';
  import Modal from '$lib/components/Modal.svelte';
  import { openExternal } from '$lib/ipc/openExternal';
  import { lastError } from '$lib/stores/notifications';
  import { toast } from '$lib/stores/toast';
  import { cloudSync, syncActions } from '$lib/stores/cloudSync';
  import * as engine from '$lib/sync/engine';
  import type { DeviceFlowStart, SyncState } from '$lib/sync/types';

  const message = (e: unknown): string => (e instanceof Error ? e.message : String(e));

  type ConnectState =
    | { kind: 'idle' }
    | { kind: 'starting' }
    | { kind: 'code'; start: DeviceFlowStart }
    | { kind: 'polling'; start: DeviceFlowStart }
    | { kind: 'error'; message: string };

  let connect = $state<ConnectState>({ kind: 'idle' });

  // Password inputs
  let new_password = $state('');
  let new_password2 = $state('');
  let unlock_password = $state('');
  let old_password = $state('');
  let change_new_password = $state('');
  let busy = $state(false);

  // History
  let historyOpen = $state(false);
  let history = $state<Array<{ sha: string; date: number }>>([]);
  let historyLoading = $state(false);

  // Dialog-visible state (driven off the engine state machine)
  let conflictPassword = $state('');
  let confirmPushLocal = $state(false); // empty-vault "push local" needs a second click

  // Merge-conflict note: a merge resolves both-sides-modified fields local-first,
  // so the remote edit is dropped — surface that instead of swallowing it. Fires
  // off the engine's per-round count, auto-clears (a stale warning outlives its
  // usefulness once the next sync reconciles).
  let mergeConflictNote = $state<number | null>(null);
  let mergeNoteTimer: ReturnType<typeof setTimeout> | undefined;
  const MERGE_NOTE_TTL_MS = 8000;
  $effect(() => {
    const count = $cloudSync.lastMergeConflicts;
    if (count == null) return;
    mergeConflictNote = count;
    if (mergeNoteTimer !== undefined) clearTimeout(mergeNoteTimer);
    mergeNoteTimer = setTimeout(() => {
      mergeNoteTimer = undefined;
      mergeConflictNote = null;
    }, MERGE_NOTE_TTL_MS);
  });

  const s = $derived($cloudSync);

  // Sync-state badge (DockPilot's SYNC_STATE_META): one pill that answers
  // "is sync healthy right now" without reading the fine print.
  const SYNC_STATE_META: Record<SyncState, { key: string; cls: string }> = {
    IDLE: { key: 'cloudSync.stateIdle', cls: 'border-default text-muted' },
    SYNCING: { key: 'cloudSync.stateSyncing', cls: 'border-accent text-fg' },
    CONFLICT: { key: 'cloudSync.stateConflict', cls: 'border-status-warn text-status-warn' },
    BLOCKED: { key: 'cloudSync.stateBlocked', cls: 'border-status-crit text-status-crit' },
    ERROR: { key: 'cloudSync.stateError', cls: 'border-status-crit text-status-crit' }
  };

  const fmtTime = (ts: number | null): string => {
    if (!ts) return '';
    const d = new Date(ts);
    return d.toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' });
  };

  const lastSyncLabel = $derived.by(() => {
    if (!s.lastSyncAt) return $t('cloudSync.neverSynced');
    const diff = Date.now() - s.lastSyncAt;
    if (diff < 60_000) return $t('cloudSync.justNow');
    if (diff < 3_600_000)
      return $t('cloudSync.minutesAgo', { values: { count: Math.floor(diff / 60_000) } });
    return fmtTime(s.lastSyncAt);
  });

  // ------------------------------------------------------------------ connect

  async function startConnect(): Promise<void> {
    connect = { kind: 'starting' };
    try {
      const start = await engine.startDeviceFlow();
      connect = { kind: 'code', start };
      // Poll in the background; the modal shows the user code meanwhile.
      try {
        connect = { kind: 'polling', start };
        await engine.completeGitHubAuth(start);
        // Success closes the dialog; the status row now shows the account.
        connect = { kind: 'idle' };
        toast.ok($t('cloudSync.toastConnected'));
      } catch (e) {
        connect = { kind: 'error', message: message(e) };
        toast.error($t('cloudSync.toastConnectFailed', { values: { error: message(e) } }));
      }
    } catch (e) {
      connect = { kind: 'error', message: message(e) };
      toast.error($t('cloudSync.toastConnectFailed', { values: { error: message(e) } }));
    }
  }

  function cancelConnect(): void {
    connect = { kind: 'idle' };
  }

  function openDevicePage(): void {
    // The system browser via tauri-plugin-opener: window.open is a no-op in the
    // Tauri WebView (and the device page is whitelisted in capabilities).
    if (connect.kind === 'code' || connect.kind === 'polling') {
      openExternal(connect.start.verificationUri).catch((e) => {
        lastError.set(message(e));
      });
    }
  }

  async function copyCode(): Promise<void> {
    if (connect.kind !== 'code' && connect.kind !== 'polling') return;
    try {
      await navigator.clipboard.writeText(connect.start.userCode);
      toast.ok($t('cloudSync.toastCodeCopied'));
    } catch {
      // Clipboard blocked (webview permission / focus): the user can still
      // select the code by hand — tell them instead of failing silently.
      toast.error($t('cloudSync.toastCodeCopyFailed'));
    }
  }

  // ------------------------------------------------------------------ actions

  async function run(action: () => Promise<void>): Promise<void> {
    if (busy) return;
    busy = true;
    try {
      await action();
    } catch (e) {
      lastError.set(message(e));
    } finally {
      busy = false;
    }
  }

  const syncNow = (): Promise<void> => run(() => syncActions.manualSync().then(() => {}));

  const setPassword = (): Promise<void> =>
    run(async () => {
      if (new_password.length < 8) throw new Error($t('cloudSync.passwordTooShort'));
      if (new_password !== new_password2) throw new Error($t('cloudSync.passwordMismatch'));
      await syncActions.setPassword(new_password);
      new_password = '';
      new_password2 = '';
    });

  const unlock = (): Promise<void> =>
    run(async () => {
      const ok = await syncActions.unlockAndRemember(unlock_password);
      if (!ok) throw new Error($t('cloudSync.wrongPassword'));
      unlock_password = '';
    });

  const changePassword = (): Promise<void> =>
    run(async () => {
      if (change_new_password.length < 8) throw new Error($t('cloudSync.passwordTooShort'));
      const ok = await syncActions.changePassword(old_password, change_new_password);
      if (!ok) throw new Error($t('cloudSync.wrongPassword'));
      old_password = '';
      change_new_password = '';
    });

  const toggleHistory = async (): Promise<void> => {
    historyOpen = !historyOpen;
    if (historyOpen) {
      historyLoading = true;
      try {
        history = await syncActions.fetchHistory();
      } catch (e) {
        lastError.set(message(e));
        history = [];
      } finally {
        historyLoading = false;
      }
    }
  };

  const restore = (sha: string): Promise<void> =>
    run(async () => {
      await syncActions.restoreRevision(sha);
      history = await syncActions.fetchHistory();
    });

  // Dialog actions
  const useRemote = (): Promise<void> =>
    run(async () => {
      const ok = await syncActions.resolveConflictUseRemote(conflictPassword);
      if (!ok.success) throw new Error(ok.error ?? '');
      conflictPassword = '';
    });

  const useLocal = (): Promise<void> =>
    run(async () => {
      await syncActions.resolveConflictUseLocal();
      conflictPassword = '';
    });

  const restoreRemote = (): Promise<void> =>
    run(async () => {
      const ok = await syncActions.restoreRemoteFromBlocked();
      if (!ok.success) throw new Error(ok.error ?? '');
    });

  const pushLocal = (): Promise<void> =>
    run(async () => {
      await syncActions.forcePushLocal();
      confirmPushLocal = false;
    });

  const emptyRestore = (): Promise<void> =>
    run(async () => {
      const ok = await syncActions.confirmEmptyVaultRestore();
      if (!ok.success) throw new Error(ok.error ?? '');
    });

  const emptyPush = (): Promise<void> =>
    run(async () => {
      if (!confirmPushLocal) {
        confirmPushLocal = true;
        return;
      }
      const ok = await syncActions.confirmEmptyVaultPush();
      confirmPushLocal = false;
      if (!ok.success) throw new Error(ok.error ?? '');
    });
</script>

  <Surface class="p-5">
    {#if !s.connected}
      <!-- Not connected: the entry point -->
      <div class="flex items-center justify-between gap-4">
        <div class="min-w-0">
          <p class="text-sm">{$t('cloudSync.githubAccount')}</p>
          <p class="text-xs text-muted">{$t('cloudSync.connectHint')}</p>
        </div>
        <button
          type="button"
          class="inline-flex shrink-0 items-center gap-1.5 rounded-full border border-strong px-4 py-1.5 text-sm text-fg transition hover:bg-accent hover:text-accent-fg disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus"
          disabled={busy || connect.kind === 'starting' || connect.kind === 'polling'}
          onclick={startConnect}
        >
          <Icon name="refresh" size={14} class={connect.kind === 'starting' ? 'animate-spin' : ''} />
          {connect.kind === 'starting' || connect.kind === 'polling'
            ? $t('cloudSync.connecting')
            : $t('cloudSync.connect')}
        </button>
      </div>
      {#if connect.kind === 'error'}
        <p class="mt-2 text-xs text-status-warn">
          {$t('cloudSync.connectFailed', { values: { error: connect.message } })}
        </p>
      {/if}
    {:else}
      <!-- Connected: account + actions -->
      <div class="flex items-center justify-between gap-4">
        <div class="flex min-w-0 items-center gap-3">
          {#if s.account?.avatarUrl}
            <img src={s.account.avatarUrl} alt="" class="h-8 w-8 rounded-full" />
          {/if}
          <div class="min-w-0">
            <p class="truncate text-sm">{s.account?.name ?? s.account?.login ?? $t('cloudSync.githubAccount')}</p>
            <p class="text-xs text-muted">
              {s.gistId
                ? $t('cloudSync.gistLinked')
                : $t('cloudSync.gistMissing')}
            </p>
          </div>
        </div>
        <div class="flex shrink-0 items-center gap-2">
          <button
            type="button"
            class="inline-flex items-center gap-1.5 rounded-full border border-strong px-3 py-1.5 text-sm text-fg transition hover:bg-accent hover:text-accent-fg disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus"
            disabled={busy || s.syncing || s.securityState !== 'UNLOCKED'}
            onclick={syncNow}
          >
            <Icon name="refresh" size={14} class={s.syncing ? 'animate-spin' : ''} />
            {$t('cloudSync.syncNow')}
          </button>
          <button
            type="button"
            class="inline-flex items-center gap-1.5 rounded-full border border-strong px-3 py-1.5 text-sm text-muted transition hover:bg-surface-inset hover:text-fg disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus"
            disabled={busy}
            onclick={() =>
              run(async () => {
                await syncActions.disconnect();
                toast.ok($t('cloudSync.toastDisconnected'));
              })}
          >
            {$t('cloudSync.disconnect')}
          </button>
        </div>
      </div>

      {#if connect.kind === 'error'}
        <p class="mt-2 text-xs text-status-warn">
          {$t('cloudSync.connectFailed', { values: { error: connect.message } })}
        </p>
      {/if}

      {#if mergeConflictNote != null}
        <p class="mt-2 text-xs text-status-warn">
          {$t('cloudSync.mergeConflictNote', { values: { count: mergeConflictNote } })}
        </p>
      {/if}

      <!-- Sync status row (DockPilot-style): the state pill answers "is sync
           healthy" at a glance; the fine print carries the version, the last
           run and the error inline so a failure needs no console. -->
      <div class="mt-4 flex items-center justify-between gap-4 border-t border-default pt-4">
        <div class="min-w-0">
          <div class="flex items-center gap-2">
            <p class="text-sm">{$t('cloudSync.statusLabel')}</p>
            <span
              class="shrink-0 rounded-full border px-2 py-0.5 text-[10px] {SYNC_STATE_META[s.syncState].cls}"
            >
              {$t(SYNC_STATE_META[s.syncState].key)}
            </span>
            {#if s.lastSyncVersion != null}
              <span class="shrink-0 rounded-full border border-default px-2 py-0.5 text-[10px] text-muted tabular-nums">
                v{s.lastSyncVersion}
              </span>
            {/if}
          </div>
          <p class="mt-0.5 truncate text-xs text-muted" title={s.lastError ?? ''}>
            {lastSyncLabel}{s.lastError ? ` · ${s.lastError}` : ''}
          </p>
        </div>
      </div>

      <!-- Auto sync -->
      <div class="mt-4 flex items-center justify-between gap-4 border-t border-default pt-4">
        <div>
          <p class="text-sm">{$t('cloudSync.autoSync')}</p>
          <p class="text-xs text-muted">{$t('cloudSync.autoSyncHint')}</p>
        </div>
        <button
          type="button"
          role="switch"
          aria-checked={s.autoSync}
          aria-label={$t('cloudSync.autoSync')}
          onclick={() => syncActions.setAutoSync(!s.autoSync)}
          class="relative h-6 w-11 shrink-0 rounded-full transition focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus {s.autoSync
            ? 'bg-accent'
            : 'bg-surface-inset'}"
        >
          <span
            class="absolute top-0.5 h-5 w-5 rounded-full bg-surface shadow-soft transition-[left] {s.autoSync
              ? 'left-[1.375rem]'
              : 'left-0.5'}"
          ></span>
        </button>
      </div>

      <!-- Sync passwords: the one genuinely dangerous switch -->
      <div class="mt-4 flex items-center justify-between gap-4 border-t border-default pt-4">
        <div class="min-w-0">
          <p class="text-sm">{$t('cloudSync.syncPasswords')}</p>
          <p class="text-xs text-muted">{$t('cloudSync.syncPasswordsHint')}</p>
        </div>
        <button
          type="button"
          role="switch"
          aria-checked={s.securityState === 'UNLOCKED' && s.syncPasswords}
          aria-label={$t('cloudSync.syncPasswords')}
          disabled={s.securityState !== 'UNLOCKED'}
          onclick={() => syncActions.setSyncPasswords(!s.syncPasswords)}
          class="relative h-6 w-11 shrink-0 rounded-full transition disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus {s.syncPasswords
            ? 'bg-accent'
            : 'bg-surface-inset'}"
        >
          <span
            class="absolute top-0.5 h-5 w-5 rounded-full bg-surface shadow-soft transition-[left] {s.syncPasswords
              ? 'left-[1.375rem]'
              : 'left-0.5'}"
          ></span>
        </button>
      </div>

      <!-- Sync credential materials: PEMs, key passphrases, identity
           passwords. Metadata always syncs; these ride the payload only when
           the user opts in (envelope-encrypted by the sync password). -->
      <div class="mt-4 flex items-center justify-between gap-4 border-t border-default pt-4">
        <div class="min-w-0">
          <p class="text-sm">{$t('cloudSync.syncCredentials')}</p>
          <p class="text-xs text-muted">{$t('cloudSync.syncCredentialsHint')}</p>
        </div>
        <button
          type="button"
          role="switch"
          aria-checked={s.securityState === 'UNLOCKED' && s.syncCredentials}
          aria-label={$t('cloudSync.syncCredentials')}
          disabled={s.securityState !== 'UNLOCKED'}
          onclick={() => syncActions.setSyncCredentials(!s.syncCredentials)}
          class="relative h-6 w-11 shrink-0 rounded-full transition disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus {s.syncCredentials
            ? 'bg-accent'
            : 'bg-surface-inset'}"
        >
          <span
            class="absolute top-0.5 h-5 w-5 rounded-full bg-surface shadow-soft transition-[left] {s.syncCredentials
              ? 'left-[1.375rem]'
              : 'left-0.5'}"
          ></span>
        </button>
      </div>

      <!-- History -->
      <div class="mt-4 border-t border-default pt-4">
        <button
          type="button"
          class="flex w-full items-center justify-between text-sm transition hover:text-fg focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus"
          onclick={toggleHistory}
        >
          <span class="flex items-center gap-2"><Icon name="layers" size={14} /> {$t('cloudSync.history')}</span>
          <span class="text-muted"><Icon name={historyOpen ? 'collapse' : 'expand'} size={14} /></span>
        </button>
        {#if historyOpen}
          {#if historyLoading}
            <p class="mt-2 text-xs text-muted">{$t('cloudSync.loadingHistory')}</p>
          {:else if history.length === 0}
            <p class="mt-2 text-xs text-muted">{$t('cloudSync.noHistory')}</p>
          {:else}
            <ul class="mt-2 space-y-1">
              {#each history.slice(0, 10) as h, i (h.sha)}
                <li class="flex items-center justify-between gap-2 text-xs">
                  <span class="min-w-0 truncate text-muted tabular-nums">
                    {fmtTime(h.date)}{i === 0 ? ` · ${$t('cloudSync.currentVersion')}` : ''}
                  </span>
                  <button
                    type="button"
                    class="shrink-0 rounded-full border border-strong px-3 py-1 text-xs transition hover:bg-accent hover:text-accent-fg disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus"
                    disabled={busy || s.syncing || i === 0}
                    title={$t('cloudSync.restore')}
                    onclick={() => restore(h.sha)}
                  >
                    {$t('cloudSync.restore')}
                  </button>
                </li>
              {/each}
            </ul>
            <p class="mt-2 text-xs text-muted">{$t('cloudSync.historyHint')}</p>
          {/if}
        {/if}
      </div>
    {/if}

    <!-- Sync password lifecycle: pure local cryptography (key derivation + secret
         store), independent of the GitHub connection — so it shows before connect
         too, letting the user establish the encryption key first. -->
    <div class="mt-4 space-y-2 border-t border-default pt-4">
      <div class="flex items-center gap-2">
        <Icon name="shield" size={14} />
        <p class="text-sm">
          {s.securityState === 'NO_KEY'
            ? $t('cloudSync.setPassword')
            : s.securityState === 'LOCKED'
              ? $t('cloudSync.unlock')
              : $t('cloudSync.passwordUnlocked')}
        </p>
      </div>
      {#if s.securityState === 'NO_KEY'}
        <p class="text-xs text-muted">{$t('cloudSync.setPasswordHint')}</p>
        <div class="flex flex-wrap items-center gap-2">
          <input
            type="password"
            bind:value={new_password}
            placeholder={$t('cloudSync.passwordPlaceholder')}
            class="w-48 rounded-lg border border-default bg-surface-inset px-3 py-1.5 text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus"
          />
          <input
            type="password"
            bind:value={new_password2}
            placeholder={$t('cloudSync.passwordRepeat')}
            class="w-48 rounded-lg border border-default bg-surface-inset px-3 py-1.5 text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus"
          />
          <button
            type="button"
            class="rounded-full border border-strong px-4 py-1.5 text-sm transition hover:bg-accent hover:text-accent-fg disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus"
            disabled={busy || !new_password || !new_password2}
            onclick={setPassword}
          >
            {$t('cloudSync.save')}
          </button>
        </div>
      {:else if s.securityState === 'LOCKED'}
        <p class="text-xs text-muted">{$t('cloudSync.unlockHint')}</p>
        <div class="flex flex-wrap items-center gap-2">
          <input
            type="password"
            bind:value={unlock_password}
            placeholder={$t('cloudSync.passwordPlaceholder')}
            class="w-48 rounded-lg border border-default bg-surface-inset px-3 py-1.5 text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus"
          />
          <button
            type="button"
            class="rounded-full border border-strong px-4 py-1.5 text-sm transition hover:bg-accent hover:text-accent-fg disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus"
            disabled={busy || !unlock_password}
            onclick={unlock}
          >
            {$t('cloudSync.unlockAction')}
          </button>
        </div>
      {:else}
        <p class="text-xs text-muted">{$t('cloudSync.lockHint')}</p>
        <div class="flex flex-wrap items-center gap-2">
          <input
            type="password"
            bind:value={old_password}
            placeholder={$t('cloudSync.oldPassword')}
            class="w-44 rounded-lg border border-default bg-surface-inset px-3 py-1.5 text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus"
          />
          <input
            type="password"
            bind:value={change_new_password}
            placeholder={$t('cloudSync.newPassword')}
            class="w-44 rounded-lg border border-default bg-surface-inset px-3 py-1.5 text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus"
          />
          <button
            type="button"
            class="rounded-full border border-strong px-4 py-1.5 text-sm transition hover:bg-accent hover:text-accent-fg disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus"
            disabled={busy || !old_password || !change_new_password}
            onclick={changePassword}
          >
            {$t('cloudSync.changeAction')}
          </button>
          <button
            type="button"
            class="rounded-full border border-strong px-4 py-1.5 text-sm text-muted transition hover:bg-surface-inset hover:text-fg focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus"
            onclick={() => syncActions.lock()}
          >
            {$t('cloudSync.lockAction')}
          </button>
        </div>
      {/if}
    </div>
  </Surface>

  <!-- Device Flow modal, DockPilot's two-step layout: ① the code with the
       open-GitHub button on one row, ② the wait note. The code stays visible
       and copyable for the whole wait (entering it takes a minute or two);
       the earlier polling view replaced it with a placeholder, leaving users
       with a code they could never read back. Errors close the dialog and
       surface in the panel instead. -->
  {#if connect.kind === 'code' || connect.kind === 'polling'}
    <Modal label={$t('cloudSync.connectTitle')} onClose={cancelConnect}>
      <div class="p-6">
        <h3 class="text-base font-semibold">{$t('cloudSync.connectTitle')}</h3>
        <p class="mt-2 text-sm">{$t('cloudSync.stepOpen')}</p>
        <div class="mt-3 flex flex-wrap items-center gap-3 rounded-xl bg-surface-inset p-3">
          <p class="select-text font-mono text-2xl tracking-[0.3em] tabular-nums">
            {connect.start.userCode}
          </p>
          <button
            type="button"
            class="ml-auto inline-flex shrink-0 items-center gap-1.5 rounded-full border border-strong px-4 py-1.5 text-sm text-fg transition hover:bg-surface hover:text-fg focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus"
            onclick={openDevicePage}
          >
            {$t('cloudSync.openGithub')}
            <Icon name="expand" size={13} />
          </button>
        </div>
        <p class="mt-4 text-sm text-muted">{$t('cloudSync.stepWait')}</p>
        <div class="mt-3 flex items-center gap-2">
          <Icon name="refresh" size={14} class="animate-spin text-muted" />
          <p class="text-sm text-muted">{$t('cloudSync.waitingAuth')}</p>
        </div>
        <div class="mt-5 flex justify-end gap-2">
          <button
            type="button"
            class="rounded-full border border-strong px-4 py-1.5 text-sm text-muted transition hover:bg-surface-inset hover:text-fg focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus"
            onclick={cancelConnect}
          >
            {$t('cloudSync.cancel')}
          </button>
          <button
            type="button"
            class="inline-flex items-center gap-1.5 rounded-full bg-accent px-4 py-1.5 text-sm text-accent-fg transition hover:opacity-90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus"
            onclick={copyCode}
          >
            <Icon name="copy" size={14} />
            {$t('cloudSync.copyDeviceCode')}
          </button>
        </div>
      </div>
    </Modal>
  {/if}

  <!-- CONFLICT: the remote uses another sync password -->
  {#if s.syncState === 'CONFLICT' && s.conflict}
    <Modal label={$t('cloudSync.conflictTitle')} onClose={() => {}}>
      <div class="p-6">
        <h3 class="mb-1 text-base font-semibold">{$t('cloudSync.conflictTitle')}</h3>
        <p class="text-xs text-muted">{$t('cloudSync.conflictHint')}</p>
        <div class="my-4 space-y-1 rounded-xl bg-surface-inset p-3 text-xs">
          <p>{$t('cloudSync.localSide', { values: { version: s.conflict.localVersion, device: s.conflict.localDeviceName ?? '' } })}</p>
          <p>{$t('cloudSync.remoteSide', { values: { version: s.conflict.remoteVersion, device: s.conflict.remoteDeviceName ?? '' } })}</p>
        </div>
        <input
          type="password"
          bind:value={conflictPassword}
          placeholder={$t('cloudSync.cloudPassword')}
          class="mb-4 w-full rounded-lg border border-default bg-surface-inset px-3 py-1.5 text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus"
        />
        <div class="flex justify-end gap-2">
          <button
            type="button"
            class="rounded-full border border-strong px-4 py-1.5 text-sm transition hover:bg-surface-inset disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus"
            disabled={busy}
            onclick={useLocal}
          >
            {$t('cloudSync.useLocal')}
          </button>
          <button
            type="button"
            class="rounded-full bg-accent px-4 py-1.5 text-sm text-accent-fg transition hover:opacity-90 disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus"
            disabled={busy || !conflictPassword}
            onclick={useRemote}
          >
            {$t('cloudSync.useRemote')}
          </button>
        </div>
      </div>
    </Modal>
  {/if}

  <!-- BLOCKED: the shrink guard caught a suspicious data loss -->
  {#if s.syncState === 'BLOCKED' && s.shrinkFinding}
    <Modal label={$t('cloudSync.blockedTitle')} onClose={() => {}}>
      <div class="p-6">
        <h3 class="mb-1 text-base font-semibold">{$t('cloudSync.blockedTitle')}</h3>
        <p class="text-xs text-muted">{$t('cloudSync.blockedHint')}</p>
        <div class="my-4 rounded-xl bg-surface-inset p-3 text-xs">
          <p class="tabular-nums">
            {$t('cloudSync.shrinkDetail', {
              values: {
                entity: $t(`cloudSync.entity.${s.shrinkFinding.entityType}`),
                base: s.shrinkFinding.baseCount,
                now: s.shrinkFinding.outgoingCount,
                lost: s.shrinkFinding.lost
              }
            })}
          </p>
        </div>
        <div class="flex justify-end gap-2">
          <button
            type="button"
            class="rounded-full border border-strong px-4 py-1.5 text-sm transition hover:bg-surface-inset disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus"
            disabled={busy}
            onclick={pushLocal}
          >
            {$t('cloudSync.forcePush')}
          </button>
          <button
            type="button"
            class="rounded-full bg-accent px-4 py-1.5 text-sm text-accent-fg transition hover:opacity-90 disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus"
            disabled={busy}
            onclick={restoreRemote}
          >
            {$t('cloudSync.restoreRemote')}
          </button>
        </div>
      </div>
    </Modal>
  {/if}

  <!-- Empty vault: no base + empty local + non-empty remote -->
  {#if s.emptyVaultPending}
    <Modal label={$t('cloudSync.emptyTitle')} onClose={() => syncActions.dismissEmptyVaultPrompt()}>
      <div class="p-6">
        <h3 class="mb-1 text-base font-semibold">{$t('cloudSync.emptyTitle')}</h3>
        <p class="text-xs text-muted">{$t('cloudSync.emptyHint')}</p>
        <div class="flex justify-end gap-2 pt-4">
          <button
            type="button"
            class="rounded-full border border-strong px-4 py-1.5 text-sm transition hover:bg-surface-inset disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus"
            disabled={busy}
            onclick={emptyPush}
          >
            {confirmPushLocal ? $t('cloudSync.emptyPushConfirm') : $t('cloudSync.forcePush')}
          </button>
          <button
            type="button"
            class="rounded-full bg-accent px-4 py-1.5 text-sm text-accent-fg transition hover:opacity-90 disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus"
            disabled={busy}
            onclick={emptyRestore}
          >
            {$t('cloudSync.emptyRestore')}
          </button>
        </div>
      </div>
    </Modal>
  {/if}
