import { useTranslation } from "react-i18next";
import type { Category, CategorySummary } from "@/lib/types";
import { formatBytes } from "@/lib/format";
import { categoryColor } from "./categoryColor";
import { CategoryIcon } from "@/components/icons/CategoryIcon";

interface Props {
  categories: CategorySummary[];
  total: number;
  onSelect?: (c: Category) => void;
  columns?: 1 | 2;
}

export function CategoryList({ categories, total, onSelect, columns = 2 }: Props) {
  const { t, i18n } = useTranslation();
  const max = Math.max(1, ...categories.map((c) => c.bytes));
  return (
    <ul className={columns === 2 ? "card-solid grid gap-x-6 gap-y-0.5 p-2 md:grid-cols-2" : "card-solid grid content-start gap-y-0.5 p-2"}>
      {categories.map((c) => {
        const share = total > 0 ? c.bytes / total : 0;
        const Row = onSelect ? "button" : "div";
        return (
          <li key={c.category}>
            <Row
              {...(onSelect ? { type: "button", onClick: () => onSelect(c.category) } : {})}
              className="grid w-full grid-cols-[28px_minmax(110px,1fr)_minmax(0,1.6fr)_84px_56px] items-center gap-3 rounded-lg px-2 py-1.5 text-left hover:bg-surface-muted focus-visible:bg-surface-muted"
            >
              <span aria-hidden className="grid h-7 w-7 place-items-center rounded-lg text-white" style={{ background: categoryColor(c.category) }}>
                <CategoryIcon category={c.category} size={15} />
              </span>
              <span className="min-w-0">
                <span className="block truncate text-sm font-medium">{t(`category.${c.category}`)}</span>
                <span className="num block text-[11px] text-fg-muted">{t("scan.files_count", { count: c.files.toLocaleString(i18n.language) })}</span>
              </span>
              <span className="h-2 overflow-hidden rounded-full bg-surface-muted">
                <span className="block h-full rounded-full" style={{ width: `${(c.bytes / max) * 100}%`, background: categoryColor(c.category) }} />
              </span>
              <span className="num text-right text-sm font-medium">{formatBytes(c.bytes, i18n.language)}</span>
              <span className="num text-right text-xs text-fg-muted">{(share * 100).toLocaleString(i18n.language, { minimumFractionDigits: 1, maximumFractionDigits: 1 })}%</span>
            </Row>
          </li>
        );
      })}
    </ul>
  );
}
