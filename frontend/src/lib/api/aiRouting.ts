import {
  invoke_aiGetProviders,
  invoke_aiSaveProvider,
  invoke_aiDeleteProvider,
  invoke_aiGetRoutingMatrix,
  invoke_aiSaveRoutingRule,
  invoke_aiGetPersonas,
  invoke_aiSavePersona,
  invoke_aiDeletePersona,
  invoke_aiGetHabits,
  invoke_aiSearchHabits,
  invoke_aiToggleHabitPin,
  invoke_aiDeleteHabit,
  invoke_aiGetSkills,
  invoke_aiSaveSkill,
  invoke_aiDeleteSkill,
  invoke_aiDispatchTask,
  invoke_aiDispatchTaskWithSkill
} from './generated_bindings';

export type TaskType =
  | 'chat'
  | 'error_diagnostic'
  | 'prompt_studio'
  | 'command_autocomplete'
  | 'security_review';

export type ProviderType =
  | 'caterm_hosted'
  | 'anthropic'
  | 'openai_compatible'
  | 'ollama';

export interface AiProvider {
  id: string;
  name: string;
  provider_type: ProviderType;
  base_url: string;
  default_model: string;
  is_active: boolean;
  has_api_key: boolean;
  created_at?: number;
  updated_at?: number;
}

export interface AiProviderConfig {
  id: string;
  name: string;
  provider_type: ProviderType;
  base_url: string;
  api_key?: string | null;
  default_model: string;
  is_active: boolean;
  custom_headers?: Record<string, string> | null;
  created_at?: number;
  updated_at?: number;
}

export interface TaskRouteRule {
  task_type: TaskType;
  primary_provider_id: string;
  primary_model: string;
  fallback_provider_id?: string | null;
  fallback_model?: string | null;
  temperature: number;
  max_tokens: number;
  system_persona_id?: string | null;
}

export interface SystemPersona {
  id: string;
  title: string;
  description?: string | null;
  system_prompt: string;
  custom_rules: string[];
  environment_constraints?: string | null;
  is_global_default: boolean;
  created_at?: number;
  updated_at?: number;
}

export interface HabitFact {
  id: string;
  category: string;
  key_tag: string;
  fact_content: string;
  source_context?: string | null;
  confidence_score: number;
  occurrence_count: number;
  is_pinned: boolean;
  is_active: boolean;
  created_at?: number;
  last_accessed_at?: number;
}

export interface TerminalContext {
  tail_lines?: string[];
  exit_code?: number | null;
  cwd?: string | null;
  user?: string | null;
  host_label?: string | null;
  active_host_id?: string | null;
  shell?: string | null;
  os_info?: string | null;
}

export interface DispatchResult {
  content: string;
  provider_id: string;
  provider_type: ProviderType;
  model: string;
  fell_back: boolean;
  prompt_tokens?: number | null;
  completion_tokens?: number | null;
}

// ---- Tauri IPC Commands ----

export async function aiGetProviders(): Promise<AiProvider[]> {
  return invoke_aiGetProviders();
}

export async function aiSaveProvider(provider: AiProviderConfig): Promise<void> {
  return invoke_aiSaveProvider(provider);
}

export async function aiDeleteProvider(id: string): Promise<void> {
  return invoke_aiDeleteProvider(id);
}

export async function aiGetRoutingMatrix(): Promise<TaskRouteRule[]> {
  return invoke_aiGetRoutingMatrix();
}

export async function aiSaveRoutingRule(rule: TaskRouteRule): Promise<void> {
  return invoke_aiSaveRoutingRule(rule);
}

export interface CustomSkill {
  id: string;
  name: string;
  title: string;
  description: string;
  category: string;
  triggers: string[];
  preferred_model_id?: string | null;
  system_instructions: string;
  allowed_tools: string[];
  is_builtin: boolean;
  is_enabled: boolean;
  created_at: number;
}

export async function aiGetPersonas(): Promise<SystemPersona[]> {
  return invoke_aiGetPersonas();
}

export async function aiSavePersona(persona: SystemPersona): Promise<void> {
  return invoke_aiSavePersona(persona);
}

export async function aiDeletePersona(id: string): Promise<void> {
  return invoke_aiDeletePersona(id);
}

export async function aiGetHabits(): Promise<HabitFact[]> {
  return invoke_aiGetHabits();
}

export async function aiSearchHabits(query: string, limit: number = 20): Promise<HabitFact[]> {
  return invoke_aiSearchHabits(query, limit);
}

export async function aiToggleHabitPin(id: string): Promise<boolean> {
  return invoke_aiToggleHabitPin(id);
}

export async function aiDeleteHabit(id: string): Promise<void> {
  return invoke_aiDeleteHabit(id);
}

export async function aiDispatchTask(
  taskType: TaskType | string,
  query: string,
  terminalCtx?: TerminalContext
): Promise<DispatchResult> {
  return invoke_aiDispatchTask(taskType as string, query, terminalCtx ?? null);
}

export async function aiGetSkills(): Promise<CustomSkill[]> {
  return invoke_aiGetSkills();
}

export async function aiSaveSkill(skill: CustomSkill): Promise<void> {
  return invoke_aiSaveSkill(skill);
}

export async function aiDeleteSkill(id: string): Promise<void> {
  return invoke_aiDeleteSkill(id);
}

export async function aiDispatchTaskWithSkill(
  taskType: TaskType | string,
  query: string,
  skillName?: string | null,
  terminalCtx?: TerminalContext
): Promise<DispatchResult> {
  return invoke_aiDispatchTaskWithSkill(taskType as string, query, skillName ?? null, terminalCtx ?? null);
}
