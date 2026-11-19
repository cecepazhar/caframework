<script lang="ts">
  import { goto } from '$app/navigation';
  import { t } from '$lib/i18n/index.svelte';
  import { getPalette, closePalette } from '$lib/stores/commandPalette.svelte';
  import { navItems } from '$lib/navItems';
  import { toggleAiChat } from '$lib/stores/aiChat.svelte';
  import { getTheme, toggleTheme } from '$lib/stores/theme.svelte';
  import { checkForUpdates } from '$lib/stores/updater.svelte';
  import { can } from '$lib/stores/session.svelte';

  interface Props {
    onLock?: () => void;
  }

  let { onLock }: Props = $props();

  const palette = getPalette();
  const theme = getTheme();
  let query = $state('');
  let selectedIndex = $state(0);

  interface CommandItem {
    id: string;
    title: string;
    category: string;
    shortcut?: string;
    action: () => void;
  }

  const activeNavItems = $derived(navItems());

  const allCommands = $derived.by<CommandItem[]>(() => {
    const list: CommandItem[] = [];

    // 1. Navigation items from registered menus
    for (const item of activeNavItems) {
      list.push({
        id: `nav-${item.key}`,
        title: `Go to ${item.label}`,
        category: 'Navigation',
        action: () => {
          goto(item.href);
          closePalette();
        }
      });
    }

    // 2. Settings navigation
    list.push({
      id: 'nav-settings',
      title: 'Go to Settings',
      category: 'Navigation',
      shortcut: 'Ctrl+,',
      action: () => {
        goto('/settings');
        closePalette();
      }
    });

    list.push({
      id: 'nav-about',
      title: 'Go to About & Brand',
      category: 'Navigation',
      action: () => {
        goto('/settings?tab=about');
        closePalette();
      }
    });

    // 3. Global actions
    list.push({
      id: 'action-theme',
      title: `Switch to ${theme.name === 'dark' ? 'Light' : 'Dark'} Mode`,
      category: 'Appearance',
      action: () => {
        toggleTheme();
        closePalette();
      }
    });

    list.push({
      id: 'action-ai',
      title: 'Toggle AI Copilot',
      category: 'AI Assistant',
      action: () => {
        toggleAiChat();
        closePalette();
      }
    });

    list.push({
      id: 'action-updates',
      title: 'Check for Updates',
      category: 'System',
      action: () => {
        checkForUpdates().catch(() => {});
        closePalette();
      }
    });

    if (can('data:export')) {
      list.push({
        id: 'action-export',
        title: 'Export Data (CSV / XLSX / JSON)',
        category: 'Data',
        action: () => {
          goto('/settings?tab=dataIo');
          closePalette();
        }
      });
    }

    if (onLock) {
      list.push({
        id: 'action-lock',
        title: 'Lock Vault Immediately',
        category: 'Security',
        shortcut: 'Ctrl+L',
        action: () => {
          closePalette();
          onLock();
        }
      });
    }

    return list;
  });

  const filteredCommands = $derived(
    query.trim() === ''
      ? allCommands
      : allCommands.filter(
          (c) =>
            c.title.toLowerCase().includes(query.toLowerCase()) ||
            c.category.toLowerCase().includes(query.toLowerCase())
        )
  );

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      closePalette();
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      selectedIndex = (selectedIndex + 1) % Math.max(1, filteredCommands.length);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selectedIndex = (selectedIndex - 1 + filteredCommands.length) % Math.max(1, filteredCommands.length);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (filteredCommands[selectedIndex]) {
        filteredCommands[selectedIndex].action();
      }
    }
  }
</script>

{#if palette.open}
  <!-- Backdrop -->
  <div
    class="fixed inset-0 z-50 flex items-start justify-center pt-20 bg-black/60 backdrop-blur-sm p-4"
    role="dialog"
    aria-modal="true"
    tabindex="-1"
    onclick={(e) => {
      if (e.target === e.currentTarget) closePalette();
    }}
    onkeydown={(e) => {
      if (e.key === 'Escape') closePalette();
    }}
  >
    <div
      class="w-full max-w-xl bg-white dark:bg-neutral-900 border border-neutral-200 dark:border-neutral-800 rounded-2xl shadow-2xl overflow-hidden animate-in fade-in zoom-in-95 duration-150"
    >
      <!-- Search input -->
      <div class="flex items-center px-4 border-b border-neutral-200 dark:border-neutral-800">
        <svg class="w-5 h-5 text-neutral-400 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
        </svg>
        <input
          type="text"
          bind:value={query}
          onkeydown={handleKeyDown}
          placeholder={"Type a command or search..."}
          class="w-full px-4 py-3.5 bg-transparent text-neutral-900 dark:text-white placeholder-neutral-400 dark:placeholder-neutral-500 focus:outline-none text-sm"
        />
      </div>

      <!-- Results list -->
      <div class="max-h-80 overflow-y-auto p-2">
        {#if filteredCommands.length === 0}
          <div class="p-4 text-center text-sm text-neutral-500">
            No matching commands found.
          </div>
        {:else}
          {#each filteredCommands as cmd, idx}
            <button
              type="button"
              onclick={() => cmd.action()}
              onmouseenter={() => (selectedIndex = idx)}
              class="w-full flex items-center justify-between px-3 py-2.5 rounded-xl text-left text-sm transition-colors cursor-pointer {selectedIndex === idx ? 'bg-sky-600 text-white' : 'text-neutral-700 dark:text-neutral-300 hover:bg-neutral-100 dark:hover:bg-neutral-800'}"
            >
              <span>{cmd.title}</span>
              <div class="flex items-center gap-2">
                {#if cmd.shortcut}
                  <kbd class="text-[10px] px-1.5 py-0.5 rounded font-mono border {selectedIndex === idx ? 'border-sky-400 text-sky-100' : 'border-neutral-300 dark:border-neutral-700 text-neutral-500'}">
                    {cmd.shortcut}
                  </kbd>
                {/if}
                <span
                  class="text-xs px-2 py-0.5 rounded font-mono {selectedIndex === idx ? 'bg-sky-700 text-sky-100' : 'bg-neutral-100 dark:bg-neutral-800 text-neutral-500 dark:text-neutral-400'}"
                >
                  {cmd.category}
                </span>
              </div>
            </button>
          {/each}
        {/if}
      </div>
    </div>
  </div>
{/if}
