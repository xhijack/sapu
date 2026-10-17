import { Callout } from "@/components/ui/Callout";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { AlertTriangle, Info, MemoryStick, RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import { DialogClose } from "@/components/ui/DialogClose";
import { memoryApi } from "@/lib/memoryApi";
import { useErrorMessage } from "@/lib/errors";
import { formatBytes } from "@/lib/format";
import type { MemorySnapshot, PurgeOutcome, QuitOutcome } from "@/lib/types";
import { cn } from "@/lib/utils";

const POLL_MS = 3000;

interface Props {
  os: string;
  onClose: () => void;
}

type Phase = "list" | "confirm" | "quitting" | "result";
type Purge = { state: "idle" } | { state: "running" } | { state: "done"; outcome: PurgeOutcome } | { state: "error"; message: string };

const levelTone = (level: MemorySnapshot["pressure"]["level"]) => (level === "high" ? "bg-gauge-danger" : level === "elevated" ? "bg-gauge-warn" : "bg-gauge-ok");

/**
 * "Bebaskan memori": what uses memory and two honest ways to ease it.
 * Only the user's own apps can be ticked; they are asked to quit the
 * normal way (never killed). On macOS a secondary, optional purge of the
 * disk cache is offered with a plain explanation of how little it does.
 */
export function MemoryDialog({ os, onClose }: Props) {
  const { t, i18n } = useTranslation();
  const errorMessage = useErrorMessage();
  const ref = useRef<HTMLDialogElement>(null);
  const [snap, setSnap] = useState<MemorySnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [phase, setPhase] = useState<Phase>("list");
  const [outcomes, setOutcomes] = useState<QuitOutcome[]>([]);
  const [purge, setPurge] = useState<Purge>({ state: "idle" });

  const refresh = useCallback(() => {
    memoryApi
      .snapshot()
      .then((s) => {
        setSnap(s);
        setError(null);
        // Drop ticks for rows that are gone or no longer quittable.
        setSelected((sel) => new Set([...sel].filter((pid) => s.processes.some((p) => p.pid === pid && p.quittable))));
      })
      .catch((e) => setError(errorMessage(e)));
    // errorMessage only depends on the active language.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    ref.current?.showModal();
    refresh();
    const id = setInterval(refresh, POLL_MS);
    return () => clearInterval(id);
  }, [refresh]);

  const lang = i18n.language;
  const toggle = (pid: number) =>
    setSelected((s) => {
      const n = new Set(s);
      if (n.has(pid)) n.delete(pid);
      else n.add(pid);
      return n;
    });

  const quit = async () => {
    setPhase("quitting");
    try {
      const out = await memoryApi.quitApps([...selected]);
      setOutcomes(out);
      setSelected(new Set());
      setPhase("result");
      refresh();
    } catch (e) {
      setError(errorMessage(e));
      setPhase("list");
    }
  };

  const runPurge = async () => {
    setPurge({ state: "running" });
    try {
      const outcome = await memoryApi.purge();
      setPurge({ state: "done", outcome });
      refresh();
    } catch (e) {
      setPurge({ state: "error", message: errorMessage(e) });
    }
  };

  const p = snap?.pressure ?? null;
  return (
    <dialog ref={ref} onClose={onClose} aria-labelledby="memory-title" className="m-auto w-full max-w-2xl card-solid p-0 text-fg shadow-xl backdrop:bg-black/40">
      <div className="grid gap-5 p-6">
        <div className="flex items-start justify-between gap-3">
          <div className="grid gap-1">
            <h2 id="memory-title" className="flex items-center gap-2 text-lg font-semibold">
              <MemoryStick aria-hidden className="h-5 w-5" />
              {t("memory.title")}
            </h2>
            <p className="text-sm text-fg-muted">{t("memory.intro")}</p>
          </div>
          <DialogClose onClick={() => ref.current?.close()} />
        </div>

        {error && (
          <p role="alert" className="text-sm text-risk-danger-fg">
            {error}
          </p>
        )}

        {p && (
          <section aria-label={t("memory.title")} className="grid gap-2 rounded-xl bg-surface-muted p-4">
            <div className="flex flex-wrap items-baseline gap-x-4 gap-y-1">
              <span className="display num text-[32px] leading-none">{p.used_pct}%</span>
              <span className="flex items-center gap-1.5 text-sm font-medium">
                <span aria-hidden className={cn("h-2 w-2 rounded-full", levelTone(p.level))} />
                {t(`memory.level_${p.level}`)}
              </span>
            </div>
            <p className="num text-sm text-fg-muted">
              {t("memory.used", { used: formatBytes(p.used_bytes, lang), total: formatBytes(p.total_bytes, lang), pct: p.used_pct })}
              {p.swap_used_bytes > 0 && <span> · {t("memory.swap", { size: formatBytes(p.swap_used_bytes, lang) })}</span>}
            </p>
            <p className="flex items-start gap-2 text-xs text-fg-muted">
              <Info aria-hidden className="mt-0.5 h-3.5 w-3.5 shrink-0" />
              {t("memory.honest")}
            </p>
          </section>
        )}

        <section className="grid gap-2">
          <div className="flex items-center justify-between gap-3">
            <h3 className="text-sm font-semibold text-fg-muted">{t("memory.processes")}</h3>
            <Button variant="ghost" size="sm" onClick={refresh}>
              <RefreshCw aria-hidden className="h-3.5 w-3.5" />
              {t("memory.refresh")}
            </Button>
          </div>
          {snap && snap.processes.length === 0 && <p className="text-sm text-fg-faint">{t("memory.empty")}</p>}
          {!snap && !error && <p className="text-sm text-fg-faint">{t("common.loading")}</p>}
          {snap && snap.processes.length > 0 && (
            <ul className="grid max-h-64 gap-1 overflow-y-auto">
              {snap.processes.map((proc) => (
                <li key={proc.pid}>
                  <label className={cn("flex items-center gap-3 rounded-lg px-2 py-1.5 text-sm", proc.quittable ? "hover:bg-surface-muted" : "text-fg-muted")} title={proc.quittable ? (proc.exe ?? undefined) : t("memory.not_quittable")}>
                    <input type="checkbox" className="h-4 w-4" checked={selected.has(proc.pid)} disabled={!proc.quittable || phase === "quitting"} onChange={() => toggle(proc.pid)} aria-label={proc.name} aria-describedby={proc.quittable ? undefined : `mem-sys-${proc.pid}`} />
                    <span className="min-w-0 flex-1 truncate font-medium">{proc.name}</span>
                    {proc.process_count > 1 && <span className="num shrink-0 text-xs text-fg-faint">{t("memory.helpers", { count: proc.process_count })}</span>}
                    {!proc.quittable && (
                      <span id={`mem-sys-${proc.pid}`} className="shrink-0 text-xs text-fg-faint">
                        {t("memory.not_quittable")}
                      </span>
                    )}
                    <span className="num w-20 shrink-0 text-right">{formatBytes(proc.memory_bytes, lang)}</span>
                  </label>
                </li>
              ))}
            </ul>
          )}
          <p className="text-xs text-fg-faint">{t("memory.select_hint")}</p>
        </section>

        {phase === "list" && (
          <div className="flex justify-end">
            <Button disabled={selected.size === 0} onClick={() => setPhase("confirm")}>
              {t("memory.quit_button", { count: selected.size })}
            </Button>
          </div>
        )}
        {phase === "confirm" && (
          <div className="grid gap-3 rounded-xl border border-risk-review-border bg-risk-review-bg p-4 text-risk-review-fg">
            <p className="flex items-center gap-2 font-semibold">
              <AlertTriangle aria-hidden className="h-4 w-4" />
              {t("memory.confirm_title", { count: selected.size })}
            </p>
            <p className="text-sm">{t("memory.confirm_body")}</p>
            <div className="flex justify-end gap-2">
              <Button variant="secondary" onClick={() => setPhase("list")}>
                {t("common.cancel")}
              </Button>
              <Button onClick={() => void quit()}>{t("memory.confirm_button")}</Button>
            </div>
          </div>
        )}
        {phase === "quitting" && <p className="text-sm text-fg-muted">{t("memory.quitting")}</p>}
        {phase === "result" && (
          <div className="grid gap-2 rounded-xl bg-surface-muted p-4 text-sm">
            <ul className="grid gap-1">
              {outcomes.map((o) => (
                <li key={o.pid}>{o.requested ? t("memory.outcome_requested", { name: o.name }) : o.method === "refused" ? t("memory.outcome_refused", { name: o.name || o.pid }) : t("memory.outcome_failed", { name: o.name || o.pid, message: o.message })}</li>
              ))}
            </ul>
            <p className="text-xs text-fg-muted">{t("memory.outcome_note")}</p>
            <div className="flex justify-end">
              <Button variant="secondary" size="sm" onClick={() => setPhase("list")}>
                {t("common.done")}
              </Button>
            </div>
          </div>
        )}

        {os === "macos" && (
          <section className="grid gap-2 border-t border-border pt-4">
            <h3 className="text-sm font-semibold text-fg-muted">{t("memory.purge_title")}</h3>
            <Callout>{t("memory.purge_body")}</Callout>
            <div className="flex flex-wrap items-center justify-between gap-3">
              <span className="num text-sm">
                {purge.state === "running" && t("memory.purge_running")}
                {purge.state === "done" && purge.outcome.ok && t("memory.purge_done", { before: formatBytes(purge.outcome.used_before_bytes, lang), after: formatBytes(purge.outcome.used_after_bytes, lang) })}
                {purge.state === "done" && !purge.outcome.ok && (purge.outcome.cancelled ? t("memory.purge_cancelled") : t("memory.purge_failed", { message: purge.outcome.message }))}
                {purge.state === "error" && purge.message}
              </span>
              <Button variant="outline" size="sm" disabled={purge.state === "running"} onClick={() => void runPurge()}>
                {t("memory.purge_button")}
              </Button>
            </div>
          </section>
        )}

        <p className="text-xs text-fg-faint">{t("memory.not_done")}</p>
      </div>
    </dialog>
  );
}
