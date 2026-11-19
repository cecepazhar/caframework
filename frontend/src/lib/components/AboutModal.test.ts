import { describe, it, expect } from 'vitest';
import { BRAND_CONFIG } from '$lib/generated/brand';
import { APP_CONFIG } from '$lib/generated/app';

describe('Brand Integration (P16.6)', () => {
  it('BRAND_CONFIG provides non-empty brand values from app.toml', () => {
    expect(BRAND_CONFIG.developer).toBeTruthy();
    expect(BRAND_CONFIG.website).toBeTruthy();
    expect(BRAND_CONFIG.email).toBeTruthy();
    expect(BRAND_CONFIG.copyright).toContain('{year}');
  });

  it('APP_CONFIG provides app name and version', () => {
    expect(APP_CONFIG.name).toBeTruthy();
    expect(APP_CONFIG.caframeworkVersion).toBeTruthy();
  });

  it('showOnLockScreen boolean matches config', () => {
    expect(typeof BRAND_CONFIG.showOnLockScreen).toBe('boolean');
  });
});
