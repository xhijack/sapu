// Typed wrappers around Tauri commands. Outside Tauri (vitest, plain
// browser) an in-memory mock is used so the UI can be developed and tested
// without the native shell. No business logic here.

import type {
  AppInfo,
  CleanSession,
  CommandError,
  DiskAccess,
  ProtectedEntry,
  RestoreReport,
  Settings,
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
    case "history_restore":
      if (mock.info.os === "macos") {
        throw { code: "restore_unsupported", message: "unsupported" } satisfies CommandError;
      }
      return { restored: [], not_found_in_trash: [] } as T;
    case "open_disk_access_settings":
      return undefined as T;
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
  historyRestore: (id: string) => invoke<RestoreReport>("history_restore", { id }),
  openDiskAccessSettings: () => invoke<void>("open_disk_access_settings"),
};

export function isCommandError(e: unknown): e is CommandError {
  return typeof e === "object" && e !== null && "code" in e && "message" in e;
}
