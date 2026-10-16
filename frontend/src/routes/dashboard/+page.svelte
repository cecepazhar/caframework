<script lang="ts">
  import {
    PageContainer,
    PageHeader,
    Card,
    Badge,
    Button,
    TelemetryCard,
    ProgressBar,
    CircularProgress,
    Table,
    Icon,
    EmptyState
  } from '@cecepazhar/caui';

  // Sample KPI metrics
  const kpis = [
    { title: 'Total Deployments', value: '1,429', change: '+12.4%', isUp: true, icon: 'server' },
    { title: 'Active Daemons', value: '38', change: '+3 new', isUp: true, icon: 'cpu' },
    { title: 'Vault Envelopes', value: '256', change: 'E2EE Synced', isUp: true, icon: 'shield' },
    { title: 'System Health', value: '99.98%', change: 'Normal', isUp: true, icon: 'activity' }
  ];

  // Sample recent activity table
  let tableData = $state([
    { id: 'DEP-901', name: 'Prod API Gateway', status: 'Healthy', latency: '24ms', region: 'sgp-1', uptime: '99.9%' },
    { id: 'DEP-902', name: 'YPC S3 Mirror Fleet', status: 'Healthy', latency: '42ms', region: 'jkt-1', uptime: '100%' },
    { id: 'DEP-903', name: 'Dokploy Runner Service', status: 'Syncing', latency: '110ms', region: 'sgp-1', uptime: '98.5%' },
    { id: 'DEP-904', name: 'Hana AI Prompt Studio', status: 'Healthy', latency: '35ms', region: 'local', uptime: '99.9%' },
    { id: 'DEP-905', name: 'Postgres STRICT Node', status: 'Healthy', latency: '12ms', region: 'jkt-1', uptime: '100%' }
  ]);

  let systemLoad = $state(68);
  let memoryUsage = $state(45);
</script>

<PageContainer class="p-6 space-y-6 max-w-7xl mx-auto">
  <!-- Page Header -->
  <PageHeader
    title={'Executive Dashboard'}
    subtitle={'Ecosystem overview, operational telemetries, and node fleet status.'}
    accent="indigo"
  >
    {#snippet action()}
      <div class="flex items-center gap-2">
        <Button variant="outline" size="sm" onclick={() => (systemLoad = Math.floor(Math.random() * 30) + 50)}>
          Refresh Fleet
        </Button>
        <Button variant="primary" size="sm">
          Deploy Slice
        </Button>
      </div>
    {/snippet}
  </PageHeader>

  <!-- KPI Metrics Grid -->
  <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
    {#each kpis as kpi}
      <Card class="p-4 flex items-center justify-between shadow-2xs hover:border-[var(--ca-brand)] transition-colors">
        <div class="space-y-1">
          <p class="text-xs text-[var(--ca-text-muted)] font-medium">{kpi.title}</p>
          <p class="text-2xl font-bold font-mono tracking-tight text-[var(--ca-text-primary)]">{kpi.value}</p>
          <div class="flex items-center gap-1.5 pt-0.5">
            <span class="w-1.5 h-1.5 rounded-full bg-emerald-500"></span>
            <span class="text-[11px] font-medium text-emerald-500">{kpi.change}</span>
          </div>
        </div>
        <div class="w-10 h-10 rounded-xl bg-[var(--ca-brand)]/10 text-[var(--ca-brand)] flex items-center justify-center font-bold">
          ★
        </div>
      </Card>
    {/each}
  </div>

  <!-- Telemetry Row -->
  <div class="grid grid-cols-1 lg:grid-cols-3 gap-4">
    <!-- Telemetry Card 1 -->
    <Card class="p-5 space-y-4">
      <div class="flex items-center justify-between">
        <h4 class="font-bold text-xs uppercase tracking-wider text-[var(--ca-text-secondary)]">Resource Allocation</h4>
        <Badge variant="outline" class="font-mono text-[10px]">6.19.10-300.fc44</Badge>
      </div>

      <div class="space-y-3">
        <div>
          <div class="flex justify-between text-xs mb-1">
            <span class="text-[var(--ca-text-muted)]">CPU Core Load</span>
            <span class="font-mono font-semibold">{systemLoad}%</span>
          </div>
          <ProgressBar value={systemLoad} max={100} variant="primary" size="sm" />
        </div>

        <div>
          <div class="flex justify-between text-xs mb-1">
            <span class="text-[var(--ca-text-muted)]">RAM Utilization</span>
            <span class="font-mono font-semibold">{memoryUsage}%</span>
          </div>
          <ProgressBar value={memoryUsage} max={100} variant="secondary" size="sm" />
        </div>
      </div>
    </Card>

    <!-- Telemetry Card 2: Health Circular -->
    <Card class="p-5 flex items-center justify-between">
      <div class="space-y-1">
        <h4 class="font-bold text-xs uppercase tracking-wider text-[var(--ca-text-secondary)]">Service Availability</h4>
        <p class="text-xs text-[var(--ca-text-muted)]">Calculated over last 30 days</p>
        <div class="pt-2">
          <Badge variant="success" class="text-xs font-mono">ALL SYSTEMS NOMINAL</Badge>
        </div>
      </div>
      <div class="flex flex-col items-center">
        <CircularProgress value={99.9} max={100} size={70} strokeWidth={6} />
      </div>
    </Card>

    <!-- Quick Action Card -->
    <Card class="p-5 flex flex-col justify-between space-y-3">
      <div>
        <h4 class="font-bold text-xs uppercase tracking-wider text-[var(--ca-text-secondary)]">Sovereign Cloud Gate</h4>
        <p class="text-xs text-[var(--ca-text-muted)] mt-1">
          Zero-Knowledge envelope sync and Dokploy automation cluster.
        </p>
      </div>
      <div class="flex items-center gap-2">
        <Button variant="secondary" size="sm" class="flex-1">Audit Logs</Button>
        <Button variant="primary" size="sm" class="flex-1">Sync Peers</Button>
      </div>
    </Card>
  </div>

  <!-- Recent Nodes Table -->
  <Card class="p-5 space-y-4">
    <div class="flex items-center justify-between">
      <div>
        <h3 class="font-bold text-sm text-[var(--ca-text-primary)]">Connected Services & Nodes</h3>
        <p class="text-xs text-[var(--ca-text-muted)]">Active daemons running within the CAFramework mesh.</p>
      </div>
      <Badge variant="outline" class="font-mono text-xs">{tableData.length} Active</Badge>
    </div>

    <div class="overflow-x-auto">
      <table class="w-full text-left text-xs border-collapse">
        <thead>
          <tr class="border-b border-[var(--ca-border)] text-[var(--ca-text-muted)] uppercase tracking-wider text-[10px]">
            <th class="py-2.5 px-3 font-semibold">Service ID</th>
            <th class="py-2.5 px-3 font-semibold">Daemon Name</th>
            <th class="py-2.5 px-3 font-semibold">Status</th>
            <th class="py-2.5 px-3 font-semibold">Latency</th>
            <th class="py-2.5 px-3 font-semibold">Region</th>
            <th class="py-2.5 px-3 font-semibold text-right">Uptime</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-[var(--ca-border)]">
          {#each tableData as row}
            <tr class="hover:bg-[var(--ca-surface-subtle)] transition-colors">
              <td class="py-3 px-3 font-mono text-[var(--ca-brand)] font-semibold">{row.id}</td>
              <td class="py-3 px-3 font-medium text-[var(--ca-text-primary)]">{row.name}</td>
              <td class="py-3 px-3">
                <Badge variant={row.status === 'Healthy' ? 'success' : 'warning'} class="text-[10px]">
                  {row.status}
                </Badge>
              </td>
              <td class="py-3 px-3 font-mono text-[var(--ca-text-muted)]">{row.latency}</td>
              <td class="py-3 px-3 font-mono uppercase text-[var(--ca-text-secondary)]">{row.region}</td>
              <td class="py-3 px-3 font-mono text-right text-emerald-500 font-semibold">{row.uptime}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  </Card>
</PageContainer>
