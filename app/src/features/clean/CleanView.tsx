import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { ChevronRight, RefreshCw, Sparkles } from "lucide-react";
import { Button } from "@/components/ui/button";
import { EmptyState, ErrorState, LoadingState } from "@/components/States";
import { SpaceRing } from "@/components/viz/SpaceRing";
import { Tips } from "@/components/Tips";
import { RiskBadge } from "@/components/RiskBadge";
import { formatBytes, formatDate, formatElapsed, formatEta } from "@/lib/format";
import { cleanApi } from "@/lib/cleanApi";
import { useErrorMessage } from "@/lib/errors";
import type { CleanCheckProgress, CleanEvaluation, CleanRunReport, Profile } from "@/lib/types";
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
  // "checking" while the background check runs; "ready" once a status
  // answer arrived (so the idle screen does not flash before a cached result).
  const [checking, setChecking] = useState<CleanCheckProgress | null | false>(false);
  const [ready, setReady] = useState(false);
  const [lastChecked, setLastChecked] = useState<number | null>(null);

  const adopt = (ev: CleanEvaluation) => {
    setEvaluation(ev);
    setLastChecked(ev.finished_at_unix);
    setSelected(new Set([...ev.preselected, ...ev.preselected_artefacts.map((p) => ARTEFACT_PREFIX + p)]));
  };

  /** Starts a check in the background; the result arrives through `onDone`. */
  const load = useCallback(() => {
    setError(null);
    setChecking(null);
    return cleanApi.start(profile).catch((e) => {
      setChecking(false);
      setError(errorMessage(e));
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [profile]);

  useEffect(() => {
    let live = true;
    cleanApi
      .status()
      .then(async (st) => {
        if (!live) return;
        if (st.running) setChecking(null);
        else if (st.has_result) {
          const ev = await cleanApi.last();
          if (!live) return;
          // A result for another profile is stale: check again.
          if (ev && ev.profile === profile) adopt(ev);
          else if (ev) void load();
        }
        if (live) setReady(true);
      })
      .catch(() => live && setReady(true));
    const subs = [
      cleanApi.onRunDone((r) => {
        setPreview(false);
        setResult(r);
      }),
      cleanApi.onProgress((p) => setChecking(p)),
      cleanApi.onDone((ev) => {
        adopt(ev);
        setChecking(false);
        setResult(null);
      }),
      cleanApi.onError((e) => {
        setChecking(false);
        setError(errorMessage(e));
      }),
    ];
    return () => {
      live = false;
      void Promise.all(subs).then((fns) => fns.forEach((f) => f()));
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

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
  if (checking !== false) {
    return <CleanChecking progress={checking} />;
  }
  if (!ready) {
    return (
      <div className="grid gap-4">
        <h1 className="display text-2xl">{t("clean.title")}</h1>
        <LoadingState />
      </div>
    );
  }
  if (!evaluation) {
    return (
      <div className="grid max-w-[1280px] gap-8">
        <section className="rise grid items-center gap-8 md:grid-cols-[auto_minmax(0,1fr)]">
          <SpaceRing size={180} thickness={16} total={1} segments={[]} label={t("clean.title")}>
            <Sparkles aria-hidden className="mx-auto h-9 w-9 text-accent" />
          </SpaceRing>
          <div className="grid gap-4">
            <div className="grid gap-2">
              <h1 className="display text-[32px] leading-tight">{t("clean.title")}</h1>
              <p className="max-w-[60ch] text-fg-muted">{t("clean.idle_body")}</p>
            </div>
            <div className="flex flex-wrap items-center gap-4">
              <Button size="lg" onClick={() => void load()}>
                <Sparkles aria-hidden className="h-5 w-5" />
                {t("clean.check_now")}
              </Button>
              <span className="num text-sm text-fg-muted">{lastChecked ? t("clean.last_checked", { date: formatDate(lastChecked, i18n.language) }) : t("clean.never_checked")}</span>
            </div>
          </div>
        </section>
        <Tips seed={3} />
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
          {lastChecked && <span className="num text-sm text-fg-muted">{t("clean.last_checked", { date: formatDate(lastChecked, i18n.language) })}</span>}
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

/** The check as a moment, like scanning: a ring, the stage, and a promise it keeps running if you leave. */
function CleanChecking({ progress }: { progress: CleanCheckProgress | null }) {
  const { t } = useTranslation();
  const fraction = progress?.fraction ?? null;
  const pct = fraction === null ? null : Math.floor(fraction * 100);
  return (
    <div className="grid max-w-[1280px] gap-8">
      <section aria-live="polite" className="rise grid items-center gap-8 md:grid-cols-[auto_minmax(0,1fr)]">
        <div role="progressbar" aria-label={t("clean.checking_title")} aria-valuemin={0} aria-valuemax={100} aria-valuenow={pct ?? undefined}>
          <SpaceRing size={200} thickness={18} total={1} spinning={fraction === null} segments={fraction === null ? [] : [{ key: "done", value: fraction, color: "var(--accent)" }]} label={t("clean.checking_title")}>
            <span className="display num text-[36px] leading-none">{pct === null ? "…" : `${pct}%`}</span>
          </SpaceRing>
        </div>
        <div className="grid gap-3">
          <h1 className="display text-[32px] leading-tight">{t("clean.checking_title")}</h1>
          <p className="text-fg">{!progress || progress.stage === "projects" ? t("clean.stage_projects") : t("clean.stage_rules", { done: progress.done, total: progress.total })}</p>
          <p className="num text-sm text-fg-muted">
            {progress ? formatEta(t, progress.eta) : t("progress.computing")}
            {progress && ` · ${formatElapsed(t, progress.elapsed_secs)}`}
          </p>
          <p className="text-sm text-fg-muted">{t("clean.checking_note")}</p>
        </div>
      </section>
      <Tips seed={3} />
    </div>
  );
}
