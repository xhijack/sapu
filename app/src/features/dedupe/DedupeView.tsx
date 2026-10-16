import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { ProgressPanel } from "@/components/ProgressPanel";
import { ErrorState } from "@/components/States";
import { dedupeApi } from "@/lib/dedupeApi";
import { scanApi } from "@/lib/scanApi";
import { useErrorMessage } from "@/lib/errors";
import type { DedupeProgressPayload, DedupeSummary, Profile } from "@/lib/types";
import { DedupeResults } from "./DedupeResults";

type Phase = { kind: "idle" } | { kind: "running"; root: string } | { kind: "done"; summary: DedupeSummary };

const MIN_SIZES = [1024, 4096, 64 * 1024, 1024 * 1024];

export function DedupeView({ onSeeHistory, profile = "general" }: { onSeeHistory: () => void; profile?: Profile }) {
  const { t } = useTranslation();
  const errorMessage = useErrorMessage();
  const [phase, setPhase] = useState<Phase>({ kind: "idle" });
  const [root, setRoot] = useState<string | null>(null);
  const [progress, setProgress] = useState<DedupeProgressPayload | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [minSize, setMinSize] = useState(4096);

  useEffect(() => {
    let live = true;
    scanApi.targets().then((tg) => live && setRoot((r) => r ?? tg.home ?? tg.disk)).catch(() => undefined);
    dedupeApi
      .summary()
      .then((s) => live && setPhase({ kind: "done", summary: s }))
      .catch(() => undefined);
    return () => {
      live = false;
    };
  }, []);

  useEffect(() => {
    const subs = [
      dedupeApi.onProgress((p) => setProgress(p)),
      dedupeApi.onDone((summary) => {
        setPhase({ kind: "done", summary });
        setProgress(null);
      }),
      dedupeApi.onError((e) => {
        setError(errorMessage(e));
        setPhase({ kind: "idle" });
      }),
    ];
    return () => {
      void Promise.all(subs).then((fns) => fns.forEach((f) => f()));
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const start = useCallback(
    (r: string) => {
      setError(null);
      setProgress(null);
      dedupeApi
        .start(r, minSize)
        .then(() => setPhase({ kind: "running", root: r }))
        .catch((e) => setError(errorMessage(e)));
    },
    [errorMessage, minSize],
  );

  const pick = async () => {
    const p = await scanApi.pickFolder().catch(() => null);
    if (p) setRoot(p);
  };

  if (phase.kind === "running") {
    const stageLabel = progress ? t(`dedupe.stage_${progress.stage.current}`) : t("dedupe.stage_1");
    const state = progress
      ? { ...progress, stage: { current: progress.stage.current, total: progress.stage.total, label: stageLabel } }
      : { fraction: null, items_done: 0, items_total: null, bytes_done: 0, bytes_total: null, elapsed_secs: 0, eta: { kind: "computing" as const }, stage: { current: 1, total: 3, label: stageLabel } };
    return (
      <div className="grid max-w-2xl gap-8">
        <ProgressPanel title={t("dedupe.searching_title", { path: phase.root })} state={state} />
        <div>
          <Button variant="ghost" onClick={() => void dedupeApi.cancel()}>
            {t("dedupe.cancel")}
          </Button>
        </div>
      </div>
    );
  }

  if (phase.kind === "done") {
    return <DedupeResults summary={phase.summary} profile={profile} onSeeHistory={onSeeHistory} onNewSearch={() => setPhase({ kind: "idle" })} onRerun={() => start(phase.summary.root)} />;
  }

  return (
    <div className="grid max-w-2xl gap-8">
      <div>
        <h1 className="text-2xl font-semibold">{t("dedupe.title")}</h1>
        <p className="mt-1 text-fg-muted">{t("dedupe.intro")}</p>
      </div>
      <div className="grid gap-2">
        <span className="text-sm text-fg-muted">{t("dedupe.target_label")}</span>
        <div className="flex items-center gap-3 card-solid px-4 py-3">
          <span className="path min-w-0 flex-1 truncate" title={root ?? ""}>
            {root ?? "…"}
          </span>
          <Button variant="outline" size="sm" onClick={() => void pick()}>
            {t("dedupe.pick_folder")}
          </Button>
        </div>
      </div>
      <label className="grid gap-1 text-sm">
        <span className="text-fg-muted">{t("dedupe.min_size")}</span>
        <select value={minSize} onChange={(e) => setMinSize(Number(e.target.value))} className="h-9 w-48 rounded-md border border-border bg-surface px-2">
          {MIN_SIZES.map((v) => (
            <option key={v} value={v}>
              {v >= 1024 * 1024 ? `${v / 1024 / 1024} MB` : `${v / 1024} KB`}
            </option>
          ))}
        </select>
      </label>
      {error && <ErrorState message={error} />}
      <div>
        <Button size="lg" disabled={!root} onClick={() => root && start(root)}>
          {t("dedupe.start")}
        </Button>
      </div>
    </div>
  );
}
