import { describe, expect, it } from 'vitest';
import en from './locales/en.json';
import zhHans from './locales/zh-Hans.json';
import { LANGUAGE_CODES, LANGUAGE_NAMES, MESSAGES, isLanguageCode } from './messages';

// Message-pack integrity (tech-gui.md i18n plan §3.8). Two failure modes this must
// catch before a release: a key present in one language and not the other (a screen
// that renders its raw key), and a placeholder used in translation but absent from the
// English source (a runtime throw from svelte-i18n's formatter).

/** Every leaf path, dotted, in a nested message object. */
function leafPaths(value: unknown, prefix = ''): string[] {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    return prefix ? [prefix] : [];
  }
  return Object.entries(value as Record<string, unknown>).flatMap(([key, child]) =>
    leafPaths(child, prefix ? `${prefix}.${key}` : key)
  );
}

function atPath(messages: unknown, path: string): unknown {
  return path
    .split('.')
    .reduce<unknown>(
      (node, part) =>
        typeof node === 'object' && node !== null
          ? (node as Record<string, unknown>)[part]
          : undefined,
      messages
    );
}

/** The `{name}` / `{count, plural, ...}` placeholders a message interpolates. */
function placeholders(message: string): string[] {
  return [...message.matchAll(/\{\s*([a-zA-Z0-9_]+)\s*[,}]/g)].map((m) => m[1]).sort();
}

const EN_PATHS = leafPaths(en).sort();

describe('message packs', () => {
  it('ship exactly the languages the picker lists', () => {
    expect(Object.keys(MESSAGES).sort()).toEqual([...LANGUAGE_CODES].sort());
  });

  it('have every key in every language, and no extras', () => {
    for (const code of LANGUAGE_CODES) {
      expect(leafPaths(MESSAGES[code]).sort(), `${code} key set`).toEqual(EN_PATHS);
    }
  });

  it('have no empty message in any language', () => {
    for (const code of LANGUAGE_CODES) {
      for (const path of EN_PATHS) {
        expect(String(atPath(MESSAGES[code], path)).trim(), `${code}:${path}`).not.toBe('');
      }
    }
  });

  it('use the same placeholders as the English source', () => {
    for (const code of LANGUAGE_CODES) {
      for (const path of EN_PATHS) {
        const source = placeholders(String(atPath(en, path)));
        const translated = placeholders(String(atPath(MESSAGES[code], path)));
        expect(translated, `${code}:${path} placeholders`).toEqual(source);
      }
    }
  });

  it('never wrap a placeholder in ASCII single quotes', () => {
    // ICU MessageFormat reads a single quote as an escape: `'{host}'` renders the
    // literal text `{host}` instead of interpolating it. Typographic quotes
    // (U+2018/U+2019) are safe and are what the packs use around a value.
    for (const code of LANGUAGE_CODES) {
      for (const path of EN_PATHS) {
        const message = String(atPath(MESSAGES[code], path));
        expect(message, `${code}:${path} must not escape its placeholders`).not.toMatch(
          /'\{[^}]*\}'/
        );
      }
    }
  });

  it('leave brand names, command tokens and metric abbreviations untranslated', () => {
    // §2.4 white list: these must survive as-is in every language.
    const INVARIANT = ['OmnySSH', 'SFTP', 'SSH', 'CPU', 'RAM', 'Disk'];
    for (const code of LANGUAGE_CODES) {
      const flat = leafPaths(MESSAGES[code])
        .map((p) => String(atPath(MESSAGES[code], p)))
        .join('\n');
      for (const token of INVARIANT) {
        // Only assert where the English source also mentions the token.
        const inSource = EN_PATHS.some(
          (p) => String(atPath(en, p)).includes(token) && String(atPath(MESSAGES[code], p)).includes(token)
        );
        if (inSource) expect(flat).toContain(token);
      }
    }
  });
});

describe('language codes', () => {
  it('accept only shipped codes', () => {
    expect(isLanguageCode('en')).toBe(true);
    expect(isLanguageCode('zh-Hans')).toBe(true);
    expect(isLanguageCode('zh')).toBe(false);
    expect(isLanguageCode('fr')).toBe(false);
    expect(isLanguageCode(null)).toBe(false);
    expect(isLanguageCode(undefined)).toBe(false);
    expect(isLanguageCode('')).toBe(false);
  });

  it('name every shipped language in both its own and English script', () => {
    for (const code of LANGUAGE_CODES) {
      expect(LANGUAGE_NAMES[code].endonym).toBeTruthy();
      expect(LANGUAGE_NAMES[code].english).toBeTruthy();
    }
    expect(LANGUAGE_NAMES['zh-Hans'].endonym).toBe('简体中文');
  });

  it('keep the Chinese pack actually translated (spot check)', () => {
    // The guard above cannot tell a real translation from a copied English string.
    expect(zhHans.settings.title).toBe('设置');
    expect(zhHans.dashboard.title).toBe('仪表盘');
    expect(en.settings.title).toBe('Settings');
  });
});
