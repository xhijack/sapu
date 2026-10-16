import { useTranslation } from "react-i18next";
import type { CategorySummary } from "@/lib/types";
import { formatBytes } from "@/lib/format";
import { categoryColor } from "./categoryColor";

/** The one memorable element: the whole scan as a single shelf of space. */
export function DiskStrip({ categories, total }: { categories: CategorySummary[]; total: number }) {
  const { t, i18n } = useTranslation();
  const parts = categories.filter((c) => c.bytes > 0);
  return (
    <div role="img" aria-label={t("scan.strip_label")} className="flex h-9 w-full overflow-hidden rounded-md bg-surface-muted">
      {parts.map((c) => {
        const pct = total > 0 ? (c.bytes / total) * 100 : 0;
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
    </div>
  );
}
