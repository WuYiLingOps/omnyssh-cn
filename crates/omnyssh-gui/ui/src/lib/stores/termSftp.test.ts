import { describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';

// termSftp 依赖 tauri IPC（sftpOpen/sftpClose），纯函数部分单独测；会话簿记经
// resetTermSftpForTests 复位后，用 mock 的 IPC 模块驱动 ensure/release 的状态机。
vi.mock('$lib/ipc/commands', () => ({
  sftpOpen: vi.fn(),
  sftpClose: vi.fn()
}));

import { rememberDir, termSftpDirs } from './termSftp';

describe('rememberDir', () => {
  it('records the directory for its host', () => {
    const next = rememberDir(new Map(), 'web-1', '/var/www');
    expect(next.get('web-1')).toBe('/var/www');
  });

  it('overwrites the previous directory of the same host', () => {
    const next = rememberDir(new Map([['web-1', '/a']]), 'web-1', '/b');
    expect(next.get('web-1')).toBe('/b');
  });

  it('does not mutate the input map', () => {
    const base = new Map([['web-1', '/a']]);
    rememberDir(base, 'web-1', '/b');
    expect(base.get('web-1')).toBe('/a');
  });

  it('ignores an empty directory', () => {
    const base = new Map([['web-1', '/a']]);
    expect(rememberDir(base, 'web-1', '')).toBe(base);
  });

  it('keeps other hosts untouched', () => {
    const next = rememberDir(new Map([['web-1', '/a']]), 'web-2', '/b');
    expect(next.get('web-1')).toBe('/a');
    expect(next.get('web-2')).toBe('/b');
  });
});

describe('termSftpDirs store', () => {
  it('exposes remembered directories to subscribers', () => {
    termSftpDirs.remember('web-1', '/srv');
    expect(get(termSftpDirs).get('web-1')).toBe('/srv');
    termSftpDirs.remember('web-1', '');
    expect(get(termSftpDirs).get('web-1')).toBe('/srv');
  });
});
