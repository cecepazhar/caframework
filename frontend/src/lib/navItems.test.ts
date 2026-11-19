import { describe, it, expect, beforeEach } from 'vitest';
import { navItems, primaryNavItem, getNavigationConfig } from '$lib/navItems';
import { setSessionPermissionsForTest, resetSessionPermissions } from '$lib/stores/session.svelte';
import { GENERATED_NAV_ITEMS } from '$lib/generated/nav';

describe('Navigation permissions filtering (P16.2)', () => {
  beforeEach(() => {
    resetSessionPermissions();
  });

  it('super role permissions ("*") returns all navigation items', () => {
    setSessionPermissionsForTest(['*']);
    const items = navItems();
    expect(items.length).toBe(GENERATED_NAV_ITEMS.length);
    expect(items.some((i) => i.key === 'notes')).toBe(true);
  });

  it('viewer role permissions ("notes:view") only returns permitted items', () => {
    setSessionPermissionsForTest(['notes:view']);
    const items = navItems();
    expect(items.length).toBeGreaterThan(0);
    expect(items.every((i) => i.permission === 'notes:view' || !i.permission)).toBe(true);
  });

  it('empty permissions returns no restricted items', () => {
    setSessionPermissionsForTest([]);
    const items = navItems();
    expect(items.length).toBe(0);
  });

  it('primary action is pinned to the top of navigation', () => {
    setSessionPermissionsForTest(['*']);
    const config = getNavigationConfig();
    const primary = primaryNavItem();
    if (config.primaryAction) {
      expect(primary).not.toBeNull();
      expect(primary?.key).toBe(config.primaryAction);
      const items = navItems();
      expect(items[0]?.key).toBe(config.primaryAction);
      expect(items[0]?.isPrimary).toBe(true);
    }
  });
});
