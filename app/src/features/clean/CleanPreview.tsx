import { useEffect, useRef, useState } from "react";
import { ChevronDown } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { DialogClose } from "@/components/ui/DialogClose";
import { formatBytes, formatDuration, formatElapsed, formatEta } from "@/lib/format";
import { api } from "@/lib/api";
import { cleanApi } from "@/lib/cleanApi";
import { useErrorMessage } from "@/lib/errors";
import type { CleanRunProgress, CleanRunReport } from "@/lib/types";
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
  const [progress, setProgress] = useState<CleanRunProgress | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const subs = [
      cleanApi.onRunProgress((p) => setProgress(p)),
      cleanApi.onRunDone((r) => onDone(r)),
      cleanApi.onRunError((e) => {
        setError(errorMessage(e));
        setBusy(false);
      }),
    ];
    return () => {
      void Promise.all(subs).then((fns) => fns.forEach((f) => f()));
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  const [estimate, setEstimate] = useState<number | null>(null);

  useEffect(() => {
    ref.current?.showModal();
  }, []);

  const items = rows.reduce((a, r) => a + r.items, 0);
  useEffect(() => {
    let live = true;
    api.estimateDuration("clean", Math.max(items, 1)).then((s) => live && setEstimate(s)).catch(() => undefined);
    return () => {
      live = false;
    };
  }, [items]);

  const hasPermanent = rows.some((r) => r.permanent);
  const hasVolumes = rows.some((r) => r.confirmSeparately);
  const commands = rows.filter((r) => r.commandLine);
  const bytes = rows.reduce((a, r) => a + (r.bytes ?? 0), 0);

  const confirm = async () => {
    setBusy(true);
    setError(null);
    try {
      await cleanApi.run(
        rows.filter((r) => !r.artefactPath).map((r) => r.id),
        rows.flatMap((r) => (r.artefactPath ? [r.artefactPath] : [])),
        true,
        hasPermanent && ack,
      );
    } catch (e) {
      setError(errorMessage(e));
      setBusy(false);
    }
  };

  const stepLabel = (p: CleanRunProgress | null) => {
    if (!p?.step) return t("clean.run_preparing");
    if (p.step.kind === "command") return t("clean.run_command", { cmd: p.step.command_line });
    const name = rows.find((r) => r.id === p.step!.rule_id || r.artefactPath === p.step!.rule_id)?.name ?? p.step.rule_id.split("/").pop() ?? p.step.rule_id;
    return t("clean.run_rule", { name, count: p.step.items });
  };

  if (busy) {
    const pct = progress?.fraction == null ? null : Math.round(progress.fraction * 100);
    return (
      <dialog ref={ref} onClose={onClose} aria-labelledby="clean-run-title" className="m-auto w-full max-w-lg card-solid p-0 text-fg shadow-xl backdrop:bg-black/40">
        <div className="grid gap-4 p-6">
          <div className="flex items-start justify-between gap-3">
            <h2 id="clean-run-title" className="text-lg font-semibold">
            {t("clean.run_title")}
          </h2>
            <DialogClose onClick={() => ref.current?.close()} />
          </div>
          <div role="progressbar" aria-label={t("clean.run_title")} aria-valuemin={0} aria-valuemax={100} aria-valuenow={pct ?? undefined} className="h-2 w-full overflow-hidden rounded-full bg-surface-muted">
            <span className={`block h-full rounded-full bg-accent transition-[width] duration-300 motion-reduce:transition-none ${pct === null ? "slide w-1/3" : ""}`} style={pct === null ? undefined : { width: `${pct}%` }} />
          </div>
          <p className="text-sm">{stepLabel(progress)}</p>
          {progress?.step?.kind === "command" && progress.step.last_line && (
            <p aria-label={t("clean.run_output")} className="path truncate rounded-md bg-surface-muted px-2 py-1 text-xs" title={progress.step.last_line}>
              {progress.step.last_line}
            </p>
          )}
          <p className="num text-sm text-fg-muted">
            {progress?.items_total ? t("clean.run_counts", { done: progress.items_done, total: progress.items_total }) : ""}
            {progress ? ` · ${formatEta(t, progress.eta)} · ${formatElapsed(t, progress.elapsed_secs)}` : ""}
          </p>
          <p className="text-sm text-fg-muted">{t("clean.run_note")}</p>
          {error && (
            <p role="alert" className="text-risk-danger-fg">
              {error}
            </p>
          )}
          <div className="flex justify-end">
            <Button variant="secondary" onClick={() => ref.current?.close()}>
              <ChevronDown aria-hidden className="h-4 w-4" />
              {t("clean.run_hide")}
            </Button>
          </div>
        </div>
      </dialog>
    );
  }

  return (
    <dialog ref={ref} onClose={onClose} aria-labelledby="clean-preview-title" className="m-auto w-full max-w-lg card-solid p-0 text-fg shadow-xl backdrop:bg-black/40">
      <div className="grid gap-4 p-6">
        <div className="flex items-start justify-between gap-3">
          <h2 id="clean-preview-title" className="text-lg font-semibold">
          {t("clean.preview_title")}
        </h2>
          <DialogClose onClick={() => ref.current?.close()} />
        </div>
        <p className="text-fg-muted">
          {t("clean.preview_body", { rules: rows.length, items, size: formatBytes(bytes, i18n.language) })}
          {estimate != null && ` ${t("clean.estimate_line", { time: formatDuration(t, estimate) })}.`}
        </p>
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
