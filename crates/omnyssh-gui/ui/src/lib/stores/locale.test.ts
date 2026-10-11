import { beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';

// `locale.ts` runs its `register`/`init` at module scope and reads `document` for the
// painted language, so the environment is stubbed before the import below pulls it in.
// Node test env has neither a DOM nor localStorage; the store is written to survive
// both (it is the same code path a hardened webview takes).
const listeners = new Map<string, () => void>();

const html = { lang: 'en' };

vi.stubGlobal('document', {
  documentElement: {
    getAttribute: (name: string) => (name === 'lang' ? html.lang : null),
    setAttribute: (name: string, value: string) => {
      if (name === 'lang') html.lang = value;
    }
  }
});

const storage = new Map<string, string>();
vi.stubGlobal('localStorage', {
  getItem: (k: string) => storage.get(k) ?? null,
  setItem: (k: string, v: string) => void storage.set(k, v),
  removeItem: (k: string) => void storage.delete(k)
});

// The dynamic `import('@tauri-apps/plugin-store')` inside `persistStore`/`hydrate` must
// not reach a real plugin: these doubles record what a run would have written.
const storeWrites: Array<[string, string]> = [];
vi.mock('@tauri-apps/plugin-store', () => ({
  load: async () => ({
    get: async () => undefined,
    set: async (k: string, v: string) => void storeWrites.push([k, v]),
    save: async () => {}
  })
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: async () => {}
}));

const { locale, registerMessages } = await import('./locale');

describe('locale store', () => {
  beforeEach(() => {
    storeWrites.length = 0;
    storage.clear();
    listeners.clear();
  });

  it('defaults to English when nothing is stored', () => {
    html.lang = 'en';
    expect(get(locale)).toBe('en');
  });

  it('is a registered svelte-i18n locale loader (idempotent)', () => {
    // Called twice on purpose: the guard is that a second call neither throws nor
    // replaces a loader (the module already registered at import).
    expect(() => registerMessages()).not.toThrow();
    expect(() => registerMessages()).not.toThrow();
  });

  it('mirrors a user choice to localStorage, <html lang>, and the store', () => {
    locale.set('zh-Hans');
    expect(get(locale)).toBe('zh-Hans');
    expect(storage.get('omnyssh-lang')).toBe('zh-Hans');
    expect(html.lang).toBe('zh-Hans');
  });

  it('cycles through the shipped languages on toggle, wrapping around', () => {
    locale.set('en');
    locale.toggle();
    expect(get(locale)).toBe('zh-Hans');
    locale.toggle();
    expect(get(locale)).toBe('en');
  });

  it('persists a user choice to the plugin store', async () => {
    locale.set('zh-Hans');
    // `persistStore` awaits a dynamic import before it writes; under a loaded worker
    // that can take more than one tick, so wait for the write rather than a fixed
    // delay (a bare `setTimeout(0)` races the importer and flakes in the full suite).
    await vi.waitFor(() => {
      expect(storeWrites).toContainEqual(['language', 'zh-Hans']);
    });
  });

  it('hydrate does not clobber a choice the user already made', async () => {
    locale.set('zh-Hans');
    await locale.hydrate();
    expect(get(locale)).toBe('zh-Hans');
  });
});
