<script lang="ts">
  // Sidebar-footer toggle (tech-gui.md §5.1): one press flips light↔dark. Sun while
  // dark (press → light), moon while light. Icon-only, so it survives collapse.
  // Compact (h-8/w-8): the sidebar footer is tighter than the generic icon button.
  import { theme } from '$lib/stores/theme';
  import { Icon } from '$lib/theme';
  import { t } from 'svelte-i18n';
</script>

<button
  type="button"
  class="grid h-8 w-8 place-items-center rounded-full text-muted transition hover:bg-surface-inset hover:text-fg focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus"
  title={$theme === 'dark' ? $t('theme.toLight') : $t('theme.toDark')}
  aria-label={$theme === 'dark' ? $t('theme.toLight') : $t('theme.toDark')}
  onclick={() => theme.toggle()}
>
  <Icon name={$theme === 'dark' ? 'sun' : 'moon'} />
</button>
