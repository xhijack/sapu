import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { formatBytes } from "@/lib/format";
import { dedupeApi } from "@/lib/dedupeApi";
import { useErrorMessage } from "@/lib/errors";
import type { DedupeRunReport, DedupeSummary, KeepStrategy } from "@/lib/types";

interface Props {
  summary: DedupeSummary;
  strategy: KeepStrategy;
  picks: Record<number, string>;
  onClose: () => void;
  onDone: (r: DedupeRunReport) => void;
}

export function DedupePreview({ summary, strategy, picks, onClose, onDone }: Props) {
  const { t, i18n } = useTranslation();
  const errorMessage = useErrorMessage();
  const ref = useRef<HTMLDialogElement>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    ref.current?.showModal();
  }, []);

  const confirm = async () => {
    setBusy(true);
    try {
      onDone(await dedupeApi.execute(strategy, picks, [], true));
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
