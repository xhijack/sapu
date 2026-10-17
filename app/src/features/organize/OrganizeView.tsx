import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Copy, FolderInput, FolderSearch, RotateCcw, Sparkles, Undo2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { ErrorState } from "@/components/States";
import { Tips } from "@/components/Tips";
import { SpaceRing } from "@/components/viz/SpaceRing";
import type { Screen } from "@/components/Shell";
import { useAiEnabled } from "@/features/ai/useAiEnabled";
import { useErrorMessage } from "@/lib/errors";
import { formatBytes, formatDate, formatElapsed, formatEta } from "@/lib/format";
import { organizeApi } from "@/lib/organizeApi";
import { scanApi } from "@/lib/scanApi";
import type { OrganizeAnalyseProgress, OrganizeAnalysis, OrganizeGroup, OrganizeRun, OrganizeRunProgress, OrganizeRunResult, OrganizeTarget, OrganizeUndoReport } from "@/lib/types";
import { cn } from "@/lib/utils";
import { GroupFilesDialog } from "./GroupFilesDialog";
import { PlanGroupCard } from "./PlanGroupCard";
import { ORGANIZE_OPEN_EVENT, takePendingOrganize } from "./organizeBus";

type Phase = "idle" | "analysing" | "plan" | "applying" | "done";

/** Rapikan: pick a folder → background analysis → tick groups → move → undo. */
export function OrganizeView({ onNavigate }: { onNavigate: (s: Screen) => void }) {
  const { t, i18n } = useTranslation();
  const lang = i18n.language;
  const errorMessage = useErrorMessage();
  const aiEnabled = useAiEnabled();
  const [phase, setPhase] = useState<Phase>("idle");
  const [targets, setTargets] = useState<OrganizeTarget[]>([]);
  const [root, setRoot] = useState<string | null>(null);
  const [useAi, setUseAi] = useState(true);
  const [stage, setStage] = useState<OrganizeAnalyseProgress | null>(null);
  const [analysis, setAnalysis] = useState<OrganizeAnalysis | null>(null);
  const [ticked, setTicked] = useState<Set<string>>(new Set());
  const [names, setNames] = useState<Record<string, string>>({});
  const [excluded, setExcluded] = useState<Set<string>>(new Set());
  const [openGroup, setOpenGroup] = useState<OrganizeGroup | null>(null);
  const [run, setRun] = useState<OrganizeRunProgress | null>(null);
  const [result, setResult] = useState<OrganizeRunResult | null>(null);
  const [undone, setUndone] = useState<OrganizeUndoReport | null>(null);
  const [history, setHistory] = useState<OrganizeRun[]>([]);
  const [error, setError] = useState<string | null>(null);

  const adopt = (a: OrganizeAnalysis) => {
    setAnalysis(a);
    setNames(Object.fromEntries(a.plan.groups.map((g) => [g.id, g.name])));
    setExcluded(new Set());
    setTicked(new Set(a.plan.groups.filter((g) => g.confidence === "sure" && movingOf(a.plan.root, g, new Set()).length > 0).map((g) => g.id)));
    setPhase("plan");
  };

  const refreshHistory = () =>
    organizeApi
      .history(5)
      .then(setHistory)
      .catch(() => undefined);

  useEffect(() => {
    let live = true;
    organizeApi
      .targets()
      .then((tg) => {
        if (!live) return;
        setTargets(tg);
        setRoot((r) => takePendingOrganize() ?? r ?? tg[0]?.path ?? null);
      })
      .catch(() => undefined);
    organizeApi
      .status()
      .then(async (st) => {
        if (!live) return;
        if (st.analysing) setPhase("analysing");
        else if (st.applying) setPhase("applying");
        else if (st.has_plan) {
          const a = await organizeApi.plan();
          if (live && a) adopt(a);
        }
      })
      .catch(() => undefined);
    void refreshHistory();
    const open = (e: Event) => {
      takePendingOrganize();
      setRoot((e as CustomEvent<string>).detail);
      setPhase("idle");
    };
    window.addEventListener(ORGANIZE_OPEN_EVENT, open);
    const subs = [
      organizeApi.onProgress((p) => setStage(p)),
      organizeApi.onPlan((a) => adopt(a)),
      organizeApi.onError((e) => {
        setError(errorMessage(e));
        setPhase("idle");
      }),
      organizeApi.onRunProgress((p) => setRun(p)),
      organizeApi.onRunDone((r) => {
        setResult(r);
        setUndone(null);
        setPhase("done");
        void refreshHistory();
      }),
      organizeApi.onRunError((e) => {
        setError(errorMessage(e));
        setPhase("plan");
      }),
    ];
    return () => {
      live = false;
      window.removeEventListener(ORGANIZE_OPEN_EVENT, open);
      void Promise.all(subs).then((fns) => fns.forEach((f) => f()));
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const start = async () => {
    if (!root) return;
    setError(null);
    setStage(null);
    setPhase("analysing");
    try {
      await organizeApi.analyse(root, aiEnabled && useAi);
    } catch (e) {
      setError(errorMessage(e));
      setPhase("idle");
    }
  };

  const groups = useMemo(() => analysis?.plan.groups ?? [], [analysis]);
  const moving = (g: OrganizeGroup) => movingOf(analysis?.plan.root ?? "", g, excluded);
  const chosen = groups.filter((g) => ticked.has(g.id) && moving(g).length > 0);

  const apply = async () => {
    if (!analysis) return;
    setError(null);
    setRun(null);
    setPhase("applying");
    const renames = chosen.filter((g) => g.source !== "existing" && g.source !== "unit" && names[g.id] && names[g.id] !== g.name).map((g) => [g.id, names[g.id]!.trim()] as [string, string]);
    try {
      await organizeApi.apply({ group_ids: chosen.map((g) => g.id), excluded: [...excluded], renames });
    } catch (e) {
      setError(errorMessage(e));
      setPhase("plan");
    }
  };

  const undo = async (id: string) => {
    setError(null);
    try {
      setUndone(await organizeApi.undo(id));
      void refreshHistory();
    } catch (e) {
      setError(errorMessage(e));
    }
  };

  const pick = async () => {
    const p = await scanApi.pickFolder().catch(() => null);
    if (p) setRoot(p);
  };

  if (phase === "analysing") {
    return (
      <div className="grid max-w-[1280px] gap-8">
        <section aria-live="polite" className="rise grid items-center gap-8 md:grid-cols-[auto_minmax(0,1fr)]">
          <SpaceRing size={180} thickness={16} total={1} spinning segments={[]} label={t("organize.analysing")}>
            <FolderSearch aria-hidden className="mx-auto h-9 w-9 text-accent" />
          </SpaceRing>
          <div className="grid gap-2">
            <h1 className="display text-[32px] leading-tight">{t("organize.analysing")}</h1>
            <p>{t(`organize.stage.${stage?.stage ?? "reading"}`)}</p>
            <p className="path truncate text-fg-muted" title={root ?? ""}>
              {root}
            </p>
            <p className="text-sm text-fg-muted">{t("organize.background_note")}</p>
          </div>
        </section>
      </div>
    );
  }

  if (phase === "applying") {
    const pct = run?.fraction == null ? null : Math.round(run.fraction * 100);
    return (
      <div className="grid max-w-[1280px] gap-6">
        <h1 className="display text-2xl">{t("organize.applying")}</h1>
        <div role="progressbar" aria-label={t("organize.applying")} aria-valuemin={0} aria-valuemax={100} aria-valuenow={pct ?? undefined} className="h-2 w-full max-w-xl overflow-hidden rounded-full bg-surface-muted">
          <span className={cn("block h-full rounded-full bg-accent transition-[width] duration-300", pct === null && "slide w-1/3")} style={pct === null ? undefined : { width: `${pct}%` }} />
        </div>
        <p className="num text-sm text-fg-muted">
          {run?.items_total ? t("clean.run_counts", { done: run.items_done, total: run.items_total }) : ""}
          {run ? ` · ${formatEta(t, run.eta)} · ${formatElapsed(t, run.elapsed_secs)}` : ""}
        </p>
        <p className="text-sm text-fg-muted">{t("organize.background_note")}</p>
      </div>
    );
  }

  if (phase === "done" && result) {
    const folders = new Set(result.session.moves.map((m) => m.to.slice(0, m.to.lastIndexOf("/")))).size;
    return (
      <div className="grid max-w-[1280px] gap-6">
        <h1 className="display text-2xl">{t("organize.done_title")}</h1>
        <p className="num text-lg">{t("organize.done_body", { count: result.session.moved, folders })}</p>
        {(result.session.skipped > 0 || result.session.failed > 0) && <p className="text-sm text-risk-review-fg">{t("organize.done_partial", { skipped: result.session.skipped, failed: result.session.failed })}</p>}
        {result.failures.length > 0 && (
          <ul className="card-solid max-h-48 overflow-y-auto p-3 text-xs">
            {result.failures.map(([p, why]) => (
              <li key={p} className="truncate" title={`${p}: ${why}`}>
                <span className="path">{p.split("/").pop()}</span> — {why}
              </li>
            ))}
          </ul>
        )}
        {undone ? (
          <p role="status" className="card-solid px-4 py-2 text-sm">
            {t("organize.undone", { count: undone.restored })}
            {undone.skipped > 0 && ` ${t("organize.undo_skipped", { count: undone.skipped })}`}
          </p>
        ) : null}
        <div className="flex flex-wrap gap-2">
          {!undone && (
            <Button variant="outline" onClick={() => void undo(result.session.id)}>
              <Undo2 aria-hidden className="h-4 w-4" />
              {t("organize.undo")}
            </Button>
          )}
          <Button onClick={() => setPhase("idle")}>{t("organize.again")}</Button>
        </div>
        {error && <ErrorState message={error} />}
      </div>
    );
  }

  if (phase === "plan" && analysis) {
    return (
      <div className="grid max-w-[1280px] gap-5">
        <div className="flex flex-wrap items-end justify-between gap-4">
          <div className="min-w-0">
            <h1 className="display text-2xl">{t("organize.plan_title")}</h1>
            <p className="path truncate text-fg-muted" title={analysis.plan.root}>
              {analysis.plan.root}
            </p>
          </div>
          <Button variant="outline" size="sm" onClick={() => void start()}>
            <RotateCcw aria-hidden className="h-4 w-4" />
            {t("organize.reanalyse")}
          </Button>
        </div>
        <p className="max-w-[80ch] text-sm text-fg-muted">
          {analysis.ai_used ? t("organize.plan_body_ai") : t("organize.plan_body")}
          {analysis.ai_note && ` ${t("organize.ai_unavailable")}`}
        </p>
        {analysis.duplicate_groups > 0 && (
          <div className="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-border bg-surface-muted px-4 py-3 text-sm">
            <span className="flex items-center gap-2">
              <Copy aria-hidden className="h-4 w-4" />
              {t("organize.duplicates", { files: analysis.duplicate_files, size: formatBytes(analysis.duplicate_bytes, lang) })}
            </span>
            <Button variant="secondary" size="sm" onClick={() => onNavigate("dedupe")}>
              {t("organize.open_duplicates")}
            </Button>
          </div>
        )}
        <div className="grid gap-3 lg:grid-cols-2">
          {groups.map((g) => (
            <PlanGroupCard
              key={g.id}
              group={g}
              moving={moving(g)}
              ticked={ticked.has(g.id)}
              name={names[g.id] ?? g.name}
              onTick={(v) =>
                setTicked((s) => {
                  const n = new Set(s);
                  if (v) n.add(g.id);
                  else n.delete(g.id);
                  return n;
                })
              }
              onRename={(v) => setNames((m) => ({ ...m, [g.id]: v }))}
              onOpen={() => setOpenGroup(g)}
            />
          ))}
        </div>
        <div className="sticky bottom-0 -mx-8 flex items-center justify-between gap-4 border-t border-border bg-bg/95 px-8 py-4 backdrop-blur">
          <span className="text-sm text-fg-muted">{t("organize.selected", { count: chosen.reduce((a, g) => a + moving(g).length, 0) })}</span>
          <Button size="lg" disabled={chosen.length === 0 || chosen.some((g) => !(names[g.id] ?? g.name).trim())} onClick={() => void apply()}>
            <FolderInput aria-hidden className="h-4 w-4" />
            {t("organize.apply", { count: chosen.length })}
          </Button>
        </div>
        {error && <ErrorState message={error} />}
        {openGroup && (
          <GroupFilesDialog
            group={openGroup}
            entries={movingOf(analysis.plan.root, openGroup, new Set())}
            excluded={excluded}
            onToggle={(p) =>
              setExcluded((s) => {
                const n = new Set(s);
                if (n.has(p)) n.delete(p);
                else n.add(p);
                return n;
              })
            }
            onClose={() => setOpenGroup(null)}
          />
        )}
      </div>
    );
  }

  const isCustom = root !== null && !targets.some((tg) => tg.path === root);
  return (
    <div className="grid max-w-[1280px] gap-6">
      <div>
        <h1 className="display text-2xl">{t("organize.title")}</h1>
        <p className="mt-1 max-w-[70ch] text-fg-muted">{t("organize.intro")}</p>
      </div>
      <div className="grid gap-6 lg:grid-cols-[minmax(0,1.4fr)_minmax(260px,0.9fr)]">
        <div className="grid content-start gap-5">
          <div role="radiogroup" aria-label={t("organize.title")} className="grid divide-y divide-border card-solid">
            {targets.map((tg) => (
              <label key={tg.key} className={cn("flex cursor-pointer items-center gap-4 px-5 py-3", root === tg.path ? "bg-surface-muted" : "hover:bg-surface-muted")}>
                <input type="radio" name="organize-target" checked={root === tg.path} onChange={() => setRoot(tg.path)} aria-label={t(`organize.target.${tg.key}`)} className="h-4 w-4 accent-[var(--accent)]" />
                <span className="min-w-0">
                  <span className="block font-medium">{t(`organize.target.${tg.key}`)}</span>
                  <span className="path block truncate text-fg-muted">{tg.path}</span>
                </span>
              </label>
            ))}
            <button type="button" onClick={() => void pick()} className={cn("flex items-center gap-4 px-5 py-3 text-left", isCustom ? "bg-surface-muted" : "hover:bg-surface-muted")}>
              <FolderSearch aria-hidden className="h-4 w-4 text-fg-muted" />
              <span className="min-w-0">
                <span className="block font-medium">{t("organize.pick")}</span>
                {isCustom && <span className="path block truncate text-fg-muted">{root}</span>}
              </span>
            </button>
          </div>
          {aiEnabled && (
            <label className="flex items-start gap-3 text-sm">
              <input type="checkbox" checked={useAi} onChange={(e) => setUseAi(e.target.checked)} className="mt-0.5 h-4 w-4 accent-[var(--accent)]" />
              <span>
                <span className="flex items-center gap-1.5 font-medium">
                  <Sparkles aria-hidden className="h-4 w-4" />
                  {t("organize.use_ai")}
                </span>
                <span className="block text-fg-muted">{t("organize.use_ai_hint")}</span>
              </span>
            </label>
          )}
          <div>
            <Button size="lg" disabled={!root} onClick={() => void start()}>
              <FolderSearch aria-hidden className="h-5 w-5" />
              {t("organize.start")}
            </Button>
          </div>
          {error && <ErrorState message={error} />}
          {undone && (
            <p role="status" className="card-solid px-4 py-2 text-sm">
              {t("organize.undone", { count: undone.restored })}
            </p>
          )}
        </div>
        <div className="grid content-start gap-4">
          <section className="card-solid grid gap-2 p-4" aria-label={t("organize.history_title")}>
            <h2 className="text-sm font-semibold text-fg-muted">{t("organize.history_title")}</h2>
            {history.length === 0 ? (
              <p className="text-sm text-fg-faint">{t("organize.history_empty")}</p>
            ) : (
              <ul className="grid gap-2 text-sm">
                {history.map((r) => (
                  <li key={r.id} className="flex items-center justify-between gap-3">
                    <span className="min-w-0">
                      <span className="path block truncate" title={r.root}>
                        {r.root.split("/").filter(Boolean).pop()}
                      </span>
                      <span className="num block text-xs text-fg-muted">
                        {formatDate(r.started_at_unix, lang)}, {t("organize.items", { count: r.moved })}
                        {r.undone ? `, ${t("organize.was_undone")}` : ""}
                      </span>
                    </span>
                    {!r.undone && r.moved > 0 && (
                      <Button variant="ghost" size="sm" onClick={() => void undo(r.id)} aria-label={t("organize.undo_run", { date: formatDate(r.started_at_unix, lang) })}>
                        <Undo2 aria-hidden className="h-4 w-4" />
                      </Button>
                    )}
                  </li>
                ))}
              </ul>
            )}
          </section>
          <Tips seed={4} />
        </div>
      </div>
    </div>
  );
}

/** Entries of a group that would move: not the group's own folder, not
 *  excluded, never a link. */
function movingOf(root: string, g: OrganizeGroup, excluded: Set<string>) {
  const own = `${root}/${g.name}`;
  return g.entries.filter((e) => e.path !== own && e.unit !== "link" && !excluded.has(e.path));
}
