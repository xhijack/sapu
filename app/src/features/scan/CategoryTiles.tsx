import { useTranslation } from "react-i18next";
import { CategoryIcon } from "@/components/icons/CategoryIcon";
import { formatBytes } from "@/lib/format";
import type { Category, CategorySummary } from "@/lib/types";
import { categoryColor } from "./categoryColor";

interface Props {
  categories: CategorySummary[];
  total: number;
  onSelect: (c: Category) => void;
}

/** Results as tiles: icon, name, size, share. Click opens the largest list filtered. */
export function CategoryTiles({ categories, total, onSelect }: Props) {
  const { t, i18n } = useTranslation();
  const max = Math.max(1, ...categories.map((c) => c.bytes));
  return (
    <ul className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
      {categories
        .filter((c) => c.bytes > 0)
        .map((c, i) => {
          const share = total > 0 ? (c.bytes / total) * 100 : 0;
          return (
            <li key={c.category} className={`rise rise-${Math.min(4, i + 1)}`}>
              <button
                type="button"
                onClick={() => onSelect(c.category)}
                className="card grid w-full gap-3 p-4 text-left transition-[transform,box-shadow] duration-200 hover:-translate-y-0.5 hover:shadow-[var(--shadow-2)] motion-reduce:transform-none"
              >
                <span className="flex items-center justify-between">
                  <span className="grid h-10 w-10 place-items-center rounded-xl text-white" style={{ background: categoryColor(c.category) }}>
                    <CategoryIcon category={c.category} />
                  </span>
                  <span className="num text-sm text-fg-muted">{share.toLocaleString(i18n.language, { maximumFractionDigits: 1 })}%</span>
                </span>
                <span className="grid gap-0.5">
                  <span className="font-medium">{t(`category.${c.category}`)}</span>
                  <span className="display num text-xl">{formatBytes(c.bytes, i18n.language)}</span>
                  <span className="num text-xs text-fg-muted">{t("scan.files_count", { count: c.files.toLocaleString(i18n.language) })}</span>
                </span>
                <span className="h-1.5 overflow-hidden rounded-full bg-surface-muted">
                  <span className="block h-full rounded-full" style={{ width: `${(c.bytes / max) * 100}%`, background: categoryColor(c.category) }} />
                </span>
              </button>
            </li>
          );
        })}
    </ul>
  );
}
