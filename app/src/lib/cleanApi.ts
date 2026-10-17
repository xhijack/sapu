import { isTauri } from "./api";
import type { CleanCheckProgress, CleanCheckStatus, CleanEvaluation, CleanRunProgress, CleanRunReport, CleanRunStatus, Profile, RunStep } from "./types";

type Unlisten = () => void;

async function listen<T>(event: string, cb: (payload: T) => void): Promise<Unlisten> {
  if (isTauri()) {
    const { listen } = await import("@tauri-apps/api/event");
    return listen<T>(event, (e) => cb(e.payload));
  }
  const handler = (e: Event) => cb((e as CustomEvent<T>).detail);
  bus.addEventListener(event, handler);
  return () => bus.removeEventListener(event, handler);
}
const bus = new EventTarget();
const emit = <T,>(event: string, detail: T) => bus.dispatchEvent(new CustomEvent(event, { detail }));

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<T>(cmd, args);
  }
  return mockInvoke<T>(cmd, args);
}

export const cleanApi = {
  /** Starts a background check; listen for `onDone`. */
  start: (profile: Profile) => invoke<void>("clean_check_start", { profile }),
  status: () => invoke<CleanCheckStatus>("clean_check_status"),
  last: () => invoke<CleanEvaluation | null>("clean_last"),
  onProgress: (cb: (p: CleanCheckProgress) => void) => listen<CleanCheckProgress>("clean://progress", cb),
  onDone: (cb: (ev: CleanEvaluation) => void) => listen<CleanEvaluation>("clean://done", cb),
  onError: (cb: (e: { code: string; message: string }) => void) => listen("clean://error", cb),
  /** Starts a clean run in the background; listen for `onRunDone`. */
  run: (ruleIds: string[], artefacts: string[], apply: boolean, permanentOk: boolean) =>
    invoke<void>("clean_run_start", { ruleIds, artefacts, apply, permanentOk }),
  runStatus: () => invoke<CleanRunStatus>("clean_run_status"),
  runLast: () => invoke<CleanRunReport | null>("clean_run_last"),
  onRunProgress: (cb: (p: CleanRunProgress) => void) => listen<CleanRunProgress>("clean://run_progress", cb),
  onRunDone: (cb: (r: CleanRunReport) => void) => listen<CleanRunReport>("clean://run_done", cb),
  onRunError: (cb: (e: { code: string; message: string }) => void) => listen("clean://run_error", cb),
  /** Convenience for flows that just want the report (Care). */
  runAndWait: async (ruleIds: string[], artefacts: string[], apply: boolean, permanentOk: boolean): Promise<CleanRunReport> => {
    const done = new Promise<CleanRunReport>((resolve, reject) => {
      const subs = [cleanApi.onRunDone((r) => { cleanup(); resolve(r); }), cleanApi.onRunError((e) => { cleanup(); reject(e); })];
      const cleanup = () => void Promise.all(subs).then((fns) => fns.forEach((f) => f()));
    });
    await cleanApi.run(ruleIds, artefacts, apply, permanentOk);
    return done;
  },
};

const GB = 1024 ** 3;
const MB = 1024 ** 2;

export const cleanMock = {
  lastExecute: null as { ruleIds: string[]; artefacts?: string[]; apply: boolean; permanentOk: boolean } | null,
  /** Tests start with a finished check so the list renders at once; `?fresh=1` in the browser starts empty. */
  cached: typeof location === "undefined" || !location.search.includes("fresh"),
  ticks: typeof location !== "undefined" && location.search.includes("slow") ? 12 : 3,
  tickMs: typeof location !== "undefined" && location.search.includes("slow") ? 700 : 15,
  running: false,
  runRunning: false,
  lastRun: null as CleanRunReport | null,
  lastEvaluation: null as CleanEvaluation | null,
  reset() {
    this.runRunning = false;
    this.lastRun = null;
    this.lastExecute = null;
    this.cached = true;
    this.ticks = 3;
    this.tickMs = 15;
    this.running = false;
    this.lastEvaluation = null;
  },
};

const rule = (id: string, name: string, risk: "green" | "yellow", group: string, permanent = false) => ({
  id,
  name: { id: name, en: name },
  description: { id: `Keterangan ${name}`, en: `About ${name}` },
  risk,
  group,
  permanent,
});

const mockEvaluation = (profile: Profile): CleanEvaluation => {
  const rules = [
    rule("macos-browser-caches", "Cache browser", "green", "browser"),
    rule("macos-old-logs", "Log lama", "green", "logs"),
    rule("macos-app-caches", "File sementara aplikasi", "yellow", "system_temp"),
    rule("macos-trash", "Kosongkan Trash", "yellow", "trash", true),
    rule("docker-images-dangling", "Image Docker tak terpakai", "green", "docker"),
    rule("docker-volumes-orphan", "Volume Docker yatim", "yellow", "docker"),
  ];
  const item = (p: string, size: number) => ({ path: `/Users/you/${p}`, size, is_dir: true, mtime: null });
  const findings = [
    { rule_id: "macos-browser-caches", risk: "green" as const, profile: "general" as const, group: "browser", permanent: false, scope: "/Users/you/Library/Caches", items: [item("Library/Caches/Google/Chrome/Default", 2.3 * GB)], bytes: 2.3 * GB, skipped_protected: 0 },
    { rule_id: "macos-browser-caches", risk: "green" as const, profile: "general" as const, group: "browser", permanent: false, scope: "/Users/you/Library/Caches", items: [item("Library/Caches/Firefox/Profiles/abc/cache2", 145 * MB)], bytes: 145 * MB, skipped_protected: 0 },
    { rule_id: "macos-old-logs", risk: "green" as const, profile: "general" as const, group: "logs", permanent: false, scope: "/Users/you/Library/Logs", items: [item("Library/Logs/old.log", 117 * MB)], bytes: 117 * MB, skipped_protected: 1 },
    { rule_id: "macos-app-caches", risk: "yellow" as const, profile: "general" as const, group: "system_temp", permanent: false, scope: "/Users/you/Library/Caches", items: [item("Library/Caches/com.spotify.client", 17.6 * GB)], bytes: 17.6 * GB, skipped_protected: 0 },
    { rule_id: "macos-trash", risk: "yellow" as const, profile: "general" as const, group: "trash", permanent: true, scope: "/Users/you/.Trash", items: [item(".Trash/old.zip", 900 * MB)], bytes: 900 * MB, skipped_protected: 0 },
  ];
  const commands =
    profile === "developer"
      ? [
          { rule_id: "docker-images-dangling", risk: "green" as const, profile: "developer" as const, group: "docker", command_line: "docker image prune -f", available: true, confirm_separately: false, estimate_bytes: 16.3 * GB },
          { rule_id: "docker-volumes-orphan", risk: "yellow" as const, profile: "developer" as const, group: "docker", command_line: "docker volume prune -f", available: true, confirm_separately: true, estimate_bytes: null },
        ]
      : [];
  const projects =
    profile === "developer"
      ? [
          {
            root: "/Users/you/dev/landing-page-2024",
            name: "landing-page-2024",
            ecosystem: "node" as const,
            last_activity: null,
            idle_days: 214,
            bytes: 1.8 * GB,
            rebuild: ["pnpm install", "pnpm run build"],
            artefacts: [
              { path: "/Users/you/dev/landing-page-2024/node_modules", kind: "node_modules", bytes: 1.4 * GB, gitignored: true, tracked: false, risk: "green" as const },
              { path: "/Users/you/dev/landing-page-2024/.next", kind: ".next", bytes: 400 * MB, gitignored: true, tracked: false, risk: "green" as const },
            ],
          },
          {
            root: "/Users/you/dev/api",
            name: "api",
            ecosystem: "rust" as const,
            last_activity: null,
            idle_days: 3,
            bytes: 6.2 * GB,
            rebuild: ["cargo build"],
            artefacts: [{ path: "/Users/you/dev/api/target", kind: "target", bytes: 6.2 * GB, gitignored: true, tracked: null, risk: "yellow" as const }],
          },
        ]
      : [];
  return {
    report: { findings, commands, projects },
    rules,
    preselected: ["macos-browser-caches", "macos-old-logs", ...(profile === "developer" ? ["docker-images-dangling"] : [])],
    preselected_artefacts: projects.flatMap((p) => p.artefacts.filter((a) => a.risk === "green").map((a) => a.path)),
    profile,
    finished_at_unix: 1_792_000_000,
  };
};

const mockInvoke = async <T,>(cmd: string, args?: Record<string, unknown>): Promise<T> => {
  switch (cmd) {
    case "clean_check_status":
      return { running: cleanMock.running, has_result: cleanMock.cached || cleanMock.lastEvaluation !== null, finished_at_unix: cleanMock.cached ? 1_792_000_000 : (cleanMock.lastEvaluation?.finished_at_unix ?? null) } as T;
    case "clean_last":
      if (cleanMock.lastEvaluation) return cleanMock.lastEvaluation as T;
      return (cleanMock.cached ? mockEvaluation("general") : null) as T;
    case "clean_check_start": {
      const profile = args?.profile as Profile;
      cleanMock.running = true;
      let i = 0;
      const step = () => {
        i += 1;
        emit<CleanCheckProgress>("clean://progress", { stage: i === 1 ? "projects" : "rules", done: i, total: cleanMock.ticks, fraction: i === 1 ? null : i / cleanMock.ticks, elapsed_secs: i, eta: i === 1 ? { kind: "indeterminate" } : { kind: "about", secs: (cleanMock.ticks - i) * 10, nanos: 0 } });
        if (i >= cleanMock.ticks) {
          cleanMock.running = false;
          cleanMock.lastEvaluation = { ...mockEvaluation(profile), finished_at_unix: 1_792_000_500 };
          emit("clean://done", cleanMock.lastEvaluation);
        } else setTimeout(step, cleanMock.tickMs);
      };
      setTimeout(step, cleanMock.tickMs);
      return undefined as T;
    }
    case "clean_run_status":
      return { running: cleanMock.runRunning, has_result: cleanMock.lastRun !== null } as T;
    case "clean_run_last":
      return cleanMock.lastRun as T;
    case "clean_run_start": {
      const ruleIds = args?.ruleIds as string[];
      const artefacts = (args?.artefacts as string[]) ?? [];
      const apply = args?.apply as boolean;
      const permanentOk = args?.permanentOk as boolean;
      cleanMock.lastExecute = { ruleIds, artefacts, apply, permanentOk };
      const ev = mockEvaluation("developer");
      const projectOutcomes = ev.report.projects
        .filter((p) => p.artefacts.some((a) => artefacts.includes(a.path)))
        .map((p) => {
          const chosen = p.artefacts.filter((a) => artefacts.includes(a.path));
          const b = chosen.reduce((n, a) => n + a.bytes, 0);
          return { kind: "project" as const, root: p.root, session: { id: "p", started_at_unix: 0, finished_at_unix: 0, dry_run: !apply, disposal: "trash" as const, scope: p.root, entries: [], bytes_freed: apply ? b : 0, disposed: chosen.length, skipped: 0, failed: 0 } };
        });
      const outcomes: import("./types").RuleOutcome[] = ruleIds.map((id) => {
        const f = ev.report.findings.find((x) => x.rule_id === id);
        if (f?.permanent && !permanentOk) return { kind: "skipped" as const, rule_id: id, reason: "permanent_confirmation_required" };
        if (f) {
          return {
            kind: "files" as const,
            rule_id: id,
            session: { id: "s", started_at_unix: 0, finished_at_unix: 0, dry_run: !apply, disposal: "trash" as const, scope: f.scope, entries: [], bytes_freed: apply ? f.bytes : 0, disposed: f.items.length, skipped: 0, failed: 0 },
          };
        }
        return { kind: "command" as const, rule_id: id, outcome: { command_line: "docker image prune -f", success: true, stdout: "", stderr: "" } };
      });
      const bytes =
        ruleIds.reduce((a, id) => a + ev.report.findings.filter((f) => f.rule_id === id && (!f.permanent || permanentOk)).reduce((b, f) => b + f.bytes, 0), 0) +
        projectOutcomes.reduce((a, o) => a + o.session.bytes_freed, 0);
      const all = [...outcomes, ...projectOutcomes];
      const report: CleanRunReport = { dry_run: !apply, outcomes: all, bytes_freed: apply ? bytes : 0, disposed: all.length, skipped: 0, failed: 0 };
      // Simulate a run: one step per outcome, then done.
      cleanMock.runRunning = true;
      const steps: RunStep[] = all.map((o) => (o.kind === "command" ? { kind: "command", rule_id: o.rule_id, command_line: o.outcome.command_line } : { kind: "rule", rule_id: o.kind === "project" ? o.root : o.rule_id, items: 3 }));
      let i = 0;
      const tick = () => {
        i += 1;
        emit<CleanRunProgress>("clean://run_progress", { items_done: i, items_total: steps.length, bytes_done: (bytes * i) / Math.max(1, steps.length), bytes_total: bytes, fraction: i / Math.max(1, steps.length), step: steps[Math.min(i, steps.length) - 1] ?? null, elapsed_secs: i, eta: { kind: "about", secs: (steps.length - i) * 2, nanos: 0 } });
        if (i >= steps.length) {
          cleanMock.runRunning = false;
          cleanMock.lastRun = report;
          emit("clean://run_done", report);
        } else setTimeout(tick, cleanMock.tickMs);
      };
      setTimeout(tick, cleanMock.tickMs);
      return undefined as T;
    }
    default:
      throw { code: "unknown_command", message: cmd };
  }
};
