import { describe, expect, it, beforeAll, beforeEach, afterAll } from 'vitest';
import { registerMessages } from '$lib/stores/locale';
import { locale as svelteLocale } from 'svelte-i18n';
import { get } from 'svelte/store';
import { errorMessage, isKnownErrorCode } from './errors';
import en from './locales/en.json';
import zhHans from './locales/zh-Hans.json';

// A coded backend error must render in the active language; one with no code, or a
// code the packs do not carry, must show the backend's own text verbatim (§3.7).
// Both are guards against the two ways this can silently go wrong: a raw code
// leaking onto the screen, and a translated error losing to a stale English one.

beforeAll(() => {
  registerMessages();
});

// Each test states its own locale, so one that flips to Chinese cannot leak into
// the next (vitest shares the module state within a file).
beforeEach(async () => {
  await svelteLocale.set('en');
});

afterAll(async () => {
  await svelteLocale.set('en');
});

describe('errorMessage', () => {
  it('translates a known code, interpolating its params', async () => {
    await svelteLocale.set('en');
    expect(get(svelteLocale)).toBe('en');
    expect(errorMessage({ code: 'unknownHost', params: { host: 'web' }, message: 'raw' })).toBe(
      'Unknown host \u2018web\u2019.'
    );
  });

  it('renders the same code in Chinese once the locale flips', async () => {
    await svelteLocale.set('zh-Hans');
    const shown = errorMessage({
      code: 'unknownHost',
      params: { host: 'web' },
      message: "unknown host 'web'"
    });
    expect(shown).toContain('web');
    expect(shown).not.toContain('Unknown host');
  });

  it('falls back to the backend message for a code with no translation', () => {
    expect(errorMessage({ code: 'engineBlewUp', message: 'connection reset by peer' })).toBe(
      'connection reset by peer'
    );
  });

  it('shows a code-less error verbatim', () => {
    expect(errorMessage({ message: 'Access is denied.' })).toBe('Access is denied.');
  });

  it('tolerates a coded error that carries no params', () => {
    expect(errorMessage({ code: 'noMatchingHosts', message: 'no matching hosts' })).toContain(
      'matching hosts'
    );
  });
  it('falls back to the backend message when its params are missing', () => {
    // The pack's `invalidForward` interpolates `{forward}`; without it the formatter
    // would throw, so the backend's own text must win instead of a raw key.
    expect(errorMessage({ code: 'invalidForward', message: 'bad forward' })).toBe('bad forward');
  });
});

describe('isKnownErrorCode', () => {
  it('knows the codes the packs carry and rejects the rest', () => {
    expect(isKnownErrorCode('unknownHost')).toBe(true);
    expect(isKnownErrorCode('invalidForward')).toBe(true);
    expect(isKnownErrorCode('engineBlewUp')).toBe(false);
    expect(isKnownErrorCode('')).toBe(false);
  });

  it('points every mapped code at a real errors.* key in both packs', () => {
    // A code mapped to a key neither pack carries would render the raw key — the
    // exact failure this module exists to prevent. `isKnownErrorCode` is the public
    // surface of the map, so probe it against both dictionaries.
    const CODES = [
      'taskFailed',
      'unknownHost',
      'hostExists',
      'snippetNotFound',
      'snippetExists',
      'noMatchingHosts',
      'noUpdateAvailable',
      'updateUnavailable',
      'updateFailed',
      'invalidForward',
      'invalidName',
      'invalidHostname',
      'invalidPort',
      'saveFailed'
    ];
    for (const code of CODES) {
      expect(isKnownErrorCode(code), code).toBe(true);
    }
    // Every `errors.*` leaf the packs carry is either reachable or a plain-string
    // message (generic/hostExists/… used directly by components).
    const errorLeaves = Object.keys(en.errors);
    const zhLeaves = Object.keys(zhHans.errors);
    expect(zhLeaves.sort()).toEqual(errorLeaves.sort());
  });
});
