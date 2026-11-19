// Navigation destinations, connected to the code-generated nav items and permission gating.
import { t } from '$lib/i18n/index.svelte';
import {
  GENERATED_NAV_ITEMS,
  NAVIGATION_CONFIG,
  type GeneratedNavItem,
  type NavigationConfig
} from '$lib/generated/nav';
import { can } from '$lib/stores/session.svelte';

export interface NavItem {
  key: string;
  href: string;
  label: string;
  /** SVG path for the icon */
  path: string;
  permission: string;
  isPrimary?: boolean;
}

export function getNavigationConfig(): NavigationConfig {
  return NAVIGATION_CONFIG;
}

/**
 * Returns active navigation items filtered by current session permissions (P16.2).
 * Pinned primary action appears at the top (A-04 §2).
 */
export function navItems(): NavItem[] {
  const primaryKey = NAVIGATION_CONFIG.primaryAction;

  const filtered = GENERATED_NAV_ITEMS.filter((item) => can(item.permission)).map((item) => ({
    key: item.key,
    href: item.key === 'notes' ? '/' : `/${item.key}`,
    label: t(`nav.${item.key}`),
    path: item.icon,
    permission: item.permission,
    isPrimary: primaryKey === item.key
  }));

  // Sort so primary action is always pinned at the top
  if (primaryKey) {
    filtered.sort((a, b) => {
      if (a.key === primaryKey) return -1;
      if (b.key === primaryKey) return 1;
      return 0;
    });
  }

  return filtered;
}

export function primaryNavItem(): NavItem | null {
  const items = navItems();
  const primaryKey = NAVIGATION_CONFIG.primaryAction;
  if (!primaryKey) return null;
  return items.find((item) => item.key === primaryKey) ?? null;
}

export function settingsNavItem(): NavItem {
  return {
    key: 'settings',
    href: '/settings',
    label: t('nav.settings'),
    path: 'M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z M15 12a3 3 0 11-6 0 3 3 0 016 0z',
    permission: 'settings:view'
  };
}
