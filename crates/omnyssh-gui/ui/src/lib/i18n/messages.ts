// Leaf modules that hold the actual message dictionaries, one file per language.
// Kept out of `index.ts` so the store can import them without pulling in the whole
// svelte-i18n runtime at module scope (matters for the no-FOUC boot path).

import en from './locales/en.json';
import zhHans from './locales/zh-Hans.json';

export type MessageDict = typeof en;

/** Every language OmnySSH ships, in the order the Settings picker lists them. */
export const MESSAGES: Record<string, MessageDict> = {
  en,
  'zh-Hans': zhHans as MessageDict
};

/** The language codes OmnySSH ships. Mirrors core `available_languages()`. */
export const LANGUAGE_CODES = ['en', 'zh-Hans'] as const;
export type LanguageCode = (typeof LANGUAGE_CODES)[number];

/** Every shipped language's own endonym + English name, for the picker. */
export const LANGUAGE_NAMES: Record<LanguageCode, { endonym: string; english: string }> = {
  en: { endonym: 'English', english: 'English' },
  'zh-Hans': { endonym: '简体中文', english: 'Chinese (Simplified)' }
};

/** Is `code` one OmnySSH ships? Unknown codes fall back to English. */
export function isLanguageCode(code: string | null | undefined): code is LanguageCode {
  return !!code && (LANGUAGE_CODES as readonly string[]).includes(code);
}
