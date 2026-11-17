import {
  invoke_listIntegrations,
  invoke_setIntegrationSecret,
  invoke_deleteIntegrationSecret,
  invoke_testIntegrationConnection,
  invoke_getIntegrationLogs,
  invoke_getOutboxItems,
  invoke_flushOutboxNow
} from './generated_bindings';

export interface IntegrationStatusDto {
  key: string;
  kind: string;
  base_url: string;
  auth_kind: string;
  has_secret: boolean;
  user_configurable: boolean;
}

export interface TestConnectionResult {
  success: boolean;
  message: string;
  duration_ms: number;
}

export interface IntegrationRequestLogRecord {
  id: string;
  integration_key: string;
  method: string;
  host: string;
  path: string;
  status: number;
  duration_ms: number;
  timestamp: number;
}

export type OutboxStatus = 'pending' | 'flushed' | 'dead_letter' | 'expired';

export interface OutboxItem {
  id: string;
  integration_key: string;
  endpoint: string;
  method: string;
  payload: string;
  idempotency_key: string;
  created_at: number;
  attempts: number;
  max_attempts: number;
  status: OutboxStatus;
  last_error: string | null;
  expires_at: number | null;
}

export interface FlushSummary {
  total: number;
  succeeded: number;
  failed: number;
  expired: number;
  dead_lettered: number;
}

export async function listIntegrations(): Promise<IntegrationStatusDto[]> {
  return invoke_listIntegrations();
}

export async function setIntegrationSecret(key: string, secret: string): Promise<void> {
  await invoke_setIntegrationSecret(key, secret);
}

export async function deleteIntegrationSecret(key: string): Promise<void> {
  await invoke_deleteIntegrationSecret(key);
}

export async function testIntegrationConnection(key: string): Promise<TestConnectionResult> {
  return invoke_testIntegrationConnection(key);
}

export async function getIntegrationLogs(
  key?: string | null,
  limit?: number | null
): Promise<IntegrationRequestLogRecord[]> {
  return invoke_getIntegrationLogs(key ?? null, limit ?? 50);
}

export async function getOutboxItems(
  status?: OutboxStatus | null,
  limit?: number | null
): Promise<OutboxItem[]> {
  return invoke_getOutboxItems(status ?? null, limit ?? 50);
}

export async function flushOutboxNow(): Promise<FlushSummary> {
  return invoke_flushOutboxNow();
}
