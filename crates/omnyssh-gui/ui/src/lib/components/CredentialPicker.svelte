<script lang="ts">
  // App-drawn dropdown for the host editor's credential picker. The native
  // select popup is OS-drawn — on Windows it shows as a bright system list
  // whose optgroup headers cram together (unstyleable, unthemeable), which
  // is exactly what `Select.svelte` exists to avoid for the closed control. The
  // open list here is ours: grouped sections, the theme's tokens, keyboard
  // navigation (↑↓/Home/End/Enter/Esc, type-ahead optional) and click-outside
  // dismissal, mirroring `ContextMenu.svelte`'s interaction contract.
  import { t } from 'svelte-i18n';
  import { Icon } from '$lib/theme';
  import type { CredentialOption, CredentialRow } from './credentialPicker';

  let {
    value = $bindable(),
    options,
    onpick,
    label
  }: {
    value: string;
    options: CredentialOption[];
    onpick: (value: string) => void;
    /** Accessible name, since this is not a native select element. */
    label: string;
  } = $props();

  let open = $state(false);
  let root = $state<HTMLDivElement>();
  let trigger = $state<HTMLButtonElement>();
  let listEl = $state<HTMLDivElement>();
  // Highlight index into `flat` (the selectable rows); -1 = nothing highlighted.
  let highlighted = $state(-1);
  // The list is `position: fixed` (a plain absolute child would be clipped by the
  // host form's `overflow-y-auto` scroll container — inside the host editor modal
  // the dropdown towards the bottom of the form never showed). Positioned from the
  // trigger's viewport rect each open; `upward` flips it when there is no room
  // below (mirrors `ContextMenu.svelte`'s edge handling).
  let listStyle = $state<{ left: string; top: string; width: string }>({
    left: '0px',
    top: '0px',
    width: '0px'
  });
  let upward = $state(false);

  function placeList(): void {
    if (!trigger) return;
    const rect = trigger.getBoundingClientRect();
    const spaceBelow = window.innerHeight - rect.bottom;
    upward = spaceBelow < 176 && rect.top > spaceBelow; // ~4 rows
    listStyle = {
      left: `${rect.left}px`,
      top: upward ? `${Math.max(4, rect.top - 4)}px` : `${rect.bottom + 4}px`,
      width: `${rect.width}px`
    };
  }

  const current = $derived(options.find((o) => o.value === value) ?? options[0]);
  const flat = $derived<CredentialRow[]>(options.map((o, index) => ({ o, index })));
  // Sections in list order: the ungrouped head first, then each named group.
  const groups = $derived.by(() => {
    const out: { name: string | null; items: CredentialRow[] }[] = [];
    for (const item of flat) {
      const last = out.at(-1);
      if (last && last.name === item.o.group) last.items.push(item);
      else out.push({ name: item.o.group, items: [item] });
    }
    return out;
  });

  function pick(item: CredentialRow): void {
    value = item.o.value;
    onpick(item.o.value);
    close();
  }

  function close(): void {
    open = false;
    highlighted = -1;
  }

  function openList(): void {
    placeList();
    open = true;
    // Start on the selected row so Enter is a no-op and ↑↓ move from here.
    highlighted = flat.findIndex(({ o }) => o.value === value);
  }

  function toggle(): void {
    open ? close() : openList();
  }

  function move(delta: number): void {
    if (flat.length === 0) return;
    const at = highlighted;
    const next = at < 0 ? (delta > 0 ? 0 : flat.length - 1) : Math.min(flat.length - 1, Math.max(0, at + delta));
    highlighted = next;
    listEl?.querySelectorAll('[role="option"]')[next]?.scrollIntoView({ block: 'nearest' });
  }

  function onKeydown(e: KeyboardEvent): void {
    if (!open) {
      if (e.key === 'Enter' || e.key === ' ' || e.key === 'ArrowDown' || e.key === 'ArrowUp') {
        e.preventDefault();
        openList();
      }
      return;
    }
    switch (e.key) {
      case 'Escape':
        e.preventDefault();
        close();
        break;
      case 'ArrowDown':
        e.preventDefault();
        move(1);
        break;
      case 'ArrowUp':
        e.preventDefault();
        move(-1);
        break;
      case 'Home':
        e.preventDefault();
        highlighted = 0;
        break;
      case 'End':
        e.preventDefault();
        highlighted = flat.length - 1;
        break;
      case 'Enter':
      case ' ':
        e.preventDefault();
        if (highlighted >= 0) pick(flat[highlighted]);
        break;
    }
  }

  // Click-outside closes; capture phase keeps the press from landing underneath.
  // A scroll or resize would leave the fixed list floating away from its trigger,
  // so it closes too — the modal's own scroll is the common case here.
  function onWindowPointerdown(e: PointerEvent): void {
    if (open && root && !root.contains(e.target as Node)) close();
  }
  function onReflow(): void {
    if (open) close();
  }
</script>

<svelte:window onpointerdown={onWindowPointerdown} onscroll={onReflow} onresize={onReflow} />

<div bind:this={root} class="relative" role="group" aria-label={label}>
  <button
    bind:this={trigger}
    type="button"
    role="combobox"
    aria-controls="credential-list"
    aria-expanded={open}
    aria-haspopup="listbox"
    aria-label={label}
    onclick={toggle}
    onkeydown={onKeydown}
    class="flex w-full items-center justify-between gap-2 rounded-lg bg-surface-inset px-3 py-2 text-left text-sm text-fg outline-none
      transition focus-visible:ring-2 focus-visible:ring-focus"
  >
    <span class="truncate">{current?.label ?? ''}</span>
    <Icon name="chevron" size={12} class="shrink-0 text-faint" />
  </button>

  {#if open}
    <div
      bind:this={listEl}
      id="credential-list"
      role="listbox"
      aria-label={label}
      tabindex="-1"
      onkeydown={onKeydown}
      class="fixed z-50 max-h-64 overflow-y-auto rounded-xl border border-default bg-surface py-1.5 shadow-soft"
      style="left: {listStyle.left}; top: {listStyle.top}; width: {listStyle.width}; transform: {upward
        ? 'translateY(-100%)'
        : 'none'};"
    >
      {#each groups as group (group.name)}
        {#if group.name !== null}
          <div class="px-3 pb-1 pt-2 text-[11px] font-medium text-faint" role="presentation">
            {group.name}
          </div>
        {/if}
        {#each group.items as item (item.o.value)}
          <button
            type="button"
            role="option"
            aria-selected={item.o.value === value}
            onclick={() => pick(item)}
            onpointerenter={() => (highlighted = item.index)}
            class="flex w-full items-center px-3 py-1.5 text-left text-sm transition
              {item.o.value === value
                ? 'bg-accent text-accent-fg'
                : highlighted === item.index
                  ? 'bg-surface-inset text-fg'
                  : 'text-fg'}"
          >
            <span class="truncate">{item.o.label}</span>
            {#if item.o.value === value}
              <Icon name="check" size={13} class="ml-auto shrink-0" />
            {/if}
          </button>
        {/each}
      {/each}
    </div>
  {/if}
</div>
