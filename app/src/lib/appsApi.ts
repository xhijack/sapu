import { isTauri } from "./api";
import type { CleanSession, InstalledApp, Leftover } from "./types";

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<T>(cmd, args);
  }
  return mockInvoke<T>(cmd, args);
}

export const appsApi = {
  list: () => invoke<InstalledApp[]>("apps_list"),
  leftovers: (app: InstalledApp) => invoke<Leftover[]>("apps_leftovers", { app }),
  orphans: () => invoke<Leftover[]>("apps_orphans"),
  isRunning: (path: string) => invoke<boolean>("apps_is_running", { path }),
  uninstall: (app: InstalledApp | null, leftovers: string[], apply: boolean) => invoke<CleanSession>("apps_uninstall", { app, leftovers, apply }),
  icon: (path: string) => invoke<string>("app_icon", { path }),
};

// ---- mock ----
const GB = 1024 ** 3;
const MB = 1024 ** 2;

export const appsMock = {
  lastUninstall: null as { app: InstalledApp | null; leftovers: string[]; apply: boolean } | null,
  running: new Set<string>(["/Applications/Slack.app"]),
  reset() {
    this.lastUninstall = null;
  },
};

const apps: InstalledApp[] = [
  { name: "Xcode", bundle_id: "com.apple.dt.Xcode", version: "16.1", path: "/Applications/Xcode.app", bytes: 14.2 * GB, last_used: { secs_since_epoch: 1_791_000_000, nanos_since_epoch: 0 }, is_system: true, support: "native" },
  { name: "Docker", bundle_id: "com.docker.docker", version: "4.34", path: "/Applications/Docker.app", bytes: 2.1 * GB, last_used: { secs_since_epoch: 1_792_000_000, nanos_since_epoch: 0 }, is_system: false, support: "native" },
  { name: "Slack", bundle_id: "com.tinyspeck.slackmacgap", version: "4.41", path: "/Applications/Slack.app", bytes: 420 * MB, last_used: { secs_since_epoch: 1_792_000_000, nanos_since_epoch: 0 }, is_system: false, support: "native" },
  { name: "Zoom", bundle_id: "us.zoom.xos", version: "6.2", path: "/Applications/zoom.us.app", bytes: 310 * MB, last_used: { secs_since_epoch: 1_780_000_000, nanos_since_epoch: 0 }, is_system: false, support: "rosetta" },
];

const mockInvoke = async <T,>(cmd: string, args?: Record<string, unknown>): Promise<T> => {
  switch (cmd) {
    case "apps_list":
      return apps as T;
    case "apps_leftovers": {
      const a = args?.app as InstalledApp;
      if (a.name === "Zoom")
        return [
          { path: "/Users/you/Library/Application Support/zoom.us", kind: "support", bytes: 180 * MB },
          { path: "/Users/you/Library/Caches/us.zoom.xos", kind: "cache", bytes: 95 * MB },
          { path: "/Users/you/Library/Preferences/us.zoom.xos.plist", kind: "preferences", bytes: 12 * 1024 },
          { path: "/Users/you/Library/Logs/zoom.us", kind: "logs", bytes: 4 * MB },
        ] as T;
      return [] as T;
    }
    case "apps_orphans":
      return [
        { path: "/Users/you/Library/Application Support/com.oldvendor.editor", kind: "support", bytes: 640 * MB },
        { path: "/Users/you/Library/Caches/org.gone.player", kind: "cache", bytes: 120 * MB },
      ] as T;
    case "apps_is_running":
      return appsMock.running.has(String(args?.path)) as T;
    case "apps_uninstall": {
      const a = (args?.app as InstalledApp | null) ?? null;
      const leftovers = args?.leftovers as string[];
      const apply = args?.apply as boolean;
      if (a?.is_system) throw { code: "protected", message: "system" };
      if (apply && a && appsMock.running.has(a.path)) throw { code: "app_running", message: "running" };
      appsMock.lastUninstall = { app: a, leftovers, apply };
      const paths = [...(a ? [a.path] : []), ...leftovers];
      const bytes = (a?.bytes ?? 0) + leftovers.length * 50 * MB;
      return {
        id: "u",
        started_at_unix: 0,
        finished_at_unix: 0,
        dry_run: !apply,
        disposal: "trash",
        scope: "/",
        entries: paths.map((p) => ({ path: p, size: 50 * MB, is_dir: true, outcome: { status: apply ? "disposed" : "would_dispose" } })),
        bytes_freed: apply ? bytes : 0,
        disposed: paths.length,
        skipped: 0,
        failed: 0,
      } as T;
    }
    case "app_icon":
      return "" as T;
    default:
      throw { code: "unknown_command", message: cmd };
  }
};
