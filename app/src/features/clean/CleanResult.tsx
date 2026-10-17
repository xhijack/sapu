import { useTranslation } from "react-i18next";
import { AlertTriangle, CheckCircle2, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { UnreadablePlaces } from "./UnreadablePlaces";
import { formatBytes } from "@/lib/format";
import type { CleanRunReport } from "@/lib/types";
import type { RuleRow } from "./rows";

interface Props {
  result: CleanRunReport;
  rows: RuleRow[];
  onSeeHistory: () => void;
  onBack: () => void;
  /** The "empty the Trash" rule, when it exists and was not part of this run. */
  trashRule?: RuleRow | null;
  onEmptyTrash?: () => void;
}

export function CleanResult({ result, rows, onSeeHistory, onBack, trashRule = null, onEmptyTrash }: Props) {
  const { t, i18n } = useTranslation();
  const name = (id: string) => rows.find((r) => r.id === id)?.name ?? id;
  const label = (o: CleanRunReport["outcomes"][number]) =>
    o.kind === "project" ? t("clean.outcome_project", { name: o.root.split("/").pop() ?? o.root }) : name(o.rule_id);
  // Moving to the Trash does not free space yet; only permanent removal does.
  const sessions = result.outcomes.flatMap((o) => (o.kind === "files" || o.kind === "project" ? [o.session] : []));
  const sum = (kind: "trash" | "permanent", k: "bytes_freed" | "disposed") => sessions.filter((x) => x.disposal === kind).reduce((a, x) => a + x[k], 0);
  const unreadable = result.unreadable ?? [];
  const partial = !result.dry_run && (result.failed > 0 || unreadable.length > 0);
  const lang = i18n.language;
  return (
    <div className="grid max-w-[1280px] gap-6">
      <h1 className="flex items-center gap-2 text-2xl font-semibold">
        {partial ? <AlertTriangle aria-hidden className="h-6 w-6 text-risk-review-fg" /> : <CheckCircle2 aria-hidden className="h-6 w-6 text-risk-safe-fg" />}
        {partial ? t("clean.done_title_partial") : t("clean.done_title")}
      </h1>
      {result.dry_run ? (
        <p className="num text-lg">{t("clean.done_dry")}</p>
      ) : (
        <ul className="num grid gap-1 text-lg">
          {sum("trash", "disposed") > 0 && <li>{t("clean.done_moved", { count: sum("trash", "disposed"), size: formatBytes(sum("trash", "bytes_freed"), lang) })}</li>}
          {sum("permanent", "disposed") > 0 && <li>{t("clean.done_permanent", { count: sum("permanent", "disposed"), size: formatBytes(sum("permanent", "bytes_freed"), lang) })}</li>}
          {(result.skipped > 0 || result.failed > 0) && <li className={result.failed > 0 ? "text-risk-danger-fg" : ""}>{t("clean.done_counts", { skipped: result.skipped, failed: result.failed })}</li>}
        </ul>
      )}
      <UnreadablePlaces places={unreadable} />
      {!result.dry_run && sum("trash", "bytes_freed") > 0 && (
        <div className="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-risk-review-border bg-risk-review-bg p-4 text-sm text-risk-review-fg">
          <span className="max-w-[70ch]">{t("clean.trash_note")}</span>
          {onEmptyTrash && (
            <Button size="sm" variant="secondary" onClick={onEmptyTrash}>
              <Trash2 aria-hidden className="h-4 w-4" />
              {trashRule?.bytes ? t("clean.trash_empty_sized", { size: formatBytes(trashRule.bytes, i18n.language) }) : t("clean.trash_empty")}
            </Button>
          )}
        </div>
      )}
      <ul className="divide-y divide-border card-solid">
        {result.outcomes.map((o, i) => (
          <li key={`${o.kind === "project" ? o.root : o.rule_id}-${i}`} className="flex items-center justify-between gap-3 px-4 py-3">
            <span className="min-w-0">
              <span className="block font-medium">{label(o)}</span>
              {o.kind === "command" && (
                <span className="path block truncate text-fg-muted" title={o.outcome.command_line}>
                  {o.outcome.command_line}
                </span>
              )}
              {o.kind === "skipped" && <span className="block text-sm text-fg-muted">{o.reason}</span>}
            </span>
            <span className="shrink-0 text-sm">
              {(o.kind === "files" || o.kind === "project") && (
                <span className={o.session.failed > 0 ? "text-risk-danger-fg" : "text-fg-muted"}>
                  {t("clean.outcome_removed", { count: o.session.disposed })}
                  {o.session.skipped > 0 && `, ${o.session.skipped} ${t("clean.outcome_skipped").toLowerCase()}`}
                  {o.session.failed > 0 && `, ${o.session.failed} ${t("clean.outcome_failed").toLowerCase()}`}
                </span>
              )}
              {o.kind === "command" && <span className={o.outcome.success ? "text-fg-muted" : "text-risk-danger-fg"}>{o.outcome.success ? t("clean.outcome_ran") : t("clean.outcome_failed")}</span>}
              {o.kind === "skipped" && <span className="text-fg-muted">{t("clean.outcome_skipped")}</span>}
            </span>
          </li>
        ))}
      </ul>
      <div className="flex gap-2">
        <Button variant="outline" onClick={onSeeHistory}>
          {t("clean.see_history")}
        </Button>
        <Button variant="ghost" onClick={onBack}>
          {t("clean.reevaluate")}
        </Button>
      </div>
    </div>
  );
}
