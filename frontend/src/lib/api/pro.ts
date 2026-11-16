import {
  cmdProStatus,
  cmdProServerAvailable,
  cmdProRegister,
  cmdProResendVerification,
  cmdProForgotPassword,
  cmdProLogin,
  cmdProStartTrial,
  cmdProAccount,
  cmdProRevokeDevice,
  cmdProLogout
} from '$lib/generated/commands';

// Mirrors caterm_core::pro (serialized camelCase). Server contract: GCC internal/catermpro/API.md.

export interface ProAccount {
  id: string;
  email: string;
  name: string;
}

export type LicenseStatus = 'trialing' | 'active' | 'past_due' | 'cancelled' | 'expired';

export interface ProLicense {
  status: LicenseStatus;
  tier: string;
  trialEndsAt: number | null;
  currentPeriodEnd: number | null;
  entitled: boolean;
}

/** What the account may do now: own plan (`tier` "pro") or team membership (`tier` "team"). */
export interface ProAccess {
  entitled: boolean;
  tier: string | null;
  features: string[];
  deviceLimit: number;
}

export interface ProPerson {
  email: string;
  name: string;
}

export interface ProMembership {
  teamId: string;
  owner: ProPerson;
  joinedAt: number;
  /** False while the owner is not subscribed. */
  active: boolean;
}

export interface ProTeamMember {
  accountId: string;
  email: string;
  name: string;
  joinedAt: number;
  activeDevices: number;
}

export interface ProTeamInvite {
  id: string;
  email: string;
  invitedAt: number;
  expiresAt: number;
}

export interface ProOwnedTeam {
  id: string;
  active: boolean;
  members: ProTeamMember[];
  invites: ProTeamInvite[];
}

/** A pending invitation addressed to the signed-in account's email. */
export interface ProInvitation {
  id: string;
  owner: ProPerson;
  invitedAt: number;
  expiresAt: number;
}

export interface ProTeamView {
  owned: ProOwnedTeam | null;
  canInvite: boolean;
  maxMembers: number;
  membership: ProMembership | null;
  invitations: ProInvitation[];
}

export interface ProDevice {
  id: string;
  name: string;
  os: string;
  clientVersion: string;
  firstSeenAt: number;
  lastSeenAt: number;
  current: boolean;
}

export type Entitlement =
  | { state: 'none' }
  | { state: 'valid'; expiresAt: number; tier: string; features: string[] }
  | { state: 'expired' }
  | { state: 'revalidationRequired'; reason: string }
  | { state: 'invalid'; reason: string };

export interface ProStatus {
  signedIn: boolean;
  /** Signed in on the lock screen; saved once the vault unlocks. */
  pending: boolean;
  account: ProAccount | null;
  license: ProLicense | null;
  entitlement: Entitlement;
  lastSyncAt: number | null;
  /** False in builds without the licence public key (dev builds, forks). */
  keyConfigured: boolean;
}

export interface DeviceLimit {
  limit: number;
  devices: ProDevice[];
}

export interface SyncOutcome {
  status: ProStatus;
  /** Set when this device couldn't be activated: the account's devices, to free a slot. */
  deviceLimit: DeviceLimit | null;
}

export interface AccountDetails {
  account: ProAccount | null;
  license: ProLicense | null;
  access: ProAccess | null;
  membership: ProMembership | null;
  devices: ProDevice[];
}

export const proStatus = () => cmdProStatus() as Promise<ProStatus>;
export const proServerAvailable = () => cmdProServerAvailable();
export const proRegister = (email: string, password: string, name: string, locale: string) =>
  cmdProRegister({ email, password, name, locale });
export const proResendVerification = (email: string) => cmdProResendVerification({ email });
export const proForgotPassword = (email: string, locale: string) => cmdProForgotPassword({ email, locale });
export const proLogin = (email: string, password: string) => cmdProLogin({ email, password }) as Promise<ProAccount>;
export const proStartTrial = () => cmdProStartTrial() as Promise<SyncOutcome>;
export const proAccount = () => cmdProAccount() as Promise<AccountDetails>;
export const proRevokeDevice = (deviceId: string) => cmdProRevokeDevice({ deviceId });
export const proLogout = () => cmdProLogout();

/**
 * The server's stable error code for a failed Pro call (`INVALID_CREDENTIALS`, `NETWORK`, ...),
 * or null for anything else. The backend sends it as the message of a `CAT-PRO-000` error.
 */
export function proErrorCode(err: unknown): string | null {
  if (!err || typeof err !== 'object') return null;
  const record = err as Record<string, unknown>;
  if (record.code !== 'CAT-PRO-000' || typeof record.message !== 'string') return null;
  const code = record.message.replace(/^pro:\s*/, '').split(':')[0].trim();
  return /^[A-Z0-9_]+$/.test(code) ? code : null;
}
