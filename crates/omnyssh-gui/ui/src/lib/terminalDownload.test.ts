import { describe, expect, it } from 'vitest';
import { manualDownloadReady } from './terminalDownload';

describe('manualDownloadReady', () => {
  it('空路径不可用', () => {
    expect(manualDownloadReady('')).toBe(false);
    expect(manualDownloadReady('   ')).toBe(false);
  });

  it('根目录不可下载', () => {
    expect(manualDownloadReady('/')).toBe(false);
  });

  it('普通绝对路径可用', () => {
    expect(manualDownloadReady('/var/log/syslog')).toBe(true);
  });

  it('相对路径也放行（交给核心侧校验）', () => {
    expect(manualDownloadReady('./file.txt')).toBe(true);
  });
});
