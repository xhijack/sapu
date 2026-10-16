import { useTranslation } from "react-i18next";
import { Pause, Play, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { SpaceRing } from "@/components/viz/SpaceRing";
import { useSmoothCounter } from "@/components/viz/useSmoothCounter";
import { formatBytes, formatElapsed, formatEta } from "@/lib/format";
import type { ScanProgressPayload } from "@/lib/types";

interface Props {
  root: string;
  progress: ScanProgressPayload | null;
  paused: boolean;
  onPause: () => void;
  onResume: () => void;
  onCancel: () => void;
}

/** The scan as a moment: one ring, one number, one honest sentence. */
export function Scanning({ root, progress, paused, onPause, onResume, onCancel }: Props) {
  const { t, i18n } = useTranslation();
  const fraction = progress?.fraction ?? null;
  const pct = fraction === null ? null : Math.floor(fraction * 100);
  const files = useSmoothCounter(progress?.items_done ?? 0);
  const bytes = useSmoothCounter(progress?.bytes_done ?? 0);
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
            <p className="display num text-[44px] leading-[1.05]">{Math.round(files).toLocaleString(i18n.language)}</p>
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
    </div>
  );
}
