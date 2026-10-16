<script lang="ts">
  import {
    PageContainer,
    PageHeader,
    Card,
    Badge,
    Button,
    ProgressBar,
    CircularProgress,
    Skeleton
  } from '@cecepazhar/caui';

  const barData = [
    { label: 'SGP-1 Cloudflare Edge', value: 94, max: 100, variant: 'primary' as const },
    { label: 'Jakarta YPC Dokploy Runner', value: 72, max: 100, variant: 'secondary' as const },
    { label: '9Router AI Gateway (Local :20128)', value: 85, max: 100, variant: 'primary' as const },
    { label: 'Hana AI Prompt Studio', value: 61, max: 100, variant: 'success' as const },
    { label: 'Postgres STRICT Vault Node', value: 12, max: 100, variant: 'danger' as const },
    { label: 'OBSIDIAN Fathforce Local Mirror', value: 100, max: 100, variant: 'primary' as const }
  ];

  const donutData = [
    { label: 'Compute', value: 72 },
    { label: 'Memory', value: 45 },
    { label: 'Storage', value: 28 }
  ];
</script>

<PageContainer class="p-6 space-y-6 max-w-7xl mx-auto">
  <PageHeader
    title={'Visual Metrics & Analytics Archetype'}
    subtitle={'High-fidelity data visualization built using CAUI progress bars, ring gauges, and skeleton loaders.'}
    accent="indigo"
  >
    {#snippet action()}
      <Button variant="outline" size="sm">Export Dashboard CSV</Button>
    {/snippet}
  </PageHeader>

  <!-- Section 1: Horizontal Progress Bars -->
  <Card class="p-6 space-y-6">
    <div>
      <h3 class="font-bold text-sm text-[var(--ca-text-primary)]">Node Cluster Throughput</h3>
      <p class="text-xs text-[var(--ca-text-muted)]">Aggregated request throughput per cluster over the last 5 minutes.</p>
    </div>

    <div class="space-y-4">
      {#each barData as bar}
        <div>
          <div class="flex items-center justify-between mb-1.5 text-xs">
            <div class="flex items-center gap-2">
              <span class="font-medium text-[var(--ca-text-secondary)]">{bar.label}</span>
              {#if bar.value === 100}
                <Badge variant="success" class="text-[9px] font-mono">MAX</Badge>
              {:else if bar.value < 20}
                <Badge variant="danger" class="text-[9px] font-mono">LOW BAND</Badge>
              {/if}
            </div>
            <span class="font-mono font-semibold text-[var(--ca-text-primary)]">{bar.value}%</span>
          </div>
          <ProgressBar {value} max={bar.max} {variant} size="md" />
        </div>
      {/each}
    </div>
  </Card>

  <!-- Section 2: Ring Gauges Grid -->
  <Card class="p-6 space-y-6">
    <div>
      <h3 class="font-bold text-sm text-[var(--ca-text-primary)]">Global Ecosystem Health</h3>
      <p class="text-xs text-[var(--ca-text-muted)]">Real-time CPU, RAM, and disk allocation monitored by CAFramework telemetry agent.</p>
    </div>

    <div class="grid grid-cols-1 sm:grid-cols-3 gap-6">
      {#each donutData as d}
        <div class="flex flex-col items-center text-center space-y-3">
          <div class="relative">
            <CircularProgress value={d.value} max={100} size={100} strokeWidth={10} />
            <span class="absolute inset-0 flex items-center justify-center text-lg font-mono font-bold text-[var(--ca-text-primary)]">
              {d.value}%
            </span>
          </div>
          <div>
            <span class="text-xs font-semibold text-[var(--ca-text-secondary)] block">{d.label}</span>
            <span class="text-[10px] text-[var(--ca-text-muted)] font-mono">{d.value}/100 Units</span>
          </div>
        </div>
      {/each}
    </div>
  </Card>

  <!-- Section 3: Loading Skeleton Demo -->
  <Card class="p-6 space-y-6">
    <div class="flex items-center justify-between">
      <div>
        <h3 class="font-bold text-sm text-[var(--ca-text-primary)]">Live Telemetry Stream</h3>
        <p class="text-xs text-[var(--ca-text-muted)]">Placeholder data while the local telemetry engine is booting up.</p>
      </div>
      <Badge variant="outline" class="font-mono text-xs border-dashed animate-pulse">SYNCING...</Badge>
    </div>

    <div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
      <Skeleton className="h-20 rounded-xl" />
      <Skeleton className="h-20 rounded-xl" />
      <Skeleton className="h-20 rounded-xl" />
      <div class="sm:col-span-3 w-full space-y-3">
        <Skeleton className="h-6 w-full" />
        <Skeleton className="h-6 w-4/5" />
      </div>
    </div>
  </Card>
</PageContainer>
