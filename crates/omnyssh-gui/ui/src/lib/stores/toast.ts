import { writable } from 'svelte/store';

// Lightweight toast bus (DockPilot surfaces the same moments through sonner):
// one-shot confirmations ("device code copied", "GitHub connected") that would
// otherwise vanish without a trace, since the status bar only carries errors.
// Rendered once by ToastHost in the app shell; theme comes from the semantic
// tokens, so it re-themes with everything else.

export type ToastKind = 'ok' | 'error';

export interface Toast {
  id: number;
  kind: ToastKind;
  /** Pre-localized message (callers format through i18n before pushing). */
  message: string;
}

/** How long a toast stays before clearing itself (ms). */
export const TOAST_TTL_MS = 3000;

let nextId = 1;

function createToastStore() {
  const { subscribe, update } = writable<Toast[]>([]);
  const timers = new Map<number, ReturnType<typeof setTimeout>>();

  function dismiss(id: number): void {
    const timer = timers.get(id);
    if (timer !== undefined) {
      clearTimeout(timer);
      timers.delete(id);
    }
    update((all) => all.filter((t) => t.id !== id));
  }

  return {
    subscribe,
    push(kind: ToastKind, message: string): number {
      const id = nextId++;
      update((all) => [...all, { id, kind, message }]);
      const timer = setTimeout(() => {
        timers.delete(id);
        dismiss(id);
      }, TOAST_TTL_MS);
      // Node's timers keep the test event loop alive; don't hold it open (no-op in
      // the webview) — same pattern as notifications.ts.
      (timer as { unref?: () => void }).unref?.();
      timers.set(id, timer);
      return id;
    },
    dismiss
  };
}

export const toastStore = createToastStore();

/** Fire-and-forget helpers: `toast.ok('…')` / `toast.error('…')`. */
export const toast = {
  ok: (message: string): number => toastStore.push('ok', message),
  error: (message: string): number => toastStore.push('error', message)
};
