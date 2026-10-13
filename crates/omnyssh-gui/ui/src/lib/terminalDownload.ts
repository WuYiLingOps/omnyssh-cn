// 终端右键下载选择器的纯函数（TerminalView 的下载弹窗与 RemotePicker 共用），
// 抽出来是为了能脱离组件单测——与 terminalFade/terminalInput 同一套路。

/** 手动下载路径是否可用：非空且不是根目录（整棵文件系统不可下载）。 */
export function manualDownloadReady(path: string): boolean {
  const trimmed = path.trim();
  return trimmed.length > 0 && trimmed !== '/';
}
