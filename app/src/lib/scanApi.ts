// Scan commands and events. Mocked outside Tauri with a simulated scan so
// the flow can be developed and tested in a browser / vitest.

import type {
  CleanSession,
  DirSummary,
  FileEntry,
  FileFilter,
  ScanProgressPayload,
  ScanStatus,
  ScanSummary,
  ScanTargets,
  TreeNode,
} from "./types";
import { isTauri } from "./api";

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
  mockBus.addEventListener(event, handler);
  return () => mockBus.removeEventListener(event, handler);
}

export const scanApi = {
  targets: () => invoke<ScanTargets>("scan_targets"),
  pickFolder: () => invoke<string | null>("pick_folder"),
  start: (root: string) => invoke<void>("scan_start", { root }),
  pause: () => invoke<void>("scan_pause"),
  resume: () => invoke<void>("scan_resume"),
  cancel: () => invoke<void>("scan_cancel"),
  status: () => invoke<ScanStatus>("scan_status"),
  summary: () => invoke<ScanSummary>("scan_summary"),
  topFiles: (filter: FileFilter, limit: number) => invoke<FileEntry[]>("scan_top_files", { filter, limit }),
  topDirs: (limit: number) => invoke<DirSummary[]>("scan_top_dirs", { limit }),
  subtree: (path: string | null, depth: number, limit: number) =>
    invoke<TreeNode | null>("scan_subtree", { path, depth, limit }),
  reveal: (path: string) => invoke<void>("reveal_path", { path }),
  trash: (paths: string[], apply: boolean) => invoke<CleanSession>("trash_paths", { paths, apply }),
  onProgress: (cb: (p: ScanProgressPayload) => void) => listen<ScanProgressPayload>("scan://progress", cb),
  onDone: (cb: (s: ScanSummary) => void) => listen<ScanSummary>("scan://done", cb),
  onError: (cb: (e: { code: string; message: string }) => void) => listen("scan://error", cb),
};

// ---- mock ----

const mockBus = new EventTarget();
const emit = <T,>(event: string, detail: T) => mockBus.dispatchEvent(new CustomEvent(event, { detail }));

const GB = 1024 ** 3;
const MB = 1024 ** 2;

export const scanMock = {
  tickMs: 30,
  ticks: 6,
  state: null as ScanStatus["state"],
  summary: null as ScanSummary | null,
  reset() {
    this.state = null;
    this.summary = null;
    this.tickMs = 30;
    this.ticks = 6;
  },
};

const mockSummary = (root: string): ScanSummary => ({
  root,
  started_at_unix: 1_792_000_000,
  finished_at_unix: 1_792_000_100,
  cancelled: scanMock.state === "cancelled",
  total_bytes: 212.4 * GB,
  file_count: 1_204_311,
  dir_count: 88_120,
  denied_count: 3,
  denied_sample: [`${root}/Library/Mail`, `${root}/Library/Safari`],
  categories: [
    { category: "video", bytes: 48.2 * GB, files: 312 },
    { category: "development", bytes: 61.7 * GB, files: 802_110 },
    { category: "photo", bytes: 27.9 * GB, files: 41_200 },
    { category: "document", bytes: 6.1 * GB, files: 9_300 },
    { category: "application", bytes: 22.4 * GB, files: 61_000 },
    { category: "system", bytes: 14.0 * GB, files: 120_000 },
    { category: "temporary", bytes: 19.3 * GB, files: 150_000 },
    { category: "other", bytes: 12.8 * GB, files: 20_389 },
  ].sort((a, b) => b.bytes - a.bytes) as ScanSummary["categories"],
});

const mockFiles = (root: string): FileEntry[] =>
  [
    ["Movies/zoom-rekaman-2025-03.mp4", 8.4 * GB, "video"],
    ["Downloads/Xcode_16.dmg", 7.1 * GB, "application"],
    ["Movies/liburan-bali.mov", 4.2 * GB, "video"],
    ["Downloads/ubuntu-24.04.iso", 3.8 * GB, "application"],
    ["Pictures/Photos Library.photoslibrary/database.db", 2.1 * GB, "photo"],
    ["Documents/skripsi-final-final(2).pdf", 420 * MB, "document"],
    ["dev/landing-page-2024/node_modules/.cache/webpack/0.pack", 310 * MB, "temporary"],
  ].map(([p, size, category], i) => ({
    path: `${root}/${p as string}`,
    size: size as number,
    mtime: { secs_since_epoch: 1_792_000_000 - i * 40 * 86_400, nanos_since_epoch: 0 },
    category: category as FileEntry["category"],
  }));

const mockDirs = (root: string): DirSummary[] =>
  [
    ["dev", 61.7 * GB, 802_110],
    ["Movies", 44.0 * GB, 212],
    ["Library/Caches", 19.3 * GB, 150_000],
    ["Pictures", 27.9 * GB, 41_200],
    ["dev/landing-page-2024/node_modules", 1.4 * GB, 90_000],
  ].map(([p, bytes, files]) => ({ path: `${root}/${p as string}`, bytes: bytes as number, files: files as number }));

const mockTree = (root: string): TreeNode => ({
  path: root,
  name: "Beranda",
  bytes: 212.4 * GB,
  files: 1_204_311,
  is_dir: true,
  category: null,
  children: [
    ...mockDirs(root)
      .slice(0, 4)
      .map((d) => ({ path: d.path, name: d.path.split("/").pop() ?? "", bytes: d.bytes, files: d.files, is_dir: true, category: null, children: [] })),
    ...mockFiles(root).slice(0, 2).map((f) => ({ path: f.path, name: f.path.split("/").pop() ?? "", bytes: f.size, files: 1, is_dir: false, category: f.category, children: [] })),
  ],
});

const mockInvoke = async <T,>(cmd: string, args?: Record<string, unknown>): Promise<T> => {
  switch (cmd) {
    case "scan_targets":
      return { home: "/Users/you", disk: "/" } as T;
    case "pick_folder":
      return "/Users/you/Downloads" as T;
    case "scan_start": {
      const root = String(args?.root);
      scanMock.state = "running";
      let i = 0;
      const total = 212.4 * GB;
      const step = () => {
        if (scanMock.state === "cancelled") {
          scanMock.summary = mockSummary(root);
          emit("scan://done", scanMock.summary);
          return;
        }
        if (scanMock.state === "paused") {
          setTimeout(step, scanMock.tickMs);
          return;
        }
        i += 1;
        const frac = Math.min(i / scanMock.ticks, 0.999);
        emit<ScanProgressPayload>("scan://progress", {
          fraction: frac,
          items_done: Math.floor(1_204_311 * frac),
          items_total: null,
          bytes_done: total * frac,
          bytes_total: total,
          elapsed_secs: i * 4,
          eta: i < 2 ? { kind: "computing" } : i < scanMock.ticks ? { kind: "about", secs: (scanMock.ticks - i) * 60, nanos: 0 } : { kind: "almost_done" },
        });
        if (i >= scanMock.ticks) {
          scanMock.state = null;
          scanMock.summary = mockSummary(root);
          emit("scan://done", scanMock.summary);
        } else {
          setTimeout(step, scanMock.tickMs);
        }
      };
      setTimeout(step, scanMock.tickMs);
      return undefined as T;
    }
    case "scan_pause":
      if (scanMock.state === "running") scanMock.state = "paused";
      return undefined as T;
    case "scan_resume":
      if (scanMock.state === "paused") scanMock.state = "running";
      return undefined as T;
    case "scan_cancel":
      scanMock.state = "cancelled";
      return undefined as T;
    case "scan_status":
      return { running: scanMock.state === "running" || scanMock.state === "paused", state: scanMock.state, has_result: !!scanMock.summary } as T;
    case "scan_summary":
      if (!scanMock.summary) throw { code: "no_scan", message: "no scan" };
      return scanMock.summary as T;
    case "scan_top_files": {
      const root = scanMock.summary?.root ?? "/Users/you";
      const f = args?.filter as FileFilter;
      return mockFiles(root)
        .filter((x) => x.size >= f.min_bytes && (f.category === null || x.category === f.category))
        .slice(0, args?.limit as number) as T;
    }
    case "scan_top_dirs":
      return mockDirs(scanMock.summary?.root ?? "/Users/you").sort((a, b) => b.bytes - a.bytes) as T;
    case "scan_subtree":
      return mockTree(scanMock.summary?.root ?? "/Users/you") as T;
    case "reveal_path":
      return undefined as T;
    case "trash_paths": {
      const paths = args?.paths as string[];
      const apply = args?.apply as boolean;
      return {
        id: "mock",
        started_at_unix: 0,
        finished_at_unix: 0,
        dry_run: !apply,
        disposal: "trash",
        scope: "/Users/you",
        entries: paths.map((p) => ({ path: p, size: 1024, is_dir: false, outcome: { status: apply ? "disposed" : "would_dispose" } })),
        bytes_freed: apply ? paths.length * 1024 : 0,
        disposed: paths.length,
        skipped: 0,
        failed: 0,
      } as T;
    }
    default:
      throw { code: "unknown_command", message: cmd };
  }
};
