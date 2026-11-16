import { writable, derived } from 'svelte/store';
import {
  cmdBillingGetEffectiveState,
  cmdBillingRequestOtp,
  cmdBillingVerifyOtp,
  cmdBillingActivateLicense,
  cmdBillingCreateCheckout,
  cmdBillingPollCheckout,
  cmdBillingSignOut
} from '$lib/generated/commands';

export interface EntitlementState {
  tier: 'free' | 'pro' | 'pro_grace' | 'free_needs_online_check';
  billingType: string;
  isPro: boolean;
  expiresAt: string | null;
  graceUntil: string | null;
  warning: string | null;
}

export interface BillingPlan {
  planId: string;
  billingType: 'monthly' | 'yearly' | 'lifetime';
  priceMinor: number;
  currency: 'IDR' | 'USD';
}

function createBillingStore() {
  const { subscribe, set, update } = writable<EntitlementState>({
    tier: 'free',
    billingType: 'free',
    isPro: false,
    expiresAt: null,
    graceUntil: null,
    warning: null
  });

  return {
    subscribe,
    set,
    update,
    async refresh(): Promise<EntitlementState> {
      try {
        const state = await cmdBillingGetEffectiveState();
        set(state);
        return state;
      } catch (err) {
        console.error('Failed to refresh entitlement state:', err);
        throw err;
      }
    },
    async requestOtp(email: string): Promise<void> {
      return cmdBillingRequestOtp({ email });
    },
    async verifyOtp(email: string, otpCode: string): Promise<void> {
      await cmdBillingVerifyOtp({ email, otpCode });
      await this.refresh();
    },
    async activateLicense(licenseKey: string): Promise<void> {
      await cmdBillingActivateLicense({ licenseKey });
      await this.refresh();
    },
    async createCheckout(planId: string): Promise<{ checkoutId: string; checkoutUrl: string; completionMode: 'poll' | 'license_key' }> {
      return cmdBillingCreateCheckout({ planId });
    },
    async pollCheckout(checkoutId: string): Promise<string> {
      const status = await cmdBillingPollCheckout({ checkoutId });
      if (status === 'paid') {
        await this.refresh();
      }
      return status;
    },
    async signOut(): Promise<void> {
      await cmdBillingSignOut();
      set({
        tier: 'free',
        billingType: 'free',
        isPro: false,
        expiresAt: null,
        graceUntil: null,
        warning: null
      });
    }
  };
}

export const billingStore = createBillingStore();
export const isPro = derived(billingStore, ($b) => $b.isPro);
export const isGrace = derived(billingStore, ($b) => $b.tier === 'pro_grace');
export const needsOnlineCheck = derived(billingStore, ($b) => $b.tier === 'free_needs_online_check');
