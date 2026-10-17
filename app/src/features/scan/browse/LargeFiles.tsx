import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { EmptyState, LoadingState } from "@/components/States";
import { scanApi } from "@/lib/scanApi";
import { CATEGORIES, type AgeFilter, type Category, type FileEntry, type FileFilter } from "@/lib/types";
import { AiSearch } from "@/features/ai/AiSearch";
import { FileTable, type FileRow } from "./FileTable";
import { FilterSelect } from "./FilterSelect";

const MB = 1024 ** 2;
const MIN_SIZES = [0, 10 * MB, 100 * MB, 1024 * MB];

interface Props {
  initialCategory?: Category | null;
  onTrash: (paths: string[]) => void;
  removed: Set<string>;
  onWhatIs?: (path: string) => void;
  aiEnabled?: boolean;
}

/** The biggest files of the scan with the three filters people actually use. */
export function LargeFiles({ initialCategory = null, onTrash, removed, onWhatIs, aiEnabled }: Props) {
  const { t, i18n } = useTranslation();
  const [filter, setFilter] = useState<FileFilter>({ min_bytes: 0, category: initialCategory, age: "any" });
  // Results are kept with the filter they answer, so a filter change shows
  // the loading state without an extra set-state inside the effect.
  const [loaded, setLoaded] = useState<{ filter: FileFilter; files: FileEntry[] } | null>(null);
  const files = loaded && loaded.filter === filter ? loaded.files : null;

  useEffect(() => {
    let live = true;
    scanApi
      .topFiles(filter, 500)
      .then((f) => live && setLoaded({ filter, files: f }))
      .catch(() => live && setLoaded({ filter, files: [] }));
    return () => {
      live = false;
    };
  }, [filter]);

  const rows: FileRow[] = useMemo(() => (files ?? []).map((f) => ({ path: f.path, name: f.path.split("/").pop() ?? f.path, kind: f.kind, is_dir: false, bytes: f.size, atime: f.atime, mtime: f.mtime })), [files]);

  return (
    <div className="grid gap-4">
      {aiEnabled && <AiSearch onTrash={onTrash} />}
      <div className="flex flex-wrap items-end gap-3">
        <FilterSelect id="f-cat" label={t("scan.filter_category")} value={filter.category ?? "any"} options={[{ value: "any", label: t("scan.filter_category_any") }, ...CATEGORIES.map((c) => ({ value: c, label: t(`category.${c}`) }))]} onChange={(v) => setFilter({ ...filter, category: v === "any" ? null : (v as Category) })} />
        <FilterSelect
          id="f-age"
          label={t("scan.filter_age")}
          value={filter.age}
          options={[
            { value: "any", label: t("scan.filter_age_any") },
            { value: "older_than_month", label: t("scan.filter_age_month") },
            { value: "older_than_six_months", label: t("scan.filter_age_six") },
            { value: "older_than_year", label: t("scan.filter_age_year") },
          ]}
          onChange={(v) => setFilter({ ...filter, age: v as AgeFilter })}
        />
        <FilterSelect id="f-min" label={t("scan.filter_min")} value={String(filter.min_bytes)} options={MIN_SIZES.map((m) => ({ value: String(m), label: m === 0 ? t("scan.filter_min_any") : m >= 1024 * MB ? "1 GB" : `${m / MB} MB` }))} onChange={(v) => setFilter({ ...filter, min_bytes: Number(v) })} />
        {files && <span className="num ml-auto text-sm text-fg-muted">{t("browse.count", { count: files.length, lng: i18n.language })}</span>}
      </div>
      {files === null ? <LoadingState /> : rows.length === 0 ? <EmptyState title={t("scan.no_match")} body="" /> : <FileTable rows={rows} onTrash={onTrash} onWhatIs={onWhatIs} removed={removed} />}
    </div>
  );
}
