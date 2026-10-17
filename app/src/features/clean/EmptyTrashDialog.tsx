import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { AlertTriangle, ChevronDown, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Callout } from "@/components/ui/Callout";
import { AccessHelp } from "@/features/access/AccessHelp";
import { DialogClose } from "@/components/ui/DialogClose";
import { cleanApi } from "@/lib/cleanApi";
import { useErrorMessage } from "@/lib/errors";
import { formatBytes, formatElapsed, formatEta } from "@/lib/format";
import type { CleanRunProgress, CleanRunReport, TrashStatus } from "@/lib/types";

/** Items the run tried to remove and could not, with the system's reason. */
function failures(r: CleanRunReport): [string, string][] {
  return r.outcomes.flatMap((o) => (o.kind === "files" || o.kind === "project" ? o.session.entries : [])).flatMap((e) => (e.outcome.status === "failed" ? [[e.path, e.outcome.message] as [string, string]] : []));
}

/** Sapu may not read the Trash: say so, and how to allow it, instead of "emptying" nothing. */
export function TrashUnreadable() {
  const { t } = useTranslation();
  return (
    <Callout tone="review" title={t("trash.unreadable_title")}>
      <p>{t("trash.unreadable_body")}</p>
      <AccessHelp />
    </Callout>
  );
}

interface Props {
  status: TrashStatus | null;
  onClose: (result: CleanRunReport | null) => void;
}

/**
 * Empty the Trash on every volume: a warning with a second confirmation,
 * then live progress, then the freed size. The run continues if the
 * dialog is hidden; the status bar shows it.
 */
export function EmptyTrashDialog({ status, onClose }: Props) {
  const { t, i18n } = useTranslation();
  const errorMessage = useErrorMessage();
  const ref = useRef<HTMLDialogElement>(null);
  const [ack, setAck] = useState(false);
  const [phase, setPhase] = useState<"confirm" | "running" | "verifying" | "done">("confirm");
  // The Trash as it is after the run, checked again: "done" is never assumed.
  const [after, setAfter] = useState<TrashStatus | null>(null);
  const [progress, setProgress] = useState<CleanRunProgress | null>(null);
  const [result, setResult] = useState<CleanRunReport | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    ref.current?.showModal();
    const subs = [
      cleanApi.onRunProgress((p) => setProgress(p)),
      cleanApi.onRunDone((r) => {
        setResult(r);
        setPhase("verifying");
        cleanApi
          .trashStatus()
          .then((s) => setAfter(s))
          .catch(() => setAfter(null))
          .finally(() => setPhase("done"));
      }),
      cleanApi.onRunError((e) => {
        setError(errorMessage(e));
        setPhase("confirm");
      }),
    ];
    return () => {
      void Promise.all(subs).then((fns) => fns.forEach((f) => f()));
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const start = async () => {
    setError(null);
    setPhase("running");
    try {
      await cleanApi.emptyTrash();
    } catch (e) {
      setError(errorMessage(e));
      setPhase("confirm");
    }
  };

  const pct = progress?.fraction == null ? null : Math.round(progress.fraction * 100);
  return (
    <dialog ref={ref} onClose={() => onClose(result)} aria-labelledby="empty-trash-title" className="m-auto w-full max-w-lg card-solid p-0 text-fg shadow-xl backdrop:bg-black/40">
      <div className="grid gap-4 p-6">
        <div className="flex items-start justify-between gap-3">
          <h2 id="empty-trash-title" className="flex items-center gap-2 text-lg font-semibold">
          <Trash2 aria-hidden className="h-5 w-5" />
          {t("trash.title")}
        </h2>
          <DialogClose onClick={() => ref.current?.close()} />
        </div>
        {phase === "confirm" && status && status.unreadable.length > 0 && <TrashUnreadable />}
        {phase === "confirm" && !(status && status.unreadable.length > 0) && (
          <>
            <p className="num text-fg-muted">{status && status.items > 0 ? t("trash.body", { size: formatBytes(status.bytes, i18n.language), count: status.items.toLocaleString(i18n.language) }) : t("trash.body_unknown")}</p>
            <label className="flex items-start gap-3 rounded-md border border-risk-review-border bg-risk-review-bg p-3 text-sm text-risk-review-fg">
              <input type="checkbox" checked={ack} onChange={(e) => setAck(e.target.checked)} className="mt-0.5 h-4 w-4" />
              <span>
                <span className="flex items-center gap-1.5 font-medium">
                  <AlertTriangle aria-hidden className="h-4 w-4" />
                  {t("clean.preview_permanent")}
                </span>
                <span className="block">{t("clean.preview_permanent_ack")}</span>
              </span>
            </label>
            {error && (
              <p role="alert" className="text-risk-danger-fg">
                {error}
              </p>
            )}
            <div className="flex justify-end gap-2">
              <Button variant="secondary" onClick={() => ref.current?.close()}>
                {t("common.cancel")}
              </Button>
              <Button disabled={!ack} onClick={() => void start()}>
                {t("trash.confirm")}
              </Button>
            </div>
          </>
        )}
        {phase === "running" && (
          <>
            <div role="progressbar" aria-label={t("trash.running")} aria-valuemin={0} aria-valuemax={100} aria-valuenow={pct ?? undefined} className="h-2 w-full overflow-hidden rounded-full bg-surface-muted">
              <span className={`block h-full rounded-full bg-accent transition-[width] duration-300 motion-reduce:transition-none ${pct === null ? "slide w-1/3" : ""}`} style={pct === null ? undefined : { width: `${pct}%` }} />
            </div>
            <p className="text-sm">{t("trash.running")}</p>
            <p className="num text-sm text-fg-muted">
              {progress?.items_total ? t("clean.run_counts", { done: progress.items_done, total: progress.items_total }) : ""}
              {progress ? ` · ${formatEta(t, progress.eta)} · ${formatElapsed(t, progress.elapsed_secs)}` : ""}
            </p>
            <p className="text-sm text-fg-muted">{t("clean.run_note")}</p>
            <div className="flex justify-end">
              <Button variant="secondary" onClick={() => ref.current?.close()}>
                <ChevronDown aria-hidden className="h-4 w-4" />
                {t("clean.run_hide")}
              </Button>
            </div>
          </>
        )}
        {phase === "verifying" && <p className="text-sm">{t("trash.verifying")}</p>}
        {phase === "done" && result && (
          <>
            {after && after.items === 0 && after.unreadable.length === 0 && result.failed === 0 ? (
              <Callout title={t("trash.empty_now")}>{t("trash.done", { size: formatBytes(result.bytes_freed, i18n.language), count: result.disposed })}</Callout>
            ) : (
              <Callout tone="review" title={t("trash.not_empty")}>
                {after && after.items > 0 && <p className="num">{t("trash.left", { count: after.items, size: formatBytes(after.bytes, i18n.language) })}</p>}
                {after === null && <p>{t("trash.recheck_failed")}</p>}
                <p className="num">{t("trash.done", { size: formatBytes(result.bytes_freed, i18n.language), count: result.disposed })}</p>
                <p>{t("trash.left_hint")}</p>
              </Callout>
            )}
            {failures(result).length > 0 && (
              <ul className="max-h-40 overflow-y-auto rounded-md border border-border text-sm">
                {failures(result).map(([path, why]) => (
                  <li key={path} className="grid gap-0.5 border-b border-border px-3 py-2 last:border-0">
                    <span className="path truncate" title={path}>
                      {path.split("/").pop()}
                    </span>
                    <span className="text-risk-danger-fg">{why}</span>
                  </li>
                ))}
              </ul>
            )}
            <div className="flex justify-end">
              <Button onClick={() => ref.current?.close()}>{t("common.close")}</Button>
            </div>
          </>
        )}
      </div>
    </dialog>
  );
}
