import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { EmptyState, LoadingState } from "@/components/States";
import { formatBytes } from "@/lib/format";
import { scanApi } from "@/lib/scanApi";
import type { DownloadGroup, DownloadsReport } from "@/lib/types";
import { cn } from "@/lib/utils";
import { FileTable, type FileRow } from "./FileTable";

const DAY = 86_400;

interface Props {
  onTrash: (paths: string[]) => void;
  removed: Set<string>;
  onWhatIs?: (path: string) => void;
  /** Bumps to reload (after a trash action). */
  refreshKey?: number;
}

/**
 * Downloads organizer: everything at the top of ~/Downloads grouped by what
 * it is. Installers and archives older than a month are the usual suspects.
 */
export function DownloadsPanel({ onTrash, removed, onWhatIs, refreshKey = 0 }: Props) {
  const { t, i18n } = useTranslation();
  const lang = i18n.language;
  const [loaded, setLoaded] = useState<{ report: DownloadsReport | null; now: number } | undefined>(undefined);
  const report = loaded?.report;
  const now = loaded?.now ?? 0;
  const [group, setGroup] = useState<DownloadGroup | null>(null);
  const [oldOnly, setOldOnly] = useState(false);

  useEffect(() => {
    let live = true;
    scanApi
      .downloads()
      .then((r) => live && setLoaded({ report: r, now: Math.floor(Date.now() / 1000) }))
      .catch(() => live && setLoaded({ report: null, now: 0 }));
    return () => {
      live = false;
    };
  }, [refreshKey]);

  const visible = useMemo(() => {
    if (!report) return [];
    return report.entries.filter((e) => !removed.has(e.path)).filter((e) => (group ? e.group === group : true)).filter((e) => (oldOnly ? (e.mtime?.secs_since_epoch ?? 0) < now - 30 * DAY : true));
  }, [report, removed, group, oldOnly, now]);
  const rows: FileRow[] = visible.map((e) => ({ path: e.path, name: e.name, kind: e.kind, is_dir: e.is_dir, bytes: e.bytes, atime: e.atime, mtime: e.mtime }));
  const visibleBytes = visible.reduce((a, e) => a + e.bytes, 0);
  const staleInstallers = report?.entries.filter((e) => !removed.has(e.path) && (e.group === "installers" || e.group === "archives") && (e.mtime?.secs_since_epoch ?? 0) < now - 30 * DAY) ?? [];

  if (report === undefined) return <LoadingState />;
  if (report === null || report.count === 0) return <EmptyState title={t("browse.downloads_empty_title")} body={t("browse.downloads_empty_body")} />;

  return (
    <div className="grid gap-4">
      <div className="flex flex-wrap items-baseline justify-between gap-3">
        <p className="num text-sm text-fg-muted">
          <span className="path">{report.dir}</span> · {t("browse.downloads_total", { count: report.count, size: formatBytes(report.bytes, lang) })}
        </p>
        {staleInstallers.length > 0 && (
          <Button size="sm" variant="secondary" onClick={() => onTrash(staleInstallers.map((e) => e.path))}>
            <Trash2 aria-hidden className="h-4 w-4" />
            {t("browse.downloads_stale_action", { count: staleInstallers.length, size: formatBytes(staleInstallers.reduce((a, e) => a + e.bytes, 0), lang) })}
          </Button>
        )}
      </div>
      <div role="radiogroup" aria-label={t("browse.downloads_groups")} className="flex flex-wrap gap-2">
        <GroupChip active={group === null} label={t("browse.group_all")} size={formatBytes(report.bytes, lang)} onClick={() => setGroup(null)} />
        {report.groups.map((g) => (
          <GroupChip key={g.group} active={group === g.group} label={t(`browse.group.${g.group}`)} size={formatBytes(g.bytes, lang)} count={g.count} onClick={() => setGroup(g.group)} />
        ))}
        <label className="ml-auto flex items-center gap-2 text-sm text-fg-muted">
          <input type="checkbox" checked={oldOnly} onChange={(e) => setOldOnly(e.target.checked)} className="h-4 w-4 accent-[var(--accent)]" />
          {t("browse.downloads_old_only")}
        </label>
      </div>
      {rows.length === 0 ? (
        <EmptyState title={t("scan.no_match")} body="" />
      ) : (
        <>
          <FileTable rows={rows} onTrash={onTrash} onWhatIs={onWhatIs} removed={removed} />
          <p className="num text-right text-sm text-fg-muted">{t("browse.selection_total", { count: rows.length, size: formatBytes(visibleBytes, lang) })}</p>
        </>
      )}
    </div>
  );
}

function GroupChip({ active, label, size, count, onClick }: { active: boolean; label: string; size: string; count?: number; onClick: () => void }) {
  return (
    <button type="button" role="radio" aria-checked={active} onClick={onClick} className={cn("rounded-full border px-3 py-1 text-sm transition-colors", active ? "border-accent bg-accent-soft text-fg" : "border-border text-fg-muted hover:text-fg")}>
      {label}
      <span className="num ml-1.5 text-xs opacity-70">
        {size}
        {count !== undefined && ` · ${count}`}
      </span>
    </button>
  );
}
