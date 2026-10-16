import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Copy, HardDrive, History, Search, ShieldCheck, Sparkles, Usb } from "lucide-react";
import { api } from "@/lib/api";
import { scanApi } from "@/lib/scanApi";
import { formatBytes, formatDate } from "@/lib/format";
import type { HistoryTotals, PreviousScan, ScanLogEntry, SystemSnapshot } from "@/lib/types";
import type { Screen } from "@/components/Shell";
import { Button } from "@/components/ui/button";
import { Tile } from "@/components/ui/tile";
import { SpaceRing } from "@/components/viz/SpaceRing";
import { useCountUp } from "@/components/viz/useCountUp";
import { cn } from "@/lib/utils";

const POLL_MS = 3000;

function greeting(t: (k: string) => string) {
  const h = new Date().getHours();
  return h < 11 ? t("home.greeting_morning") : h < 18 ? t("home.greeting_day") : t("home.greeting_evening");
}

/** Home: the space ring for the main disk, one action, three doors. */
export function HomeView({ onNavigate, onScanNow }: { onNavigate: (s: Screen) => void; onScanNow?: () => void }) {
  const { t, i18n } = useTranslation();
  const [snap, setSnap] = useState<SystemSnapshot | null>(null);
  const [previous, setPrevious] = useState<PreviousScan | null>(null);
  const [totals, setTotals] = useState<HistoryTotals | null>(null);
  const [recent, setRecent] = useState<ScanLogEntry[]>([]);

  useEffect(() => {
    let live = true;
    const tick = () => api.systemSnapshot().then((s) => live && setSnap(s)).catch(() => undefined);
    void tick();
    const id = setInterval(() => void tick(), POLL_MS);
    scanApi
      .targets()
      .then((tg) => (tg.home ? scanApi.previous(tg.home) : null))
      .then((p) => live && setPrevious(p))
      .catch(() => undefined);
    api.historyTotals().then((x) => live && setTotals(x)).catch(() => undefined);
    scanApi.history(4).then((x) => live && setRecent(x)).catch(() => undefined);
    return () => {
      live = false;
      clearInterval(id);
    };
  }, []);

  const lang = i18n.language;
  const main = snap?.disks.find((d) => !d.removable) ?? snap?.disks[0] ?? null;
  const others = snap?.disks.filter((d) => d !== main) ?? [];
  const used = main ? main.total_bytes - main.available_bytes : 0;
  const pct = main && main.total_bytes > 0 ? Math.round((used / main.total_bytes) * 100) : 0;
  const freeAnimated = useCountUp(main?.available_bytes ?? 0);

  return (
    <div className="grid max-w-[1040px] gap-10">
      <section className="rise grid items-center gap-10 md:grid-cols-[auto_1fr]">
        <SpaceRing
          size={236}
          thickness={20}
          total={main?.total_bytes ?? 1}
          label={main ? `${main.name || main.mount}: ${pct}%` : t("home.measuring")}
          segments={main ? [{ key: "used", value: used, color: pct >= 90 ? "var(--gauge-danger)" : pct >= 75 ? "var(--gauge-warn)" : "var(--gauge-ok)" }] : []}
        >
          <div className="grid gap-0.5">
            <span className="display num text-[40px] leading-none">{pct}%</span>
            <span className="text-xs text-fg-muted">{main?.name || main?.mount || ""}</span>
          </div>
        </SpaceRing>
        <div className="grid gap-5">
          <div className="grid gap-1">
            <p className="text-fg-muted">{greeting(t)}</p>
            <p className="display num text-[52px] leading-[1.05]">
              {main ? formatBytes(freeAnimated, lang) : t("home.measuring")}
              {main && <span className="ml-3 text-2xl font-semibold text-fg-muted">{t("home.hero_free_word")}</span>}
            </p>
            {main && <p className="num text-fg-muted">{t("home.hero_of", { total: formatBytes(main.total_bytes, lang), pct })}</p>}
          </div>
          <div className="flex flex-wrap items-center gap-4">
            <Button size="lg" onClick={() => (onScanNow ? onScanNow() : onNavigate("scan"))}>
              <Search aria-hidden className="h-5 w-5" />
              {t("home.hero_cta")}
            </Button>
            <span className="num text-sm text-fg-muted">
              {previous ? t("home.hero_last", { date: formatDate(previous.finished_at_unix, lang), size: formatBytes(previous.total_bytes, lang) }) : t("home.hero_never")}
            </span>
          </div>
          <p className="flex items-center gap-2 text-sm text-fg-faint">
            <ShieldCheck aria-hidden className="h-4 w-4 text-accent" />
            {t("home.safety_line")}
          </p>
        </div>
      </section>

      <section className="grid gap-4 md:grid-cols-3">
        <Tile className="rise rise-2" tone="var(--cat-video)" icon={<Search className="h-5 w-5" />} title={t("home.tile_scan")} body={t("home.tile_scan_body")} onClick={() => onNavigate("scan")} />
        <Tile className="rise rise-3" tone="var(--accent)" icon={<Sparkles className="h-5 w-5" />} title={t("home.tile_clean")} body={t("home.tile_clean_body")} onClick={() => onNavigate("clean")} />
        <Tile className="rise rise-4" tone="var(--cat-application)" icon={<Copy className="h-5 w-5" />} title={t("home.tile_dedupe")} body={t("home.tile_dedupe_body")} onClick={() => onNavigate("dedupe")} />
      </section>

      <section className="grid gap-4 md:grid-cols-[1.2fr_1fr_1fr]">
        <div className="card rise grid gap-3 p-5">
          <h2 className="flex items-center gap-2 text-sm font-semibold text-fg-muted">
            <History aria-hidden className="h-4 w-4" />
            {t("home.activity")}
          </h2>
          {totals && totals.sessions > 0 ? (
            <div>
              <p className="display num text-2xl">{t("home.freed_total", { size: formatBytes(totals.bytes_freed, lang) })}</p>
              <p className="num text-xs text-fg-muted">{t("home.freed_sessions", { count: totals.sessions, items: totals.items_disposed.toLocaleString(lang) })}</p>
            </div>
          ) : (
            <p className="text-sm text-fg-faint">{t("home.freed_none")}</p>
          )}
          {recent.length > 0 && (
            <div className="grid gap-1">
              <span className="text-xs text-fg-muted">{t("home.recent_scans")}</span>
              <ul className="grid gap-1 text-sm">
                {recent.map((r) => (
                  <li key={`${r.root}-${r.finished_at_unix}`} className="flex items-baseline justify-between gap-3">
                    <span className="path min-w-0 truncate" title={r.root}>
                      {r.root.split("/").filter(Boolean).pop() || r.root}
                    </span>
                    <span className="num shrink-0 text-xs text-fg-muted">
                      {formatDate(r.finished_at_unix, lang)}, {t("home.scan_line", { size: formatBytes(r.total_bytes, lang), files: r.file_count.toLocaleString(lang) })}
                    </span>
                  </li>
                ))}
              </ul>
            </div>
          )}
          <button type="button" onClick={() => onNavigate("history")} className="w-fit text-xs text-accent underline-offset-2 hover:underline">
            {t("clean.see_history")}
          </button>
        </div>
        <div className="card rise grid gap-4 p-5">
          <h2 className="text-sm font-semibold text-fg-muted">{t("home.system_now")}</h2>
          <div className="grid gap-6">
            <div className="grid gap-2">
              <MiniMeter
                label={t("home.cpu")}
                value={snap?.cpu_percent ?? null}
                detail={snap ? t("home.cores_hint", { count: snap.cpu_count, avg: Math.round(snap.cpu_percent ?? 0) }) : ""}
                measuring={t("home.measuring")}
              />
              {snap && snap.cpu_per_core.length > 0 && (
                <ul aria-label={t("home.cpu")} className="flex h-6 items-end gap-[3px]">
                  {snap.cpu_per_core.map((c, i) => (
                    <li key={i} title={`#${i + 1}: ${Math.round(c)}%`} className="flex-1 rounded-sm bg-surface-muted">
                      <div className={cn("w-full rounded-sm transition-[height] duration-500", c >= 90 ? "bg-gauge-danger" : c >= 75 ? "bg-gauge-warn" : "bg-gauge-ok")} style={{ height: `${Math.max(8, Math.round(c))}%` }} />
                    </li>
                  ))}
                </ul>
              )}
            </div>
            <MiniMeter
              label={t("home.memory")}
              value={snap ? Math.round((snap.mem_used_bytes / Math.max(1, snap.mem_total_bytes)) * 100) : null}
              detail={snap ? t("home.disk_used_of", { used: formatBytes(snap.mem_used_bytes, lang), total: formatBytes(snap.mem_total_bytes, lang) }) : ""}
              measuring={t("home.measuring")}
            />
          </div>
        </div>
        <div className="card rise grid gap-3 p-5">
          <h2 className="text-sm font-semibold text-fg-muted">{t("home.other_disks")}</h2>
          {others.length === 0 ? (
            <p className="text-sm text-fg-faint">—</p>
          ) : (
            <ul className="grid gap-3">
              {others.map((d) => {
                const u = d.total_bytes - d.available_bytes;
                const p = d.total_bytes > 0 ? Math.round((u / d.total_bytes) * 100) : 0;
                return (
                  <li key={d.mount} className="flex items-center gap-3">
                    <SpaceRing size={44} thickness={6} total={d.total_bytes} label={`${d.name || d.mount}: ${p}%`} segments={[{ key: "used", value: u, color: "var(--accent)" }]}>
                      <span className="num text-[10px] font-semibold">{p}%</span>
                    </SpaceRing>
                    <span className="min-w-0 flex-1">
                      <span className="flex items-center gap-1.5 text-sm font-medium">
                        {d.removable ? <Usb aria-hidden className="h-3.5 w-3.5 text-fg-muted" /> : <HardDrive aria-hidden className="h-3.5 w-3.5 text-fg-muted" />}
                        <span className="truncate">{d.name || d.mount}</span>
                      </span>
                      <span className="num block text-xs text-fg-muted">{t("home.disk_used_of", { used: formatBytes(u, lang), total: formatBytes(d.total_bytes, lang) })}</span>
                    </span>
                  </li>
                );
              })}
            </ul>
          )}
        </div>
      </section>
    </div>
  );
}

function MiniMeter({ label, value, detail, measuring }: { label: string; value: number | null; detail: string; measuring: string }) {
  const v = value ?? 0;
  const tone = v >= 90 ? "bg-gauge-danger" : v >= 75 ? "bg-gauge-warn" : "bg-gauge-ok";
  return (
    <div className="grid gap-2">
      <div className="flex items-baseline justify-between">
        <span className="text-sm font-medium">{label}</span>
        <span className="display num text-xl">{value === null ? <span className="text-xs font-normal text-fg-muted">{measuring}</span> : `${Math.round(value)}%`}</span>
      </div>
      <div role="meter" aria-label={label} aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(v)} className="h-1.5 w-full overflow-hidden rounded-full bg-surface-muted">
        <div className={cn("h-full rounded-full transition-[width] duration-700 motion-reduce:transition-none", tone)} style={{ width: `${v}%` }} />
      </div>
      <span className="num text-xs text-fg-muted">{detail}</span>
    </div>
  );
}
