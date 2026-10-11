// Renders a backend `CommandError` in the active language (i18n plan §3.7).
//
// The backend sends a stable `code` plus substitution `params` for the errors the
// UI should translate, and nothing but a `message` for the ones that are really the
// engine's own text (an I/O failure, an SSH rejection). This turns the first into a
// localized string and falls back to the second verbatim — so an error never renders
// as a raw key, and a translated one never leaks English.

import { unwrapFunctionStore, format } from 'svelte-i18n';
import type { CommandError } from '$lib/bindings';

/** The `errors.*` key a backend code maps to. Only codes the packs actually carry
 *  appear here; an unknown code falls back to the backend's own `message`. */
const ERROR_KEYS: Readonly<Record<string, string>> = {
  taskFailed: 'errors.taskFailed',
  unknownHost: 'errors.unknownHost',
  hostExists: 'errors.hostExists',
  snippetNotFound: 'errors.snippetNotFound',
  snippetExists: 'errors.snippetExists',
  noMatchingHosts: 'errors.noMatchingHosts',
  noUpdateAvailable: 'errors.noUpdateAvailable',
  updateUnavailable: 'errors.updateUnavailable',
  updateFailed: 'errors.updateFailed',
  invalidForward: 'errors.invalidForward',
  invalidName: 'errors.invalidName',
  invalidHostname: 'errors.invalidHostname',
  invalidPort: 'errors.invalidPort',
  saveFailed: 'errors.saveFailed'
};

/** The `{name}` placeholders a message interpolates, read off the English pack. */
const NEEDED_PARAMS: Readonly<Record<string, readonly string[]>> = {
  taskFailed: ['error'],
  unknownHost: ['host'],
  hostExists: ['name'],
  snippetNotFound: ['snippet'],
  snippetExists: ['name'],
  updateUnavailable: ['error'],
  updateFailed: ['error'],
  invalidForward: ['forward'],
  saveFailed: ['error']
};

/** Whether `code` is one the packs translate. */
export function isKnownErrorCode(code: string): boolean {
  return code in ERROR_KEYS;
}

/**
 * The message to show for a failed command: the localized text for a known `code`
 * (interpolating its `params`), otherwise the backend's own `message`.
 *
 * A code whose translation needs a value the backend did not send must NOT reach the
 * formatter: svelte-i18n would log a syntax error and return the raw `{placeholder}`
 * template, which is worse to read than the backend's own text. So the params are
 * checked first and a missing one falls back to `message`.
 */
export function errorMessage(error: CommandError): string {
  const code = error.code ?? undefined;
  const key = code === undefined ? undefined : ERROR_KEYS[code];
  if (key === undefined || code === undefined) return error.message;
  const params = error.params ?? {};
  const needed: readonly string[] = NEEDED_PARAMS[code] ?? [];
  if (needed.some((name) => params[name] === undefined)) return error.message;
  // `$t` is a store-of-formatter; outside a component it needs the locale resolved
  // first. `unwrapFunctionStore` gives a callable that reads it.
  const t = unwrapFunctionStore(format);
  return t(key, { values: params });
}
