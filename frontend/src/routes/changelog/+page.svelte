<script lang="ts">
  import {
    PageContainer,
    PageHeader,
    Card,
    Badge,
    Button,
    Breadcrumb
  } from '@cecepazhar/caui';

  interface ReleaseEntry {
    version: string;
    date: string;
    isLatest?: boolean;
    codename?: string;
    changes: Array<{
      type: 'added' | 'changed' | 'fixed' | 'security';
      description: string;
    }>;
  }

  const releases: ReleaseEntry[] = [
    {
      version: 'v1.3.0',
      date: 'October 16, 2026',
      isLatest: true,
      codename: 'Platinum Sovereign',
      changes: [
        { type: 'added', description: '19 new sovereign UI components in @cecepazhar/caui (EmptyState, LoadingState, ErrorState, TitleBar, WindowControls, PaneHeader, GlobalSplitControls, DualSplitViewport, TabGroupStrip, WorkspaceMenu, etc.).' },
        { type: 'added', description: '5 Core Archetype screens for future apps: Table, Form, Cards, Analytics/Bars, and Dashboard.' },
        { type: 'added', description: 'Web & Documentation portal routes: Landing, Blog, Changelog, and Docs.' },
        { type: 'changed', description: 'Full 1:1 component synchronization between CAFramework and CAUI design system.' },
        { type: 'security', description: 'Hardened zero-knowledge lock screen with 5-minute lockout on 5 consecutive failed attempts.' }
      ]
    },
    {
      version: 'v1.2.0',
      date: 'October 11, 2026',
      codename: 'CAUI Bridge',
      changes: [
        { type: 'added', description: 'Mobile bottom navigation & all-menus overlay sheet.' },
        { type: 'added', description: 'Feedback modal and about dialog primitives.' },
        { type: 'changed', description: 'Unified CSS color tokens across 17 apps in the CA family.' },
        { type: 'fixed', description: 'Focus-visible ring outline styling in input controls.' }
      ]
    },
    {
      version: 'v1.1.0',
      date: 'October 5, 2026',
      codename: 'Runes Awakening',
      changes: [
        { type: 'added', description: 'Svelte 5 runes migration ($state, $derived, $effect) across all core widgets.' },
        { type: 'added', description: 'Dual split viewport with in-pane and global layout modes.' },
        { type: 'security', description: 'Argon2id password hashing upgrade for AES-256-GCM vault.' }
      ]
    }
  ];

  function badgeVariant(type: string): 'success' | 'primary' | 'warning' | 'danger' {
    switch (type) {
      case 'added': return 'success';
      case 'changed': return 'primary';
      case 'fixed': return 'warning';
      case 'security': return 'danger';
      default: return 'primary';
    }
  }
</script>

<PageContainer class="p-6 space-y-6 max-w-4xl mx-auto">
  <PageHeader
    title={'Changelog & Release Notes'}
    subtitle={'Chronological audit of new features, performance enhancements, and security updates.'}
    accent="indigo"
  >
    {#snippet action()}
      <Button variant="outline" size="sm" href="https://github.com/cecepazhar" target="_blank">
        GitHub Releases ↗
      </Button>
    {/snippet}
  </PageHeader>

  <Breadcrumb items={['Home', 'Changelog']} />

  <!-- Timeline container -->
  <div class="relative pl-6 space-y-8 before:absolute before:left-2 before:top-3 before:bottom-3 before:w-0.5 before:bg-[var(--ca-border)]">
    {#each releases as rel}
      <div class="relative space-y-3">
        <!-- Dot marker -->
        <span class="absolute -left-6 top-1.5 w-3.5 h-3.5 rounded-full border-2 border-[var(--ca-surface-base)] {rel.isLatest ? 'bg-[var(--ca-brand)] ring-4 ring-indigo-500/20' : 'bg-[var(--ca-border)]'}"></span>

        <!-- Version Header -->
        <div class="flex items-center gap-3">
          <h3 class="text-xl font-black font-mono tracking-tight text-[var(--ca-text-primary)]">{rel.version}</h3>
          {#if rel.isLatest}
            <Badge variant="primary" class="font-mono text-[10px]">LATEST</Badge>
          {/if}
          {#if rel.codename}
            <span class="text-xs text-[var(--ca-brand)] font-medium">"{rel.codename}"</span>
          {/if}
          <span class="text-xs text-[var(--ca-text-muted)] ml-auto font-mono">{rel.date}</span>
        </div>

        <!-- Release Notes Card -->
        <Card class="p-5 space-y-3">
          <ul class="space-y-2.5">
            {#each rel.changes as change}
              <li class="flex items-start gap-2.5 text-xs text-[var(--ca-text-secondary)]">
                <Badge variant={badgeVariant(change.type)} class="text-[9px] font-mono uppercase mt-0.5 shrink-0 w-16 text-center justify-center">
                  {change.type}
                </Badge>
                <span class="leading-relaxed flex-1">{change.description}</span>
              </li>
            {/each}
          </ul>
        </Card>
      </div>
    {/each}
  </div>
</PageContainer>
