import { describe, it, expect, beforeEach } from 'vitest';
import {
  registerSettingsSection,
  unregisterSettingsSection,
  clearCustomSettingsSections,
  getVisibleSettingsSections,
  FRAMEWORK_SECTIONS
} from './registry';
import { setSessionPermissionsForTest, resetSessionPermissions } from '$lib/stores/session.svelte';

describe('Settings Registry (FR-3, P16.4)', () => {
  beforeEach(() => {
    clearCustomSettingsSections();
    resetSessionPermissions();
  });

  it('super role sees all enabled framework sections', () => {
    setSessionPermissionsForTest(['*']);
    const sections = getVisibleSettingsSections();
    expect(sections.length).toBeGreaterThanOrEqual(10);
    expect(sections.some((s) => s.id === 'profile')).toBe(true);
    expect(sections.some((s) => s.id === 'backup')).toBe(true);
    expect(sections.some((s) => s.id === 'features')).toBe(true);
  });

  it('viewer role with only "notes:view" does not see protected sections like backup or features', () => {
    setSessionPermissionsForTest(['notes:view']);
    const sections = getVisibleSettingsSections();
    // Profile and about have no permission gate, so they are visible
    expect(sections.some((s) => s.id === 'profile')).toBe(true);
    expect(sections.some((s) => s.id === 'about')).toBe(true);
    // Protected sections must be hidden
    expect(sections.some((s) => s.id === 'backup')).toBe(false);
    expect(sections.some((s) => s.id === 'features')).toBe(false);
    expect(sections.some((s) => s.id === 'schedules')).toBe(false);
  });

  it('product section registered dynamically appears when permitted', () => {
    setSessionPermissionsForTest(['*']);
    registerSettingsSection({
      id: 'custom_product_catalog',
      labelKey: 'catalog.settings',
      defaultLabel: 'Product Catalog',
      moduleKey: 'catalog',
      permission: 'catalog:view',
      order: 25
    });

    const sections = getVisibleSettingsSections();
    expect(sections.some((s) => s.id === 'custom_product_catalog')).toBe(true);
    const catalogSec = sections.find((s) => s.id === 'custom_product_catalog');
    expect(catalogSec?.defaultLabel).toBe('Product Catalog');
  });

  it('product section is hidden when session lacks its permission', () => {
    setSessionPermissionsForTest(['notes:view']);
    registerSettingsSection({
      id: 'custom_product_catalog',
      labelKey: 'catalog.settings',
      defaultLabel: 'Product Catalog',
      moduleKey: 'catalog',
      permission: 'catalog:view',
      order: 25
    });

    const sections = getVisibleSettingsSections();
    expect(sections.some((s) => s.id === 'custom_product_catalog')).toBe(false);
  });
});
