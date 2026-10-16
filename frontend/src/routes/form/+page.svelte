<script lang="ts">
  import {
    PageContainer,
    PageHeader,
    Card,
    Button,
    Input,
    NumberInput,
    Textarea,
    Switch,
    Checkbox,
    RadioGroup,
    Slider,
    RangeSlider,
    PinInput,
    ColorPicker,
    Stepper,
    FileUpload,
    Badge,
    Alert
  } from '@cecepazhar/caui';

  let currentStep = $state(0);
  const steps = [
    { label: 'Basic Info', description: 'Application naming & slug' },
    { label: 'Security & Auth', description: 'PIN, Master Key & RBAC' },
    { label: 'Customization', description: 'Branding accent & lighting' },
    { label: 'Review', description: 'Confirm deployment slice' }
  ];

  // Step 1 state
  let appName = $state('My Custom Node');
  let appSlug = $state('my-custom-node');
  let appCategory = $state('devops');
  let description = $state('High-performance automated node running with CAFramework runtime.');

  // Step 2 state
  let pinCode = $state('');
  let e2eeEnabled = $state(true);
  let rbacRole = $state('owner');
  let rateLimit = $state(120);

  // Step 3 state
  let accentColor = $state('#6366f1');
  let ambientUnderglow = $state(true);
  let memoryThreshold = $state([20, 80]);

  // Submission state
  let isSubmitting = $state(false);
  let submitSuccess = $state(false);

  function handleNext() {
    if (currentStep < steps.length - 1) {
      currentStep += 1;
    } else {
      isSubmitting = true;
      setTimeout(() => {
        isSubmitting = false;
        submitSuccess = true;
      }, 1000);
    }
  }

  function handleBack() {
    if (currentStep > 0) currentStep -= 1;
  }
</script>

<PageContainer class="p-6 space-y-6 max-w-4xl mx-auto">
  <PageHeader
    title={'Form Archetype'}
    subtitle={'Comprehensive form system featuring multi-step Stepper, PIN input, ColorPicker, Sliders, and validation.'}
    accent="indigo"
  />

  {#if submitSuccess}
    <Alert variant="success" title={'Slice Configured Successfully!'}>
      Configuration has been verified and saved to the local encrypted SQLite datastore.
    </Alert>
  {/if}

  <!-- Stepper Indicator -->
  <Card class="p-5">
    <Stepper {steps} activeIndex={currentStep} onstepclick={(idx) => (currentStep = idx)} />
  </Card>

  <!-- Step Content Card -->
  <Card class="p-6 space-y-6">
    {#if currentStep === 0}
      <div class="space-y-4">
        <h3 class="text-sm font-bold text-[var(--ca-text-primary)]">Step 1: Application Identity</h3>
        <p class="text-xs text-[var(--ca-text-muted)]">Define unique manifest parameters for this vertical slice.</p>

        <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
          <div>
            <label for="step1-app-name" class="block text-xs font-semibold text-[var(--ca-text-secondary)] mb-1">Application Name</label>
            <Input id="step1-app-name" bind:value={appName} placeholder={'e.g. CATerminal'} />
          </div>

          <div>
            <label for="step1-app-slug" class="block text-xs font-semibold text-[var(--ca-text-secondary)] mb-1">Bundle Slug</label>
            <Input id="step1-app-slug" bind:value={appSlug} placeholder={'com.fathforce.caterm'} />
          </div>
        </div>

        <div>
          <label for="step1-app-desc" class="block text-xs font-semibold text-[var(--ca-text-secondary)] mb-1">Description</label>
          <Textarea id="step1-app-desc" bind:value={description} rows={3} placeholder={'Describe the purpose of this slice...'} />
        </div>
      </div>
    {:else if currentStep === 1}
      <div class="space-y-4">
        <h3 class="text-sm font-bold text-[var(--ca-text-primary)]">Step 2: Security & Authentication</h3>
        <p class="text-xs text-[var(--ca-text-muted)]">Configure zero-knowledge access controls and limits.</p>

        <div>
          <label for="step2-pin" class="block text-xs font-semibold text-[var(--ca-text-secondary)] mb-2">6-Digit Profile PIN</label>
          <PinInput length={6} bind:value={pinCode} />
        </div>

        <div class="pt-2">
          <Switch bind:checked={e2eeEnabled} label="Enable End-to-End Vault Encryption" />
        </div>

        <div>
          <label for="step2-ratelimit" class="block text-xs font-semibold text-[var(--ca-text-secondary)] mb-1">Request Rate Limit (req/min)</label>
          <NumberInput id="step2-ratelimit" bind:value={rateLimit} min={10} max={1000} step={10} />
        </div>
      </div>
    {:else if currentStep === 2}
      <div class="space-y-5">
        <h3 class="text-sm font-bold text-[var(--ca-text-primary)]">Step 3: Customization & Branding</h3>
        <p class="text-xs text-[var(--ca-text-muted)]">Fine-tune the design system accent and hardware rim lighting.</p>

        <div>
          <label for="step3-accent" class="block text-xs font-semibold text-[var(--ca-text-secondary)] mb-2">Primary Accent Color</label>
          <div class="flex items-center gap-3">
            <ColorPicker bind:value={accentColor} />
            <span class="font-mono text-xs">{accentColor}</span>
          </div>
        </div>

        <div class="pt-2">
          <Switch bind:checked={ambientUnderglow} label="Enable Ambient GPU Rim Lighting" />
        </div>

        <div>
          <div class="flex justify-between text-xs mb-1">
            <span class="text-[var(--ca-text-secondary)] font-medium">Memory Cache Range (% RAM)</span>
            <span class="font-mono text-[var(--ca-text-muted)]">{memoryThreshold[0]}% — {memoryThreshold[1]}%</span>
          </div>
          <RangeSlider bind:values={memoryThreshold} min={0} max={100} step={5} />
        </div>
      </div>
    {:else}
      <div class="space-y-4">
        <h3 class="text-sm font-bold text-[var(--ca-text-primary)]">Step 4: Review Configuration</h3>
        <p class="text-xs text-[var(--ca-text-muted)]">Verify your settings before finalizing this application slice.</p>

        <div class="divide-y divide-[var(--ca-border)] rounded-xl border border-[var(--ca-border)] p-4 bg-[var(--ca-surface-subtle)] space-y-2 text-xs">
          <div class="flex justify-between py-1.5">
            <span class="text-[var(--ca-text-muted)]">Application Name</span>
            <span class="font-semibold">{appName}</span>
          </div>
          <div class="flex justify-between py-1.5">
            <span class="text-[var(--ca-text-muted)]">Bundle Slug</span>
            <span class="font-mono text-[var(--ca-brand)]">{appSlug}</span>
          </div>
          <div class="flex justify-between py-1.5">
            <span class="text-[var(--ca-text-muted)]">E2EE Security</span>
            <Badge variant={e2eeEnabled ? 'success' : 'neutral'}>{e2eeEnabled ? 'Active' : 'Disabled'}</Badge>
          </div>
          <div class="flex justify-between py-1.5">
            <span class="text-[var(--ca-text-muted)]">Brand Accent</span>
            <div class="flex items-center gap-1.5 font-mono">
              <span class="w-3 h-3 rounded-full" style="background-color: {accentColor}"></span>
              {accentColor}
            </div>
          </div>
        </div>
      </div>
    {/if}

    <!-- Navigation Buttons -->
    <div class="flex items-center justify-between pt-4 border-t border-[var(--ca-border)]">
      <Button
        variant="ghost"
        size="md"
        disabled={currentStep === 0}
        onclick={handleBack}
      >
        Back
      </Button>

      <Button
        variant="primary"
        size="md"
        disabled={isSubmitting}
        onclick={handleNext}
      >
        {#if isSubmitting}
          Saving...
        {:else if currentStep === steps.length - 1}
          Confirm & Save
        {:else}
          Next Step →
        {/if}
      </Button>
    </div>
  </Card>
</PageContainer>
