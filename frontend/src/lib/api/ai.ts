import {
  cmdGetAiSettings,
  cmdSaveAiSettings,
  cmdAiChat,
  type AiSettings as GeneratedAiSettings
} from '$lib/generated/commands';

export interface AiPlanStep {
  id?: string;
  step_number: number;
  title: string;
  command: string;
  description: string;
  is_sudo?: boolean;
  is_danger?: boolean;
  action_type?: string;
  action_name?: string;
  action_params?: any;
}

export interface AiSettings {
  enabled?: boolean;
  provider: string;
  base_url?: string;
  endpoint?: string | null;
  api_key: string;
  model: string;
}

export interface AiChatMessage {
  role: 'user' | 'assistant' | 'system';
  content: string;
}

export interface AiChatReply {
  reply: string;
  ready: boolean;
  steps: AiPlanStep[];
}

export async function getAiSettings(): Promise<AiSettings> {
  const s = await cmdGetAiSettings();
  if (s && typeof s === 'object' && !('base_url' in s)) {
    Object.defineProperty(s, 'base_url', {
      get() {
        return this.endpoint ?? '';
      },
      set(v: string) {
        this.endpoint = v;
      },
      enumerable: false,
      configurable: true
    });
  }
  return s as any;
}

export async function saveAiSettings(settings: AiSettings): Promise<void> {
  const s: any = { ...settings };
  if (s.endpoint === undefined && s.base_url !== undefined) {
    s.endpoint = s.base_url || null;
  }
  if (s.enabled === undefined) {
    s.enabled = true;
  }
  return cmdSaveAiSettings({ settings: s });
}

export async function aiChat(
  messages: AiChatMessage[],
  _hostLabel?: string,
  _hosted: boolean = false
): Promise<AiChatReply> {
  const prompt = messages.map(m => `${m.role}: ${m.content}`).join('\n');
  const res = await cmdAiChat({ prompt });
  return {
    reply: res?.message ?? '',
    ready: true,
    steps: []
  };
}
