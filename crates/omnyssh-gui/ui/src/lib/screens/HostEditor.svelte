<script lang="ts">
  // Add/edit host form (tech-gui.md §4.1). Always writes a manual entry: editing an
  // SSH-config import adopts it, leaving ~/.ssh/config untouched. Validation mirrors the TUI via
  // `formToInput`; on submit the parent persists + reloads, and a rejected save
  // surfaces inline without closing. Semantic tokens only.
  import { onMount, tick } from 'svelte';
  import { t } from 'svelte-i18n';
  import type { HostInputDto, IdentityDto, KeyEntryDto } from '$lib/bindings';
  import { listIdentities, listKeyEntries, testHostConnection } from '$lib/ipc/commands';
  import { Button, Icon, StatusDot } from '$lib/theme';
  import Modal from '$lib/components/Modal.svelte';
  import CredentialPicker from '$lib/components/CredentialPicker.svelte';
  import type { CredentialOption } from '$lib/components/credentialPicker';
  import Select from '$lib/components/Select.svelte';
  import { isWindows } from '$lib/platform';
  import { maskAddress, maskHostname, maskText, streamerMode } from '$lib/stores/streamer';
  import { toast } from '$lib/stores/toast';
  import {
    bracketed,
    credentialFromHost,
    emptyForwardRow,
    formToInput,
    formErrorMessage,
    type FormError,
    type HostFormFields
  } from './hostForm';

  let {
    mode,
    initial,
    previousName,
    imported = false,
    onSubmit,
    onCancel
  }: {
    mode: 'add' | 'edit';
    initial: HostFormFields;
    previousName?: string;
    /** Editing an `~/.ssh/config` import, so the save is an adoption — say so. */
    imported?: boolean;
    onSubmit: (input: HostInputDto, previousName: string | undefined) => Promise<void>;
    onCancel: () => void;
  } = $props();

  // Seeded once from `initial`; the editor is remounted per open, so the prop never
  // changes under a live instance.
  // svelte-ignore state_referenced_locally
  let fields = $state<HostFormFields>({ ...initial });
  // A validation failure is a message key resolved on render; a rejected save carries
  // its own already-translated message.
  let error = $state<FormError | string | null>(null);
  const errorText = $derived(
    error === null ? null : typeof error === 'string' ? error : formErrorMessage(error, $t)
  );
  let saving = $state(false);
  let nameEl = $state<HTMLInputElement>();
  let hostnameEl = $state<HTMLInputElement>();
  let userEl = $state<HTMLInputElement>();
  let localEls = $state<HTMLInputElement[]>([]);

  // The credential picker's options: keychain key entries and reusable login
  // identities of `identities.toml`, loaded once per open (the editor remounts
  // per open, so no live-refresh machinery is needed).
  let keys = $state<KeyEntryDto[]>([]);
  let identities = $state<IdentityDto[]>([]);
  // A pin that resolved when the form was seeded but no longer exists (e.g. a
  // cloud-sync merge dropped it while this form was closed) still round-trips:
  // a missing option would render the select blank and a save would silently
  // clear the pin.
  const credentialStale = $derived(
    (fields.credential.startsWith('key:') || fields.credential.startsWith('identity:')) &&
      !credentialListed(fields.credential, keys, identities)
  );

  function credentialListed(
    choice: string,
    keys: KeyEntryDto[],
    identities: IdentityDto[]
  ): boolean {
    const id = choice.startsWith('key:') ? choice.slice(4) : choice.slice(9);
    return choice.startsWith('key:')
      ? keys.some((k) => k.id === id)
      : identities.some((i) => i.id === id);
  }

  // In streamer mode an address present when the form opened shows its disguise,
  // read-only, until Reveal; a reveal lasts while this form is open (it is remounted per
  // open) and streamer mode stays on. What the user adds or types is theirs and stays
  // editable, so a field never turns read-only under their cursor, even when the mode is
  // switched on mid-edit. A field the disguise leaves as is (a bare port, localhost) is
  // not held, so editing it never takes a Reveal.
  let revealed = $state(false);
  const hide = $derived($streamerMode && !revealed);
  $effect(() => {
    if (!$streamerMode) revealed = false;
  });
  // svelte-ignore state_referenced_locally
  let heldHostname = $state(maskHostname(initial.hostname) !== initial.hostname);
  // Parallel to `fields.forwards`.
  // svelte-ignore state_referenced_locally
  let heldRows = $state(
    initial.forwards.map((r) => ({
      local: maskListen(r.local) !== r.local,
      remote: maskAddress(r.remoteHost) !== r.remoteHost
    }))
  );
  const hostnameMasked = $derived(hide && heldHostname);
  const firstHeldRow = $derived(heldRows.findIndex((r) => r.local || r.remote));

  async function reveal(target: () => HTMLInputElement | undefined): Promise<void> {
    revealed = true;
    await tick();
    target()?.focus();
  }

  /** `[address:]port` with the address disguised, as the card shows a forward. */
  function maskListen(raw: string): string {
    const i = raw.lastIndexOf(':');
    if (i < 0) return raw;
    return `${bracketed(maskAddress(raw.slice(0, i).replace(/^\[(.*)\]$/, '$1')))}${raw.slice(i)}`;
  }

  // The name is the on-disk key; a rename can't carry backend-only secrets across the
  // boundary (§3.4), so on edit it is immutable — rename by delete + re-add. Focus the
  // first editable field accordingly.
  onMount(() => {
    void (async () => {
      try {
        const [k, i] = await Promise.all([listKeyEntries(), listIdentities()]);
        keys = k;
        identities = i;
      } catch {
        // A failed load leaves the picker with just "none" + the stale pin
        // placeholder; saving still works and the backend re-validates ids.
      }
    })();
    (mode === 'add' ? nameEl : hostnameMasked ? userEl : hostnameEl)?.focus();
  });

  async function save(): Promise<void> {
    const result = formToInput(fields);
    if (!result.ok) {
      error = result.error;
      return;
    }
    error = null;
    saving = true;
    try {
      await onSubmit(result.input, previousName);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      saving = false;
    }
  }

  // Connection test: validates exactly what will be saved (it builds its payload
  // through the same `formToInput` the save runs) but persists nothing. Feedback
  // mirrors DockPilot's connection test: the button turns a spinner while the
  // dial is in flight, the result lands as a badge beside it — success with the
  // measured round-trip (colour-graded: ok < 300 ms, warn < 1 s, plain past
  // that), failure with the engine's reason — and an IPC fault (a form that
  // cannot even be tested) toasts. One run at a time.
  let testing = $state(false);
  let testResult = $state<{ ok: boolean; ms?: number; text: string } | null>(null);
  const testStatus = $derived.by(() => {
    if (!testResult?.ok) return 'crit' as const;
    if ((testResult.ms ?? 0) < 300) return 'ok' as const;
    if ((testResult.ms ?? 0) < 1000) return 'warn' as const;
    return 'unknown' as const;
  });

  async function testConnection(): Promise<void> {
    if (testing) return;
    const result = formToInput(fields);
    if (!result.ok) {
      // Same validation the save would hit, so a bad form tests nothing.
      error = result.error;
      toast.error(formErrorMessage(result.error, $t));
      return;
    }
    error = null;
    testing = true;
    testResult = null;
    try {
      const outcome = await testHostConnection(result.input);
      testResult = outcome.ok
        ? { ok: true, ms: outcome.latency_ms ?? 0, text: $t('hostEditor.testOk') }
        : { ok: false, text: outcome.error ?? $t('hostEditor.testFailed') };
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e);
      testResult = { ok: false, text: message };
      toast.error(message);
    } finally {
      testing = false;
    }
  }

  // On edit the DTO omits identity/password (§3.4), so the fields start blank and mean
  // "keep the stored value"; on add they mean "none".
  const secretHint = $derived(mode === 'edit' ? $t('hostEditor.keepCurrent') : undefined);

  // What the pinned credential takes over at connect time, one hint line under the
  // picker (mirrors the resolve order of `auth_resolve.rs`).
  const credentialHint = $derived.by(() => {
    if (fields.credential.startsWith('identity:')) return $t('hostEditor.credentialHintIdentity');
    if (fields.credential.startsWith('key:')) return $t('hostEditor.credentialHintKey');
    return null;
  });

  // The picker's flat option list, sections in display order: "none" first, then
  // keychain keys, then login identities, then a stale pin that no longer resolves.
  const credentialOptions = $derived.by<CredentialOption[]>(() => {
    const out: CredentialOption[] = [
      { value: 'none', label: $t('hostEditor.credentialNone'), group: null }
    ];
    for (const k of keys)
      out.push({ value: `key:${k.id}`, label: k.label, group: $t('hostEditor.credentialGroupKeys') });
    for (const i of identities)
      out.push({
        value: `identity:${i.id}`,
        label: i.label,
        group: $t('hostEditor.credentialGroupIdentities')
      });
    if (credentialStale)
      out.push({ value: fields.credential, label: $t('hostEditor.credentialStale'), group: null });
    return out;
  });

  const label = 'block space-y-1 text-xs font-medium text-muted';
  const forwardGrid = 'grid grid-cols-[8.5rem,1fr,4.5rem,1.75rem] items-center gap-2';
  const smallBtn =
    'inline-flex items-center gap-1 rounded-full border border-default px-2 py-0.5 text-xs text-muted transition ' +
    'hover:border-strong hover:bg-accent hover:text-accent-fg ' +
    'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus';
  const field =
    'w-full rounded-lg bg-surface-inset px-3 py-2 text-sm text-fg outline-none ' +
    'focus-visible:ring-2 focus-visible:ring-focus placeholder:text-faint';
  const maskedField = 'cursor-default text-muted';
</script>

<Modal label={mode === 'add' ? $t('hostEditor.addTitle') : $t('hostEditor.editTitle')} onClose={onCancel}>
  <form
    onsubmit={(e) => {
      e.preventDefault();
      void save();
    }}
    class="flex min-h-0 flex-col"
  >
    <header class="border-b border-default px-5 py-3.5">
      <h2 class="text-sm font-semibold">
        {mode === 'add' ? $t('hostEditor.addTitle') : $t('hostEditor.editTitle')}
      </h2>
    </header>

    <div class="min-h-0 flex-1 space-y-3.5 overflow-y-auto px-5 py-4">
      {#if imported}
        <p class="rounded-lg bg-surface-inset px-3 py-2 text-xs text-muted">
          {$t('hostEditor.importedNotice')} <span class="font-mono">~/.ssh/config</span
          >{$t('hostEditor.importedNoticeMid')} <span class="font-mono">hosts.toml</span>
          {$t('hostEditor.importedNoticeEnd')}
        </p>
      {/if}
      <label class={label}>
        <span>{mode === 'edit' ? $t('hostEditor.nameFixed') : $t('hostEditor.name')}</span>
        <input
          bind:this={nameEl}
          bind:value={fields.name}
          class="{field} {mode === 'edit' ? 'cursor-not-allowed text-muted' : ''}"
          placeholder={$t('hostEditor.namePlaceholder')}
          readonly={mode === 'edit'}
          title={mode === 'edit' ? $t('hostEditor.nameRenameTitle') : undefined}
        />
      </label>

      <div class="flex items-end gap-2">
        <label class="{label} min-w-0 flex-1">
          <span>{$t('hostEditor.hostnameIp')}</span>
          <input
            bind:this={hostnameEl}
            bind:value={
              () => (hostnameMasked ? maskHostname(fields.hostname) : fields.hostname),
              (v) => {
                fields.hostname = v;
                heldHostname = false;
              }
            }
            readonly={hostnameMasked}
            class="{field} font-mono {hostnameMasked ? maskedField : ''}"
            placeholder="10.0.0.1"
          />
        </label>
        {#if hostnameMasked}
          <button type="button" class="{smallBtn} mb-1.5 shrink-0" onclick={() => reveal(() => hostnameEl)}>
            {$t('hostEditor.reveal')}
          </button>
        {/if}
      </div>

      <div class="grid grid-cols-[1fr,7rem] gap-3">
        <label class={label}>
          <span>{$t('hostEditor.user')}</span>
          <input
            bind:this={userEl}
            bind:value={fields.user}
            class={field}
            placeholder={$t('hostEditor.userPlaceholder')}
          />
        </label>
        <label class={label}>
          <span>{$t('hostEditor.port')}</span>
          <input bind:value={fields.port} inputmode="numeric" class={field} placeholder="22" />
        </label>
      </div>

      <!-- The single credential picker: pinning a key or identity takes over the
           login at connect time; `none` falls back to the identity file / password
           fields below (resolve order of `auth_resolve.rs`). App-drawn dropdown —
           the native select popup is OS-drawn and unthemeable on Windows. -->
      <div class="space-y-1">
        <span class={label}>{ $t('hostEditor.credential') }</span>
        <CredentialPicker
          bind:value={fields.credential}
          options={credentialOptions}
          onpick={(v) => (fields.credential = v)}
          label={$t('hostEditor.credential')}
        />
        {#if credentialHint}
          <p class="text-xs text-faint">{credentialHint}</p>
        {/if}
      </div>

      <label class={label}>
        <span>{$t('hostEditor.identityFile')}</span>
        <input
          bind:value={fields.identityFile}
          class="{field} font-mono"
          placeholder={secretHint ?? $t('hostEditor.identityFilePlaceholder')}
        />
      </label>

      <label class={label}>
        <span>{$t('hostEditor.password')}</span>
        <input
          type="password"
          bind:value={fields.password}
          class={field}
          placeholder={secretHint ?? $t('hostEditor.passwordPlaceholder')}
          autocomplete="off"
        />
      </label>

      <div class="space-y-1">
        <label class={label}>
          <span>{$t('hostEditor.tags')}</span>
          <input
            bind:value={fields.tags}
            class={field}
            placeholder={$t('hostEditor.tagsPlaceholder')}
            aria-describedby="host-tags-hint"
          />
        </label>
        <p id="host-tags-hint" class="text-xs text-faint">{$t('hostEditor.tagsHint')}</p>
      </div>

      <label class={label}>
        <span>{$t('hostEditor.notes')}</span>
        <textarea
          bind:value={fields.notes}
          rows="2"
          class="{field} resize-y"
          placeholder={$t('hostEditor.notesPlaceholder')}
        ></textarea>
      </label>

      <div class="grid grid-cols-2 gap-3">
        <label class={label}>
          <span>{$t('hostEditor.monitoring')}</span>
          <Select bind:value={fields.monitoring} class={field}>
            <option value="ssh">{$t('hostEditor.monitoringSsh')}</option>
            <option value="tcpPort">{$t('hostEditor.monitoringTcp')}</option>
          </Select>
        </label>
        {#if fields.monitoring === 'tcpPort'}
          <label class={label}>
            <span>{$t('hostEditor.probePort')}</span>
            <input
              bind:value={fields.monitorPort}
              inputmode="numeric"
              class={field}
              placeholder={fields.port || '22'}
            />
          </label>
        {/if}
      </div>
      {#if fields.monitoring === 'tcpPort'}
        <p class="text-xs text-faint">{$t('hostEditor.probeHint')}</p>
      {/if}

      <div class="flex items-center justify-between gap-4 border-t border-default pt-3.5">
        <div class="min-w-0">
          <p class="text-sm text-fg">{$t('hostEditor.forwardAgent')}</p>
          <p class="text-xs text-faint">
            {#if isWindows}
              {$t('hostEditor.forwardAgentUnavailable')}
            {:else}
              {$t('hostEditor.forwardAgentHint')} <span class="font-mono">ssh -A</span>
              {$t('hostEditor.forwardAgentHintEnd')}
            {/if}
          </p>
        </div>
        <button
          type="button"
          role="switch"
          aria-checked={fields.forwardAgent}
          aria-label={$t('hostEditor.forwardAgent')}
          disabled={isWindows}
          onclick={() => (fields.forwardAgent = !fields.forwardAgent)}
          class="relative h-6 w-11 shrink-0 rounded-full transition focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus disabled:opacity-50 {fields.forwardAgent
            ? 'bg-accent'
            : 'bg-surface-inset'}"
        >
          <span
            class="absolute top-0.5 h-5 w-5 rounded-full bg-surface shadow-soft transition-[left] {fields.forwardAgent
              ? 'left-[1.375rem]'
              : 'left-0.5'}"
          ></span>
        </button>
      </div>

      <!-- Port forwarding (`ssh -L`): each row listens on a local port and carries it to
           a host:port the server reaches. One tunnel per host carries every row. -->
      <div class="space-y-2 border-t border-default pt-3.5">
        <div class="flex items-center justify-between gap-3">
          <span class="text-xs font-medium text-muted">{$t('hostEditor.portForwarding')}</span>
          <div class="flex gap-2">
            {#if hide && firstHeldRow >= 0}
              <button type="button" class={smallBtn} onclick={() => reveal(() => localEls[firstHeldRow])}>
                {$t('hostEditor.reveal')}
              </button>
            {/if}
            <button
              type="button"
              class={smallBtn}
              onclick={() => {
                fields.forwards.push(emptyForwardRow());
                heldRows.push({ local: false, remote: false });
              }}
            >
              <Icon name="plus" size={12} />
              {$t('hostEditor.addForward')}
            </button>
          </div>
        </div>
        {#if fields.forwards.length}
          <div class="{forwardGrid} text-[11px] text-faint">
            <span>{$t('hostEditor.localPort')}</span>
            <span>{$t('hostEditor.remoteHost')}</span>
            <span>{$t('hostEditor.fwdPort')}</span>
            <span></span>
          </div>
          {#each fields.forwards as row, i (i)}
            {@const localMasked = hide && heldRows[i].local}
            {@const remoteMasked = hide && heldRows[i].remote}
            <div class={forwardGrid}>
              <input
                bind:this={localEls[i]}
                bind:value={
                  () => (localMasked ? maskListen(row.local) : row.local),
                  (v) => {
                    row.local = v;
                    heldRows[i].local = false;
                  }
                }
                readonly={localMasked}
                class="{field} font-mono {localMasked ? maskedField : ''}"
                placeholder="9443"
                aria-label={$t('hostEditor.fwdLocalPort', { values: { n: i + 1 } })}
              />
              <input
                bind:value={
                  () => (remoteMasked ? maskAddress(row.remoteHost) : row.remoteHost),
                  (v) => {
                    row.remoteHost = v;
                    heldRows[i].remote = false;
                  }
                }
                readonly={remoteMasked}
                class="{field} font-mono {remoteMasked ? maskedField : ''}"
                placeholder="localhost"
                aria-label={$t('hostEditor.fwdRemoteHost', { values: { n: i + 1 } })}
              />
              <input
                bind:value={row.remotePort}
                inputmode="numeric"
                class="{field} font-mono"
                placeholder="9443"
                aria-label={$t('hostEditor.fwdRemotePort', { values: { n: i + 1 } })}
              />
              <button
                type="button"
                class="grid h-7 w-7 place-items-center rounded-lg text-muted transition hover:bg-surface-inset hover:text-fg focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus"
                title={$t('hostEditor.removeForward', { values: { n: i + 1 } })}
                aria-label={$t('hostEditor.removeForward', { values: { n: i + 1 } })}
                onclick={() => {
                  fields.forwards.splice(i, 1);
                  heldRows.splice(i, 1);
                }}
              >
                <Icon name="close" size={13} />
              </button>
            </div>
          {/each}
          <p class="text-xs text-faint">
            {$t('hostEditor.forwardsHint')} <span class="font-mono">0.0.0.0:8080</span>
            {$t('hostEditor.forwardsHintMid')}
            <span class="font-mono">localhost</span>{$t('hostEditor.forwardsHintEnd')}
          </p>
          <div class="flex items-center justify-between gap-4 pt-1">
            <span class="text-sm text-fg">{$t('hostEditor.tunnelAutostart')}</span>
            <button
              type="button"
              role="switch"
              aria-checked={fields.tunnelAutostart}
              aria-label={$t('hostEditor.tunnelAutostart')}
              onclick={() => (fields.tunnelAutostart = !fields.tunnelAutostart)}
              class="relative h-6 w-11 shrink-0 rounded-full transition focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus {fields.tunnelAutostart
                ? 'bg-accent'
                : 'bg-surface-inset'}"
            >
              <span
                class="absolute top-0.5 h-5 w-5 rounded-full bg-surface shadow-soft transition-[left] {fields.tunnelAutostart
                  ? 'left-[1.375rem]'
                  : 'left-0.5'}"
              ></span>
            </button>
          </div>
        {:else}
          <p class="text-xs text-faint">
            {$t('hostEditor.noForwardsHint')} <span class="font-mono">ssh -L</span>.
          </p>
        {/if}
      </div>

      {#if errorText}
        <p class="text-xs text-status-crit">{$maskText(errorText)}</p>
      {/if}
    </div>

    <footer class="flex items-center justify-between gap-2 border-t border-default px-5 py-3">
      <div class="flex min-w-0 items-center gap-2">
        <!-- The button stays static while testing (disabled only): swapping a
             spinner into it changes its width and shifts the footer, and a
             second spinner beside it was one animation too many. -->
        <Button variant="ghost" disabled={testing || saving} onclick={() => void testConnection()}>
          {$t('hostEditor.testConnection')}
        </Button>
        <!-- One subtle indicator: a small spinner + label while the dial is in
             flight, then the graded result in the same spot. -->
        {#if testing}
          <span class="flex items-center gap-1.5 text-xs text-faint">
            <Icon name="refresh" size={12} class="animate-spin" />
            {$t('hostEditor.testing')}
          </span>
        {:else if testResult}
          {#if testResult.ok}
            <span
              class="flex items-center gap-1.5 text-xs text-status-ok"
              title={testResult.text}
            >
              <StatusDot status={testStatus} size={7} />
              {testResult.ms} {$t('hostEditor.testMs')}
            </span>
          {:else}
            <span
              class="flex min-w-0 items-center gap-1.5 text-xs text-status-crit"
              title={testResult.text}
            >
              <StatusDot status="crit" size={7} />
              <span class="truncate">{$t('hostEditor.testUnreachable')} · {testResult.text}</span>
            </span>
          {/if}
        {/if}
      </div>
      <div class="flex gap-2">
        <Button variant="ghost" onclick={onCancel}>{$t('hostEditor.cancel')}</Button>
        <Button variant="primary" type="submit" disabled={saving}>
          {mode === 'add' ? $t('hostEditor.addTitle') : $t('hostEditor.save')}
        </Button>
      </div>
    </footer>
  </form>
</Modal>
