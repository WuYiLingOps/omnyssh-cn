<script lang="ts">
  // A live terminal tab (tech-gui.md §3.1). One instance per terminal session, kept
  // mounted for the session's whole life — hidden, not destroyed, when another entity
  // is active — so scrollback and the byte stream survive tab switches. Raw output
  // arrives on a per-session channel (§3.3/§3.6); keystrokes/resizes go back over the
  // terminal commands. Subscribes to the theme store and re-themes live (§5.1).
  import '@xterm/xterm/css/xterm.css';
  import { onMount, onDestroy } from 'svelte';
  import { get } from 'svelte/store';
  import { t } from 'svelte-i18n';
  import type { Terminal } from '@xterm/xterm';
  import type { FitAddon } from '@xterm/addon-fit';
  import { Channel } from '@tauri-apps/api/core';
  import { open as dialogOpen, save as dialogSave } from '@tauri-apps/plugin-dialog';
  import { theme } from '$lib/stores/theme';
  import { xtermTheme } from '$lib/theme/terminalTheme';
  import { sessions, type Session } from '$lib/stores/sessions';
  import { closeSession, spawnSession } from '$lib/stores/navigation';
  import { terminalDidExit } from '$lib/ipc/router';
  import { lastError } from '$lib/stores/notifications';
  import { maskText } from '$lib/stores/streamer';
  import { dialogs } from '$lib/stores/dialogs';
  import { palette } from '$lib/stores/palette';
  import { isStreamerChord } from '$lib/stores/ui';
  import { activeEntity } from '$lib/stores/activeEntity';
  import {
    terminalOpen,
    terminalWrite,
    terminalResize,
    terminalClose,
    terminalPaste,
    terminalProbeCwd,
    sftpUpload,
    sftpDownload,
    sftpCancel
  } from '$lib/ipc/commands';
  import { sftp, baseName, formatBytes, transferState } from '$lib/stores/sftp';
  import { ensureTermSftp, releaseTermSftp, termSftpDirs } from '$lib/stores/termSftp';
  import ContextMenu, { type ContextMenuItem } from '$lib/components/ContextMenu.svelte';
  import Modal from '$lib/components/Modal.svelte';
  import { shouldFadeTop } from './terminalFade';
  import { chunkBytes, closesEndedTab, isCopyShortcut, layoutFallback } from './terminalInput';
  import { isMac } from '$lib/platform';
  import type { TerminalBytes } from '$lib/bindings';

  let { session, active }: { session: Session; active: boolean } = $props();

  // The Nerd Font families come after the generic `monospace`, not merely after the
  // named system ones: the named list is macOS/Windows-only, so on a Linux desktop a
  // patched font ahead of the generic would become the terminal's Latin face and size
  // its cell from itself. Per-character fallback continues past a generic family, so the
  // Private Use Area glyphs (starship, powerlevel10k, eza --icons) still reach the tail.
  // Within the tail, the single-width variants come first — Nerd Fonts v3 ships icons at
  // double width in the bare family and one cell wide in its `Mono` twin.
  const MONO =
    'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace, ' +
    '"Symbols Nerd Font Mono", "Symbols Nerd Font", "MesloLGS NF", ' +
    '"JetBrainsMono Nerd Font Mono", "JetBrainsMono Nerd Font", ' +
    '"Hack Nerd Font Mono", "Hack Nerd Font", ' +
    '"FiraCode Nerd Font Mono", "FiraCode Nerd Font"';
  // One encoder for the keystroke hot path instead of one per input event.
  const ENCODER = new TextEncoder();

  // A large paste arrives as one onData; sending it as a single number[] would freeze
  // the UI thread (§9). Split into bounded chunks and await each so paint yields between
  // them; a serialization chain keeps all input strictly in order across events.
  let writeChain: Promise<void> = Promise.resolve();
  function sendInput(bytes: Uint8Array): void {
    if (termId == null || bytes.length === 0 || ended()) return;
    writeChain = writeChain.then(async () => {
      for (const chunk of chunkBytes(bytes)) {
        if (destroyed || termId == null) return;
        try {
          await terminalWrite(termId, Array.from(chunk));
        } catch {
          // Stop this input on a write failure rather than sending a gapped stream.
          return;
        }
      }
    });
  }

  /** No dialog or palette holds the keyboard: keystrokes meant for a key passphrase
   *  must never reach the shell. */
  function keyboardFree(): boolean {
    return get(dialogs).length === 0 && !get(palette).open;
  }

  /** The remote side ended the session: the tab stays for its last output, and the
   *  backend has nothing left to write or resize. Read from the store, which a late
   *  channel message can follow the exit into. */
  function ended(): boolean {
    return get(sessions).some((s) => s.id === session.id && s.status === 'closed');
  }

  let container: HTMLDivElement;
  let term: Terminal | undefined;
  let fitAddon: FitAddon | undefined;
  let termId: number | undefined;
  let destroyed = false;
  let connected = false;
  let ready = $state(false);
  let themeUnsub: (() => void) | undefined;
  let resizeObserver: ResizeObserver | undefined;
  let fitScheduled = false;
  // The top-edge fade dissolves scrolled output into the top edge, but never the live
  // prompt: after `clear`/Ctrl+L the cursor homes to the top, so the fade must lift
  // there (see terminalFade). Recomputed after every write too, since those resets
  // move the viewport without firing onScroll.
  let scrolled = $state(false);
  function syncScrolled(): void {
    const buf = term?.buffer.active;
    scrolled = !!buf && shouldFadeTop(buf.viewportY, buf.baseY, buf.cursorY);
  }

  /** Fit the terminal to its container and tell the backend, but only while visible —
   *  a hidden (display:none) container measures 0, so it refits when shown instead. */
  function safeFit(): void {
    if (!term || !fitAddon || !active) return;
    try {
      fitAddon.fit();
    } catch {
      return;
    }
    if (termId != null && !ended()) void terminalResize(termId, term.cols, term.rows).catch(() => {});
  }

  function scheduleFit(): void {
    if (fitScheduled) return;
    fitScheduled = true;
    requestAnimationFrame(() => {
      fitScheduled = false;
      safeFit();
    });
  }

  // --- 右键菜单与 SFTP 传输（tech-gui.md §4.2）-----------------------------------
  // 菜单在右键处弹出；远端程序自己启用鼠标上报（vim/htop）时右键属于远端，不拦。
  // 传输复用全局 sftp store：按需 ensureTermSftp 打开连接后，进度/完成事件经事件
  // 桥自动路由到本标签的会话状态，底部的浮动条因此能实时显示。
  let menu = $state<{ x: number; y: number; items: ContextMenuItem[] } | null>(null);
  // 排队的传输动作：一次只派发一个（核心命令通道有界，避免溢出打乱 op-done 的 FIFO）。
  let outbox = $state<Array<{ transfer: boolean; run: () => void }>>([]);
  // 上传确认弹窗（已选本地路径 + 远端目标目录）与下载弹窗（远端路径输入）。
  let uploadPick = $state<{ paths: string[] } | null>(null);
  let uploadDir = $state('/');
  let downloadAsk = $state(false);
  let downloadRemote = $state('/');
  let termSftpId = $state<number | undefined>(undefined);

  const termSftpView = $derived(termSftpId != null ? $sftp.get(termSftpId) : undefined);
  const menuTransfer = $derived(termSftpView ? transferState(termSftpView) : undefined);

  // 该主机当前正在浏览的远端目录：先看本标签自己的 SFTP 会话，再看任意同主机会话
  // （如 SFTP 屏幕）。`remote.path` 仅在目录列表返回后非空——连接中或从未浏览时
  // 为 ''，此时回退到上次传输的记忆目录。
  const browsedRemoteDir = $derived.by(() => {
    if (termSftpView?.remote.path) return termSftpView.remote.path;
    for (const s of $sftp.values()) {
      if (s.hostName === session.hostName && s.remote.path) return s.remote.path;
    }
    return undefined;
  });

  // 交互 shell 的工作目录，两个来源，按可信度排序：
  // 1. `oscCwd` — 远端 shell 自己上报的 OSC 7（shell 集成）序列，每次提示符刷新，
  //    因此可以放心缓存；
  // 2. `terminalProbeCwd` — 打开传输弹窗时按需探测（同连接 exec 一段 /proc 脚本
  //    找 shell 兄弟进程的 cwd，Linux 专属），结果只填当前弹窗，不做长期缓存，
  //    因为用户下一次 cd 不会触发任何上报。
  let oscCwd: string | undefined;

  /** `data` 是 OSC 7 负载：`file://host/path` 或裸绝对路径。 */
  function osc7Path(data: string): string | undefined {
    if (data.startsWith('file://')) {
      try {
        return decodeURIComponent(new URL(data).pathname);
      } catch {
        return undefined;
      }
    }
    return data.startsWith('/') ? data : undefined;
  }

  /** 传输弹窗的默认远端目录：shell cwd 优先，其次正在浏览的目录，再是上次
   *  传输记忆，最后根目录。探测结果随后由 `probeShellCwd` 异步补上。 */
  function defaultRemoteDir(): string {
    return (
      oscCwd ?? browsedRemoteDir ?? get(termSftpDirs).get(session.hostName) ?? '/'
    );
  }

  /** 打开弹窗时的基线值：探测返回后只有用户仍未改动时才覆盖，避免抢输入。 */
  let uploadBaseline = '';
  let downloadBaseline = '';

  /** 按需探测 shell cwd（无 OSC 7、未浏览 SFTP 时才有意义），静默失败。 */
  async function probeShellCwd(): Promise<void> {
    if (oscCwd || browsedRemoteDir || termId == null || ended()) return;
    try {
      const cwd = await terminalProbeCwd(termId);
      if (!cwd) return;
      if (uploadPick && uploadDir === uploadBaseline) uploadDir = cwd;
      if (downloadAsk && downloadRemote === downloadBaseline) downloadRemote = cwd;
    } catch {
      // 探测失败（非 Linux / 受限 /proc / 无 shell）：保持回退默认值。
    }
  }

  function errMsg(err: unknown): string {
    return err instanceof Error ? err.message : String(err);
  }

  function joinRemote(dir: string, name: string): string {
    return dir.endsWith('/') ? `${dir}${name}` : `${dir}/${name}`;
  }

  /** `path` 去掉末段后的目录部分；根目录归一为 `/`。用于记住上次传输目录。 */
  function dirOf(path: string): string {
    const trimmed = path.replace(/\/+$/, '');
    const cut = trimmed.lastIndexOf('/');
    return cut <= 0 ? '/' : trimmed.slice(0, cut);
  }

  /** 打开本标签的 SFTP 连接（失败经通知条展示；标签已在销毁中的静默吞掉）。 */
  async function sftpId(): Promise<number | null> {
    try {
      const id = await ensureTermSftp(session.id, session.hostName);
      termSftpId = id;
      return id;
    } catch (err) {
      const message = errMsg(err);
      if (!message.includes('released')) lastError.set(message);
      return null;
    }
  }

  function onContextMenu(e: MouseEvent): void {
    e.preventDefault();
    if (!term || ended()) return;
    // 远端应用在用鼠标：右键已由 xterm 转发出去，这里只吞掉浏览器默认菜单。
    if (term.modes.mouseTrackingMode !== 'none') return;
    menu = {
      x: e.clientX,
      y: e.clientY,
      items: [
        { id: 'uploadFile', label: String($t('terminal.menuUploadFile')), icon: 'upload' },
        { id: 'uploadFolder', label: String($t('terminal.menuUploadFolder')), icon: 'folder' },
        { id: 'download', label: String($t('terminal.menuDownload')), icon: 'download' },
        { id: 'openSftp', label: String($t('terminal.menuOpenSftp')), icon: 'sftp' },
        { id: 'sep', separator: true, label: '' },
        {
          id: 'copy',
          label: String($t('terminal.menuCopy')),
          icon: 'copy',
          disabled: !term.hasSelection()
        },
        { id: 'paste', label: String($t('terminal.menuPaste')), icon: 'clipboard' }
      ]
    };
  }

  function closeMenu(): void {
    menu = null;
    if (active && keyboardFree()) term?.focus();
  }

  async function onMenuSelect(id: string): Promise<void> {
    const t2 = term;
    switch (id) {
      case 'uploadFile': {
        const picked = await dialogOpen({ multiple: true, title: String($t('terminal.menuUploadFile')) });
        if (!picked) return;
        uploadPick = { paths: Array.isArray(picked) ? picked : [picked] };
        uploadDir = defaultRemoteDir();
        uploadBaseline = uploadDir;
        void probeShellCwd();
        break;
      }
      case 'uploadFolder': {
        const picked = await dialogOpen({ directory: true, title: String($t('terminal.menuUploadFolder')) });
        if (typeof picked !== 'string') return;
        uploadPick = { paths: [picked] };
        uploadDir = defaultRemoteDir();
        uploadBaseline = uploadDir;
        void probeShellCwd();
        break;
      }
      case 'download':
        downloadRemote = defaultRemoteDir();
        downloadBaseline = downloadRemote;
        downloadAsk = true;
        void probeShellCwd();
        break;
      case 'openSftp':
        spawnSession('sftp', session.hostName);
        break;
      case 'copy':
        if (t2?.hasSelection()) {
          navigator.clipboard.writeText(t2.getSelection()).catch((err) => {
            lastError.set(
              $t('terminal.copyFailed', {
                values: { error: err instanceof Error ? err.message : String(err) }
              })
            );
          });
        }
        break;
      case 'paste':
        try {
          sendInput(ENCODER.encode(await navigator.clipboard.readText()));
        } catch (err) {
          lastError.set(
            $t('terminal.pasteFailed', {
              values: { error: err instanceof Error ? err.message : String(err) }
            })
          );
        }
        break;
    }
  }

  /** 上传确认：本地路径逐个入队，落到远端目录 `uploadDir` 并记住它。 */
  async function confirmUpload(): Promise<void> {
    const id = await sftpId();
    if (id == null || !uploadPick) return;
    const dir = uploadDir.trim() || '/';
    termSftpDirs.remember(session.hostName, dir);
    const picks = uploadPick.paths;
    uploadPick = null;
    enqueue(
      picks
        .map((path) => ({ path, name: baseName(path) }))
        .filter((item) => item.name)
        .map((item) => () => {
          sftp.pushOp(id, { kind: 'upload', name: item.name, refresh: 'remote' });
          void sftpUpload(id, item.path, joinRemote(dir, item.name)).catch(onDispatchError(id));
        }),
      true
    );
  }

  /** 下载确认：远端路径 → 原生保存对话框（默认文件名取路径末段）→ 入队。 */
  async function confirmDownload(): Promise<void> {
    const remote = downloadRemote.trim();
    if (!remote || remote === '/') return;
    const local = await dialogSave({
      title: String($t('terminal.menuDownload')),
      defaultPath: baseName(remote) || 'download'
    });
    if (!local) return;
    const id = await sftpId();
    if (id == null) return;
    termSftpDirs.remember(session.hostName, dirOf(remote));
    downloadAsk = false;
    enqueue(
      [
        () => {
          sftp.pushOp(id, { kind: 'download', name: baseName(remote), refresh: 'remote' });
          void sftpDownload(id, local, remote).catch(onDispatchError(id));
        }
      ],
      true
    );
  }

  // 派发泵：上一个操作 ack（pending 清空）后才放行下一个，核心命令通道永不溢出。
  $effect(() => {
    if (!termSftpView || termSftpView.pending.length > 0 || outbox.length === 0) return;
    const [next, ...rest] = outbox;
    outbox = rest;
    next.run();
  });

  /** 传输入队；开传前清掉上一批遗留的错误，排队中的错误不许被冲掉（见 applyOpDone）。 */
  function enqueue(actions: Array<() => void>, transfer = false): void {
    if (!actions.length) return;
    const draining = outbox.length > 0 || (termSftpView?.pending.length ?? 0) > 0;
    if (termSftpId != null && !draining) sftp.clearError(termSftpId);
    outbox = [...outbox, ...actions.map((run) => ({ transfer, run }))];
  }

  // invoke 本身拒绝时把占位 op 弹掉，派发泵才不会卡死。
  function onDispatchError(id: number): (err: unknown) => void {
    return (err) => {
      lastError.set(errMsg(err));
      sftp.opDone(id, false, errMsg(err));
    };
  }

  /** 取消：丢掉排队中的传输并停掉在跑的那个；核心的 op-done 会说明已取消。 */
  function cancelMenuTransfers(): void {
    const id = termSftpId;
    if (id == null) return;
    outbox = outbox.filter((op) => !op.transfer);
    void sftpCancel(id).catch((err) => lastError.set(errMsg(err)));
  }


  onMount(() => {
    void (async () => {
      const [{ Terminal }, { FitAddon }] = await Promise.all([
        import('@xterm/xterm'),
        import('@xterm/addon-fit')
      ]);
      if (destroyed) return;

      term = new Terminal({
        fontFamily: MONO,
        fontSize: 13,
        cursorBlink: true,
        scrollback: 5000
      });
      fitAddon = new FitAddon();
      term.loadAddon(fitAddon);
      term.open(container);
      term.onScroll(syncScrolled);

      // Shell integration (OSC 7): a configured remote shell reports its cwd at
      // every prompt, making transfer-dialog defaults exact. Unhandled by us on
      // purpose (false return) so xterm's own handlers still see the sequence.
      term.parser.registerOscHandler(7, (data) => {
        oscCwd = osc7Path(data);
        return false;
      });

      // The #1 theme-regression guard (§5.1): push the matching xterm theme to this
      // terminal — including already-open ones — whenever the store flips. Its
      // synchronous first call (pre-paint) also sets the initial theme.
      themeUnsub = theme.subscribe((t) => {
        if (term) term.options.theme = xtermTheme(t);
      });

      // Route raw output into xterm. The channel is typed `number[]`, but the raw path
      // actually delivers an `ArrayBuffer` (§3.3); `Uint8Array` wraps either.
      const channel = new Channel<TerminalBytes>();
      channel.onmessage = (msg) => {
        if (!term) return;
        if (!connected) {
          connected = true;
          // A large first chunk travels asynchronously and can land after the exit.
          if (!ended()) sessions.setStatus(session.id, 'connected');
        }
        term.write(new Uint8Array(msg as unknown as ArrayBuffer), syncScrolled);
      };

      // Fit before opening so the remote PTY starts at the visible size.
      safeFit();
      const id = await terminalOpen(session.hostName, term.cols || 80, term.rows || 24, channel);
      if (destroyed) {
        void terminalClose(id).catch(() => {});
        return;
      }
      termId = id;
      sessions.setTermId(session.id, id);
      // The remote may have already exited before this id was recorded (fast-fail
      // connect race): terminal-exited couldn't match the tab, so settle it now.
      const exited = terminalDidExit(id);
      if (exited === false) {
        closeSession(session.id);
        return;
      }
      if (exited) sessions.setStatus(session.id, 'closed');

      // Copy takes Ctrl+Shift+C whether or not anything is selected, so the chord never
      // reaches the shell. Returning false only keeps xterm out of it; the default is
      // ours to stop. The write happens inside the keydown, which WebKit requires.
      term.attachCustomKeyEventHandler((e) => {
        // The streamer chord belongs to the window (AppShell), never to the shell, where
        // a ^S would stop its output.
        if (isStreamerChord(e)) return false;
        if (isCopyShortcut(e, isMac)) {
          e.preventDefault();
          if (term?.hasSelection()) {
            navigator.clipboard.writeText(term.getSelection()).catch((err) => {
              lastError.set(
                $t('terminal.copyFailed', {
                  values: { error: err instanceof Error ? err.message : String(err) }
                })
              );
            });
          }
          return false;
        }
        // An ended session takes no input: Enter or Esc closes the tab, and xterm gets
        // no other key (`sendInput` drops anything that still gets through).
        if (ended()) {
          if (closesEndedTab(e)) {
            e.preventDefault();
            closeSession(session.id);
          } else if (e.type === 'keydown' && e.key === 'Tab') {
            // xterm no longer takes Tab here, and moving focus off the terminal
            // would hand the Enter that closes the tab to a button.
            e.preventDefault();
          }
          return false;
        }
        // Under a non-Latin layout WebKitGTK names no key; the physical one stands in.
        const fallback = layoutFallback(e);
        if (!fallback) return true;
        // As xterm does with a key it handles: nothing else acts on it.
        e.preventDefault();
        e.stopPropagation();
        if (fallback.kind === 'control') {
          term?.input(fallback.data);
        } else {
          terminalPaste().catch((err) => {
            lastError.set(
              $t('terminal.pasteFailed', {
                values: { error: err instanceof Error ? err.message : String(err) }
              })
            );
          });
        }
        return false;
      });
      // Text keystrokes/paste are UTF-8; onBinary carries raw 8-bit sequences
      // (e.g. legacy mouse reporting) that must go byte-for-byte, not re-encoded.
      term.onData((data) => sendInput(ENCODER.encode(data)));
      term.onBinary((data) => sendInput(Uint8Array.from(data, (ch) => ch.charCodeAt(0) & 0xff)));

      resizeObserver = new ResizeObserver(() => scheduleFit());
      resizeObserver.observe(container);

      ready = true;
      if (active && keyboardFree()) term.focus();
    })().catch((err) => {
      // `terminal_open` itself failed (e.g. the session could not be spawned): no
      // PtyExited follows, so mark the tab failed here instead of leaving it hung.
      lastError.set(err instanceof Error ? err.message : String(err));
      sessions.setStatus(session.id, 'failed');
    });
  });

  onDestroy(() => {
    destroyed = true;
    themeUnsub?.();
    resizeObserver?.disconnect();
    // 标签没了，右键菜单开的 SFTP 会话也一起收掉（含在途的 sftp_open）。
    releaseTermSftp(session.id);
    // Idempotent: a remote-exit teardown already dropped this id backend-side (§3.4).
    if (termId != null) void terminalClose(termId).catch(() => {});
    term?.dispose();
    // Null it so a byte still in flight (destroyed-before-open race) can't write to
    // a disposed terminal — the channel callback's `if (!term)` guard then bails.
    term = undefined;
  });

  // Becoming visible: a hidden container measured 0, so refit and take focus. Also
  // when a dialog or the palette closes, and when the active tab is clicked again,
  // which moves focus to the tab without changing `active`. The keyboard check runs
  // in the frame, since a dialog can open before it.
  $effect(() => {
    if (active && ready) {
      void [$dialogs, $palette, $activeEntity];
      requestAnimationFrame(() => {
        safeFit();
        if (keyboardFree()) term?.focus();
        syncScrolled();
      });
    }
  });
</script>

<!-- bg-surface fills behind the macOS traffic lights (no seam). Text selection stays
     disabled app-wide (app.css); the terminal is the one selectable surface, handled
     by xterm's own selection (not CSS). -->
<div class="absolute inset-0 overflow-hidden bg-surface {active ? '' : 'hidden'}">
  <!-- Inset via this wrapper, not the xterm host: padding on the element xterm mounts
       into makes FitAddon over-size, sliding the last row under the status bar. The top
       inset clears the macOS traffic-light strip; the bottom gap clears the footer. -->
  <div class="h-full w-full" style="padding: max(var(--titlebar-h), 0.75rem) 0.5rem 1rem;">
    <div
      bind:this={container}
      class="h-full w-full"
      class:term-fade={scrolled}
      role="presentation"
      oncontextmenu={onContextMenu}
    ></div>
  </div>

  <!-- 右键发起的传输进度浮条：贴底部居中，样式对齐 SFTP 视图的传输条。 -->
  {#if menuTransfer}
    <div
      class="absolute inset-x-0 bottom-4 z-40 mx-auto w-[min(34rem,90%)] rounded-xl border
        border-default bg-surface px-4 py-2.5 shadow-soft"
      aria-label={$t('sftp.transferProgress')}
    >
      <div class="flex items-center justify-between gap-3 text-xs text-muted">
        <span class="min-w-0 truncate">
          {menuTransfer.preparing
            ? $t('sftp.preparing')
            : menuTransfer.kind === 'upload'
              ? $t('sftp.uploading')
              : $t('sftp.downloading')}
          <span class="font-mono text-fg">{menuTransfer.name}</span>{menuTransfer.preparing ? '…' : ''}
        </span>
        <span class="flex shrink-0 items-center gap-3">
          {#if !menuTransfer.preparing}
            <span class="tabular-nums">
              {formatBytes(menuTransfer.done)}{menuTransfer.total > 0
                ? ` / ${formatBytes(menuTransfer.total)}`
                : ''}
            </span>
          {/if}
          <button
            type="button"
            class="inline-flex items-center gap-1 rounded-full border border-default px-2 py-1
              text-xs font-medium text-muted transition hover:border-strong hover:bg-accent
              hover:text-accent-fg"
            title={$t('sftp.cancelTransfer')}
            onclick={cancelMenuTransfers}
          >
            {$t('sftp.cancel')}
          </button>
        </span>
      </div>
      <div class="mt-1.5 h-1.5 overflow-hidden rounded-full bg-surface-inset">
        {#if menuTransfer.preparing}
          <div class="h-full w-1/3 rounded-full bg-accent motion-safe:animate-pulse"></div>
        {:else}
          <div
            class="h-full rounded-full bg-accent transition-[width]"
            style="width: {menuTransfer.total > 0
              ? Math.min(100, Math.round((menuTransfer.done / menuTransfer.total) * 100))
              : 0}%"
          ></div>
        {/if}
      </div>
    </div>
  {:else if termSftpView?.error}
    <!-- 用户主动取消是消息不是故障；其余错误原样透出（脱敏后）。 -->
    <div
      class="absolute inset-x-0 bottom-4 z-40 mx-auto w-[min(34rem,90%)] rounded-xl border
        border-default bg-surface px-4 py-2 text-xs shadow-soft
        {termSftpView.cancelled ? 'text-muted' : 'text-status-crit'}"
    >
      {termSftpView.cancelled ? $t('sftp.cancelled') : $maskText(termSftpView.error)}
    </div>
  {/if}
</div>

{#if menu}
  <ContextMenu
    x={menu.x}
    y={menu.y}
    items={menu.items}
    onselect={(id) => void onMenuSelect(id)}
    onclose={closeMenu}
  />
{/if}

{#if uploadPick}
  <Modal label={$t('terminal.menuUploadFile')} onClose={() => (uploadPick = null)}>
    <form
      onsubmit={(e) => {
        e.preventDefault();
        void confirmUpload();
      }}
    >
      <header class="border-b border-default px-5 py-3.5">
        <h2 class="text-sm font-semibold">
          {$t('terminal.uploadTitle', { values: { count: uploadPick.paths.length } })}
        </h2>
      </header>
      <div class="space-y-3 px-5 py-4">
        <ul class="max-h-32 space-y-1 overflow-auto text-xs text-muted">
          {#each uploadPick.paths as path (path)}
            <li class="truncate font-mono" title={path}>{baseName(path) || path}</li>
          {/each}
        </ul>
        <!-- svelte-ignore a11y_autofocus -->
        <input
          autofocus
          bind:value={uploadDir}
          class="w-full rounded-lg bg-surface-inset px-3 py-2 text-sm text-fg outline-none
            focus-visible:ring-2 focus-visible:ring-focus placeholder:text-faint"
          placeholder={$t('terminal.remoteDir')}
          aria-label={$t('terminal.remoteDir')}
        />
      </div>
      <footer class="flex justify-end gap-2 border-t border-default px-5 py-3">
        <button
          type="button"
          class="rounded-full px-4 py-2 text-sm text-muted transition hover:bg-surface-inset hover:text-fg"
          onclick={() => (uploadPick = null)}
        >
          {$t('sftp.cancel')}
        </button>
        <button
          type="submit"
          class="rounded-full bg-accent px-5 py-2 text-sm font-medium text-accent-fg transition hover:opacity-90"
        >
          {$t('sftp.upload')}
        </button>
      </footer>
    </form>
  </Modal>
{/if}

{#if downloadAsk}
  <Modal label={$t('terminal.menuDownload')} onClose={() => (downloadAsk = false)}>
    <form
      onsubmit={(e) => {
        e.preventDefault();
        void confirmDownload();
      }}
    >
      <header class="border-b border-default px-5 py-3.5">
        <h2 class="text-sm font-semibold">{$t('terminal.menuDownload')}</h2>
      </header>
      <div class="px-5 py-4">
        <!-- svelte-ignore a11y_autofocus -->
        <input
          autofocus
          bind:value={downloadRemote}
          class="w-full rounded-lg bg-surface-inset px-3 py-2 font-mono text-sm text-fg outline-none
            focus-visible:ring-2 focus-visible:ring-focus placeholder:text-faint"
          placeholder={$t('terminal.remotePath')}
          aria-label={$t('terminal.remotePath')}
          spellcheck="false"
        />
        <p class="mt-2 text-xs text-faint">{$t('terminal.downloadHint')}</p>
      </div>
      <footer class="flex justify-end gap-2 border-t border-default px-5 py-3">
        <button
          type="button"
          class="rounded-full px-4 py-2 text-sm text-muted transition hover:bg-surface-inset hover:text-fg"
          onclick={() => (downloadAsk = false)}
        >
          {$t('sftp.cancel')}
        </button>
        <button
          type="submit"
          class="rounded-full bg-accent px-5 py-2 text-sm font-medium text-accent-fg transition hover:opacity-90 disabled:opacity-50"
          disabled={!downloadRemote.trim() || downloadRemote.trim() === '/'}
        >
          {$t('sftp.download')}
        </button>
      </footer>
    </form>
  </Modal>
{/if}

<style>
  /* Scrolled output dissolves into the top edge instead of hard-clipping (on only while
     scrolled, so the first line stays crisp). black/transparent are mask alphas. */
  .term-fade {
    -webkit-mask-image: linear-gradient(to bottom, transparent, black 2.25rem);
    mask-image: linear-gradient(to bottom, transparent, black 2.25rem);
  }
</style>
