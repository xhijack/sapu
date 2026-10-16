import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { formatBytes } from "@/lib/format";
import { cleanApi } from "@/lib/cleanApi";
import { useErrorMessage } from "@/lib/errors";
import type { CleanRunReport } from "@/lib/types";
import type { RuleRow } from "./rows";

interface Props {
  rows: RuleRow[];
  onClose: () => void;
  onDone: (r: CleanRunReport) => void;
}

/** Preview → confirm → apply (PRD P3). Permanent rules need a second tick. */
export function CleanPreview({ rows, onClose, onDone }: Props) {
  const { t, i18n } = useTranslation();
  const errorMessage = useErrorMessage();
  const ref = useRef<HTMLDialogElement>(null);
  const [ack, setAck] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    ref.current?.showModal();
  }, []);

  const hasPermanent = rows.some((r) => r.permanent);
  const hasVolumes = rows.some((r) => r.confirmSeparately);
  const commands = rows.filter((r) => r.commandLine);
  const items = rows.reduce((a, r) => a + r.items, 0);
  const bytes = rows.reduce((a, r) => a + (r.bytes ?? 0), 0);

  const confirm = async () => {
    setBusy(true);
    try {
      const r = await cleanApi.execute(
        rows.map((r) => r.id),
        true,
        hasPermanent && ack,
      );
      onDone(r);
    } catch (e) {
      setError(errorMessage(e));
      setBusy(false);
    }
  };

  return (
    <dialog ref={ref} onClose={onClose} aria-labelledby="clean-preview-title" className="m-auto w-full max-w-lg rounded-xl border border-border bg-surface p-0 text-fg shadow-xl backdrop:bg-black/40">
      <div className="grid gap-4 p-6">
        <h2 id="clean-preview-title" className="text-lg font-semibold">
          {t("clean.preview_title")}
        </h2>
        <p className="text-fg-muted">{t("clean.preview_body", { rules: rows.length, items, size: formatBytes(bytes, i18n.language) })}</p>
        <ul className="max-h-48 overflow-y-auto rounded-md border border-border text-sm">
          {rows.map((r) => (
            <li key={r.id} className="flex items-center justify-between gap-3 border-b border-border px-3 py-2 last:border-0">
              <span className="truncate">{r.name}</span>
              <span className="num shrink-0 text-fg-muted">{r.bytes === null ? "—" : formatBytes(r.bytes, i18n.language)}</span>
            </li>
          ))}
        </ul>
        {commands.length > 0 && (
          <div className="grid gap-1 text-sm">
            <span className="text-fg-muted">{t("clean.preview_commands")}</span>
            {commands.map((c) => (
              <code key={c.id} className="path rounded bg-surface-muted px-2 py-1">
                {c.commandLine}
              </code>
            ))}
            {hasVolumes && <p className="text-risk-review-fg">{t("clean.preview_volume_warning")}</p>}
          </div>
        )}
        {hasPermanent && (
          <label className="flex items-start gap-3 rounded-md border border-risk-review-border bg-risk-review-bg p-3 text-sm text-risk-review-fg">
            <input type="checkbox" checked={ack} onChange={(e) => setAck(e.target.checked)} className="mt-0.5 h-4 w-4" />
            <span>
              <span className="block">{t("clean.preview_permanent")}</span>
              <span className="block font-medium">{t("clean.preview_permanent_ack")}</span>
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
          <Button onClick={() => void confirm()} disabled={busy || (hasPermanent && !ack)}>
            {busy ? t("clean.running") : t("clean.confirm")}
          </Button>
        </div>
      </div>
    </dialog>
  );
}
