<script lang="ts">
  // Cloud sync as its own selector screen (moved out of Settings so the vault
  // controls are one sidebar click away). The body is the shared CloudSync
  // section component, which also owns the three decision dialogs.
  import { t } from 'svelte-i18n';
  import CloudSync from '$lib/components/CloudSync.svelte';
</script>

<section class="mx-auto min-h-full max-w-2xl p-6">
  <h1 class="mb-5 text-lg font-semibold tracking-tight">{$t('cloudSync.title')}</h1>
  <CloudSync />
</section>
