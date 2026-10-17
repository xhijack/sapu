import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { ArrowDown, ArrowUp, Eye, FolderOpen, HelpCircle, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { KindIcon } from "@/components/icons/KindIcon";
import { formatBytes, formatDate } from "@/lib/format";
import { scanApi } from "@/lib/scanApi";
import type { FileKind, UnixTime } from "@/lib/types";
import { cn } from "@/lib/utils";
import { sortRows, type SortKey } from "./sortRows";

/** One row of any browser list: a file or a folder. */
export interface FileRow {
  path: string;
  name: string;
  kind: FileKind;
  is_dir: boolean;
  bytes: number;
  atime: UnixTime | null;
  mtime: UnixTime | null;
}

export type { SortKey } from "./sortRows";

interface Props {
  rows: FileRow[];
  /** Folders become links that open them. */
  onOpenDir?: (path: string) => void;
  onTrash: (paths: string[]) => void;
  onWhatIs?: (path: string) => void;
  /** Paths removed in this session: shown struck through, no actions. */
  removed?: Set<string>;
  defaultSort?: SortKey;
  /** Rendered after the name, e.g. a group badge. */
  nameExtra?: (row: FileRow) => React.ReactNode;
}

/**
 * Finder-like table: name, kind, last opened, size. Sortable by clicking a
 * header; Space previews with Quick Look; actions appear per row.
 */
export function FileTable({ rows, onOpenDir, onTrash, onWhatIs, removed, defaultSort = "size", nameExtra }: Props) {
  const { t, i18n } = useTranslation();
  const [sort, setSort] = useState<{ key: SortKey; desc: boolean }>({ key: defaultSort, desc: defaultSort !== "name" });
  const sorted = useMemo(() => sortRows(rows, sort.key, sort.desc), [rows, sort]);

  const header = (key: SortKey, label: string, align: "left" | "right" = "left") => {
    const active = sort.key === key;
    return (
      <th scope="col" className={cn("px-3 py-2 font-medium", align === "right" ? "text-right" : "text-left")} aria-sort={active ? (sort.desc ? "descending" : "ascending") : "none"}>
        <button type="button" className={cn("inline-flex items-center gap-1 rounded hover:text-fg", active ? "text-fg" : "text-fg-muted")} onClick={() => setSort((s) => ({ key, desc: s.key === key ? !s.desc : key !== "name" }))}>
          {label}
          {active && (sort.desc ? <ArrowDown aria-hidden className="h-3 w-3" /> : <ArrowUp aria-hidden className="h-3 w-3" />)}
        </button>
      </th>
    );
  };

  return (
    <div className="card-solid overflow-hidden">
      <table className="w-full table-fixed border-collapse text-sm">
        <colgroup>
          <col style={{ width: "44%" }} />
          <col style={{ width: "15%" }} />
          <col style={{ width: "19%" }} />
          <col style={{ width: "10%" }} />
          <col style={{ width: "132px" }} />
        </colgroup>
        <thead className="border-b border-border text-xs">
          <tr>
            {header("name", t("browse.col_name"))}
            {header("kind", t("browse.col_kind"))}
            {header("accessed", t("browse.col_accessed"))}
            {header("size", t("browse.col_size"), "right")}
            <th scope="col">
              <span className="sr-only">{t("browse.col_actions")}</span>
            </th>
          </tr>
        </thead>
        <tbody>
          {sorted.map((r) => {
            const gone = removed?.has(r.path) ?? false;
            return (
              <tr
                key={r.path}
                tabIndex={0}
                onKeyDown={(e) => {
                  if (e.key === " " && e.target === e.currentTarget) {
                    e.preventDefault();
                    void scanApi.quickLook(r.path);
                  }
                }}
                className={cn("group border-b border-border/60 outline-none last:border-0 odd:bg-surface-muted/40 focus-visible:bg-surface-muted hover:bg-surface-muted", gone && "opacity-50")}
              >
                <td className="min-w-0 px-3 py-1.5">
                  <span className="flex min-w-0 items-center gap-2">
                    <KindIcon kind={r.kind} size={16} />
                    {r.is_dir && onOpenDir && !gone ? (
                      <button type="button" className="min-w-0 truncate text-left font-medium hover:underline" title={r.path} onClick={() => onOpenDir(r.path)}>
                        {r.name}
                      </button>
                    ) : (
                      <span className={cn("min-w-0 truncate", gone && "line-through")} title={r.path}>
                        {r.name}
                      </span>
                    )}
                    {nameExtra?.(r)}
                  </span>
                </td>
                <td className="truncate px-3 py-1.5 text-fg-muted">{t(`kind.${r.kind}`)}</td>
                <td className="num truncate px-3 py-1.5 text-fg-muted">{r.atime ? formatDate(r.atime.secs_since_epoch, i18n.language) : r.mtime ? formatDate(r.mtime.secs_since_epoch, i18n.language) : t("browse.never")}</td>
                <td className="num px-3 py-1.5 text-right">{formatBytes(r.bytes, i18n.language)}</td>
                <td className="px-1 py-0.5">
                  {!gone && (
                    <span className="flex justify-end gap-0.5 opacity-0 transition-opacity group-focus-within:opacity-100 group-focus-visible:opacity-100 group-hover:opacity-100">
                      <Button variant="ghost" size="sm" aria-label={t("scan.quicklook")} title={t("scan.quicklook")} onClick={() => void scanApi.quickLook(r.path)}>
                        <Eye aria-hidden className="h-4 w-4" />
                      </Button>
                      {onWhatIs && (
                        <Button variant="ghost" size="sm" aria-label={t("ai.whatis")} title={t("ai.whatis")} onClick={() => onWhatIs(r.path)}>
                          <HelpCircle aria-hidden className="h-4 w-4" />
                        </Button>
                      )}
                      <Button variant="ghost" size="sm" aria-label={t("scan.action_reveal")} title={t("scan.action_reveal")} onClick={() => void scanApi.reveal(r.path)}>
                        <FolderOpen aria-hidden className="h-4 w-4" />
                      </Button>
                      <Button variant="ghost" size="sm" aria-label={t("scan.action_trash")} title={t("scan.action_trash")} onClick={() => onTrash([r.path])}>
                        <Trash2 aria-hidden className="h-4 w-4" />
                      </Button>
                    </span>
                  )}
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
