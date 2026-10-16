// Typed wrappers around Tauri commands. Outside Tauri (vitest, plain
// browser) an in-memory mock is used so the UI can be developed and tested
// without the native shell. No business logic here.

import type {
  AppInfo,
  CleanSession,
  CommandError,
  DiskAccess,
  HistoryTotals,
  OpKind,
  ProtectedEntry,
  RestoreReport,
  Settings,
  SystemSnapshot,
} from "./types";

type Invoke = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

export const isTauri = () => typeof window !== "undefined" && !!window.__TAURI_INTERNALS__;

export const defaultSettings: Settings = {
  onboarding_done: false,
  profile: "general",
  locale: null,
  ai_mode: "off",
  theme: "system",
  notify_when_done: true,
};

/** Mock state, exported for tests. */
export const mock = {
  settings: { ...defaultSettings } as Settings,
  info: {
    version: "0.1.0-mock",
    os: "macos",
    full_disk_access: "denied",
    dev_tools: ["docker", "node"],
  } as AppInfo,
  history: [] as CleanSession[],
  protected: [
    { path: "/", protection: "Exact", reason: "filesystem-root" },
    { path: "/System", protection: "Subtree", reason: "os-system" },
    { path: "/Users/you/.ssh", protection: "Subtree", reason: "credentials" },
  ] as ProtectedEntry[],
  system: {
    cpu_percent: 23.5,
    cpu_per_core: [71, 40, 22, 18, 15, 12, 9, 8, 20, 20],
    cpu_count: 10,
    mem_total_bytes: 16 * 1024 ** 3,
    mem_used_bytes: 9.92 * 1024 ** 3,
    swap_used_bytes: 1.1 * 1024 ** 3,
    disks: [
      { name: "Macintosh HD", mount: "/", total_bytes: 494 * 1024 ** 3, available_bytes: 61 * 1024 ** 3, removable: false, file_system: "apfs" },
      { name: "Backup", mount: "/Volumes/Backup", total_bytes: 2 * 1024 ** 4, available_bytes: 0.8 * 1024 ** 4, removable: true, file_system: "exfat" },
    ],
    os_name: "macOS 26.0",
  } as SystemSnapshot,
  reset() {
    this.settings = { ...defaultSettings };
    this.history = [];
    this.info = { ...this.info, full_disk_access: "denied" };
  },
};

const mockInvoke: Invoke = async <T,>(cmd: string, args?: Record<string, unknown>) => {
  switch (cmd) {
    case "app_info":
      return mock.info as T;
    case "disk_access_status":
      return mock.info.full_disk_access as T;
    case "settings_get":
      return { ...mock.settings } as T;
    case "settings_set":
      mock.settings = { ...(args?.settings as Settings) };
      return undefined as T;
    case "protected_paths":
      return mock.protected as T;
    case "history_list":
      return mock.history as T;
    case "history_totals":
      return { sessions: 3, bytes_freed: 41.2 * 1024 ** 3, items_disposed: 1240, last_session_unix: 1_791_900_000 } as T;
    case "history_restore":
      if (mock.info.os === "macos") {
        throw { code: "restore_unsupported", message: "unsupported" } satisfies CommandError;
      }
      return { restored: [], not_found_in_trash: [] } as T;
    case "open_disk_access_settings":
    case "reveal_app_binary":
    case "relaunch_as_admin":
      return undefined as T;
    case "system_snapshot":
      return mock.system as T;
    case "estimate_duration":
      return ((args?.kind as OpKind) === "scan" ? 120 : 30) as T;
    default:
      throw { code: "unknown_command", message: cmd } satisfies CommandError;
  }
};

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<T>(cmd, args);
  }
  return mockInvoke<T>(cmd, args);
}

export const api = {
  appInfo: () => invoke<AppInfo>("app_info"),
  diskAccessStatus: () => invoke<DiskAccess>("disk_access_status"),
  settingsGet: () => invoke<Settings>("settings_get"),
  settingsSet: (settings: Settings) => invoke<void>("settings_set", { settings }),
  protectedPaths: () => invoke<ProtectedEntry[]>("protected_paths"),
  historyList: () => invoke<CleanSession[]>("history_list"),
  historyTotals: () => invoke<HistoryTotals>("history_totals"),
  historyRestore: (id: string) => invoke<RestoreReport>("history_restore", { id }),
  openDiskAccessSettings: () => invoke<void>("open_disk_access_settings"),
  revealAppBinary: () => invoke<void>("reveal_app_binary"),
  relaunchAsAdmin: () => invoke<void>("relaunch_as_admin"),
  systemSnapshot: () => invoke<SystemSnapshot>("system_snapshot"),
  estimateDuration: (kind: OpKind, amount: number) => invoke<number | null>("estimate_duration", { kind, amount: Math.round(amount) }),
};

export function isCommandError(e: unknown): e is CommandError {
  return typeof e === "object" && e !== null && "code" in e && "message" in e;
}
