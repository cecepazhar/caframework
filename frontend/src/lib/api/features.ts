import {
  invoke_listFeatureFlags,
  invoke_getFeatureFlag,
  invoke_setFeatureFlag,
  type FeatureToggleItem,
} from './generated_bindings';

export type { FeatureToggleItem };

export async function listFeatureFlags(): Promise<FeatureToggleItem[]> {
  return invoke_listFeatureFlags();
}

export async function getFeatureFlag(key: string): Promise<FeatureToggleItem> {
  return invoke_getFeatureFlag(key);
}

export async function setFeatureFlag(
  key: string,
  enabled: boolean,
): Promise<FeatureToggleItem> {
  return invoke_setFeatureFlag(key, enabled);
}
