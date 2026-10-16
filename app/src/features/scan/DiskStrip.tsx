import { useTranslation } from "react-i18next";
import type { CategorySummary } from "@/lib/types";
import { formatBytes } from "@/lib/format";
import { categoryColor } from "./categoryColor";

/** The one memorable element: the whole scan as a single shelf of space. */
interface Props {
  categories: CategorySummary[];
  total: number;
  /** When known, the strip is drawn against the whole volume: scanned
   *  categories, then other data on the volume, then free space. */
  diskTotal?: number | null;
  diskFree?: number | null;
}

export function DiskStrip({ categories, total, diskTotal, diskFree }: Props) {
  const { t, i18n } = useTranslation();
  const parts = categories.filter((c) => c.bytes > 0);
  const volume = diskTotal && diskTotal > total ? diskTotal : total;
  const free = diskTotal && diskFree != null ? Math.min(diskFree, Math.max(0, volume - total)) : 0;
  const other = diskTotal ? Math.max(0, volume - total - free) : 0;
  return (
    <div role="img" aria-label={t("scan.strip_label")} className="flex h-9 w-full overflow-hidden rounded-md bg-surface-muted">
      {parts.map((c) => {
        const pct = volume > 0 ? (c.bytes / volume) * 100 : 0;
        return (
          <div
            key={c.category}
            data-category={c.category}
            title={`${t(`category.${c.category}`)}: ${formatBytes(c.bytes, i18n.language)}`}
            style={{ width: `${pct}%`, background: categoryColor(c.category) }}
            className="h-full min-w-px transition-[width] duration-700 ease-out motion-reduce:transition-none"
          />
        );
      })}
      {other > 0 && <div data-category="other-volume" title={`${t("scan.strip_other_volume")}: ${formatBytes(other, i18n.language)}`} style={{ width: `${(other / volume) * 100}%` }} className="h-full bg-border" />}
      {free > 0 && <div data-category="free" title={`${t("scan.strip_free")}: ${formatBytes(free, i18n.language)}`} style={{ width: `${(free / volume) * 100}%` }} className="h-full bg-surface" />}
    </div>
  );
}
