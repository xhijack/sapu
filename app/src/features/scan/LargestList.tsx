import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Copy, FolderOpen, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { EmptyState, LoadingState } from "@/components/States";
import { formatBytes, formatDate } from "@/lib/format";
import { scanApi } from "@/lib/scanApi";
import { CATEGORIES, type AgeFilter, type Category, type DirSummary, type FileEntry, type FileFilter } from "@/lib/types";
import { categoryColor } from "./categoryColor";

const MB = 1024 ** 2;
const MIN_SIZES: { value: number; label: string }[] = [
  { value: 0, label: "" },
  { value: 10 * MB, label: "10 MB" },
  { value: 100 * MB, label: "100 MB" },
  { value: 1024 * MB, label: "1 GB" },
];

interface Props {
  initialCategory?: Category | null;
  onTrash: (paths: string[]) => void;
  removed: Set<string>;
  onCopied: () => void;
}

export function LargestList({ initialCategory = null, onTrash, removed, onCopied }: Props) {
  const { t, i18n } = useTranslation();
  const [filter, setFilter] = useState<FileFilter>({ min_bytes: 0, category: initialCategory, age: "any" });
  const [files, setFiles] = useState<FileEntry[] | null>(null);
  const [dirs, setDirs] = useState<DirSummary[] | null>(null);

  useEffect(() => {
    let live = true;
    scanApi.topFiles(filter, 100).then((f) => live && setFiles(f)).catch(() => live && setFiles([]));
    return () => {
      live = false;
    };
  }, [filter]);

  useEffect(() => {
    let live = true;
    scanApi.topDirs(30).then((d) => live && setDirs(d)).catch(() => live && setDirs([]));
    return () => {
      live = false;
    };
  }, []);

  const copy = async (p: string) => {
    try {
      await navigator.clipboard.writeText(p);
      onCopied();
    } catch {
      /* clipboard unavailable: nothing to do */
    }
  };

  const select = (id: string, label: string, value: string, options: { value: string; label: string }[], onChange: (v: string) => void) => (
    <label className="grid gap-1 text-sm">
      <span className="text-fg-muted">{label}</span>
      <select id={id} value={value} onChange={(e) => onChange(e.target.value)} className="h-9 rounded-md border border-border bg-surface px-2">
        {options.map((o) => (
          <option key={o.value} value={o.value}>
            {o.label}
          </option>
        ))}
      </select>
    </label>
  );

  const rowActions = (path: string) => (
    <span className="flex shrink-0 gap-1">
      <Button variant="ghost" size="sm" aria-label={t("scan.action_reveal")} title={t("scan.action_reveal")} onClick={() => void scanApi.reveal(path)}>
        <FolderOpen aria-hidden className="h-4 w-4" />
      </Button>
      <Button variant="ghost" size="sm" aria-label={t("scan.action_copy")} title={t("scan.action_copy")} onClick={() => void copy(path)}>
        <Copy aria-hidden className="h-4 w-4" />
      </Button>
      <Button variant="ghost" size="sm" aria-label={t("scan.action_trash")} title={t("scan.action_trash")} onClick={() => onTrash([path])}>
        <Trash2 aria-hidden className="h-4 w-4" />
      </Button>
    </span>
  );

  const visibleFiles = files?.filter((f) => !removed.has(f.path)) ?? null;

  return (
    <div className="grid gap-8">
      <section className="grid gap-3">
        <h3 className="text-lg font-semibold">{t("scan.largest_files")}</h3>
        <div className="flex flex-wrap gap-3">
          {select("f-cat", t("scan.filter_category"), filter.category ?? "any", [{ value: "any", label: t("scan.filter_category_any") }, ...CATEGORIES.map((c) => ({ value: c, label: t(`category.${c}`) }))], (v) =>
            setFilter({ ...filter, category: v === "any" ? null : (v as Category) }),
          )}
          {select(
            "f-age",
            t("scan.filter_age"),
            filter.age,
            [
              { value: "any", label: t("scan.filter_age_any") },
              { value: "older_than_month", label: t("scan.filter_age_month") },
              { value: "older_than_six_months", label: t("scan.filter_age_six") },
              { value: "older_than_year", label: t("scan.filter_age_year") },
            ],
            (v) => setFilter({ ...filter, age: v as AgeFilter }),
          )}
          {select(
            "f-min",
            t("scan.filter_min"),
            String(filter.min_bytes),
            MIN_SIZES.map((m) => ({ value: String(m.value), label: m.value === 0 ? t("scan.filter_min_any") : m.label })),
            (v) => setFilter({ ...filter, min_bytes: Number(v) }),
          )}
        </div>
        {visibleFiles === null ? (
          <LoadingState />
        ) : visibleFiles.length === 0 ? (
          <EmptyState title={t("scan.no_match")} body="" />
        ) : (
          <ul className="divide-y divide-border">
            {visibleFiles.map((f) => (
              <li key={f.path} className="flex items-center gap-3 py-2">
                <span aria-hidden className="h-3 w-3 shrink-0 rounded-sm" style={{ background: categoryColor(f.category) }} />
                <span className="min-w-0 flex-1">
                  <span className="path block truncate" title={f.path}>
                    {f.path}
                  </span>
                  <span className="block text-xs text-fg-muted">
                    {t(`category.${f.category}`)}
                    {f.mtime && `, ${formatDate(f.mtime.secs_since_epoch, i18n.language)}`}
                  </span>
                </span>
                <span className="num w-20 shrink-0 text-right">{formatBytes(f.size, i18n.language)}</span>
                {rowActions(f.path)}
              </li>
            ))}
          </ul>
        )}
      </section>
      <section className="grid gap-3">
        <h3 className="text-lg font-semibold">{t("scan.largest_folders")}</h3>
        {dirs === null ? (
          <LoadingState />
        ) : (
          <ul className="divide-y divide-border">
            {dirs.map((d) => (
              <li key={d.path} className="flex items-center gap-3 py-2">
                <span className="min-w-0 flex-1">
                  <span className="path block truncate" title={d.path}>
                    {d.path}
                  </span>
                  <span className="block text-xs text-fg-muted">{t("scan.files_count", { count: d.files })}</span>
                </span>
                <span className="num w-20 shrink-0 text-right">{formatBytes(d.bytes, i18n.language)}</span>
                {rowActions(d.path)}
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
