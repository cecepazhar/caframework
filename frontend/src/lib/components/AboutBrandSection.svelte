<script lang="ts">
  import { BRAND_CONFIG } from '$lib/generated/brand';
  import { APP_NAME, APP_VERSION } from '$lib/appInfo';
  import Logo from '$lib/components/Logo.svelte';
  import { t } from '$lib/i18n/index.svelte';

  const currentYear = new Date().getFullYear().toString();
  const copyrightText = BRAND_CONFIG.copyright.replace('{year}', currentYear);
</script>

<div class="space-y-6">
  <div class="bg-white dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 rounded-lg p-6 shadow-sm dark:shadow-none text-neutral-900 dark:text-white">
    <div class="flex items-center gap-4 pb-6 border-b border-neutral-200 dark:border-neutral-800">
      <Logo size={48} mode="brand" />
      <div>
        <h2 class="text-xl font-bold tracking-tight text-neutral-900 dark:text-white flex items-center gap-2">
          {APP_NAME}
          <span class="text-xs px-2 py-0.5 rounded-full border border-neutral-300 dark:border-neutral-700 text-neutral-500 font-mono">
            v{APP_VERSION}
          </span>
        </h2>
        <p class="text-sm text-neutral-500 dark:text-neutral-400 mt-0.5">{BRAND_CONFIG.developer} • {BRAND_CONFIG.company}</p>
      </div>
    </div>

    <div class="grid grid-cols-1 md:grid-cols-2 gap-4 py-6 border-b border-neutral-200 dark:border-neutral-800 text-sm">
      <div>
        <span class="block text-xs font-medium text-neutral-500 dark:text-neutral-400 uppercase tracking-wider mb-1">
          {t('common.developer') || 'Pengembang'}
        </span>
        <span class="text-neutral-900 dark:text-white font-medium">{BRAND_CONFIG.developer}</span>
      </div>

      {#if BRAND_CONFIG.company}
        <div>
          <span class="block text-xs font-medium text-neutral-500 dark:text-neutral-400 uppercase tracking-wider mb-1">
            {t('common.company') || 'Perusahaan / Studio'}
          </span>
          <span class="text-neutral-900 dark:text-white font-medium">{BRAND_CONFIG.company}</span>
        </div>
      {/if}

      <div>
        <span class="block text-xs font-medium text-neutral-500 dark:text-neutral-400 uppercase tracking-wider mb-1">
          {t('common.website') || 'Situs Web'}
        </span>
        <a
          href={BRAND_CONFIG.website}
          target="_blank"
          rel="noopener noreferrer"
          class="text-sky-600 dark:text-sky-400 hover:underline inline-flex items-center gap-1 font-medium"
        >
          {BRAND_CONFIG.website}
          <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14" />
          </svg>
        </a>
      </div>

      <div>
        <span class="block text-xs font-medium text-neutral-500 dark:text-neutral-400 uppercase tracking-wider mb-1">
          {t('common.email') || 'Email Kontak'}
        </span>
        <a href="mailto:{BRAND_CONFIG.email}" class="text-neutral-900 dark:text-white hover:text-sky-600 dark:hover:text-sky-400 font-medium">
          {BRAND_CONFIG.email}
        </a>
      </div>

      {#if BRAND_CONFIG.phone}
        <div>
          <span class="block text-xs font-medium text-neutral-500 dark:text-neutral-400 uppercase tracking-wider mb-1">
            {t('common.phone') || 'Telepon'}
          </span>
          <span class="text-neutral-900 dark:text-white font-mono">{BRAND_CONFIG.phone}</span>
        </div>
      {/if}

      {#if BRAND_CONFIG.supportUrl}
        <div>
          <span class="block text-xs font-medium text-neutral-500 dark:text-neutral-400 uppercase tracking-wider mb-1">
            {t('common.support') || 'Dukungan'}
          </span>
          <a
            href={BRAND_CONFIG.supportUrl}
            target="_blank"
            rel="noopener noreferrer"
            class="text-sky-600 dark:text-sky-400 hover:underline inline-flex items-center gap-1 font-medium"
          >
            {BRAND_CONFIG.supportUrl}
          </a>
        </div>
      {/if}
    </div>

    <div class="pt-6 flex flex-col sm:flex-row sm:items-center sm:justify-between gap-2 text-xs text-neutral-500">
      <span>{copyrightText}</span>
      <span class="font-mono">Engine: CAFramework (v0.1.0)</span>
    </div>
  </div>
</div>

