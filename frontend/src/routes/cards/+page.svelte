<script lang="ts">
  import {
    PageContainer,
    PageHeader,
    Card,
    Badge,
    Button,
    Avatar,
    AvatarGroup,
    Tooltip,
    HoverCard,
    TotpBadge,
    SearchInput
  } from '@cecepazhar/caui';

  interface ProjectCard {
    id: string;
    title: string;
    description: string;
    category: string;
    status: 'ACTIVE' | 'BUILDING' | 'STABLE';
    accent: string;
    stars: number;
    members: Array<{ name: string; avatar?: string }>;
    totpCode?: string;
  }

  const projects: ProjectCard[] = [
    {
      id: 'proj-caterm',
      title: 'CATerm',
      description: 'Sovereign SSH/SFTP terminal client with zero-knowledge encrypted vault and Hana AI.',
      category: 'Terminal',
      status: 'STABLE',
      accent: '#ef4444',
      stars: 48,
      members: [{ name: 'Cecep' }, { name: 'Harun' }, { name: 'Fatih' }],
      totpCode: '492 108'
    },
    {
      id: 'proj-camark',
      title: 'CAMark',
      description: 'Distraction-free markdown editor & personal knowledge base with bi-directional wikilinks.',
      category: 'Productivity',
      status: 'ACTIVE',
      accent: '#06b6d4',
      stars: 29,
      members: [{ name: 'Cecep' }, { name: 'Rini' }]
    },
    {
      id: 'proj-catable',
      title: 'CATable',
      description: 'Modern offline-first spreadsheet and reactive database client with formula computation.',
      category: 'Data',
      status: 'BUILDING',
      accent: '#3b82f6',
      stars: 14,
      members: [{ name: 'Cecep' }, { name: 'Ibrahim' }]
    },
    {
      id: 'proj-cagames',
      title: 'CAGames',
      description: 'Retro game engine & emulator launcher built with WebAssembly and local ROM library.',
      category: 'Entertainment',
      status: 'BUILDING',
      accent: '#10b981',
      stars: 9,
      members: [{ name: 'Fatih' }, { name: 'Harun' }]
    },
    {
      id: 'proj-cabook',
      title: 'CABook',
      description: 'Digital reading companion, PDF annotator, and EPUB synchronizer with local TTS.',
      category: 'Reader',
      status: 'ACTIVE',
      accent: '#f59e0b',
      stars: 33,
      members: [{ name: 'Cecep' }, { name: 'Khadijah' }]
    },
    {
      id: 'proj-caui',
      title: 'CAUI Design System',
      description: 'Unified Svelte 5 component suite, iconography, and design tokens for the entire CA ecosystem.',
      category: 'Foundation',
      status: 'STABLE',
      accent: '#6366f1',
      stars: 120,
      members: [{ name: 'Cecep' }, { name: 'Abdul Hamid' }, { name: 'Coach Tom' }]
    }
  ];

  let selectedCategory = $state('ALL');
  let searchQuery = $state('');

  let filtered = $derived(
    projects.filter((p) => {
      const matchCat = selectedCategory === 'ALL' || p.category.toUpperCase() === selectedCategory;
      const matchQ = !searchQuery || p.title.toLowerCase().includes(searchQuery.toLowerCase()) || p.description.toLowerCase().includes(searchQuery.toLowerCase());
      return matchCat && matchQ;
    })
  );
</script>

<PageContainer class="p-6 space-y-6 max-w-7xl mx-auto">
  <PageHeader
    title={'Cards & Catalog Archetype'}
    subtitle={'Entity cards grid featuring badge status, avatar stacks, category filters, and quick action buttons.'}
    accent="indigo"
  >
    {#snippet action()}
      <Button variant="primary" size="sm">+ New Project</Button>
    {/snippet}
  </PageHeader>

  <!-- Filter & Search Bar -->
  <div class="flex flex-col sm:flex-row gap-3 justify-between items-center">
    <div class="flex items-center gap-1.5 overflow-x-auto w-full sm:w-auto pb-1 sm:pb-0">
      {#each ['ALL', 'TERMINAL', 'PRODUCTIVITY', 'DATA', 'FOUNDATION'] as cat}
        <button
          type="button"
          onclick={() => (selectedCategory = cat)}
          class="px-3 py-1.5 rounded-lg text-xs font-semibold transition-all cursor-pointer {selectedCategory === cat ? 'bg-[var(--ca-brand)] text-white shadow-xs' : 'bg-[var(--ca-surface-subtle)] text-[var(--ca-text-muted)] hover:text-[var(--ca-text-primary)]'}"
        >
          {cat}
        </button>
      {/each}
    </div>

    <div class="w-full sm:w-64">
      <SearchInput bind:value={searchQuery} placeholder={'Search projects...'} />
    </div>
  </div>

  <!-- Cards Grid -->
  <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-5">
    {#each filtered as card (card.id)}
      <Card class="p-5 flex flex-col justify-between space-y-4 hover:border-[var(--ca-border-strong)] transition-all group">
        <!-- Top Row -->
        <div class="space-y-3">
          <div class="flex items-start justify-between gap-3">
            <div class="flex items-center gap-2.5">
              <span class="w-3.5 h-3.5 rounded-full shrink-0 shadow-xs" style="background-color: {card.accent}"></span>
              <h3 class="font-bold text-sm text-[var(--ca-text-primary)] group-hover:text-[var(--ca-brand)] transition-colors">
                {card.title}
              </h3>
            </div>
            <Badge
              variant={card.status === 'STABLE' ? 'success' : card.status === 'ACTIVE' ? 'primary' : 'neutral'}
              class="text-[10px] font-mono"
            >
              {card.status}
            </Badge>
          </div>

          <p class="text-xs text-[var(--ca-text-muted)] line-clamp-2 leading-relaxed">
            {card.description}
          </p>

          {#if card.totpCode}
            <div class="pt-1">
              <TotpBadge code={card.totpCode} label="2FA Auth Key" secondsRemaining={24} />
            </div>
          {/if}
        </div>

        <!-- Card Footer -->
        <div class="pt-3 border-t border-[var(--ca-border)] flex items-center justify-between">
          <!-- Member Avatars -->
          <div class="flex -space-x-1.5 overflow-hidden">
            {#each card.members as member}
              <div
                class="w-6 h-6 rounded-full bg-[var(--ca-surface-subtle)] border border-[var(--ca-surface)] flex items-center justify-center text-[10px] font-bold text-[var(--ca-text-secondary)]"
                title={member.name}
              >
                {member.name.slice(0, 1)}
              </div>
            {/each}
          </div>

          <div class="flex items-center gap-2">
            <span class="text-xs font-mono text-[var(--ca-text-muted)]">★ {card.stars}</span>
            <Button variant="ghost" size="sm" class="text-xs">
              Manage →
            </Button>
          </div>
        </div>
      </Card>
    {/each}
  </div>
</PageContainer>
