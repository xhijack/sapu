import { isTauri } from "./api";
import type { MemoryProcess, MemorySnapshot, PurgeOutcome, QuitOutcome } from "./types";

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<T>(cmd, args);
  }
  return mockInvoke<T>(cmd, args);
}

export const memoryApi = {
  snapshot: () => invoke<MemorySnapshot>("memory_snapshot"),
  quitApps: (pids: number[]) => invoke<QuitOutcome[]>("memory_quit_apps", { pids }),
  purge: () => invoke<PurgeOutcome>("memory_purge"),
};

// ---- mock ----
const GB = 1024 ** 3;
const MB = 1024 ** 2;

const processes = (): MemoryProcess[] => [
  { pid: 300, name: "Google Chrome", memory_bytes: 4.2 * GB, process_count: 23, exe: "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", bundle_id: "com.google.Chrome", quittable: true },
  { pid: 150, name: "kernel_task", memory_bytes: 2.1 * GB, process_count: 1, exe: "/System/Library/Kernels/kernel", bundle_id: null, quittable: false },
  { pid: 400, name: "Slack", memory_bytes: 1.3 * GB, process_count: 5, exe: "/Applications/Slack.app/Contents/MacOS/Slack", bundle_id: "com.tinyspeck.slackmacgap", quittable: true },
  { pid: 410, name: "WindowServer", memory_bytes: 900 * MB, process_count: 1, exe: "/System/Library/PrivateFrameworks/SkyLight.framework/Resources/WindowServer", bundle_id: null, quittable: false },
  { pid: 600, name: "node", memory_bytes: 512 * MB, process_count: 1, exe: "/opt/homebrew/bin/node", bundle_id: null, quittable: true },
  { pid: 77, name: "Sapu", memory_bytes: 120 * MB, process_count: 1, exe: "/Applications/Sapu.app/Contents/MacOS/sapu", bundle_id: "id.sopwer.sapu", quittable: false },
];

const freshSnapshot = (): MemorySnapshot => ({
  pressure: { total_bytes: 16 * GB, used_bytes: 13.6 * GB, available_bytes: 2.4 * GB, used_pct: 85, swap_used_bytes: 1.1 * GB, level: "elevated" },
  processes: processes(),
});

export const memoryMock = {
  snapshot: freshSnapshot(),
  quitCalls: [] as number[][],
  purgeCalls: 0,
  purgeCancelled: false,
  reset() {
    this.snapshot = freshSnapshot();
    this.quitCalls = [];
    this.purgeCalls = 0;
    this.purgeCancelled = false;
  },
};

const levelOf = (frac: number) => (frac >= 0.9 ? "high" : frac >= 0.8 ? "elevated" : "normal");

const mockInvoke = async <T,>(cmd: string, args?: Record<string, unknown>): Promise<T> => {
  switch (cmd) {
    case "memory_snapshot":
      return memoryMock.snapshot as T;
    case "memory_quit_apps": {
      const pids = (args?.pids as number[] | undefined) ?? [];
      if (pids.length === 0) throw { code: "validation", message: "nothing selected" };
      memoryMock.quitCalls.push(pids);
      const out: QuitOutcome[] = pids.map((pid) => {
        const p = memoryMock.snapshot.processes.find((x) => x.pid === pid);
        if (!p || !p.quittable) return { pid, name: p?.name ?? "", method: "refused", requested: false, message: p ? "not a user app" : "already gone" };
        return { pid, name: p.name, method: p.bundle_id ? "apple_event" : "sigterm", requested: true, message: "" };
      });
      // The picked apps are gone in the next snapshot; the used figure drops with them.
      const quit = (p: MemoryProcess) => pids.includes(p.pid) && p.quittable;
      const freed = memoryMock.snapshot.processes.filter(quit).reduce((n, p) => n + p.memory_bytes, 0);
      const pressure = memoryMock.snapshot.pressure;
      const used = Math.max(0, pressure.used_bytes - freed);
      memoryMock.snapshot = {
        pressure: { ...pressure, used_bytes: used, available_bytes: pressure.total_bytes - used, used_pct: Math.round((used / pressure.total_bytes) * 100), level: levelOf(used / pressure.total_bytes) },
        processes: memoryMock.snapshot.processes.filter((p) => !quit(p)),
      };
      return out as T;
    }
    case "memory_purge": {
      memoryMock.purgeCalls += 1;
      const before = memoryMock.snapshot.pressure.used_bytes;
      if (memoryMock.purgeCancelled) return { ok: false, cancelled: true, used_before_bytes: before, used_after_bytes: before, message: "User canceled. (-128)" } as T;
      return { ok: true, cancelled: false, used_before_bytes: before, used_after_bytes: before - 1.2 * GB, message: "" } as T;
    }
    default:
      throw { code: "unknown_command", message: cmd };
  }
};
