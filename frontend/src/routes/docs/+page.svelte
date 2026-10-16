<script lang="ts">
  import {
    PageContainer,
    PageHeader,
    Card,
    Badge,
    Button,
    Tabs,
    Accordion,
    Breadcrumb
  } from '@cecepazhar/caui';

  let activeTab = $state('getting-started');

  const docsTabs = [
    { id: 'getting-started', label: 'Getting Started' },
    { id: 'architecture', label: 'Architecture' },
    { id: 'faq', label: 'FAQ' }
  ];

  const installCommands = `# 1. Install prerequisites (Rust & Node.js)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
npm install -g pnpm

# 2. Clone the framework
git clone https://github.com/cecepazhar/caframework.git
cd caframework

# 3. Generate a new application slice
cargo run --package caf-xtask -- new-app --name "MYAPP"

# 4. Run development environment (Tauri + Vite)
npm run tauri dev`;

  const faqItems = [
    {
      title: 'How do I add a new menu item?',
      content: 'Add a [[menus]] block to your `app.toml`, then run `cargo run --package caf-xtask -- codegen`. This auto-generates both the TypeScript nav items and Rust IPC command bindings.'
    },
    {
      title: 'Where are the UI components?',
      content: 'All shared design components are imported from `@cecepazhar/caui`. You can also override or extend them in `frontend/src/lib/components`.'
    },
    {
      title: 'Does the app work on Android?',
      content: 'Yes. The Tauri v2 mobile setup is configured in `caf-app/gen/android`, producing both aarch64 and armv7 universal APK builds.'
    }
  ];
</script>

<PageContainer class="p-6 space-y-6 max-w-4xl mx-auto">
  <PageHeader
    title={'Developer Documentation'}
    subtitle={'API reference, installation guides, and architectural principles of CAFramework.'}
    accent="indigo"
  />

  <Breadcrumb items={['Home', 'Docs']} />

  <!-- Tabs Navigation -->
  <Card class="p-0 overflow-hidden">
    <div class="p-5 space-y-8">
      <Tabs {tabs} bind:activeTab />

      <!-- Tab Content -->
      {#if activeTab === 'getting-started'}
        <div class="space-y-4">
          <h3 class="text-sm font-bold text-[var(--ca-text-primary)]">Installation & Initialization</h3>
          <p class="text-xs text-[var(--ca-text-muted)] leading-relaxed">
            To set up the CAFramework environment locally, you will need the Rust toolchain (with Tauri v2 targets) and a modern Node.js runtime (v20+) with pnpm.
          </p>

          <div class="rounded-xl border border-[var(--ca-border)] bg-[var(--ca-surface-subtle)] p-4 font-mono text-xs space-y-2 overflow-x-auto">
            <div class="flex items-center gap-2 mb-2">
              <span class="w-2 h-2 rounded-full bg-rose-500"></span>
              <span class="w-2 h-2 rounded-full bg-amber-500"></span>
              <span class="w-2 h-2 rounded-full bg-emerald-500"></span>
              <span class="ml-2 text-[10px] text-[var(--ca-text-muted)] uppercase tracking-wider font-bold">Terminal</span>
            </div>
            <pre class="whitespace-pre-wrap text-[var(--ca-text-secondary)] leading-relaxed">{installCommands}</pre>
            <Button variant="ghost" size="sm" class="text-xs">Copy to clipboard</Button>
          </div>
        </div>
      {:else if activeTab === 'architecture'}
        <div class="space-y-4">
          <h3 class="text-sm font-bold text-[var(--ca-text-primary)]">System Topology</h3>
          <p class="text-xs text-[var(--ca-text-muted)] leading-relaxed">
            CAFramework is architected around the "Vertical Slice" pattern. Each module contains a Rust backend logic unit, a Tauri IPC command, and a Svelte route that is strictly decoupled from one another.
          </p>
          <div class="flex items-center gap-2">
            <Badge variant="neutral">Rust Tauri Backend</Badge>
            <span class="text-[var(--ca-text-muted)]">IPC →</span>
            <Badge variant="neutral">CAUI Design System</Badge>
            <span class="text-[var(--ca-text-muted)]">UI →</span>
            <Badge variant="primary">Encrypted Local DB</Badge>
          </div>
        </div>
      {:else}
        <div class="space-y-4">
          <h3 class="text-sm font-bold text-[var(--ca-text-primary)]">Frequently Asked Questions</h3>
          <Accordion items={faqItems} />
        </div>
      {/if}
    </div>
  </Card>
</PageContainer>
