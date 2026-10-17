import type { CloudKind } from "@/lib/types";

/** Cloud providers Sapu knows the address of. Anything else is "custom". */
export interface ProviderPreset {
  id: string;
  label: string;
  kind: CloudKind;
  url: string;
  /** Where to create a key; shown as a hint. */
  keysUrl?: string;
}

export const PRESETS: ProviderPreset[] = [
  { id: "openai", label: "OpenAI", kind: "openai_compatible", url: "https://api.openai.com", keysUrl: "https://platform.openai.com/api-keys" },
  { id: "anthropic", label: "Anthropic (Claude)", kind: "anthropic", url: "https://api.anthropic.com", keysUrl: "https://console.anthropic.com/settings/keys" },
  { id: "deepseek", label: "DeepSeek", kind: "openai_compatible", url: "https://api.deepseek.com", keysUrl: "https://platform.deepseek.com/api_keys" },
  { id: "gemini", label: "Google Gemini", kind: "openai_compatible", url: "https://generativelanguage.googleapis.com/v1beta/openai", keysUrl: "https://aistudio.google.com/apikey" },
  { id: "groq", label: "Groq", kind: "openai_compatible", url: "https://api.groq.com/openai/v1", keysUrl: "https://console.groq.com/keys" },
  { id: "mistral", label: "Mistral", kind: "openai_compatible", url: "https://api.mistral.ai/v1", keysUrl: "https://console.mistral.ai/api-keys" },
  { id: "openrouter", label: "OpenRouter", kind: "openai_compatible", url: "https://openrouter.ai/api/v1", keysUrl: "https://openrouter.ai/keys" },
  { id: "xai", label: "xAI (Grok)", kind: "openai_compatible", url: "https://api.x.ai/v1", keysUrl: "https://console.x.ai" },
  { id: "together", label: "Together AI", kind: "openai_compatible", url: "https://api.together.xyz/v1", keysUrl: "https://api.together.ai/settings/api-keys" },
  { id: "moonshot", label: "Moonshot (Kimi)", kind: "openai_compatible", url: "https://api.moonshot.ai/v1", keysUrl: "https://platform.moonshot.ai" },
];

export const CUSTOM = "custom";

const norm = (u: string) => u.trim().replace(/\/+$/, "").toLowerCase();

/** The preset whose address matches `url`, or "custom". */
export function presetFor(kind: CloudKind, url: string): string {
  return PRESETS.find((p) => p.kind === kind && norm(p.url) === norm(url))?.id ?? CUSTOM;
}
