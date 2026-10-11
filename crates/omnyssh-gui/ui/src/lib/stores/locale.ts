// The app-side source of truth for the active UI language. Mirrors `theme.ts`:
// a synchronous localStorage mirror seeds the first paint (and app.html stamps
// `<html lang>` from it), while the tauri-plugin-store copy is canonical and
// reconciled by `hydrate()` once the Tauri API is reachable.
//
// The canonical language also lives in the *shared config* (`ui.language`, written
// by the TUI and the `--lang` flag); a Tauri command is not needed to read it here
// because the Rust side mirrors the resolved language into the plugin store at
// startup. See `setLanguage`'s comment for the one-way sync.

import { writable } from 'svelte/store';
import { register, init, locale as svelteLocale } from 'svelte-i18n';
import { MESSAGES, LANGUAGE_CODES, isLanguageCode, type LanguageCode } from '$lib/i18n/messages';

export type { LanguageCode };

const LOCAL_KEY = 'omnyssh-lang';
const STORE_FILE = 'settings.json';
const STORE_KEY = 'language';

/** The language app.html already stamped onto `<html lang>`, so the store agrees
 *  with the first frame. Defaults to English. */
function paintedLanguage(): LanguageCode {
  if (typeof document !== 'undefined') {
    const attr = document.documentElement.getAttribute('lang');
    if (isLanguageCode(attr)) return attr;
  }
  return 'en';
}

function mirrorLocal(code: LanguageCode): void {
  try {
    localStorage.setItem(LOCAL_KEY, code);
  } catch {
    // localStorage unavailable (hardened webview): the store copy is canonical.
  }
}

function reflectHtmlLang(code: LanguageCode): void {
  if (typeof document !== 'undefined') {
    document.documentElement.setAttribute('lang', code);
  }
}

async function persistStore(code: LanguageCode): Promise<void> {
  try {
    const { load } = await import('@tauri-apps/plugin-store');
    const store = await load(STORE_FILE);
    await store.set(STORE_KEY, code);
    await store.save();
  } catch {
    // Not under Tauri (tests, vite preview): the localStorage mirror suffices.
  }
}

/** Point the native window's decorations (macOS menu bar, dialogs) at `code`, and
 *  ask the backend to rebuild its own native strings (tray tooltip, menu items).
 *  Best-effort: off Tauri there is nothing to follow, and the in-app UI has already
 *  flipped by the time this resolves. */
async function syncNative(code: LanguageCode): Promise<void> {
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('set_language', { code });
  } catch {
    // No backend / command not yet registered (tests, vite preview): fine.
  }
}

/** Register every shipped dictionary with svelte-i18n exactly once. Idempotent, so
 *  it is safe to call from both the module's top level and any test harness. */
export function registerMessages(): void {
  for (const code of LANGUAGE_CODES) register(code, () => Promise.resolve(MESSAGES[code]));
}

// Register + seed synchronously at import time, so a component that calls `$t`
// during its first render never sees an uninitialised locale (which would render the
// message key instead of text).
registerMessages();

function createLocale() {
  const painted = paintedLanguage();
  const { subscribe, set: setStore } = writable<LanguageCode>(painted);
  let current = painted;
  let interacted = false;

  function apply(code: LanguageCode, user: boolean): void {
    current = code;
    reflectHtmlLang(code);
    setStore(code);
    mirrorLocal(code);
    // svelte-i18n's own store drives `$t`; `initialLocale` was set at init so this
    // only ever swaps between already-registered dictionaries (a sync lookup).
    void svelteLocale.set(code);
    if (user) {
      interacted = true;
      void persistStore(code);
      void syncNative(code);
    }
  }

  return {
    subscribe,
    /** A deliberate user choice: persists to the canonical store and re-strings the
     *  native surfaces. `hydrate` never clobbers it afterwards. */
    set: (code: LanguageCode) => apply(code, true),
    /** Cycle to the next shipped language (used by a keyboard shortcut / menu item). */
    toggle: () => {
      const i = LANGUAGE_CODES.indexOf(current);
      apply(LANGUAGE_CODES[(i + 1) % LANGUAGE_CODES.length], true);
    },
    /** Reconcile with the canonical tauri-plugin-store value (called from the
     *  layout's onMount). The Rust side mirrors the shared config's `ui.language`
     *  here at startup, so this picks up a choice made in the TUI or via `--lang`. */
    async hydrate(): Promise<void> {
      try {
        const { load } = await import('@tauri-apps/plugin-store');
        const store = await load(STORE_FILE);
        const saved = await store.get<string>(STORE_KEY);
        // Skip if the user already chose during the async load (no clobber).
        if (!interacted && isLanguageCode(saved)) apply(saved, false);
      } catch {
        // Store unreachable: keep the painted/localStorage language.
      }
    }
  };
}

export const locale = createLocale();

// Initialise svelte-i18n once, after the dictionaries are registered. `fallbackLocale`
// is the safety net; `initialLocale` is the already-painted language so the first
// synchronous `$t` resolves without a flash of the fallback.
init({
  fallbackLocale: 'en',
  initialLocale: paintedLanguage()
});
