// Organize (Rapikan) commands and events. Mocked outside Tauri so the
// flow can be developed and tested in a browser / vitest.

import { isTauri } from "./api";
import type {
  OrganizeAnalyseProgress,
  OrganizeAnalysis,
  OrganizeEntry,
  OrganizeGroup,
  OrganizeRun,
  OrganizeRunProgress,
  OrganizeRunResult,
  OrganizeSelection,
  OrganizeTarget,
  OrganizeUndoReport,
} from "./types";

type Unlisten = () => void;
const bus = new EventTarget();
const emit = <T,>(event: string, detail: T) => bus.dispatchEvent(new CustomEvent(event, { detail }));

async function listen<T>(event: string, cb: (payload: T) => void): Promise<Unlisten> {
  if (isTauri()) {
    const { listen } = await import("@tauri-apps/api/event");
    return listen<T>(event, (e) => cb(e.payload));
  }
  const handler = (e: Event) => cb((e as CustomEvent<T>).detail);
  bus.addEventListener(event, handler);
  return () => bus.removeEventListener(event, handler);
}

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<T>(cmd, args);
  }
  return mockInvoke<T>(cmd, args);
}

export const organizeApi = {
  targets: () => invoke<OrganizeTarget[]>("organize_targets"),
  analyse: (root: string, useAi: boolean) => invoke<void>("organize_analyze_start", { root, useAi }),
  plan: () => invoke<OrganizeAnalysis | null>("organize_plan"),
  status: () => invoke<{ analysing: boolean; applying: boolean; has_plan: boolean }>("organize_status"),
  apply: (selection: OrganizeSelection) => invoke<void>("organize_apply_start", { selection }),
  undo: (id: string) => invoke<OrganizeUndoReport>("organize_undo", { id }),
  history: (limit: number) => invoke<OrganizeRun[]>("organize_history", { limit }),
  onProgress: (cb: (p: OrganizeAnalyseProgress) => void) => listen<OrganizeAnalyseProgress>("organize://progress", cb),
  onPlan: (cb: (a: OrganizeAnalysis) => void) => listen<OrganizeAnalysis>("organize://plan", cb),
  onError: (cb: (e: { code: string; message: string }) => void) => listen("organize://error", cb),
  onRunProgress: (cb: (p: OrganizeRunProgress) => void) => listen<OrganizeRunProgress>("organize://run_progress", cb),
  onRunDone: (cb: (r: OrganizeRunResult) => void) => listen<OrganizeRunResult>("organize://run_done", cb),
  onRunError: (cb: (e: { code: string; message: string }) => void) => listen("organize://run_error", cb),
};

// ---- mock ----

const KB = 1024;
const MB = 1024 ** 2;
const ROOT = "/Users/you/Documents";

const entry = (name: string, bytes: number, unit: OrganizeEntry["unit"] = "file", kind: OrganizeEntry["kind"] = "document"): OrganizeEntry => ({
  path: `${ROOT}/${name}`,
  name,
  unit,
  kind,
  bytes,
  mtime: { secs_since_epoch: 1_791_000_000, nanos_since_epoch: 0 },
});

const group = (id: string, name: string, source: OrganizeGroup["source"], confidence: OrganizeGroup["confidence"], entries: OrganizeEntry[]): OrganizeGroup => ({
  id,
  name,
  source,
  confidence,
  entries,
  bytes: entries.reduce((a, e) => a + e.bytes, 0),
});

const mockAnalysis = (useAi: boolean): OrganizeAnalysis => ({
  plan: {
    root: ROOT,
    groups: [
      group("g1", "Dokumen", "existing", "sure", [entry("Dokumen", 40 * MB, "folder", "folder"), entry("surat.docx", 80 * KB), entry("cv.pdf", 300 * KB, "file", "pdf")]),
      group("g2", "Proyek", "unit", "sure", [entry("sopwer-hrms", 1.2 * 1024 * MB, "project", "folder"), entry("landing-page", 340 * MB, "project", "folder")]),
      group("g3", useAi ? "Tagihan" : "Invoice", useAi ? "ai" : "prefix", "check", [entry("Invoice-2024-01.pdf", 120 * KB, "file", "pdf"), entry("Invoice-2024-02.pdf", 118 * KB, "file", "pdf"), entry("invoice March.pdf", 99 * KB, "file", "pdf")]),
      group("g4", "Laporan Keuangan", "prefix", "sure", [entry("Laporan Keuangan Q1.xlsx", 2 * MB, "file", "spreadsheet"), entry("Laporan Keuangan Q2.xlsx", 2 * MB, "file", "spreadsheet"), entry("Laporan Keuangan Q3.xlsx", 2 * MB, "file", "spreadsheet")]),
      group("g5", "Gambar", "kind", "sure", [entry("IMG_4021.HEIC", 3 * MB, "file", "image"), entry("screenshot.png", 900 * KB, "file", "image")]),
    ],
  },
  unreadable: [],
  duplicate_groups: 2,
  duplicate_files: 5,
  duplicate_bytes: 48 * MB,
  ai_used: useAi,
  ai_note: null,
});

export const organizeMock = {
  tickMs: 15,
  analysis: null as OrganizeAnalysis | null,
  lastSelection: null as OrganizeSelection | null,
  runs: [] as OrganizeRun[],
  reset() {
    this.tickMs = 15;
    this.analysis = null;
    this.lastSelection = null;
    this.runs = [];
  },
};

const mockInvoke = async <T,>(cmd: string, args?: Record<string, unknown>): Promise<T> => {
  switch (cmd) {
    case "organize_targets":
      return [
        { key: "documents", path: ROOT },
        { key: "desktop", path: "/Users/you/Desktop" },
        { key: "downloads", path: "/Users/you/Downloads" },
      ] as T;
    case "organize_status":
      return { analysing: false, applying: false, has_plan: organizeMock.analysis !== null } as T;
    case "organize_plan":
      return organizeMock.analysis as T;
    case "organize_history":
      return organizeMock.runs as T;
    case "organize_analyze_start": {
      const useAi = Boolean(args?.useAi);
      const stages: OrganizeAnalyseProgress["stage"][] = useAi ? ["reading", "grouping", "ai", "duplicates"] : ["reading", "grouping", "duplicates"];
      let i = 0;
      const tick = () => {
        if (i < stages.length) {
          emit<OrganizeAnalyseProgress>("organize://progress", { stage: stages[i]!, elapsed_secs: i });
          i += 1;
          setTimeout(tick, organizeMock.tickMs);
        } else {
          organizeMock.analysis = mockAnalysis(useAi);
          emit("organize://plan", organizeMock.analysis);
        }
      };
      setTimeout(tick, organizeMock.tickMs);
      return undefined as T;
    }
    case "organize_apply_start": {
      const selection = args?.selection as OrganizeSelection;
      organizeMock.lastSelection = selection;
      const groups = organizeMock.analysis?.plan.groups.filter((g) => selection.group_ids.includes(g.id)) ?? [];
      const moves = groups.flatMap((g) => {
        const name = selection.renames.find(([id]) => id === g.id)?.[1] ?? g.name;
        return g.entries.filter((e) => e.path !== `${ROOT}/${g.name}` && !selection.excluded.includes(e.path)).map((e) => ({ from: e.path, to: `${ROOT}/${name}/${e.name}`, bytes: e.bytes, len: e.bytes }));
      });
      let i = 0;
      const total = Math.max(1, moves.length);
      const tick = () => {
        i += 1;
        emit<OrganizeRunProgress>("organize://run_progress", { items_done: Math.min(i, total), items_total: total, fraction: Math.min(1, i / total), elapsed_secs: i, eta: { kind: "about", secs: Math.max(0, total - i), nanos: 0 } });
        if (i >= total) {
          const session: OrganizeRun = { id: `run-${organizeMock.runs.length + 1}`, root: ROOT, started_at_unix: 1_792_000_000, finished_at_unix: 1_792_000_010, moves, created_dirs: [], moved: moves.length, skipped: 0, failed: 0, undone: false };
          organizeMock.runs = [session, ...organizeMock.runs];
          organizeMock.analysis = null;
          emit<OrganizeRunResult>("organize://run_done", { session, failures: [] });
        } else setTimeout(tick, organizeMock.tickMs);
      };
      setTimeout(tick, organizeMock.tickMs);
      return undefined as T;
    }
    case "organize_undo": {
      const run = organizeMock.runs.find((r) => r.id === args?.id);
      if (!run) throw { code: "io", message: "no such session" };
      run.undone = true;
      return { restored: run.moved, skipped: 0, failed: 0 } as T;
    }
    default:
      throw { code: "io", message: `unknown command ${cmd}` };
  }
};
