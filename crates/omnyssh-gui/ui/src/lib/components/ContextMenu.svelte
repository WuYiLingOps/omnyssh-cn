<script lang="ts">
  // 终端右键菜单（tech-gui.md §4.2 传输）。设计为通用浮动操作列表：条目带可选图标、
  // 分隔线、禁用态；支持键盘导航（↑↓/Enter/Esc/Home/End）与点击外部关闭。
  // 打开时接管焦点，让 xterm 停止捕获按键；关闭时经 `onclose` 把焦点还给终端。
  import { t } from 'svelte-i18n';
  import { Icon, type IconName } from '$lib/theme';

  export interface ContextMenuItem {
    /** 传给 `onselect` 的稳定动作 id。 */
    id: string;
    label?: string;
    icon?: IconName;
    disabled?: boolean;
    /** 为 true 时在该条目上方渲染分隔线，而非操作行。 */
    separator?: boolean;
  }

  let {
    x,
    y,
    items,
    onselect,
    onclose
  }: {
    x: number;
    y: number;
    items: ContextMenuItem[];
    onselect: (id: string) => void;
    onclose: () => void;
  } = $props();

  let menu: HTMLDivElement;
  // 默认选中第一个可用条目，按 Enter 即可触发最常用动作。组件随菜单开合整体重建
  // （外层 {#if}），初始值就是本次打开的最新值，因此这里捕获一次是刻意的。
  // svelte-ignore state_referenced_locally
  let selected = $state(Math.max(0, items.findIndex((item) => !item.disabled && !item.separator)));

  // 菜单贴近窗口边缘时向内翻转：终端铺满窗口，在底部/右侧打开的菜单必须朝上/朝左展开。
  $effect(() => {
    const rect = menu.getBoundingClientRect();
    const left = Math.max(4, Math.min(x, window.innerWidth - rect.width - 4));
    const top = Math.max(4, Math.min(y, window.innerHeight - rect.height - 4));
    menu.style.left = `${left}px`;
    menu.style.top = `${top}px`;
    menu.focus();
  });

  // 可选中的行（非禁用、非分隔线），按序排列——↑↓ 与 Home/End 只在这些行间移动。
  const selectable = $derived(
    items.map((item, index) => ({ item, index })).filter(({ item }) => !item.disabled && !item.separator)
  );

  function move(delta: number): void {
    if (selectable.length === 0) return;
    const at = selectable.findIndex(({ index }) => index === selected);
    const next = selectable[(at + delta + selectable.length) % selectable.length];
    selected = next.index;
  }

  function choose(): void {
    const item = items[selected];
    if (!item || item.disabled || item.separator) return;
    onselect(item.id);
    onclose();
  }

  function onKeydown(e: KeyboardEvent): void {
    switch (e.key) {
      case 'Escape':
      case 'ArrowLeft':
      case 'ArrowRight':
        e.preventDefault();
        onclose();
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
        if (selectable[0]) selected = selectable[0].index;
        break;
      case 'End':
        e.preventDefault();
        if (selectable.length > 0) selected = selectable[selectable.length - 1].index;
        break;
      case 'Enter':
      case ' ':
        e.preventDefault();
        choose();
        break;
    }
  }

  // 菜单外任意按下即关闭。用捕获阶段：这次点击不会再落到下层元素上（避免二次触发）。
  function onWindowPointerdown(e: PointerEvent): void {
    if (!menu.contains(e.target as Node)) {
      e.stopPropagation();
      onclose();
    }
  }
</script>

<svelte:window onpointerdown={onWindowPointerdown} oncontextmenu={(e) => e.preventDefault()} />

<div
  bind:this={menu}
  role="menu"
  tabindex="-1"
  class="fixed z-50 min-w-52 rounded-xl border border-default bg-surface py-1.5 shadow-soft
    outline-none focus-visible:ring-2 focus-visible:ring-focus"
  style="left: {x}px; top: {y}px"
  onkeydown={onKeydown}
>
  {#each items as item, index (item.id)}
    {#if item.separator}
      <div class="my-1.5 border-t border-default" role="separator"></div>
    {:else}
      <button
        type="button"
        role="menuitem"
        disabled={item.disabled}
        class="flex w-full items-center gap-2.5 px-3.5 py-1.5 text-left text-sm transition
          {selected === index
            ? 'bg-accent text-accent-fg'
            : 'text-fg hover:bg-surface-inset'}
          disabled:cursor-not-allowed disabled:opacity-40 disabled:hover:bg-transparent"
        onpointerenter={() => (selected = index)}
        onclick={() => {
          if (item.disabled) return;
          onselect(item.id);
          onclose();
        }}
      >
        {#if item.icon}
          <Icon name={item.icon} size={14} />
        {/if}
        {item.label}
      </button>
    {/if}
  {/each}
  <!-- 一个没有任何可选项的菜单属于调用方 bug，但空菜单也得有出口：Escape 和点击
       外部都能关闭；这个 sr-only 文案让无障碍语义保持完整。 -->
  {#if selectable.length === 0}
    <span class="sr-only">{$t('terminal.menuEmpty')}</span>
  {/if}
</div>
