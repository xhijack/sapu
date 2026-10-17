import { isTauri } from "./api";
import type {
  CareLogEntry, CareProgressPayload, CareReport } from "./types";

type Unlisten = () => void;

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<T>(cmd, args);
  }
  return mockInvoke<T>(cmd, args);
}

async function listen<T>(event: string, cb: (payload: T) => void): Promise<Unlisten> {
  if (isTauri()) {
    const { listen } = await import("@tauri-apps/api/event");
    return listen<T>(event, (e) => cb(e.payload));
  }
  const handler = (e: Event) => cb((e as CustomEvent<T>).detail);
  bus.addEventListener(event, handler);
  return () => bus.removeEventListener(event, handler);
}

export const careApi = {
  run: () => invoke<void>("care_run"),
  last: () => invoke<CareReport | null>("care_last"),
  history: (limit: number) => invoke<CareLogEntry[]>("care_history", { limit }),
  onProgress: (cb: (p: CareProgressPayload) => void) => listen<CareProgressPayload>("care://progress", cb),
  onDone: (cb: (r: CareReport) => void) => listen<CareReport>("care://done", cb),
  onError: (cb: (e: { code: string; message: string }) => void) => listen("care://error", cb),
};

// ---- mock ----
const bus = new EventTarget();
const emit = <T,>(event: string, detail: T) => bus.dispatchEvent(new CustomEvent(event, { detail }));
const GB = 1024 ** 3;

export const careMock = {
  tickMs: 20,
  last: null as CareReport | null,
  reset() {
    this.last = null;
    this.tickMs = 20;
  },
};

const mockReport = (): CareReport => ({
  score: {
    score: 58,
    components: [
      { key: "free_space", points: 20, max: 40 },
      { key: "safe_clutter", points: 12, max: 30 },
      { key: "review_clutter", points: 11, max: 15 },
      { key: "stale_projects", points: 15, max: 15 },
    ],
    free_pct: 12,
    safe_bytes: 22.6 * GB,
    review_bytes: 39.5 * GB,
    stale_project_bytes: 1.8 * GB,
    stale_projects: 1,
    recommendations: [
      { kind: "clean_safe", bytes: 22.6 * GB },
      { kind: "stale_projects", count: 1, bytes: 1.8 * GB },
      { kind: "review_clutter", bytes: 39.5 * GB },
      { kind: "largest_folder", path: "/Users/you/dev", bytes: 61.7 * GB },
    ],
  },
  scan: {
    root: "/Users/you",
    diff: null,
    disk_total_bytes: 494 * GB,
    disk_free_bytes: 61 * GB,
    started_at_unix: 0,
    finished_at_unix: 1_792_000_100,
    cancelled: false,
    total_bytes: 212.4 * GB,
    file_count: 1_204_311,
    dir_count: 88_120,
    denied_count: 3,
    denied_sample: [],
    categories: [],
  },
  preselected: ["macos-browser-caches", "macos-old-logs"],
  preselected_artefacts: ["/Users/you/dev/landing-page-2024/node_modules"],
  finished_at_unix: 1_792_000_100,
});

// eslint-disable-next-line @typescript-eslint/no-unused-vars
const mockInvoke = async <T,>(cmd: string, _args?: Record<string, unknown>): Promise<T> => {
  switch (cmd) {
    case "care_last":
      return careMock.last as T;
    case "care_history":
      return [
        { finished_at_unix: 1_791_900_000, score: 72, free_pct: 14, safe_bytes: 12.1 * GB },
        { finished_at_unix: 1_790_700_000, score: 48, free_pct: 6, safe_bytes: 31.4 * GB },
      ] as T;
    case "care_run": {
      let i = 0;
      const step = () => {
        i += 1;
        emit<CareProgressPayload>("care://progress", {
          stage: i <= 3 ? 1 : 2,
          total: 2,
          fraction: i <= 3 ? i / 4 : null,
          items_done: i * 300_000,
          bytes_done: i * 50 * GB,
          elapsed_secs: i * 20,
          eta: i <= 3 ? { kind: "about", secs: (4 - i) * 30, nanos: 0 } : { kind: "indeterminate" },
        });
        if (i >= 5) {
          careMock.last = mockReport();
          emit("care://done", careMock.last);
        } else setTimeout(step, careMock.tickMs);
      };
      setTimeout(step, careMock.tickMs);
      return undefined as T;
    }
    default:
      throw { code: "unknown_command", message: cmd };
  }
};
