<script lang="ts">
  import { t } from '$lib/i18n/index.svelte';

  interface Props {
    title?: string;
    message?: string;
    retryLabel?: string;
    onretry?: () => void;
  }

  let {
    title = 'Backend Service Unavailable',
    message = 'Cannot communicate with the core background service. Please verify the application process is running.',
    retryLabel = 'Reconnect',
    onretry
  }: Props = $props();
</script>

<div class="flex flex-col items-center justify-center py-16 px-4 text-center">
  <div class="w-16 h-16 rounded-2xl bg-amber-500/10 border border-amber-500/20 flex items-center justify-center text-amber-500 mb-4 shadow-inner">
    <svg class="w-8 h-8" fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M18.364 5.636a9 9 0 010 12.728m0 0l-2.829-2.829m2.829 2.829L21 21M15.536 8.464a5 5 0 010 7.072m0 0l-2.829-2.829m-4.243 4.243a9 9 0 01-12.728 0m0 0l2.829-2.829m-2.829 2.829L3 21m2.828-12.536a5 5 0 017.072 0m-4.243 4.243L8.464 9.88" />
    </svg>
  </div>

  <h3 class="text-base font-medium text-neutral-900 dark:text-zinc-200">{title}</h3>
  <p class="text-sm text-neutral-500 dark:text-zinc-400 mt-1 max-w-sm">{message}</p>

  {#if onretry}
    <button
      onclick={onretry}
      class="mt-5 inline-flex items-center gap-2 px-4 py-2 bg-neutral-900 hover:bg-neutral-800 text-white dark:bg-zinc-800 dark:hover:bg-zinc-700 dark:text-zinc-200 rounded-lg text-sm font-medium transition border border-neutral-700 cursor-pointer"
    >
      <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
      </svg>
      {retryLabel}
    </button>
  {/if}
</div>

