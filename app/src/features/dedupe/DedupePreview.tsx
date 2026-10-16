import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { formatBytes, formatDuration } from "@/lib/format";
import { api } from "@/lib/api";
import { dedupeApi } from "@/lib/dedupeApi";
import { useErrorMessage } from "@/lib/errors";
import type { DedupeRunReport, DedupeSummary, KeepStrategy } from "@/lib/types";

interface Props {
  summary: DedupeSummary;
  strategy: KeepStrategy;
  picks: Record<number, string>;
  allowHardlink?: boolean;
  onClose: () => void;
  onDone: (r: DedupeRunReport) => void;
}

export function DedupePreview({ summary, strategy, picks, allowHardlink = false, onClose, onDone }: Props) {
  const { t, i18n } = useTranslation();
  const errorMessage = useErrorMessage();
  const ref = useRef<HTMLDialogElement>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [hardlink, setHardlink] = useState(false);
  const [estimate, setEstimate] = useState<number | null>(null);

  useEffect(() => {
    ref.current?.showModal();
    let live = true;
    api.estimateDuration("dedupe", Math.max(summary.wasted_bytes, 1)).then((s) => live && setEstimate(s)).catch(() => undefined);
    return () => {
      live = false;
    };
  }, [summary.wasted_bytes]);

  const confirm = async () => {
    setBusy(true);
    try {
      onDone(await dedupeApi.execute(strategy, picks, [], true, hardlink));
    } catch (e) {
      setError(errorMessage(e));
      setBusy(false);
    }
  };

  return (
    <dialog ref={ref} onClose={onClose} aria-labelledby="dedupe-preview-title" className="m-auto w-full max-w-lg rounded-xl border border-border bg-surface p-0 text-fg shadow-xl backdrop:bg-black/40">
      <div className="grid gap-4 p-6">
        <h2 id="dedupe-preview-title" className="text-lg font-semibold">
          {t("dedupe.preview_title")}
        </h2>
        <p className="text-fg-muted">
          {t("dedupe.preview_body", { files: summary.duplicate_files.toLocaleString(i18n.language), groups: summary.groups.toLocaleString(i18n.language), size: formatBytes(summary.wasted_bytes, i18n.language) })}
        </p>
        {estimate != null && <p className="num text-sm text-fg-muted">{t("dedupe.estimate_line", { time: formatDuration(t, estimate) })}</p>}
        {allowHardlink && (
          <label className="flex items-start gap-3 rounded-md border border-border p-3 text-sm">
            <input type="checkbox" checked={hardlink} onChange={(e) => setHardlink(e.target.checked)} className="mt-0.5 h-4 w-4" />
            <span>
              <span className="block font-medium">{t("dedupe.hardlink")}</span>
              <span className="block text-fg-muted">{t("dedupe.hardlink_hint")}</span>
            </span>
          </label>
        )}
        {error && (
          <p role="alert" className="text-risk-danger-fg">
            {error}
          </p>
        )}
        <div className="flex justify-end gap-2">
          <Button variant="secondary" onClick={() => ref.current?.close()}>
            {t("common.cancel")}
          </Button>
          <Button onClick={() => void confirm()} disabled={busy}>
            {busy ? t("dedupe.running") : t("dedupe.confirm")}
          </Button>
        </div>
      </div>
    </dialog>
  );
}
