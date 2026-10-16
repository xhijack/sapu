import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { formatBytes, formatDate } from "@/lib/format";
import type { Category, CleanSession, ScanSummary } from "@/lib/types";
import { cn } from "@/lib/utils";
import { CategoryList } from "./CategoryList";
import { DiskStrip } from "./DiskStrip";
import { LargestList } from "./LargestList";
import { Treemap } from "./Treemap";
import { TrashDialog } from "./TrashDialog";
import { AiReport } from "@/features/ai/AiReport";
import { AiSearch } from "@/features/ai/AiSearch";
import { WhatIsDialog } from "@/features/ai/WhatIsDialog";
import { useAiEnabled } from "@/features/ai/useAiEnabled";

type Tab = "summary" | "map" | "largest";

interface Props {
  summary: ScanSummary;
  onRescan: () => void;
  onNewScan: () => void;
}

export function Results({ summary, onRescan, onNewScan }: Props) {
  const { t, i18n } = useTranslation();
  const [tab, setTab] = useState<Tab>("summary");
  const [category, setCategory] = useState<Category | null>(null);
  const [trashing, setTrashing] = useState<string[] | null>(null);
  const [removed, setRemoved] = useState<Set<string>>(new Set());
  const [notice, setNotice] = useState<string | null>(null);
  const [whatIs, setWhatIs] = useState<string | null>(null);
  const aiEnabled = useAiEnabled();

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
        ? t("scan.trash_done", { count: s.disposed, size: formatBytes(s.bytes_freed, i18n.language) })
        : t("scan.trash_partial", { ok: s.disposed, skipped: s.skipped, failed: s.failed }),
    );
  };

  return (
    <div className="grid max-w-5xl gap-8">
      <header className="grid gap-1">
        <p className="num text-4xl font-semibold tracking-tight">
          {summary.disk_total_bytes
            ? t("scan.result_total_of", { size: formatBytes(summary.total_bytes, i18n.language), total: formatBytes(summary.disk_total_bytes, i18n.language) })
            : t("scan.result_total", { size: formatBytes(summary.total_bytes, i18n.language) })}
        </p>
        <p className="path truncate text-fg-muted" title={summary.root}>
          {summary.root}
        </p>
        <p className="text-sm text-fg-muted">
          {t("scan.result_counts", { files: summary.file_count.toLocaleString(i18n.language), dirs: summary.dir_count.toLocaleString(i18n.language) })}
        </p>
        {summary.diff && <DiffLine diff={summary.diff} />}
        {summary.cancelled && <p className="text-sm text-risk-review-fg">{t("scan.cancelled_note")}</p>}
        {summary.denied_count > 0 && <p className="text-sm text-fg-muted">{t("scan.denied_note", { count: summary.denied_count })}</p>}
        <p className="text-xs text-fg-muted">{t("scan.size_note")}</p>
      </header>

      <DiskStrip categories={summary.categories} total={summary.total_bytes} diskTotal={summary.disk_total_bytes} diskFree={summary.disk_free_bytes} />

      {notice && (
        <p role="status" className="rounded-md bg-surface-muted px-3 py-2 text-sm">
          {notice} {removed.size > 0 && t("scan.stale_note")}
        </p>
      )}

      <div className="flex items-center justify-between gap-4 border-b border-border">
        <div role="tablist" className="flex gap-1">
          {tabs.map((tb) => (
            <button
              key={tb.id}
              role="tab"
              type="button"
              aria-selected={tab === tb.id}
              onClick={() => setTab(tb.id)}
              className={cn("-mb-px border-b-2 px-3 py-2 text-sm font-medium", tab === tb.id ? "border-accent text-fg" : "border-transparent text-fg-muted hover:text-fg")}
            >
              {tb.label}
            </button>
          ))}
        </div>
        <div className="flex gap-2 pb-2">
          <Button variant="ghost" size="sm" onClick={onNewScan}>
            {t("scan.new_scan")}
          </Button>
          <Button variant="outline" size="sm" onClick={onRescan}>
            {t("scan.rescan")}
          </Button>
        </div>
      </div>

      <div role="tabpanel">
        {tab === "summary" && aiEnabled && (
          <div className="mb-6">
            <AiReport />
          </div>
        )}
        {tab === "summary" && (
          <CategoryList
            categories={summary.categories}
            total={summary.total_bytes}
            onSelect={(c) => {
              setCategory(c);
              setTab("largest");
            }}
          />
        )}
        {tab === "map" && <Treemap root={summary.root} />}
        {tab === "largest" && aiEnabled && (
          <div className="mb-6">
            <AiSearch onTrash={setTrashing} />
          </div>
        )}
        {tab === "largest" && (
          <LargestList key={category ?? "all"} initialCategory={category} onTrash={setTrashing} removed={removed} onCopied={() => setNotice(t("scan.copied"))} onWhatIs={aiEnabled ? setWhatIs : undefined} />
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
