import { useTranslation } from "react-i18next";
import { Pause, Play, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { SpaceRing } from "@/components/viz/SpaceRing";
import { useSmoothCounter } from "@/components/viz/useSmoothCounter";
import { RollingNumber } from "@/components/viz/RollingNumber";
import { useThrottledValue } from "@/components/viz/useThrottledValue";
import { CategoryIcon } from "@/components/icons/CategoryIcon";
import { categoryColor } from "./categoryColor";
import { formatBytes, formatElapsed, formatEta } from "@/lib/format";
import type { LiveSnapshot, ScanProgressPayload } from "@/lib/types";

interface Props {
  root: string;
  progress: ScanProgressPayload | null;
  live: LiveSnapshot | null;
  paused: boolean;
  onPause: () => void;
  onResume: () => void;
  onCancel: () => void;
}

/** The scan as a moment: one ring, one number, one honest sentence. */
export function Scanning({ root, progress, live, paused, onPause, onResume, onCancel }: Props) {
  const { t, i18n } = useTranslation();
  const fraction = progress?.fraction ?? null;
  const pct = fraction === null ? null : Math.floor(fraction * 100);
  const files = useThrottledValue(useSmoothCounter(progress?.items_done ?? 0), 250);
  const bytes = useSmoothCounter(progress?.bytes_done ?? 0);
  const liveShown = useThrottledValue(live, 400);
  const catMax = Math.max(1, ...(liveShown?.categories.map(([, b]) => b) ?? [1]));
  return (
    <div className="grid max-w-[1040px] gap-10">
      <section aria-live="polite" className="rise grid items-center gap-10 md:grid-cols-[auto_1fr]">
        <div role="progressbar" aria-label={t("scan.scanning_title", { path: root })} aria-valuemin={0} aria-valuemax={100} aria-valuenow={pct ?? undefined}>
        <SpaceRing
          size={236}
          thickness={20}
          total={1}
          spinning={fraction === null && !paused}
          segments={fraction === null ? [] : [{ key: "done", value: fraction, color: "var(--accent)" }]}
          label={t("scan.scanning_title", { path: root })}
        >
          <div className="grid gap-0.5">
            <span className="display num text-[40px] leading-none">{pct === null ? "…" : `${pct}%`}</span>
            {paused && <span className="text-xs text-fg-muted">{t("scan.pause")}</span>}
          </div>
        </SpaceRing>
        </div>
        <div className="grid gap-5">
          <div className="grid gap-1">
            <p className="text-fg-muted">{t("scan.scanning_title", { path: "" }).trim()}</p>
            <p className="path truncate text-fg-muted" title={root}>
              {root}
            </p>
            <RollingNumber value={Math.round(files).toLocaleString(i18n.language)} className="display text-[44px] leading-[1.05]" />
            <p className="text-fg-muted">{t("scan.scanning_files")}</p>
          </div>
          <div className="grid gap-1 text-sm text-fg-muted">
            <p className="num">
              {formatBytes(bytes, i18n.language)} {t("scan.scanning_bytes")}
            </p>
            <p data-testid="eta" className="font-medium text-fg">
              {progress ? formatEta(t, progress.eta) : t("progress.computing")}
            </p>
            {progress && <p className="num">{formatElapsed(t, progress.elapsed_secs)}</p>}
          </div>
          <div className="flex gap-2">
            {paused ? (
              <Button onClick={onResume}>
                <Play aria-hidden className="h-4 w-4" />
                {t("scan.resume")}
              </Button>
            ) : (
              <Button variant="secondary" onClick={onPause}>
                <Pause aria-hidden className="h-4 w-4" />
                {t("scan.pause")}
              </Button>
            )}
            <Button variant="ghost" onClick={onCancel}>
              <X aria-hidden className="h-4 w-4" />
              {t("scan.cancel")}
            </Button>
          </div>
        </div>
      </section>

      {liveShown && (
        <section aria-live="off" className="rise grid gap-6 md:grid-cols-[1.1fr_1fr]">
          <div className="grid content-start gap-5">
            <div className="grid gap-1">
              <span className="text-xs font-semibold text-fg-muted">{t("scan.live_reading")}</span>
              <p className="path truncate text-left text-fg-muted" dir="rtl" title={liveShown.current_dir}>
                <bdi>{liveShown.current_dir}</bdi>
              </p>
            </div>
            <div className="grid gap-2">
              <span className="text-xs font-semibold text-fg-muted">{t("scan.live_categories")}</span>
              <ul className="grid gap-1.5">
                {liveShown.categories.slice(0, 8).map(([c, b]) => (
                  <li key={c} className="grid grid-cols-[18px_120px_1fr_84px] items-center gap-3 text-sm">
                    <span style={{ color: categoryColor(c) }}>
                      <CategoryIcon category={c} size={16} />
                    </span>
                    <span className="truncate">{t(`category.${c}`)}</span>
                    <span className="h-1.5 overflow-hidden rounded-full bg-surface-muted">
                      <span className="block h-full rounded-full transition-[width] duration-500 ease-out motion-reduce:transition-none" style={{ width: `${(b / catMax) * 100}%`, background: categoryColor(c) }} />
                    </span>
                    <span className="num text-right text-fg-muted">{formatBytes(b, i18n.language)}</span>
                  </li>
                ))}
              </ul>
            </div>
          </div>
          <div className="grid content-start gap-4">
            <TopList title={t("scan.live_top")} items={liveShown.top_dirs} lang={i18n.language} />
            <TopList title={t("scan.live_top_files")} items={liveShown.top_files} lang={i18n.language} />
          </div>
        </section>
      )}
    </div>
  );
}

function TopList({ title, items, lang }: { title: string; items: [string, number][]; lang: string }) {
  if (items.length === 0) return null;
  return (
    <div className="card grid content-start gap-2 p-4">
      <span className="text-xs font-semibold text-fg-muted">{title}</span>
      <ol className="grid gap-1.5 text-sm">
        {items.map(([p, b], i) => (
          <li key={p} className="flex items-center gap-3">
            <span className="num w-4 text-fg-faint">{i + 1}</span>
            <span className="path min-w-0 flex-1 truncate text-left" dir="rtl" title={p}>
              <bdi>{p}</bdi>
            </span>
            <span className="num shrink-0 text-fg-muted">{formatBytes(b, lang)}</span>
          </li>
        ))}
      </ol>
    </div>
  );
}
