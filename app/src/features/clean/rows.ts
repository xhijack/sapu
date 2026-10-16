import type { CleanEvaluation, Risk } from "@/lib/types";

/** One row per rule: core reports one finding per match. */
export interface RuleRow {
  id: string;
  name: string;
  description: string;
  risk: Risk;
  group: string;
  permanent: boolean;
  bytes: number | null;
  items: number;
  skippedProtected: number;
  commandLine: string | null;
  available: boolean;
  confirmSeparately: boolean;
  /** Set for project artefacts: the folder path sent to the backend. */
  artefactPath?: string;
}

export const ARTEFACT_PREFIX = "artefact:";

export function buildRows(ev: CleanEvaluation, lang: string): RuleRow[] {
  const l = (x: { id: string; en: string }) => (lang.startsWith("en") ? x.en : x.id);
  const rows = new Map<string, RuleRow>();
  for (const f of ev.report.findings) {
    const rule = ev.rules.find((r) => r.id === f.rule_id);
    const row = rows.get(f.rule_id) ?? {
      id: f.rule_id,
      name: rule ? l(rule.name) : f.rule_id,
      description: rule ? l(rule.description) : "",
      risk: f.risk,
      group: f.group,
      permanent: f.permanent,
      bytes: 0,
      items: 0,
      skippedProtected: 0,
      commandLine: null,
      available: true,
      confirmSeparately: false,
    };
    row.bytes = (row.bytes ?? 0) + f.bytes;
    row.items += f.items.length;
    row.skippedProtected += f.skipped_protected;
    rows.set(f.rule_id, row);
  }
  for (const c of ev.report.commands) {
    const rule = ev.rules.find((r) => r.id === c.rule_id);
    rows.set(c.rule_id, {
      id: c.rule_id,
      name: rule ? l(rule.name) : c.rule_id,
      description: rule ? l(rule.description) : "",
      risk: c.risk,
      group: c.group,
      permanent: false,
      bytes: c.estimate_bytes,
      items: 0,
      skippedProtected: 0,
      commandLine: c.command_line,
      available: c.available,
      confirmSeparately: c.confirm_separately,
    });
  }
  // Green first, then by size.
  return [...rows.values()].sort((a, b) => (a.risk === b.risk ? (b.bytes ?? 0) - (a.bytes ?? 0) : a.risk === "green" ? -1 : 1));
}
