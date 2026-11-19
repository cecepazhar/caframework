// Typed Settings Registry (FR-3, P16.4)
// Allows framework modules and product modules to register settings tabs without editing the settings page.

import { ENABLED_MODULES } from '$lib/generated/modules';
import { can } from '$lib/stores/session.svelte';
import type { Component } from 'svelte';

export interface SettingsSection {
  id: string;
  labelKey: string;
  defaultLabel: string;
  labelEn?: string;
  labelId?: string;
  icon?: string;
  /** Framework or product module key. If provided and disabled, section is hidden. */
  moduleKey?: string;
  /** Permission required to view this section. If provided and session lacks permission, section is hidden. */
  permission?: string;
  /** Display order (lower numbers appear first) */
  order: number;
  /** Optional custom component for dynamic product sections */
  component?: Component<any>;
}

const customSections = new Map<string, SettingsSection>();

/**
 * Built-in framework sections specified in FR-3 and A-03.
 */
export const FRAMEWORK_SECTIONS: readonly SettingsSection[] = [
  { id: 'profile', labelKey: 'settings.tabs.profile', defaultLabel: 'Profile', order: 10 },
  { id: 'updates', labelKey: 'settings.tabs.updates', defaultLabel: 'Updates', moduleKey: 'updater', permission: 'settings:manage', order: 20 },
  { id: 'features', labelKey: 'settings.tabs.features', defaultLabel: 'Features', permission: 'settings:manage', order: 30 },
  { id: 'ai', labelKey: 'settings.tabs.ai', defaultLabel: 'AI Assistant', moduleKey: 'ai', permission: 'ai:configure', order: 40 },
  { id: 'subscription', labelKey: 'settings.tabs.subscription', defaultLabel: 'Pro License', moduleKey: 'pro', permission: 'pro:manage', order: 50 },
  { id: 'security', labelKey: 'settings.tabs.security', defaultLabel: 'Security & Vault', permission: 'vault:manage', order: 60 },
  { id: 'backup', labelKey: 'settings.tabs.backup', defaultLabel: 'Backup & Restore', permission: 'backup:export', order: 70 },
  { id: 'schedules', labelKey: 'settings.tabs.schedules', defaultLabel: 'Schedules', moduleKey: 'scheduler', permission: 'scheduler:view', order: 80 },
  { id: 'integrations', labelKey: 'settings.tabs.integrations', defaultLabel: 'Integrations', moduleKey: 'integrations', permission: 'integrations:view', order: 90 },
  { id: 'dataIo', labelKey: 'settings.tabs.dataIo', defaultLabel: 'Data Import/Export', permission: 'data:export', order: 100 },
  { id: 'performance', labelKey: 'settings.tabs.performance', defaultLabel: 'Performance & Privacy', permission: 'settings:manage', order: 110 },
  { id: 'shortcuts', labelKey: 'settings.tabs.shortcuts', defaultLabel: 'Shortcuts', permission: 'settings:view', order: 120 },
  { id: 'about', labelKey: 'settings.tabs.about', defaultLabel: 'About', order: 130 }
] as const;

/**
 * Registers a product or plugin settings section.
 */
export function registerSettingsSection(section: SettingsSection): void {
  customSections.set(section.id, section);
}

/**
 * Unregisters a settings section (used for cleanup or tests).
 */
export function unregisterSettingsSection(id: string): void {
  customSections.delete(id);
}

/**
 * Clears all custom registered settings sections (for tests).
 */
export function clearCustomSettingsSections(): void {
  customSections.clear();
}

/**
 * Checks if a module is currently enabled in the app.
 */
function isModuleActive(key?: string): boolean {
  if (!key) return true;
  const modKey = key as keyof typeof ENABLED_MODULES;
  if (modKey in ENABLED_MODULES) {
    return Boolean(ENABLED_MODULES[modKey]);
  }
  // For product modules, default to true unless explicitly disabled
  return true;
}

/**
 * Returns all active settings sections (framework + registered product sections)
 * filtered by module switch and session permissions, sorted by order.
 */
export function getVisibleSettingsSections(): SettingsSection[] {
  const allSections: SettingsSection[] = [
    ...FRAMEWORK_SECTIONS,
    ...customSections.values()
  ];

  return allSections
    .filter((sec) => isModuleActive(sec.moduleKey))
    .filter((sec) => (sec.permission ? can(sec.permission) : true))
    .sort((a, b) => a.order - b.order);
}
