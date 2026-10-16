import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Copy, HardDrive, Search, Sparkles, Usb } from "lucide-react";
import { api } from "@/lib/api";
import { formatBytes } from "@/lib/format";
import type { SystemSnapshot } from "@/lib/types";
import type { Screen } from "@/components/Shell";
import { cn } from "@/lib/utils";

const POLL_MS = 3000;

/** Home: the state of this computer right now, then one place to start. */
export function HomeView({ onNavigate }: { onNavigate: (s: Screen) => void }) {
  const { t, i18n } = useTranslation();
  const [snap, setSnap] = useState<SystemSnapshot | null>(null);

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

  const lang = i18n.language;
  const pct = (used: number, total: number) => (total > 0 ? Math.min(100, Math.round((used / total) * 100)) : 0);

  return (
    <div className="grid max-w-4xl gap-10">
      <div>
        <h1 className="text-2xl font-semibold">{t("home.title")}</h1>
        <p className="mt-1 text-fg-muted">{t("home.intro")}</p>
      </div>

      <section className="grid gap-3">
        <h2 className="text-sm font-medium text-fg-muted">{t("home.disks")}</h2>
        <ul className="grid gap-3 sm:grid-cols-2">
          {(snap?.disks ?? []).map((d) => {
            const used = d.total_bytes - d.available_bytes;
            const p = pct(used, d.total_bytes);
            return (
              <li key={d.mount} className="grid gap-3 rounded-xl border border-border bg-surface p-5">
                <div className="flex items-start justify-between gap-3">
                  <div className="min-w-0">
                    <div className="flex items-center gap-2 font-medium">
                      {d.removable ? <Usb aria-hidden className="h-4 w-4 text-fg-muted" /> : <HardDrive aria-hidden className="h-4 w-4 text-fg-muted" />}
                      <span className="truncate">{d.name || d.mount}</span>
                    </div>
                    <div className="path truncate text-fg-muted" title={d.mount}>
                      {d.mount}
                    </div>
                  </div>
                  <span className="num text-2xl font-semibold tabular-nums">{p}%</span>
                </div>
                <Gauge value={p} label={d.name || d.mount} />
                <div className="flex justify-between text-sm text-fg-muted">
                  <span className="num">{t("home.disk_used_of", { used: formatBytes(used, lang), total: formatBytes(d.total_bytes, lang) })}</span>
                  <span className="num">{t("home.disk_free", { free: formatBytes(d.available_bytes, lang) })}</span>
                </div>
              </li>
            );
          })}
        </ul>
      </section>

      <section className="grid gap-3 sm:grid-cols-2">
        <Meter
          title={t("home.cpu")}
          value={snap?.cpu_percent ?? null}
          detail={snap ? t("home.cpu_cores", { count: snap.cpu_count }) : ""}
          measuring={t("home.measuring")}
        />
        <Meter
          title={t("home.memory")}
          value={snap ? pct(snap.mem_used_bytes, snap.mem_total_bytes) : null}
          detail={
            snap
              ? `${t("home.disk_used_of", { used: formatBytes(snap.mem_used_bytes, lang), total: formatBytes(snap.mem_total_bytes, lang) })}${snap.swap_used_bytes > 0 ? `. ${t("home.swap", { size: formatBytes(snap.swap_used_bytes, lang) })}` : ""}`
              : ""
          }
          measuring={t("home.measuring")}
        />
      </section>

      <section className="grid gap-3">
        <h2 className="text-sm font-medium text-fg-muted">{t("home.actions")}</h2>
        <div className="grid divide-y divide-border rounded-xl border border-border bg-surface">
          {(
            [
              { id: "scan", Icon: Search, label: t("home.action_scan"), hint: t("home.action_scan_hint") },
              { id: "clean", Icon: Sparkles, label: t("home.action_clean"), hint: t("home.action_clean_hint") },
              { id: "dedupe", Icon: Copy, label: t("home.action_dedupe"), hint: t("home.action_dedupe_hint") },
            ] as { id: Screen; Icon: typeof Search; label: string; hint: string }[]
          ).map(({ id, Icon, label, hint }) => (
            <button key={id} type="button" onClick={() => onNavigate(id)} className="flex items-center gap-4 px-5 py-4 text-left first:rounded-t-xl last:rounded-b-xl hover:bg-surface-muted">
              <Icon aria-hidden className="h-5 w-5 text-accent" />
              <span>
                <span className="block font-medium">{label}</span>
                <span className="block text-sm text-fg-muted">{hint}</span>
              </span>
            </button>
          ))}
        </div>
      </section>
    </div>
  );
}

function Gauge({ value, label }: { value: number; label: string }) {
  const tone = value >= 90 ? "bg-risk-danger-fg" : value >= 75 ? "bg-risk-review-fg" : "bg-accent";
  return (
    <div role="meter" aria-label={label} aria-valuemin={0} aria-valuemax={100} aria-valuenow={value} className="h-2.5 w-full overflow-hidden rounded-full bg-surface-muted">
      <div className={cn("h-full rounded-full transition-[width] duration-700 motion-reduce:transition-none", tone)} style={{ width: `${value}%` }} />
    </div>
  );
}

function Meter({ title, value, detail, measuring }: { title: string; value: number | null; detail: string; measuring: string }) {
  return (
    <div className="grid gap-3 rounded-xl border border-border bg-surface p-5">
      <div className="flex items-baseline justify-between">
        <span className="font-medium">{title}</span>
        <span className="num text-2xl font-semibold">{value === null ? <span className="text-sm font-normal text-fg-muted">{measuring}</span> : `${Math.round(value)}%`}</span>
      </div>
      <Gauge value={value ?? 0} label={title} />
      <span className="num text-sm text-fg-muted">{detail}</span>
    </div>
  );
}
