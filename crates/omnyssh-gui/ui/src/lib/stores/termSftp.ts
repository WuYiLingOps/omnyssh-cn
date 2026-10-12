import { writable } from 'svelte/store';
import { sftpOpen, sftpClose } from '$lib/ipc/commands';
import { sftp } from './sftp';

// 终端右键菜单的 SFTP 支撑层（tech-gui.md §4.2 传输）。两部分：
// 1. 每主机记住的上次传输目录——菜单上传/下载的默认目标，进程内生效；
// 2. 终端标签 → 后端 SFTP 会话的懒开与复用——首次使用菜单时才 `sftp_open`，
//    同一标签的多次操作共享一条连接；标签销毁时经 `releaseTermSftp` 关闭。

/** 把 `dir` 记为 `host` 的上次传输目录（纯函数，便于单测）；空目录忽略。 */
export function rememberDir(
  map: Map<string, string>,
  host: string,
  dir: string
): Map<string, string> {
  if (!dir) return map;
  return new Map(map).set(host, dir);
}

function createDirs() {
  const { subscribe, update } = writable<Map<string, string>>(new Map());
  return {
    subscribe,
    remember(host: string, dir: string): void {
      update((m) => rememberDir(m, host, dir));
    }
  };
}

export const termSftpDirs = createDirs();

// 每个终端标签的会话状态。`pending` 是还在连接中的 `sftp_open`（去重并发调用）；
// `live` 是已就绪的后端会话 id。标签在 `sftp_open` 尚未返回时被销毁是合法的——
// `released` 记下这种标签，连接返回后立即关闭，不让它泄漏。
const pending = new Map<number, Promise<number>>();
const live = new Map<number, number>();
const released = new Set<number>();

/** `sftp_open` 在标签已被关闭后返回的占位错误：调用方是正在销毁的标签，不展示。 */
const RELEASED = 'terminal sftp released';

/** 返回该标签的 SFTP 会话 id——已有则复用，连接中则等待同一次连接，否则新开。
 *  连接成功时向 sftp store 注册会话，之后 `sftp-*` 事件（进度/完成）自动路由进来。 */
export function ensureTermSftp(tabId: number, hostName: string): Promise<number> {
  const ready = live.get(tabId);
  if (ready != null) return Promise.resolve(ready);
  const inFlight = pending.get(tabId);
  if (inFlight) return inFlight;
  const opening = (async () => {
    const id = await sftpOpen(hostName);
    if (released.delete(tabId)) {
      // 标签在连接期间被销毁：立即拆掉这条连接，绝不注册。
      void sftpClose(id).catch(() => {});
      sftp.remove(id);
      throw new Error(RELEASED);
    }
    live.set(tabId, id);
    sftp.open(id, hostName);
    return id;
  })();
  pending.set(tabId, opening);
  // 无论成败，这次连接尝试都已终结；失败由调用方展示，下次调用可重试。
  void opening.catch(() => {
    if (pending.get(tabId) === opening) pending.delete(tabId);
  });
  return opening;
}

/** 终端标签销毁：关闭已就绪的会话；连接还在途则记入 `released`，等它返回后关闭。 */
export function releaseTermSftp(tabId: number): void {
  const ready = live.get(tabId);
  if (ready != null) {
    live.delete(tabId);
    void sftpClose(ready).catch(() => {});
    sftp.remove(ready);
    return;
  }
  if (pending.has(tabId)) released.add(tabId);
}

/** 供单测使用：清空模块级会话簿记（store 本身不可跨测复用）。 */
export function resetTermSftpForTests(): void {
  pending.clear();
  live.clear();
  released.clear();
}
