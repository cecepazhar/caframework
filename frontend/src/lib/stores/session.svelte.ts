import { invoke_sessionPermissions } from '$lib/api/generated_bindings';

let currentPermissions = $state<string[]>([]);
let isLoaded = $state(false);

export function getSessionPermissions() {
  return {
    get list() {
      return currentPermissions;
    },
    get isLoaded() {
      return isLoaded;
    }
  };
}

export async function refreshSessionPermissions(): Promise<string[]> {
  try {
    const perms = await invoke_sessionPermissions();
    currentPermissions = perms;
    isLoaded = true;
    return perms;
  } catch {
    currentPermissions = [];
    isLoaded = true;
    return [];
  }
}

export function resetSessionPermissions() {
  currentPermissions = [];
  isLoaded = false;
}

export function setSessionPermissionsForTest(perms: string[]) {
  currentPermissions = [...perms];
  isLoaded = true;
}

/**
 * Checks whether the active session has the requested permission.
 * Wildcard '*' grants all permissions.
 * 'resource:*' grants all actions for resource.
 * Exact match 'resource:action' grants that permission.
 * If no session is active or permission is missing, returns false.
 */
export function can(permission: string): boolean {
  if (!permission || permission.trim() === '') return true;
  if (currentPermissions.includes('*')) return true;
  if (currentPermissions.includes(permission)) return true;
  if (permission.includes(':')) {
    const [res] = permission.split(':');
    if (currentPermissions.includes(`${res}:*`)) return true;
  }
  return false;
}
