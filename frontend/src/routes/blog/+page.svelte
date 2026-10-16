<script lang="ts">
  import {
    PageContainer,
    PageHeader,
    Card,
    Badge,
    Button,
    Avatar,
    SearchInput,
    Breadcrumb,
    Kbd
  } from '@cecepazhar/caui';

  interface BlogPost {
    id: string;
    slug: string;
    title: string;
    excerpt: string;
    author: string;
    authorInitial: string;
    date: string;
    category: string;
    readTime: string;
    isFeatured?: boolean;
  }

  const posts: BlogPost[] = [
    {
      id: '1',
      slug: 'zero-knowledge-vault-guide',
      title: 'Deep Dive: Building a Zero-Knowledge Encrypted Vault',
      excerpt: 'How CAFramework secures your credentials locally using Argon2id key stretching and AES-256-GCM authenticated encryption without relying on a centralized key management server.',
      author: 'Cecep Saeful Azhar',
      authorInitial: 'C',
      date: '2026-10-15',
      category: 'Security',
      readTime: '6 min read',
      isFeatured: true
    },
    {
      id: '2',
      slug: 'svelte-5-runes-guide',
      title: 'Svelte 5 Runes: State Management, $state & $derived',
      excerpt: 'Moving away from legacy Svelte 3/4 stores into the reactivity primitives of Svelte 5. A practical comparison on building desktop-first application states.',
      author: 'Cecep Saeful Azhar',
      authorInitial: 'C',
      date: '2026-10-12',
      category: 'Architecture',
      readTime: '4 min read'
    },
    {
      id: '3',
      slug: 'rust-tauri-v2-security',
      title: 'Sovereign Desktop Apps with Rust & Tauri v2',
      excerpt: 'Why we moved our entire CA family stack from Electron to Tauri v2, achieving 15% faster binary payloads and seamless Linux/Android multi-platform support.',
      author: 'Fathforce Studio',
      authorInitial: 'F',
      date: '2026-10-08',
      category: 'Development',
      readTime: '5 min read'
    },
    {
      id: '4',
      slug: 'design-tokens-unified-system',
      title: 'CAUI: Building a Single Source of Truth for 10+ Apps',
      excerpt: 'How we consolidated 163 component duplications across 6 desktop applications into a single 80+ component design system with CSS variables and Tailwind tokens.',
      author: 'Cecep Saeful Azhar',
      authorInitial: 'C',
      date: '2026-10-02',
      category: 'Design System',
      readTime: '7 min read'
    }
  ];

  let searchQuery = $state('');
  let selectedCat = $state('All');

  let filtered = $derived(
    posts.filter((p) => {
      const matchCat = selectedCat === 'All' || p.category === selectedCat;
      const matchQ = !searchQuery || p.title.toLowerCase().includes(searchQuery.toLowerCase());
      return matchCat && matchQ;
    })
  );

  const categories = ['All', ...Array.from(new Set(posts.map((p) => p.category)))];
</script>

<PageContainer class="p-6 space-y-6 max-w-5xl mx-auto">
  <PageHeader
    title={'CA Framework Blog & Updates'}
    subtitle={'Engineering deep dives, architectural patterns, and release announcements.'}
    accent="indigo"
  />

  <Breadcrumb items={['Home', 'Blog']} />

  <!-- Search & Filters -->
  <div class="flex flex-col sm:flex-row gap-3 justify-between items-stretch sm:items-center">
    <div class="flex-1 max-w-md">
      <SearchInput bind:value={searchQuery} placeholder={'Search articles by keyword...'} />
    </div>
    <div class="flex flex-wrap gap-2">
      {#each categories as cat}
        <button
          type="button"
          onclick={() => (selectedCat = cat)}
          class="px-3 py-1.5 rounded-lg text-xs font-semibold transition-all cursor-pointer {selectedCat === cat ? 'bg-[var(--ca-brand)] text-white shadow-xs' : 'bg-[var(--ca-surface-subtle)] text-[var(--ca-text-muted)] hover:text-[var(--ca-text-primary)]'}"
        >
          {cat}
        </button>
      {/each}
    </div>
  </div>

  <!-- Featured Post Hero Card -->
  {#if filtered.find((p) => p.isFeatured) && selectedCat === 'All' && !searchQuery}
    {@const featured = filtered.find((p) => p.isFeatured)}
    <Card variant="primary" class="p-6 md:p-8 space-y-4 border-[var(--ca-brand)] shadow-xl relative overflow-hidden">
      <div class="absolute top-0 right-0 p-4">
        <Badge variant="primary" class="text-[10px] font-bold">FEATURED</Badge>
      </div>
      <h3 class="text-2xl font-black tracking-tight text-[var(--ca-text-primary)] pr-16 leading-snug">
        {featured.title}
      </h3>
      <p class="text-xs text-[var(--ca-text-muted)] leading-relaxed max-w-2xl">
        {featured.excerpt}
      </p>
      <div class="flex items-center justify-between pt-2">
        <div class="flex items-center gap-2.5">
          <div class="w-8 h-8 rounded-full bg-[var(--ca-brand)]/15 text-[var(--ca-brand)] flex items-center justify-center font-bold text-xs">
            {featured.authorInitial}
          </div>
          <div>
            <span class="text-xs font-semibold text-[var(--ca-text-secondary)] block">{featured.author}</span>
            <span class="text-[10px] text-[var(--ca-text-muted)] font-mono">{featured.date} · {featured.readTime}</span>
          </div>
        </div>
        <Button variant="primary" size="sm">Read Article →</Button>
      </div>
    </Card>
  {/if}

  <!-- Grid of Posts -->
  <div class="grid grid-cols-1 md:grid-cols-2 gap-5">
    {#each filtered as post}
      {#if post.isFeatured && selectedCat === 'All' && !searchQuery}
      {:else}
        <Card class="p-5 flex flex-col justify-between space-y-4 hover:translate-y-0.5 hover:shadow-md transition-all">
          <div class="space-y-3">
            <Badge variant="outline" class="text-[10px] font-mono uppercase">{post.category}</Badge>
            <h4 class="text-sm font-bold text-[var(--ca-text-primary)] leading-snug line-clamp-2">
              {post.title}
            </h4>
            <p class="text-xs text-[var(--ca-text-muted)] leading-relaxed line-clamp-2">
              {post.excerpt}
            </p>
          </div>

          <div class="flex items-center justify-between pt-3 border-t border-[var(--ca-border)]">
            <div class="text-[10px] text-[var(--ca-text-muted)] font-mono space-y-0.5">
              <span class="block">{post.date}</span>
              <span class="block">{post.readTime}</span>
            </div>
            <Button variant="ghost" size="sm" class="text-xs">
              View Details
            </Button>
          </div>
        </Card>
      {/if}
    {/each}
  </div>
</PageContainer>
