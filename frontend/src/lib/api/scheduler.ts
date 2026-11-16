import {
  invoke_schedulerListJobs,
  invoke_schedulerSetJobEnabled,
  invoke_schedulerUpdateSchedule,
  invoke_schedulerRunJobNow,
  invoke_schedulerGetHistory,
  invoke_schedulerGetDesktopSettings,
  invoke_schedulerSetDesktopSettings,
  type JobStatusDto,
  type JobRunRecord,
  type SchedulerDesktopSettings
} from './generated_bindings';

export type { JobStatusDto, JobRunRecord, SchedulerDesktopSettings };

export async function listJobs(): Promise<JobStatusDto[]> {
  return invoke_schedulerListJobs();
}

export async function setJobEnabled(key: string, enabled: boolean): Promise<void> {
  await invoke_schedulerSetJobEnabled(key, enabled);
}

export async function updateSchedule(key: string, schedule: string): Promise<void> {
  await invoke_schedulerUpdateSchedule(key, schedule);
}

export async function runJobNow(key: string): Promise<string> {
  return invoke_schedulerRunJobNow(key);
}

export async function getHistory(key?: string | null, limit?: number | null): Promise<JobRunRecord[]> {
  return invoke_schedulerGetHistory(key ?? null, limit ?? 50);
}

export async function getDesktopSettings(): Promise<SchedulerDesktopSettings> {
  return invoke_schedulerGetDesktopSettings();
}

export async function setDesktopSettings(settings: SchedulerDesktopSettings): Promise<void> {
  await invoke_schedulerSetDesktopSettings(settings);
}
