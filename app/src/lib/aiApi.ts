import { isTauri } from "./api";
import type { AiConfig, AiResult, AiSearchResult, AiStatus, ArtefactAnswer, ReportAnswer, WhatIsAnswer } from "./types";

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<T>(cmd, args);
  }
  return mockInvoke<T>(cmd, args);
}

export const aiApi = {
  status: () => invoke<AiStatus>("ai_status"),
  configSet: (config: AiConfig) => invoke<void>("ai_config_set", { config }),
  setApiKey: (key: string) => invoke<void>("ai_set_api_key", { key }),
  models: () => invoke<string[]>("ai_models"),
  test: () => invoke<string>("ai_test"),
  whatis: (path: string, dryRun = false) => invoke<AiResult<WhatIsAnswer>>("ai_whatis", { path, dryRun }),
  artefact: (root: string, dryRun = false) => invoke<AiResult<ArtefactAnswer>>("ai_artefact", { root, dryRun }),
  search: (sentence: string, limit = 100) => invoke<AiSearchResult>("ai_search", { sentence, limit }),
  report: (dryRun = false) => invoke<AiResult<ReportAnswer>>("ai_report", { dryRun }),
  saveRule: (path: string, nameId: string, nameEn: string, descriptionId: string, descriptionEn: string) =>
    invoke<string>("ai_save_rule", { path, nameId, nameEn, descriptionId, descriptionEn }),
};

// ---- mock ----

export const aiMock = {
  config: { mode: "off", local_url: "http://localhost:11434", local_model: "", cloud_kind: "openai_compatible", cloud_url: "https://api.openai.com", cloud_model: "", anonymize: true } as AiConfig,
  hasKey: false,
  savedRules: [] as string[],
  reset() {
    this.config = { ...this.config, mode: "off", local_model: "" };
    this.hasKey = false;
    this.savedRules = [];
  },
};

const preview = (task: string, user: object) => ({ provider: aiMock.config.mode === "cloud" ? "openai_compatible" : "ollama", model: aiMock.config.local_model || "qwen2.5:3b", system: `system prompt for ${task}`, user: JSON.stringify(user) });

const mockInvoke = async <T,>(cmd: string, args?: Record<string, unknown>): Promise<T> => {
  if (cmd !== "ai_status" && cmd !== "ai_config_set" && cmd !== "ai_set_api_key" && cmd !== "ai_models" && aiMock.config.mode === "off") {
    throw { code: "ai_disabled", message: "off" };
  }
  switch (cmd) {
    case "ai_status":
      return { enabled: aiMock.config.mode !== "off", config: aiMock.config, has_key: aiMock.hasKey, key_hint: aiMock.hasKey ? "…k3y1" : null } as T;
    case "ai_config_set":
      aiMock.config = { ...(args?.config as AiConfig) };
      return undefined as T;
    case "ai_set_api_key":
      aiMock.hasKey = String(args?.key).trim().length > 0;
      return undefined as T;
    case "ai_models":
      if (aiMock.config.mode === "cloud") return (aiMock.config.cloud_kind === "anthropic" ? ["claude-fable-5-1", "claude-opus-5-5", "claude-sonnet-5-5", "claude-haiku-4-5-20251001"] : ["gpt-5", "gpt-5-mini", "deepseek-chat"]) as T;
      return ["qwen2.5:3b", "llama3.2"] as T;
    case "ai_test":
      return (aiMock.config.local_model || "qwen2.5:3b") as T;
    case "ai_whatis": {
      const path = String(args?.path);
      const p = preview("whatis", { path: path.replace("/Users/you", "~"), bytes: 1 });
      if (args?.dryRun) return { answer: null, preview: p } as T;
      return {
        answer: { summary: "Folder cache aplikasi Spotify.", likely_owner: "Spotify", verdict: "check_first", consequence: "Lagu offline perlu diunduh ulang.", confidence: 0.82, reason: "Berada di Library/Caches dan berisi file biner." },
        preview: p,
      } as T;
    }
    case "ai_artefact": {
      const p = preview("artefact", { project: "landing-page-2024" });
      if (args?.dryRun) return { answer: null, preview: p } as T;
      return {
        answer: {
          project_summary: "Proyek Next.js yang tidak disentuh 7 bulan.",
          artefacts: [
            { kind: "node_modules", regenerable: true, verdict: "check_first", explanation: "Dependensi yang dipasang ulang oleh pnpm install.", rebuild: "pnpm install", confidence: 0.9 },
            { kind: ".next", regenerable: true, verdict: "check_first", explanation: "Hasil build Next.js.", rebuild: "pnpm run build", confidence: 0.85 },
          ],
        },
        preview: p,
      } as T;
    }
    case "ai_search": {
      const sentence = String(args?.sentence);
      return {
        filter: { category: "video", min_bytes: 1073741824, min_age_days: 365, max_age_days: null, extensions: [], name_contains: null, description: `video besar yang lama (${sentence})` },
        files: [{ path: "/Users/you/Movies/zoom-rekaman-2025-03.mp4", size: 8.4 * 1024 ** 3, mtime: null, category: "video" }],
        preview: preview("search", { sentence }),
      } as T;
    }
    case "ai_report": {
      const p = preview("report", { total: 1 });
      if (args?.dryRun) return { answer: null, preview: p } as T;
      return { answer: { what_fills: "Disk kamu penuh terutama karena video rekaman dan proyek development.", start_here: "Mulai dari cache dan installer lama di Bersih.", leave_alone: "Biarkan Dokumen dan Foto." }, preview: p } as T;
    }
    case "ai_save_rule":
      aiMock.savedRules.push(String(args?.path));
      return "/config/sapu/rules/ai-x.toml" as T;
    default:
      throw { code: "unknown_command", message: cmd };
  }
};
