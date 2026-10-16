import { isTauri } from "./api";
import type { CleanEvaluation, CleanRunReport, Profile } from "./types";

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri()) {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<T>(cmd, args);
  }
  return mockInvoke<T>(cmd, args);
}

export const cleanApi = {
  evaluate: (profile: Profile) => invoke<CleanEvaluation>("clean_evaluate", { profile }),
  execute: (ruleIds: string[], artefacts: string[], apply: boolean, permanentOk: boolean) =>
    invoke<CleanRunReport>("clean_execute", { ruleIds, artefacts, apply, permanentOk }),
};

const GB = 1024 ** 3;
const MB = 1024 ** 2;

export const cleanMock = {
  lastExecute: null as { ruleIds: string[]; artefacts?: string[]; apply: boolean; permanentOk: boolean } | null,
  reset() {
    this.lastExecute = null;
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
  };
};

const mockInvoke = async <T,>(cmd: string, args?: Record<string, unknown>): Promise<T> => {
  switch (cmd) {
    case "clean_evaluate":
      return mockEvaluation(args?.profile as Profile) as T;
    case "clean_execute": {
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
      return { dry_run: !apply, outcomes: all, bytes_freed: apply ? bytes : 0, disposed: all.length, skipped: 0, failed: 0 } as T;
    }
    default:
      throw { code: "unknown_command", message: cmd };
  }
};
