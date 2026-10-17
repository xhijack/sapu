import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Cpu, HardDrive, MemoryStick } from "lucide-react";
import { api } from "@/lib/api";
import { careApi } from "@/lib/careApi";
import { cleanApi } from "@/lib/cleanApi";
import { dedupeApi } from "@/lib/dedupeApi";
import { formatBytes, formatEta } from "@/lib/format";
import { scanApi } from "@/lib/scanApi";
import type { Eta, SystemSnapshot } from "@/lib/types";
import { cn } from "@/lib/utils";
import type { Screen } from "./Shell";

const POLL_MS = 5000;

type Task = { screen: Screen; label: string; fraction: number | null; eta: Eta } | null;

const tone = (pct: number) => (pct >= 90 ? "bg-gauge-danger" : pct >= 75 ? "bg-gauge-warn" : "bg-gauge-ok");

/**
 * One quiet line at the bottom of every screen: main disk, memory, CPU
 * (with one bar per core) and whatever long task is running right now.
 * Replaces the big "computer now" card so Home has room to breathe.
 */
export function StatusBar({ onNavigate }: { onNavigate: (s: Screen) => void }) {
  const { t, i18n } = useTranslation();
  const [snap, setSnap] = useState<SystemSnapshot | null>(null);
  const [task, setTask] = useState<Task>(null);

  useEffect(() => {
    let live = true;
    const tick = () => api.systemSnapshot().then((s) => live && setSnap(s)).catch(() => undefined);
    void tick();
    const id = setInterval(() => void tick(), POLL_MS);
    return () => {
      live = false;
      clearInterval(id);
    };
  }, []);

  useEffect(() => {
    const clear = () => setTask(null);
    const subs = [
      scanApi.onProgress((p) => setTask({ screen: "scan", label: "task_scan", fraction: p.fraction, eta: p.eta })),
      scanApi.onDone(clear),
      scanApi.onError(clear),
      careApi.onProgress((p) => setTask({ screen: "care", label: "task_care", fraction: p.fraction, eta: p.eta })),
      careApi.onDone(clear),
      careApi.onError(clear),
      dedupeApi.onProgress((p) => setTask({ screen: "dedupe", label: "task_dedupe", fraction: p.fraction, eta: p.eta })),
      dedupeApi.onDone(clear),
      dedupeApi.onError(clear),
      cleanApi.onProgress((p) => setTask({ screen: "clean", label: "task_clean", fraction: p.fraction, eta: p.eta })),
      cleanApi.onDone(clear),
      cleanApi.onError(clear),
      cleanApi.onRunProgress((p) => setTask({ screen: "clean", label: "task_clean_run", fraction: p.fraction, eta: p.eta })),
      cleanApi.onRunDone(clear),
      cleanApi.onRunError(clear),
    ];
    return () => {
      void Promise.all(subs).then((fns) => fns.forEach((f) => f()));
    };
  }, []);

  const lang = i18n.language;
  const main = snap?.disks.find((d) => !d.removable) ?? snap?.disks[0] ?? null;
  const diskPct = main && main.total_bytes > 0 ? Math.round(((main.total_bytes - main.available_bytes) / main.total_bytes) * 100) : 0;
  const memPct = snap ? Math.round((snap.mem_used_bytes / Math.max(1, snap.mem_total_bytes)) * 100) : null;
  const cpuPct = snap?.cpu_percent == null ? null : Math.round(snap.cpu_percent);

  return (
    <footer aria-label={t("statusbar.label")} className="flex h-8 shrink-0 items-center gap-4 overflow-hidden whitespace-nowrap border-t border-border bg-surface px-4 text-[12px] text-fg-muted" style={{ backdropFilter: "blur(16px)" }}>
      <span className="flex shrink-0 items-center gap-1.5" title={main ? `${main.name || main.mount}: ${diskPct}%` : undefined}>
        <HardDrive aria-hidden className="h-3.5 w-3.5" />
        <span aria-hidden className={cn("h-1.5 w-1.5 rounded-full", tone(diskPct))} />
        <span className="num">{main ? t("statusbar.disk", { size: formatBytes(main.available_bytes, lang), total: formatBytes(main.total_bytes, lang), pct: 100 - diskPct }) : t("home.measuring")}</span>
        {main && main.available_bytes < main.recommended_free_bytes && (
          <span className="num rounded-full bg-risk-review-bg px-1.5 text-[11px] text-risk-review-fg" title={t("statusbar.disk_low_hint")}>
            {t("statusbar.disk_low", { size: formatBytes(main.recommended_free_bytes, lang) })}
          </span>
        )}
      </span>
      <span className="flex shrink-0 items-center gap-1.5" title={snap ? t("home.disk_used_of", { used: formatBytes(snap.mem_used_bytes, lang), total: formatBytes(snap.mem_total_bytes, lang) }) : undefined}>
        <MemoryStick aria-hidden className="h-3.5 w-3.5" />
        <span aria-hidden className={cn("h-1.5 w-1.5 rounded-full", tone(memPct ?? 0))} />
        <span className="num">{memPct === null ? "…" : t("statusbar.memory", { pct: memPct })}</span>
      </span>
      <span className="flex shrink-0 items-center gap-1.5" title={snap ? t("home.cores_hint", { count: snap.cpu_count, avg: cpuPct ?? 0 }) : undefined}>
        <Cpu aria-hidden className="h-3.5 w-3.5" />
        <span className="num">{cpuPct === null ? "…" : t("statusbar.cpu", { pct: cpuPct })}</span>
        {snap && snap.cpu_per_core.length > 0 && (
          <span role="img" aria-label={t("home.cores_hint", { count: snap.cpu_count, avg: cpuPct ?? 0 })} className="flex h-3 items-end gap-[2px]">
            {snap.cpu_per_core.map((c, i) => (
              <span key={i} title={`#${i + 1}: ${Math.round(c)}%`} className="flex h-full w-[3px] items-end rounded-[1px] bg-surface-muted">
                <span className={cn("block w-full rounded-[1px] transition-[height] duration-500 motion-reduce:transition-none", tone(c))} style={{ height: `${Math.max(15, Math.round(c))}%` }} />
              </span>
            ))}
          </span>
        )}
      </span>
      {task && (
        <button type="button" onClick={() => onNavigate(task.screen)} className="ml-auto flex min-w-0 items-center gap-2 rounded-md px-2 py-0.5 text-fg hover:bg-surface-muted" aria-label={t("statusbar.open_task")}>
          <span role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={task.fraction === null ? undefined : Math.round(task.fraction * 100)} className="h-1 w-24 overflow-hidden rounded-full bg-surface-muted">
            <span className={cn("block h-full rounded-full bg-accent transition-[width] duration-500 motion-reduce:transition-none", task.fraction === null && "slide w-1/3")} style={task.fraction === null ? undefined : { width: `${task.fraction * 100}%` }} />
          </span>
          <span className="truncate">
            {t(`statusbar.${task.label}`)}
            {task.fraction !== null && <span className="num"> · {Math.round(task.fraction * 100)}%</span>}
            <span className="text-fg-muted"> · {formatEta(t, task.eta)}</span>
          </span>
        </button>
      )}
    </footer>
  );
}
