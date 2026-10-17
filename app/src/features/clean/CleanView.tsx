import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { ChevronRight, RefreshCw, Sparkles } from "lucide-react";
import { Button } from "@/components/ui/button";
import { EmptyState, ErrorState, LoadingState } from "@/components/States";
import { RiskBadge } from "@/components/RiskBadge";
import { formatBytes } from "@/lib/format";
import { cleanApi } from "@/lib/cleanApi";
import { useErrorMessage } from "@/lib/errors";
import type { CleanEvaluation, CleanRunReport, Profile } from "@/lib/types";
import { ARTEFACT_PREFIX, buildRows, type RuleRow } from "./rows";
import { ProjectsSection } from "./ProjectsSection";
import { cn } from "@/lib/utils";
import { CleanPreview } from "./CleanPreview";
import { CleanResult } from "./CleanResult";

export function CleanView({ profile, onSeeHistory }: { profile: Profile; onSeeHistory: () => void }) {
  const { t, i18n } = useTranslation();
  const errorMessage = useErrorMessage();
  const [evaluation, setEvaluation] = useState<CleanEvaluation | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [preview, setPreview] = useState(false);
  const [result, setResult] = useState<CleanRunReport | null>(null);
  const [tab, setTab] = useState<"all" | "dev">("all");

  const load = useCallback(
    () =>
      cleanApi
        .evaluate(profile)
        .then((ev) => {
          setEvaluation(ev);
          setSelected(new Set([...ev.preselected, ...ev.preselected_artefacts.map((p) => ARTEFACT_PREFIX + p)]));
        })
        .catch((e) => setError(errorMessage(e))),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [profile],
  );

  useEffect(() => {
    void load();
  }, [load]);

  const rows = useMemo(() => (evaluation ? buildRows(evaluation, i18n.language) : []), [evaluation, i18n.language]);
  const artefactRows: RuleRow[] = useMemo(
    () =>
      (evaluation?.report.projects ?? []).flatMap((p) =>
        p.artefacts.map((a) => ({
          id: ARTEFACT_PREFIX + a.path,
          name: `${p.name}/${a.kind}`,
          description: p.rebuild.join(" && "),
          risk: a.risk,
          group: "projects",
          permanent: false,
          bytes: a.bytes,
          items: 1,
          skippedProtected: 0,
          commandLine: null,
          available: true,
          confirmSeparately: false,
          artefactPath: a.path,
          profile: "developer" as const,
        })),
      ),
    [evaluation],
  );
  const selectedRows = [...rows, ...artefactRows].filter((r) => selected.has(r.id));
  const selectedBytes = selectedRows.reduce((a, r) => a + (r.bytes ?? 0), 0);

  const toggle = (id: string) =>
    setSelected((s) => {
      const n = new Set(s);
      if (n.has(id)) n.delete(id);
      else n.add(id);
      return n;
    });

  if (error) {
    return (
      <ErrorState
        message={error}
        onRetry={() => {
          setError(null);
          void load();
        }}
      />
    );
  }
  if (!evaluation) {
    return (
      <div className="grid gap-4">
        <h1 className="display text-2xl">{t("clean.title")}</h1>
        <p className="text-fg-muted">{t("clean.evaluating")}</p>
        <LoadingState />
      </div>
    );
  }
  if (result) {
    return (
      <CleanResult
        result={result}
        rows={rows}
        onSeeHistory={onSeeHistory}
        onBack={() => {
          setResult(null);
          void load();
        }}
      />
    );
  }

  const devRows = rows.filter((r) => r.profile === "developer");
  const visibleRows = tab === "dev" ? devRows : rows;
  const groups = [...new Set(visibleRows.map((r) => r.group))];
  const projectBytes = evaluation.report.projects.reduce((a, p) => a + p.bytes, 0);
  const devSources = [...new Set(devRows.map((r) => r.group))].map((g) => ({ group: g, bytes: devRows.filter((r) => r.group === g).reduce((a, r) => a + (r.bytes ?? 0), 0) }));
  if (projectBytes > 0) devSources.push({ group: "projects", bytes: projectBytes });
  devSources.sort((a, b) => b.bytes - a.bytes);
  const devBytes = devSources.reduce((a, s) => a + s.bytes, 0);

  return (
    <div className="grid max-w-[1280px] gap-6">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <h1 className="display text-2xl">{t("clean.title")}</h1>
          <p className="mt-1 text-fg-muted">{tab === "dev" ? t("clean.dev_intro") : t("clean.intro")}</p>
        </div>
        <div className="flex items-center gap-2">
          {profile === "developer" && (
            <div role="tablist" aria-label={t("clean.title")} className="inline-flex rounded-full bg-surface-muted p-1">
              {(["all", "dev"] as const).map((id) => (
                <button key={id} role="tab" type="button" aria-selected={tab === id} onClick={() => setTab(id)} className={cn("rounded-full px-4 py-1.5 text-sm font-medium transition-colors", tab === id ? "bg-surface-solid text-fg shadow-[var(--shadow-1)]" : "text-fg-muted hover:text-fg")}>
                  {t(`clean.tab_${id}`)}
                </button>
              ))}
            </div>
          )}
          <Button variant="outline" size="sm" onClick={() => void load()}>
            <RefreshCw aria-hidden className="h-4 w-4" />
            {t("clean.reevaluate")}
          </Button>
        </div>
      </div>

      {tab === "dev" && devSources.length > 0 && (
        <div className="flex flex-wrap items-center gap-2">
          <span className="num mr-2 text-sm font-semibold">{t("clean.dev_total", { size: formatBytes(devBytes, i18n.language), count: devSources.length })}</span>
          {devSources.map((src) => (
            <span key={src.group} className="num inline-flex items-center gap-1.5 rounded-full border border-border bg-surface-solid px-3 py-1 text-sm">
              <span className="font-medium">{src.group === "projects" ? t("clean.dev_projects") : t(`clean.group_${src.group}`, { defaultValue: src.group })}</span>
              <span className="text-fg-muted">{formatBytes(src.bytes, i18n.language)}</span>
            </span>
          ))}
        </div>
      )}

      {rows.length === 0 && artefactRows.length === 0 ? (
        <EmptyState title={t("clean.empty_title")} body={t("clean.empty_body")} action={<Button variant="outline" size="sm" onClick={() => void load()}>{t("clean.reevaluate")}</Button>} />
      ) : (
        <>
          <div className="grid gap-6">
            {evaluation.report.projects.length > 0 && <ProjectsSection projects={evaluation.report.projects} selected={selected} onToggle={toggle} />}
            {groups.map((g) => {
              const inGroup = visibleRows.filter((r) => r.group === g);
              const groupBytes = inGroup.reduce((a, r) => a + (r.bytes ?? 0), 0);
              const picked = inGroup.filter((r) => selected.has(r.id)).length;
              // Open when something is preselected or there is real size to see;
              // small, unselected groups fold away so the page fits one view.
              const open = picked > 0 || groupBytes >= 100 * 1024 * 1024;
              return (
              <details key={g} open={open} className="group/acc card-solid">
                <summary className="flex cursor-pointer list-none items-center gap-3 px-4 py-2.5 text-sm [&::-webkit-details-marker]:hidden">
                  <ChevronRight aria-hidden className="h-4 w-4 text-fg-faint transition-transform group-open/acc:rotate-90" />
                  <span className="font-semibold">{t(`clean.group_${g}`, { defaultValue: g })}</span>
                  <span className="num text-fg-muted">{t("clean.group_summary", { count: inGroup.length, picked })}</span>
                  <span className="num ml-auto font-medium">{groupBytes > 0 ? formatBytes(groupBytes, i18n.language) : ""}</span>
                </summary>
                <ul className="divide-y divide-border border-t border-border">
                  {inGroup
                    .map((r) => {
                      const disabled = !r.available;
                      const checked = selected.has(r.id);
                      return (
                        <li key={r.id}>
                          <label className={cn("flex cursor-pointer items-start gap-4 px-4 py-2.5", disabled && "cursor-not-allowed opacity-60")} style={{ boxShadow: `inset 3px 0 0 ${r.risk === "green" ? "var(--risk-safe-border)" : "var(--risk-review-border)"}` }}>
                            <input
                              type="checkbox"
                              className="mt-1 h-4 w-4 accent-[var(--accent)]"
                              checked={checked}
                              disabled={disabled}
                              onChange={() => toggle(r.id)}
                              aria-label={r.name}
                            />
                            <span className="min-w-0 flex-1">
                              <span className="flex flex-wrap items-center gap-2">
                                <span className="font-medium">{r.name}</span>
                                <RiskBadge level={r.risk === "green" ? "safe" : "review"} />
                              </span>
                              <span className="mt-0.5 block text-sm text-fg-muted">{r.description}</span>
                              <span className="mt-1 block text-xs text-fg-muted">
                                {r.commandLine
                                  ? disabled
                                    ? t("clean.not_installed")
                                    : t("clean.command_note", { cmd: r.commandLine })
                                  : [t("clean.items", { count: r.items }), r.skippedProtected > 0 ? t("clean.protected_skipped", { count: r.skippedProtected }) : null].filter(Boolean).join(", ")}
                                {r.permanent && <span className="text-risk-review-fg"> {t("clean.permanent_note")}</span>}
                              </span>
                            </span>
                            <span className="num shrink-0 text-right font-medium">{r.bytes === null ? <span className="text-sm text-fg-muted">{t("clean.estimate_unknown")}</span> : formatBytes(r.bytes, i18n.language)}</span>
                          </label>
                        </li>
                      );
                    })}
                </ul>
              </details>
              );
            })}
          </div>

          <div className="sticky bottom-0 -mx-8 flex items-center justify-between gap-4 border-t border-border bg-bg/95 px-8 py-4 backdrop-blur">
            <span className="text-sm text-fg-muted">{t("clean.selected_summary", { count: selectedRows.length })}</span>
            <Button size="lg" disabled={selectedRows.length === 0} onClick={() => setPreview(true)}>
              <Sparkles aria-hidden className="h-4 w-4" />
              {selectedRows.length === 0 ? t("clean.primary_none") : t("clean.primary", { size: formatBytes(selectedBytes, i18n.language) })}
            </Button>
          </div>
        </>
      )}

      {preview && (
        <CleanPreview
          rows={selectedRows}
          onClose={() => setPreview(false)}
          onDone={(r) => {
            setPreview(false);
            setResult(r);
          }}
        />
      )}
    </div>
  );
}
