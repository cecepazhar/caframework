// CATerm Pro state for the UI: who is signed in, what this device is entitled to, and whether
// the Pro server is reachable. Entitlement comes from the signed token checked offline by the
// backend, so being offline never downgrades a paying user (within the 14-day grace).
import {
  proStatus,
  proServerAvailable,
  proErrorCode,
  type ProStatus,
  type SyncOutcome
} from '$lib/api/pro';

/** How often an unlocked app re-syncs its licence (the server re-issues a 14-day token). */
const SYNC_INTERVAL_MS = 6 * 60 * 60 * 1000;

let status = $state<ProStatus | null>(null);
let serverAvailable = $state<boolean | null>(null);
let syncing = $state(false);
let syncTimer: ReturnType<typeof setInterval> | null = null;

export function getPro() {
  return {
    get status() {
      return status;
    },
    /** This account's own Pro plan or trial is valid on this device. */
    get isPro() {
      const ent = status?.entitlement;
      return ent?.state === 'valid' && ent.features.includes('cloud_sync');
    },
    /** Access through someone else's team only (free account in an entitled team). */
    get isTeamMember() {
      const ent = status?.entitlement;
      return ent?.state === 'valid' && ent.tier === 'team';
    },
    /** null while unknown (not checked yet / browser preview). */
    get serverAvailable() {
      return serverAvailable;
    },
    get syncing() {
      return syncing;
    }
  };
}

export async function refreshProStatus(): Promise<void> {
  try {
    status = await proStatus();
  } catch {
    // Browser preview or vault locked mid-call: keep what we had.
  }
}

export async function checkProServer(): Promise<boolean> {
  try {
    serverAvailable = await proServerAvailable();
  } catch {
    serverAvailable = false;
  }
  return serverAvailable;
}

/** Heartbeat. The status is refreshed. */
export async function syncPro(): Promise<SyncOutcome | null> {
  if (syncing) return null;
  syncing = true;
  try {
    await refreshProStatus();
    return null;
  } finally {
    syncing = false;
  }
}

export function setProStatus(next: ProStatus): void {
  status = next;
}

/**
 * Called once the vault is open: refreshes status and checks server.
 */
export async function onVaultUnlocked(): Promise<void> {
  await refreshProStatus();
  void checkProServer();
  if (status?.signedIn) void syncPro().catch(() => {});
  if (syncTimer) clearInterval(syncTimer);
  syncTimer = setInterval(() => {
    if (status?.signedIn) void syncPro().catch(() => {});
  }, SYNC_INTERVAL_MS);
}

/** Vault locked or app signed out: forget in-memory Pro state until the next unlock. */
export function onVaultLocked(): void {
  if (syncTimer) clearInterval(syncTimer);
  syncTimer = null;
  status = null;
}
