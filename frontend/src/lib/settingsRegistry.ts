export * from '$lib/settings/registry';
import { getVisibleSettingsSections, type SettingsSection } from '$lib/settings/registry';

export function getRegisteredSettingsSections(): SettingsSection[] {
  return getVisibleSettingsSections();
}
