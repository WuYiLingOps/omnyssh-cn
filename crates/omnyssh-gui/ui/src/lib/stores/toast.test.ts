import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';
import { toastStore, toast, TOAST_TTL_MS } from './toast';

// The toast bus is a self-clearing list: push appends, the TTL timer removes,
// and an explicit dismiss cancels the timer. The store must also not keep the
// Node test event loop alive (same contract as notifications.ts).
describe('toast store', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    // Module-level singleton: drain leftovers from a previous test (their TTL
    // timers belong to the fake clock that just got replaced).
    for (const t of get(toastStore)) toastStore.dismiss(t.id);
  });
  afterEach(() => vi.useRealTimers());

  it('pushes ok/error toasts with monotonically increasing ids', () => {
    const first = toast.ok('a');
    const second = toast.error('b');
    expect(first).toBeLessThan(second);
    expect(get(toastStore)).toEqual([
      { id: first, kind: 'ok', message: 'a' },
      { id: second, kind: 'error', message: 'b' }
    ]);
  });

  it('auto-clears a toast after the TTL', () => {
    const id = toast.ok('bye');
    vi.advanceTimersByTime(TOAST_TTL_MS - 1);
    expect(get(toastStore)).toHaveLength(1);
    vi.advanceTimersByTime(1);
    expect(get(toastStore)).toEqual([]);
  });

  it('dismiss cancels the pending auto-clear', () => {
    const id = toast.ok('gone');
    toastStore.dismiss(id);
    expect(get(toastStore)).toEqual([]);
    expect(() => vi.advanceTimersByTime(TOAST_TTL_MS)).not.toThrow();
    expect(get(toastStore)).toEqual([]);
  });
});
