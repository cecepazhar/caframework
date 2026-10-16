<script lang="ts">
  import {
    PageContainer,
    Button,
    Badge,
    Card,
    Accordion,
    SponsorWall
  } from '@cecepazhar/caui';

  const sponsors = [
    { name: 'Coach Tom', tier: 'FOUNDER', contribution: 'Gold Patron', date: '2026' },
    { name: 'Fathforce Studio', tier: 'CORE', contribution: 'Ecosystem Partner', date: '2026' },
    { name: 'Gerlink Medical', tier: 'ENTERPRISE', contribution: 'Enterprise Sponsor', date: '2026' },
    { name: 'Abdul Hamid III', tier: 'FOUNDER', contribution: 'Patron', date: '2026' }
  ];

  const faqs = [
    {
      title: 'What is CAFramework?',
      content: 'CAFramework is a privacy-first modular application starter framework built with Rust (Tauri v2) and Svelte 5 runes. It enables building sovereign desktop & mobile apps with unified zero-knowledge encrypted vaults.'
    },
    {
      title: 'How does Zero-Knowledge Encryption work?',
      content: 'All sensitive credentials, API keys, and notes are encrypted on-device using AES-256-GCM and Argon2id key derivation. Your master password never leaves your hardware.'
    },
    {
      title: 'Can I publish commercial applications with it?',
      content: 'Yes. CAFramework is designed as a foundational template. You can generate custom branded applications using `cargo xtask new-app`.'
    },
    {
      title: 'What platforms are supported?',
      content: 'Linux (Fedora/Debian/Arch), macOS, Windows, and Android (aarch64 / armv7).'
    }
  ];
</script>

<div class="min-h-full bg-[var(--ca-surface-base)] text-[var(--ca-text-primary)]">
  <!-- Hero Section -->
  <section class="py-20 px-6 text-center max-w-4xl mx-auto space-y-6">
    <div class="inline-flex items-center gap-2 px-3 py-1 rounded-full bg-[var(--ca-brand)]/10 text-[var(--ca-brand)] border border-[var(--ca-brand)]/30 text-xs font-semibold">
      <span>CA Sovereign Ecosystem v1.3.0</span>
      <span class="w-1.5 h-1.5 rounded-full bg-[var(--ca-brand)]"></span>
      <span>100% Svelte 5 Native</span>
    </div>

    <h1 class="text-4xl sm:text-6xl font-black tracking-tight leading-tight">
      Sovereign Software for <span class="text-transparent bg-clip-text bg-gradient-to-r from-sky-400 to-indigo-500">Autonomous Thinkers</span>
    </h1>

    <p class="text-sm sm:text-base text-[var(--ca-text-muted)] max-w-2xl mx-auto leading-relaxed">
      Local-first, zero-knowledge encrypted applications powered by Rust Tauri v2 and the unified CAUI design system. Fast, resilient, and yours forever.
    </p>

    <div class="flex flex-wrap items-center justify-center gap-3 pt-4">
      <Button variant="primary" size="lg" class="shadow-lg shadow-indigo-500/20">
        Download Latest Release (.apk / desktop)
      </Button>
      <Button variant="outline" size="lg" href="/docs">
        Read Architecture Docs →
      </Button>
    </div>
  </section>

  <!-- Features Grid -->
  <section class="py-12 px-6 max-w-6xl mx-auto space-y-10">
    <div class="text-center space-y-2">
      <h2 class="text-2xl font-bold tracking-tight">Engineered for Sovereign Privacy</h2>
      <p class="text-xs text-[var(--ca-text-muted)]">No telemetry tracking, no mandatory cloud dependencies, no rent-seeking.</p>
    </div>

    <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
      <Card class="p-6 space-y-3">
        <div class="w-10 h-10 rounded-xl bg-sky-500/10 text-sky-500 flex items-center justify-center font-bold text-lg">
          🛡️
        </div>
        <h3 class="font-bold text-sm text-[var(--ca-text-primary)]">Zero-Knowledge Vault</h3>
        <p class="text-xs text-[var(--ca-text-muted)] leading-relaxed">
          AES-256-GCM authenticated encryption paired with Argon2id password hashing protects all your secrets and sessions locally.
        </p>
      </Card>

      <Card class="p-6 space-y-3">
        <div class="w-10 h-10 rounded-xl bg-indigo-500/10 text-indigo-500 flex items-center justify-center font-bold text-lg">
          ⚡
        </div>
        <h3 class="font-bold text-sm text-[var(--ca-text-primary)]">Svelte 5 Runes & Rust</h3>
        <p class="text-xs text-[var(--ca-text-muted)] leading-relaxed">
          Near-instant startup times and minimal memory footprint. Run on lightweight VPS nodes, laptops, or Android tablets without lag.
        </p>
      </Card>

      <Card class="p-6 space-y-3">
        <div class="w-10 h-10 rounded-xl bg-emerald-500/10 text-emerald-500 flex items-center justify-center font-bold text-lg">
          🎨
        </div>
        <h3 class="font-bold text-sm text-[var(--ca-text-primary)]">Unified CAUI System</h3>
        <p class="text-xs text-[var(--ca-text-muted)] leading-relaxed">
          Over 80+ meticulously designed components with responsive frameless title bars, ambient glows, and dark mode standard.
        </p>
      </Card>
    </div>
  </section>

  <!-- Pricing Section -->
  <section class="py-16 px-6 max-w-4xl mx-auto space-y-8">
    <div class="text-center space-y-2">
      <h2 class="text-2xl font-bold tracking-tight">Fair, Transparent Pricing</h2>
      <p class="text-xs text-[var(--ca-text-muted)]">One lifetime purchase. Zero monthly subscriptions.</p>
    </div>

    <div class="grid grid-cols-1 sm:grid-cols-2 gap-6">
      <Card class="p-6 space-y-4">
        <div>
          <Badge variant="neutral">COMMUNITY EDITION</Badge>
          <div class="mt-2 text-2xl font-bold">Free Forever</div>
          <p class="text-xs text-[var(--ca-text-muted)] mt-1">Full core features for solo operators.</p>
        </div>
        <ul class="text-xs space-y-2 text-[var(--ca-text-secondary)]">
          <li>✓ Complete local vault & encryption</li>
          <li>✓ All 5 Archetype screens</li>
          <li>✓ Svelte 5 + Rust Tauri source code</li>
          <li>✓ Local Hana AI assistant support</li>
        </ul>
        <Button variant="outline" class="w-full">Get Started</Button>
      </Card>

      <Card class="p-6 space-y-4 border-[var(--ca-brand)] shadow-lg shadow-indigo-500/10">
        <div>
          <Badge variant="primary">FOUNDER LIFETIME</Badge>
          <div class="mt-2 text-2xl font-bold">Rp 149.000 <span class="text-xs text-emerald-500 font-normal">Promo Rp 79.000</span></div>
          <p class="text-xs text-[var(--ca-text-muted)] mt-1">Permanent license with team sync & priority upgrades.</p>
        </div>
        <ul class="text-xs space-y-2 text-[var(--ca-text-secondary)]">
          <li>✓ Everything in Community</li>
          <li>✓ 7 Team collaboration seats</li>
          <li>✓ E2EE envelope syncing via Pro Gate</li>
          <li>✓ GPU Ambient Underglow & RGB Spectrum</li>
          <li>✓ Direct VIP founder support</li>
        </ul>
        <Button variant="primary" class="w-full">Purchase Lifetime Key</Button>
      </Card>
    </div>
  </section>

  <!-- FAQ Accordion -->
  <section class="py-12 px-6 max-w-3xl mx-auto space-y-6">
    <div class="text-center space-y-1">
      <h2 class="text-xl font-bold">Frequently Asked Questions</h2>
      <p class="text-xs text-[var(--ca-text-muted)]">Everything you need to know about the architecture and licensing.</p>
    </div>

    <Accordion items={faqs} />
  </section>
</div>
