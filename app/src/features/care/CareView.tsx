import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { HeartPulse, Sparkles } from "lucide-react";
import { Button } from "@/components/ui/button";
import { ErrorState } from "@/components/States";
import { SpaceRing } from "@/components/viz/SpaceRing";
import { useCountUp } from "@/components/viz/useCountUp";
import { useSmoothCounter } from "@/components/viz/useSmoothCounter";
import { careApi } from "@/lib/careApi";
import { cleanApi } from "@/lib/cleanApi";
import { useErrorMessage } from "@/lib/errors";
import { formatBytes, formatDate, formatEta } from "@/lib/format";
import type { CareProgressPayload, CareReport, CleanRunReport, Recommendation } from "@/lib/types";
import type { Screen } from "@/components/Shell";
import { cn } from "@/lib/utils";

type Phase = { kind: "idle" } | { kind: "running" } | { kind: "report"; report: CareReport } | { kind: "cleaned"; run: CleanRunReport; report: CareReport };

function scoreTone(score: number) {
  return score >= 80 ? "var(--gauge-ok)" : score >= 60 ? "var(--accent)" : score >= 40 ? "var(--gauge-warn)" : "var(--gauge-danger)";
}

function scoreText(t: (k: string) => string, score: number) {
  return score >= 80 ? t("care.score_great") : score >= 60 ? t("care.score_good") : score >= 40 ? t("care.score_fair") : t("care.score_poor");
}

/** Smart Care: scan + evaluate in one click, then a score and the safest next steps. */
export function CareView({ onNavigate }: { onNavigate: (s: Screen) => void }) {
  const { t, i18n } = useTranslation();
  const lang = i18n.language;
  const errorMessage = useErrorMessage();
  const [phase, setPhase] = useState<Phase>({ kind: "idle" });
  const [progress, setProgress] = useState<CareProgressPayload | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [last, setLast] = useState<CareReport | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let live = true;
    careApi
      .last()
      .then((r) => live && setLast(r))
      .catch(() => undefined);
    const subs = [
      careApi.onProgress((p) => setProgress(p)),
      careApi.onDone((report) => {
        setPhase({ kind: "report", report });
        setLast(report);
        setProgress(null);
      }),
      careApi.onError((e) => {
        setError(errorMessage(e));
        setPhase({ kind: "idle" });
      }),
    ];
    return () => {
      live = false;
      void Promise.all(subs).then((fns) => fns.forEach((f) => f()));
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const start = () => {
    setError(null);
    setProgress(null);
    careApi
      .run()
      .then(() => setPhase({ kind: "running" }))
      .catch((e) => setError(errorMessage(e)));
  };

  const cleanSafe = async (report: CareReport) => {
    setBusy(true);
    try {
      const run = await cleanApi.execute(report.preselected, report.preselected_artefacts, true, false);
      setPhase({ kind: "cleaned", run, report });
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  if (phase.kind === "running") return <Running progress={progress} />;
  if (phase.kind === "report") return <Report report={phase.report} busy={busy} onClean={() => void cleanSafe(phase.report)} onNavigate={onNavigate} onAgain={start} error={error} />;
  if (phase.kind === "cleaned") {
    return (
      <div className="grid max-w-[1040px] gap-6">
        <h1 className="display text-2xl">{t("care.done_title")}</h1>
        <p className="num text-lg">{t("care.done_body", { size: formatBytes(phase.run.bytes_freed, lang) })}</p>
        <div className="flex gap-2">
          <Button onClick={start}>{t("care.again")}</Button>
          <Button variant="outline" onClick={() => onNavigate("history")}>
            {t("clean.see_history")}
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div className="grid max-w-[1040px] gap-10">
      <section className="rise grid items-center gap-10 md:grid-cols-[auto_1fr]">
        <SpaceRing size={236} thickness={20} total={100} label={t("care.score_label")} segments={last ? [{ key: "score", value: last.score.score, color: scoreTone(last.score.score) }] : []}>
          <div className="grid gap-0.5">
            {last ? (
              <>
                <span className="display num text-[44px] leading-none">{last.score.score}</span>
                <span className="text-xs text-fg-muted">{t("care.score_label")}</span>
              </>
            ) : (
              <HeartPulse aria-hidden className="mx-auto h-10 w-10 text-accent" />
            )}
          </div>
        </SpaceRing>
        <div className="grid gap-5">
          <div className="grid gap-2">
            <h1 className="display text-[32px] leading-tight">{t("care.title")}</h1>
            <p className="max-w-[56ch] text-fg-muted">{t("care.intro")}</p>
          </div>
          <div className="flex flex-wrap items-center gap-4">
            <Button size="lg" onClick={start}>
              <Sparkles aria-hidden className="h-5 w-5" />
              {t("care.start")}
            </Button>
            {last && <span className="num text-sm text-fg-muted">{t("care.last_care", { date: formatDate(last.finished_at_unix, lang), score: last.score.score })}</span>}
          </div>
          {error && <ErrorState message={error} />}
        </div>
      </section>
    </div>
  );
}

function Running({ progress }: { progress: CareProgressPayload | null }) {
  const { t, i18n } = useTranslation();
  const files = useSmoothCounter(progress?.items_done ?? 0);
  const stage = progress?.stage ?? 1;
  const fraction = progress?.fraction ?? null;
  return (
    <div className="grid max-w-[1040px] gap-10">
      <section aria-live="polite" className="rise grid items-center gap-10 md:grid-cols-[auto_1fr]">
        <div role="progressbar" aria-label={t("care.running_title")} aria-valuemin={0} aria-valuemax={100} aria-valuenow={fraction === null ? undefined : Math.floor(fraction * 100)}>
          <SpaceRing size={236} thickness={20} total={1} spinning={fraction === null} segments={fraction === null ? [] : [{ key: "done", value: fraction, color: "var(--accent)" }]} label={t("care.running_title")}>
            <span className="display num text-[40px] leading-none">{fraction === null ? "…" : `${Math.floor(fraction * 100)}%`}</span>
          </SpaceRing>
        </div>
        <div className="grid gap-3">
          <p className="text-sm text-fg-muted">{t("progress.step_of", { current: stage, total: progress?.total ?? 2 })}</p>
          <p className="display text-[28px] leading-tight">{t(`care.stage_${stage}`)}</p>
          {stage === 1 && (
            <p className="num text-fg-muted">
              {Math.round(files).toLocaleString(i18n.language)} {t("scan.scanning_files")}
            </p>
          )}
          <p data-testid="eta" className="font-medium">
            {progress ? formatEta(t, progress.eta) : t("progress.computing")}
          </p>
        </div>
      </section>
    </div>
  );
}

function Report({ report, busy, onClean, onNavigate, onAgain, error }: { report: CareReport; busy: boolean; onClean: () => void; onNavigate: (s: Screen) => void; onAgain: () => void; error: string | null }) {
  const { t, i18n } = useTranslation();
  const lang = i18n.language;
  const s = report.score;
  const shown = useCountUp(s.score, 1200);
  const rec = (r: Recommendation) => {
    switch (r.kind) {
      case "clean_safe":
        return t("care.rec_clean_safe", { size: formatBytes(r.bytes, lang) });
      case "review_clutter":
        return t("care.rec_review", { size: formatBytes(r.bytes, lang) });
      case "stale_projects":
        return t("care.rec_stale", { count: r.count, size: formatBytes(r.bytes, lang) });
      case "largest_folder":
        return t("care.rec_largest", { name: r.path.split("/").filter(Boolean).pop() ?? r.path, size: formatBytes(r.bytes, lang) });
      case "disk_nearly_full":
        return t("care.rec_full", { pct: r.free_pct });
    }
  };
  const target = (r: Recommendation): Screen | null => (r.kind === "clean_safe" || r.kind === "stale_projects" || r.kind === "review_clutter" ? "clean" : r.kind === "largest_folder" ? "scan" : null);

  return (
    <div className="grid max-w-[1040px] gap-10">
      <section className="rise grid items-center gap-10 md:grid-cols-[auto_1fr]">
        <SpaceRing size={236} thickness={20} total={100} label={t("care.score_label")} segments={[{ key: "score", value: s.score, color: scoreTone(s.score) }]}>
          <div className="grid gap-0.5">
            <span className="display num text-[52px] leading-none">{Math.round(shown)}</span>
            <span className="text-xs text-fg-muted">{t("care.score_label")}</span>
          </div>
        </SpaceRing>
        <div className="grid gap-5">
          <div className="grid gap-1">
            <p className="display text-[28px] leading-tight">{scoreText(t, s.score)}</p>
            <p className="num text-sm text-fg-muted">
              {t("home.hero_of", { total: formatBytes(report.scan.disk_total_bytes ?? report.scan.total_bytes, lang), pct: 100 - s.free_pct })}
            </p>
          </div>
          <ul className="grid grid-cols-2 gap-x-6 gap-y-2 text-sm md:grid-cols-4">
            {s.components.map((c) => (
              <li key={c.key} className="grid gap-1">
                <span className="flex items-baseline justify-between gap-2">
                  <span className="text-fg-muted">{t(`care.comp_${c.key}`)}</span>
                  <span className="num font-medium">{t("care.points", { points: c.points, max: c.max })}</span>
                </span>
                <span className="h-1.5 overflow-hidden rounded-full bg-surface-muted">
                  <span className="block h-full rounded-full" style={{ width: `${(c.points / c.max) * 100}%`, background: scoreTone((c.points / c.max) * 100) }} />
                </span>
              </li>
            ))}
          </ul>
          <div className="flex flex-wrap items-center gap-3">
            {s.safe_bytes > 0 ? (
              <Button size="lg" onClick={onClean} disabled={busy}>
                <Sparkles aria-hidden className="h-5 w-5" />
                {busy ? t("clean.running") : t("care.cta_clean", { size: formatBytes(s.safe_bytes, lang) })}
              </Button>
            ) : (
              <span className="text-fg-muted">{t("care.cta_nothing")}</span>
            )}
            <Button variant="ghost" onClick={onAgain}>
              {t("care.again")}
            </Button>
          </div>
          {error && <ErrorState message={error} />}
        </div>
      </section>

      <section className="grid gap-3">
        <h2 className="text-sm font-semibold text-fg-muted">{t("care.rec_title")}</h2>
        <ol className="grid gap-2">
          {s.recommendations.map((r, i) => {
            const to = target(r);
            return (
              <li key={`${r.kind}-${i}`} className={cn("card flex items-center justify-between gap-4 px-4 py-3", `rise rise-${Math.min(4, i + 1)}`)}>
                <span className="flex items-center gap-3">
                  <span className="display num grid h-7 w-7 place-items-center rounded-full bg-accent-soft text-sm text-accent">{i + 1}</span>
                  <span>{rec(r)}</span>
                </span>
                {to && (
                  <Button variant="ghost" size="sm" onClick={() => onNavigate(to)}>
                    {to === "scan" ? t("care.open_scan") : t("care.open_clean")}
                  </Button>
                )}
              </li>
            );
          })}
        </ol>
      </section>
    </div>
  );
}
