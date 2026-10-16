<script lang="ts">
  import {
    PageContainer,
    PageHeader,
    Card,
    Badge,
    Button,
    Input,
    Select,
    Checkbox,
    Pagination,
    SearchInput,
    EmptyState,
    DropdownMenu
  } from '@cecepazhar/caui';

  interface RecordItem {
    id: string;
    customer: string;
    email: string;
    plan: 'PRO' | 'STARTER' | 'ENTERPRISE';
    amount: string;
    status: 'paid' | 'pending' | 'failed';
    date: string;
  }

  const allRecords: RecordItem[] = [
    { id: 'TX-1001', customer: 'Cecep Saeful Azhar', email: 'cecep@fathforce.com', plan: 'ENTERPRISE', amount: 'Rp 2.450.000', status: 'paid', date: '2026-10-15' },
    { id: 'TX-1002', customer: 'Coach Tom', email: 'finance@rollerskool.com', plan: 'PRO', amount: 'Rp 149.000', status: 'paid', date: '2026-10-14' },
    { id: 'TX-1003', customer: 'RBS Automations', email: 'rbs@fathforce.com', plan: 'PRO', amount: 'Rp 149.000', status: 'paid', date: '2026-10-14' },
    { id: 'TX-1004', customer: 'Gerlink Medical', email: 'dev@gerlink.id', plan: 'ENTERPRISE', amount: 'Rp 5.200.000', status: 'paid', date: '2026-10-12' },
    { id: 'TX-1005', customer: 'Fathforce Studio', email: 'team@fathforce.com', plan: 'STARTER', amount: 'Rp 79.000', status: 'pending', date: '2026-10-11' },
    { id: 'TX-1006', customer: 'Abdul Hamid III', email: 'hamid@fathforce.com', plan: 'PRO', amount: 'Rp 149.000', status: 'paid', date: '2026-10-10' },
    { id: 'TX-1007', customer: 'Sovereign Lab Node', email: 'ops@cecepazhar.com', plan: 'PRO', amount: 'Rp 149.000', status: 'failed', date: '2026-10-09' },
    { id: 'TX-1008', customer: 'PT Digital Solusi', email: 'billing@digitalsolusi.co.id', plan: 'ENTERPRISE', amount: 'Rp 3.800.000', status: 'paid', date: '2026-10-08' }
  ];

  let searchQuery = $state('');
  let filterPlan = $state('all');
  let filterStatus = $state('all');
  let selectedIds = $state<string[]>([]);
  let currentPage = $state(1);
  const pageSize = 5;

  let filtered = $derived(
    allRecords.filter((r) => {
      const q = searchQuery.toLowerCase().trim();
      const matchQuery = !q || r.customer.toLowerCase().includes(q) || r.email.toLowerCase().includes(q) || r.id.toLowerCase().includes(q);
      const matchPlan = filterPlan === 'all' || r.plan === filterPlan;
      const matchStatus = filterStatus === 'all' || r.status === filterStatus;
      return matchQuery && matchPlan && matchStatus;
    })
  );

  let paginated = $derived(
    filtered.slice((currentPage - 1) * pageSize, currentPage * pageSize)
  );

  let isAllSelected = $derived(
    paginated.length > 0 && paginated.every((r) => selectedIds.includes(r.id))
  );

  function toggleSelectAll() {
    if (isAllSelected) {
      selectedIds = selectedIds.filter((id) => !paginated.some((r) => r.id === id));
    } else {
      const toAdd = paginated.map((r) => r.id).filter((id) => !selectedIds.includes(id));
      selectedIds = [...selectedIds, ...toAdd];
    }
  }

  function toggleSelect(id: string) {
    if (selectedIds.includes(id)) {
      selectedIds = selectedIds.filter((x) => x !== id);
    } else {
      selectedIds = [...selectedIds, id];
    }
  }
</script>

<PageContainer class="p-6 space-y-6 max-w-7xl mx-auto">
  <PageHeader
    title={'Data Table Archetype'}
    subtitle={'Enterprise data grid pattern featuring fuzzy search, column filters, bulk row selection, and pagination.'}
    accent="indigo"
  >
    {#snippet action()}
      <div class="flex items-center gap-2">
        <Button variant="outline" size="sm">Export CSV</Button>
        <Button variant="primary" size="sm">+ New Transaction</Button>
      </div>
    {/snippet}
  </PageHeader>

  <Card class="p-5 space-y-4">
    <!-- Controls Bar -->
    <div class="flex flex-col sm:flex-row gap-3 justify-between items-stretch sm:items-center">
      <div class="flex flex-1 items-center gap-2 max-w-md">
        <SearchInput
          placeholder={'Search by customer, email or transaction ID...'}
          bind:value={searchQuery}
          class="w-full"
        />
      </div>

      <div class="flex items-center gap-2 shrink-0">
        <select
          bind:value={filterPlan}
          class="px-3 py-1.5 rounded-lg border border-[var(--ca-border)] bg-[var(--ca-surface-subtle)] text-xs text-[var(--ca-text-primary)] focus:outline-none"
        >
          <option value="all">All Plans</option>
          <option value="ENTERPRISE">Enterprise</option>
          <option value="PRO">Pro Lifetime</option>
          <option value="STARTER">Starter</option>
        </select>

        <select
          bind:value={filterStatus}
          class="px-3 py-1.5 rounded-lg border border-[var(--ca-border)] bg-[var(--ca-surface-subtle)] text-xs text-[var(--ca-text-primary)] focus:outline-none"
        >
          <option value="all">All Status</option>
          <option value="paid">Paid</option>
          <option value="pending">Pending</option>
          <option value="failed">Failed</option>
        </select>

        {#if selectedIds.length > 0}
          <Badge variant="primary" class="font-mono text-xs">
            {selectedIds.length} Selected
          </Badge>
        {/if}
      </div>
    </div>

    <!-- Table -->
    {#if paginated.length === 0}
      <EmptyState
        title={'No matching transactions'}
        description="Try adjusting your filter or search query."
        icon="search"
      />
    {:else}
      <div class="overflow-x-auto border border-[var(--ca-border)] rounded-xl">
        <table class="w-full text-left text-xs border-collapse">
          <thead class="bg-[var(--ca-surface-subtle)] text-[var(--ca-text-muted)] uppercase text-[10px] tracking-wider border-b border-[var(--ca-border)]">
            <tr>
              <th class="py-3 px-3 w-8">
                <input
                  type="checkbox"
                  checked={isAllSelected}
                  onchange={toggleSelectAll}
                  class="rounded border-[var(--ca-border)] bg-[var(--ca-surface)] text-[var(--ca-brand)]"
                  aria-label={'Select all rows'}
                />
              </th>
              <th class="py-3 px-3 font-semibold">Transaction ID</th>
              <th class="py-3 px-3 font-semibold">Customer</th>
              <th class="py-3 px-3 font-semibold">Plan</th>
              <th class="py-3 px-3 font-semibold">Amount</th>
              <th class="py-3 px-3 font-semibold">Status</th>
              <th class="py-3 px-3 font-semibold">Date</th>
              <th class="py-3 px-3 font-semibold text-right">Actions</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-[var(--ca-border)]">
            {#each paginated as item}
              {@const isSelected = selectedIds.includes(item.id)}
              <tr class="hover:bg-[var(--ca-surface-subtle)] transition-colors {isSelected ? 'bg-[var(--ca-brand)]/5' : ''}">
                <td class="py-3 px-3">
                  <input
                    type="checkbox"
                    checked={isSelected}
                    onchange={() => toggleSelect(item.id)}
                    class="rounded border-[var(--ca-border)] bg-[var(--ca-surface)] text-[var(--ca-brand)]"
                    aria-label={`Select row ${item.id}`}
                  />
                </td>
                <td class="py-3 px-3 font-mono font-semibold text-[var(--ca-brand)]">{item.id}</td>
                <td class="py-3 px-3">
                  <span class="block font-medium text-[var(--ca-text-primary)]">{item.customer}</span>
                  <span class="block text-[10px] text-[var(--ca-text-muted)] font-mono">{item.email}</span>
                </td>
                <td class="py-3 px-3 font-mono text-[10px]">
                  <Badge variant="outline">{item.plan}</Badge>
                </td>
                <td class="py-3 px-3 font-mono font-semibold text-[var(--ca-text-primary)]">{item.amount}</td>
                <td class="py-3 px-3">
                  <Badge
                    variant={item.status === 'paid' ? 'success' : item.status === 'pending' ? 'warning' : 'danger'}
                    class="capitalize text-[10px]"
                  >
                    {item.status}
                  </Badge>
                </td>
                <td class="py-3 px-3 font-mono text-[var(--ca-text-muted)]">{item.date}</td>
                <td class="py-3 px-3 text-right">
                  <Button variant="ghost" size="sm" class="text-xs">
                    Details
                  </Button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>

      <!-- Pagination -->
      <div class="flex items-center justify-between pt-2">
        <p class="text-xs text-[var(--ca-text-muted)]">
          Showing {paginated.length} of {filtered.length} entries
        </p>
        <Pagination
          total={filtered.length}
          perPage={pageSize}
          page={currentPage}
          onchange={(p) => (currentPage = p)}
        />
      </div>
    {/if}
  </Card>
</PageContainer>
