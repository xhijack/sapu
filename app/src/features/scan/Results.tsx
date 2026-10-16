import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { SpaceRing, type RingSegment } from "@/components/viz/SpaceRing";
import { useCountUp } from "@/components/viz/useCountUp";
import { formatBytes, formatDate } from "@/lib/format";
import type { Category, CleanSession, Profile, ScanSummary } from "@/lib/types";
import { useEffect } from "react";
import { cleanApi } from "@/lib/cleanApi";
import { Tile } from "@/components/ui/tile";
import { Copy, Search, Sparkles } from "lucide-react";
import { cn } from "@/lib/utils";
import { CategoryTiles } from "./CategoryTiles";
import { LargestList } from "./LargestList";
import { Sunburst } from "./Sunburst";
import { Treemap } from "./Treemap";
import { TrashDialog } from "./TrashDialog";
import { categoryColor } from "./categoryColor";
import { AiReport } from "@/features/ai/AiReport";
import { AiSearch } from "@/features/ai/AiSearch";
import { WhatIsDialog } from "@/features/ai/WhatIsDialog";
import { useAiEnabled } from "@/features/ai/useAiEnabled";

type Tab = "summary" | "map" | "largest";
type MapMode = "sunburst" | "treemap";

interface Props {
  summary: ScanSummary;
  profile?: Profile;
  onRescan: () => void;
  onNewScan: () => void;
  onNavigate?: (s: "clean" | "dedupe") => void;
}

export function Results({ summary, profile = "general", onRescan, onNewScan, onNavigate }: Props) {
  const { t, i18n } = useTranslation();
  const lang = i18n.language;
  const [tab, setTab] = useState<Tab>("summary");
  const [mapMode, setMapMode] = useState<MapMode>("sunburst");
  const [category, setCategory] = useState<Category | null>(null);
  const [trashing, setTrashing] = useState<string[] | null>(null);
  const [removed, setRemoved] = useState<Set<string>>(new Set());
  const [notice, setNotice] = useState<string | null>(null);
  const [whatIs, setWhatIs] = useState<string | null>(null);
  const aiEnabled = useAiEnabled();
  const total = useCountUp(summary.total_bytes);
  // What next: evaluate the clean rules right after a scan so the first
  // action already carries a number.
  const [safeBytes, setSafeBytes] = useState<number | null | undefined>(undefined);
  useEffect(() => {
    let live = true;
    cleanApi
      .evaluate(profile)
      .then((ev) => {
        if (!live) return;
        const ids = new Set(ev.preselected);
        const bytes =
          ev.report.findings.filter((f) => ids.has(f.rule_id)).reduce((a, f) => a + f.bytes, 0) +
          ev.report.commands.filter((c) => ids.has(c.rule_id)).reduce((a, c) => a + (c.estimate_bytes ?? 0), 0) +
          ev.report.projects.flatMap((p) => p.artefacts).filter((a) => ev.preselected_artefacts.includes(a.path)).reduce((a, x) => a + x.bytes, 0);
        setSafeBytes(bytes);
      })
      .catch(() => live && setSafeBytes(null));
    return () => {
      live = false;
    };
  }, [profile, summary.finished_at_unix]);

  const diskTotal = summary.disk_total_bytes ?? summary.total_bytes;
  const free = summary.disk_free_bytes ?? 0;
  const ringSegments: RingSegment[] = summary.categories.filter((c) => c.bytes > 0).map((c) => ({ key: c.category, value: c.bytes, color: categoryColor(c.category), label: `${t(`category.${c.category}`)}: ${formatBytes(c.bytes, lang)}` }));
  const other = Math.max(0, diskTotal - summary.total_bytes - free);
  if (other > 0) ringSegments.push({ key: "other-volume", value: other, color: "var(--border-strong)", label: `${t("scan.strip_other_volume")}: ${formatBytes(other, lang)}` });

  const tabs: { id: Tab; label: string }[] = [
    { id: "summary", label: t("scan.tab_summary") },
    { id: "map", label: t("scan.tab_map") },
    { id: "largest", label: t("scan.tab_largest") },
  ];

  const onTrashClosed = (s: CleanSession | null) => {
    setTrashing(null);
    if (!s) return;
    const done = s.entries.filter((e) => e.outcome.status === "disposed").map((e) => e.path);
    setRemoved((r) => new Set([...r, ...done]));
    setNotice(
      s.skipped === 0 && s.failed === 0
        ? t("scan.trash_done", { count: s.disposed, size: formatBytes(s.bytes_freed, lang) })
        : t("scan.trash_partial", { ok: s.disposed, skipped: s.skipped, failed: s.failed }),
    );
  };

  return (
    <div className="grid max-w-[1040px] gap-8">
      <header className="rise grid items-center gap-8 md:grid-cols-[auto_1fr]">
        <SpaceRing size={200} thickness={18} total={diskTotal} segments={ringSegments} label={t("scan.ring_label")}>
          <div className="grid gap-0.5">
            <span className="display num text-[30px] leading-none">{diskTotal > 0 ? Math.round((summary.total_bytes / diskTotal) * 100) : 100}%</span>
            <span className="text-[11px] text-fg-muted">{summary.root.split("/").filter(Boolean).pop() || summary.root}</span>
          </div>
        </SpaceRing>
        <div className="grid gap-3">
          <div className="grid gap-1">
            <p data-testid="scan-total" className="display num text-[44px] leading-[1.05]">
              {formatBytes(total, lang)}
              <span className="ml-3 text-xl font-semibold text-fg-muted">{summary.disk_total_bytes ? t("scan.result_used_of", { total: formatBytes(summary.disk_total_bytes, lang) }) : t("scan.result_used")}</span>
            </p>
            <p className="path truncate text-fg-muted" title={summary.root}>
              {summary.root}
            </p>
            <p className="num text-sm text-fg-muted">
              {t("scan.result_counts", { files: summary.file_count.toLocaleString(lang), dirs: summary.dir_count.toLocaleString(lang) })}
            </p>
            {summary.diff && <DiffLine diff={summary.diff} />}
            {summary.cancelled && <p className="text-sm text-risk-review-fg">{t("scan.cancelled_note")}</p>}
            {summary.denied_count > 0 && <p className="text-sm text-fg-muted">{t("scan.denied_note", { count: summary.denied_count })}</p>}
          </div>
          <div className="flex flex-wrap gap-2">
            <Button variant="outline" size="sm" onClick={onRescan}>
              {t("scan.rescan")}
            </Button>
            <Button variant="ghost" size="sm" onClick={onNewScan}>
              {t("scan.new_scan")}
            </Button>
          </div>
        </div>
      </header>

      <section className="grid gap-3">
        <h2 className="text-sm font-semibold text-fg-muted">{t("scan.next_title")}</h2>
        <div className="grid gap-4 md:grid-cols-3">
          <Tile
            className="rise rise-1"
            tone="var(--accent)"
            icon={<Sparkles className="h-5 w-5" />}
            title={t("scan.next_clean")}
            body={safeBytes === undefined ? t("scan.next_clean_checking") : safeBytes ? t("scan.next_clean_body", { size: formatBytes(safeBytes, lang) }) : t("scan.next_clean_none")}
            onClick={() => onNavigate?.("clean")}
          />
          <Tile className="rise rise-2" tone="var(--cat-application)" icon={<Copy className="h-5 w-5" />} title={t("scan.next_dupes")} body={t("scan.next_dupes_body")} onClick={() => onNavigate?.("dedupe")} />
          <Tile className="rise rise-3" tone="var(--cat-video)" icon={<Search className="h-5 w-5" />} title={t("scan.next_largest")} body={t("scan.next_largest_body")} onClick={() => setTab("largest")} />
        </div>
      </section>

      {notice && (
        <p role="status" className="card-solid px-4 py-2 text-sm">
          {notice} {removed.size > 0 && t("scan.stale_note")}
        </p>
      )}

      <div className="flex flex-wrap items-center justify-between gap-3">
        <div role="tablist" className="inline-flex rounded-full bg-surface-muted p-1">
          {tabs.map((tb) => (
            <button
              key={tb.id}
              role="tab"
              type="button"
              aria-selected={tab === tb.id}
              onClick={() => setTab(tb.id)}
              className={cn("rounded-full px-4 py-1.5 text-sm font-medium transition-colors", tab === tb.id ? "bg-surface-solid text-fg shadow-[var(--shadow-1)]" : "text-fg-muted hover:text-fg")}
            >
              {tb.label}
            </button>
          ))}
        </div>
        {tab === "map" && (
          <div role="radiogroup" aria-label={t("scan.tab_map")} className="inline-flex rounded-full bg-surface-muted p-1 text-sm">
            {(["sunburst", "treemap"] as MapMode[]).map((m) => (
              <button key={m} type="button" role="radio" aria-checked={mapMode === m} onClick={() => setMapMode(m)} className={cn("rounded-full px-3 py-1 font-medium", mapMode === m ? "bg-surface-solid shadow-[var(--shadow-1)]" : "text-fg-muted")}>
                {t(`scan.map_${m}`)}
              </button>
            ))}
          </div>
        )}
      </div>

      <div role="tabpanel">
        {tab === "summary" && (
          <div className="grid gap-6">
            {aiEnabled && <AiReport />}
            <h2 className="text-sm font-semibold text-fg-muted">{t("scan.categories_title")}</h2>
            <CategoryTiles
              categories={summary.categories}
              total={summary.total_bytes}
              onSelect={(c) => {
                setCategory(c);
                setTab("largest");
              }}
            />
          </div>
        )}
        {tab === "map" && (mapMode === "sunburst" ? <Sunburst root={summary.root} onTrash={setTrashing} /> : <Treemap root={summary.root} />)}
        {tab === "largest" && (
          <div className="grid gap-6">
            {aiEnabled && <AiSearch onTrash={setTrashing} />}
            <LargestList key={category ?? "all"} initialCategory={category} onTrash={setTrashing} removed={removed} onCopied={() => setNotice(t("scan.copied"))} onWhatIs={aiEnabled ? setWhatIs : undefined} />
          </div>
        )}
      </div>

      {trashing && <TrashDialog paths={trashing} onClose={onTrashClosed} />}
      {whatIs && <WhatIsDialog path={whatIs} onClose={() => setWhatIs(null)} />}
    </div>
  );
}

function DiffLine({ diff }: { diff: NonNullable<ScanSummary["diff"]> }) {
  const { t, i18n } = useTranslation();
  const date = formatDate(diff.since_unix, i18n.language);
  const size = formatBytes(Math.abs(diff.delta_bytes), i18n.language);
  const head = diff.delta_bytes > 0 ? t("scan.diff_up", { size, date }) : diff.delta_bytes < 0 ? t("scan.diff_down", { size, date }) : t("scan.diff_same", { date });
  const main = diff.dirs.filter((d) => Math.sign(d.delta_bytes) === Math.sign(diff.delta_bytes)).slice(0, 2);
  return (
    <p data-testid="scan-diff" className="num text-sm text-fg-muted">
      {head}
      {main.length > 0 && (
        <>
          , {t("scan.diff_mainly")}{" "}
          {main.map((d, i) => (
            <span key={d.path}>
              {i > 0 && ", "}
              <span className="path" title={d.path}>
                {d.path.split("/").pop()}
              </span>{" "}
              ({diff.delta_bytes > 0 ? "+" : "−"}
              {formatBytes(Math.abs(d.delta_bytes), i18n.language)}
              {d.is_new ? `, ${t("scan.diff_new")}` : ""})
            </span>
          ))}
        </>
      )}
    </p>
  );
}
