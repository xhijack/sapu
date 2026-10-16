import { useTranslation } from "react-i18next";
import { ProgressBar } from "./ui/progress";
import { formatBytes, formatElapsed, formatEta } from "@/lib/format";
import type { ProgressState } from "@/lib/types";

/** Honest progress: percentage, counts, ETA sentence, elapsed (PRD §10.1). */
export function ProgressPanel({ state, title }: { state: ProgressState; title: string }) {
  const { t, i18n } = useTranslation();
  const pct = state.fraction === null ? null : Math.floor(state.fraction * 100);
  return (
    <section aria-live="polite" className="grid gap-3">
      <div className="flex items-baseline justify-between gap-4">
        <h3 className="font-semibold">{title}</h3>
        {pct !== null && <span className="tabular-nums text-sm text-fg-muted">{pct}%</span>}
      </div>
      {state.stage && (
        <p className="text-sm text-fg-muted">
          {t("progress.step_of", { current: state.stage.current, total: state.stage.total })}: {state.stage.label}
        </p>
      )}
      <ProgressBar value={state.fraction} label={title} />
      <div className="flex flex-wrap justify-between gap-x-4 gap-y-1 text-sm text-fg-muted">
        <span data-testid="eta">{formatEta(t, state.eta)}</span>
        <span>
          {state.items_done.toLocaleString(i18n.language)}
          {state.items_total !== null && ` / ${state.items_total.toLocaleString(i18n.language)}`}
          {state.bytes_total !== null && ` · ${formatBytes(state.bytes_done, i18n.language)} / ${formatBytes(state.bytes_total, i18n.language)}`}
        </span>
        <span>{formatElapsed(t, state.elapsed_secs)}</span>
      </div>
    </section>
  );
}
