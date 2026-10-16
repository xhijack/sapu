import { isTauri } from "./api";
import type { DedupeProgressPayload, DedupeRunReport, DedupeSummary, DuplicateGroup, FilePreview, FolderPair, KeepStrategy } from "./types";

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

export const dedupeApi = {
  start: (root: string, minSize?: number) => invoke<void>("dedupe_start", { root, minSize: minSize ?? null }),
  cancel: () => invoke<void>("dedupe_cancel"),
  summary: () => invoke<DedupeSummary>("dedupe_summary"),
  groups: (offset: number, limit: number) => invoke<DuplicateGroup[]>("dedupe_groups", { offset, limit }),
  folderPairs: (limit: number) => invoke<FolderPair[]>("dedupe_folder_pairs", { limit }),
  execute: (strategy: KeepStrategy, keeps: Record<number, string>, groupIds: number[], apply: boolean, hardlink = false) =>
    invoke<DedupeRunReport>("dedupe_execute", { strategy, keeps, groupIds, apply, hardlink }),
  preview: (path: string) => invoke<FilePreview>("file_preview", { path }),
  onProgress: (cb: (p: DedupeProgressPayload) => void) => listen<DedupeProgressPayload>("dedupe://progress", cb),
  onDone: (cb: (s: DedupeSummary) => void) => listen<DedupeSummary>("dedupe://done", cb),
  onError: (cb: (e: { code: string; message: string }) => void) => listen("dedupe://error", cb),
};

// ---- mock ----
const bus = new EventTarget();
const emit = <T,>(event: string, detail: T) => bus.dispatchEvent(new CustomEvent(event, { detail }));
const MB = 1024 ** 2;

export const dedupeMock = {
  tickMs: 20,
  lastExecute: null as { strategy: KeepStrategy; keeps: Record<number, string>; groupIds: number[]; apply: boolean; hardlink?: boolean } | null,
  lastMinSize: null as number | null,
  summary: null as DedupeSummary | null,
  reset() {
    this.lastExecute = null;
    this.lastMinSize = null;
    this.summary = null;
    this.tickMs = 20;
  },
};

const mt = (daysAgo: number) => ({ secs_since_epoch: 1_792_000_000 - daysAgo * 86_400, nanos_since_epoch: 0 });
const mockGroups = (root: string): DuplicateGroup[] => [
  {
    id: 1,
    size: 4.2 * MB,
    hash: "a1",
    wasted_bytes: 8.4 * MB,
    files: [
      { path: `${root}/Pictures/liburan/IMG_2041.jpg`, mtime: mt(400), name_score: 0 },
      { path: `${root}/Downloads/IMG_2041 (1).jpg`, mtime: mt(30), name_score: -15 },
      { path: `${root}/Desktop/IMG_2041 copy.jpg`, mtime: mt(10), name_score: -10 },
    ],
  },
  {
    id: 2,
    size: 1.1 * MB,
    hash: "b2",
    wasted_bytes: 1.1 * MB,
    files: [
      { path: `${root}/Documents/skripsi-final.pdf`, mtime: mt(200), name_score: 0 },
      { path: `${root}/Downloads/skripsi-final.pdf`, mtime: mt(190), name_score: 0 },
    ],
  },
];

const mockSummary = (root: string): DedupeSummary => ({
  root,
  files_seen: 48_210,
  candidates: 3_120,
  groups: 2,
  duplicate_files: 3,
  wasted_bytes: 9.5 * MB,
  cancelled: false,
  stage_reached: 3,
});

const mockInvoke = async <T,>(cmd: string, args?: Record<string, unknown>): Promise<T> => {
  switch (cmd) {
    case "dedupe_start": {
      const root = String(args?.root);
      dedupeMock.lastMinSize = (args?.minSize as number | null) ?? null;
      let i = 0;
      const step = () => {
        i += 1;
        const stage = i <= 2 ? 1 : i <= 4 ? 2 : 3;
        emit<DedupeProgressPayload>("dedupe://progress", {
          fraction: stage === 3 ? (i - 4) / 3 : null,
          items_done: i * 1000,
          items_total: stage === 1 ? null : 6000,
          bytes_done: 0,
          bytes_total: null,
          elapsed_secs: i,
          eta: stage === 3 ? { kind: "about", secs: 60, nanos: 0 } : { kind: "computing" },
          stage: { current: stage, total: 3 },
        });
        if (i >= 6) {
          dedupeMock.summary = mockSummary(root);
          emit("dedupe://done", dedupeMock.summary);
        } else setTimeout(step, dedupeMock.tickMs);
      };
      setTimeout(step, dedupeMock.tickMs);
      return undefined as T;
    }
    case "dedupe_cancel":
      return undefined as T;
    case "dedupe_summary":
      if (!dedupeMock.summary) throw { code: "no_scan", message: "none" };
      return dedupeMock.summary as T;
    case "dedupe_groups":
      return mockGroups(dedupeMock.summary?.root ?? "/Users/you").slice(args?.offset as number, (args?.offset as number) + (args?.limit as number)) as T;
    case "dedupe_folder_pairs":
      return [
        { a: "/Users/you/Downloads", b: "/Users/you/Pictures/liburan", files: 1, bytes: 4.2 * MB },
        { a: "/Users/you/Documents", b: "/Users/you/Downloads", files: 1, bytes: 1.1 * MB },
      ] as T;
    case "dedupe_execute": {
      const a = args as { strategy: KeepStrategy; keeps: Record<number, string>; groupIds: number[]; apply: boolean; hardlink: boolean };
      dedupeMock.lastExecute = a;
      const groups = mockGroups(dedupeMock.summary?.root ?? "/Users/you").filter((g) => a.groupIds.length === 0 || a.groupIds.includes(g.id));
      const removed = groups.reduce((n, g) => n + g.files.length - 1, 0);
      const bytes = groups.reduce((n, g) => n + g.wasted_bytes, 0);
      return {
        dry_run: !a.apply,
        outcomes: groups.map((g) => ({ kind: "done" as const, group_id: g.id, linked: a.hardlink ? g.files.length - 1 : 0, link_failed: [], session: { id: "s", started_at_unix: 0, finished_at_unix: 0, dry_run: !a.apply, disposal: "trash" as const, scope: "/", entries: [], bytes_freed: a.apply ? g.wasted_bytes : 0, disposed: g.files.length - 1, skipped: 0, failed: 0 } })),
        bytes_freed: a.apply ? bytes : 0,
        disposed: removed,
        skipped: 0,
        failed: 0,
      } as T;
    }
    case "file_preview":
      return (String(args?.path).endsWith(".jpg")
        ? { kind: "image", data_url: "data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='120' height='90'><rect width='120' height='90' fill='%23355c9e'/></svg>" }
        : { kind: "none" }) as T;
    default:
      throw { code: "unknown_command", message: cmd };
  }
};
