import { useTranslation } from "react-i18next";
import type { Category, CategorySummary } from "@/lib/types";
import { formatBytes } from "@/lib/format";
import { categoryColor } from "./categoryColor";

interface Props {
  categories: CategorySummary[];
  total: number;
  onSelect?: (c: Category) => void;
}

export function CategoryList({ categories, total, onSelect }: Props) {
  const { t, i18n } = useTranslation();
  const max = Math.max(1, ...categories.map((c) => c.bytes));
  return (
    <ul className="grid gap-1">
      {categories.map((c) => {
        const share = total > 0 ? c.bytes / total : 0;
        const Row = onSelect ? "button" : "div";
        return (
          <li key={c.category}>
            <Row
              {...(onSelect ? { type: "button", onClick: () => onSelect(c.category) } : {})}
              className="grid w-full grid-cols-[14px_minmax(120px,1fr)_minmax(0,2fr)_90px_70px] items-center gap-4 rounded-md px-2 py-2 text-left hover:bg-surface-muted focus-visible:bg-surface-muted"
            >
              <span aria-hidden className="h-3.5 w-3.5 rounded-sm" style={{ background: categoryColor(c.category) }} />
              <span className="truncate">{t(`category.${c.category}`)}</span>
              <span className="h-2 overflow-hidden rounded-full bg-surface-muted">
                <span className="block h-full rounded-full" style={{ width: `${(c.bytes / max) * 100}%`, background: categoryColor(c.category) }} />
              </span>
              <span className="num text-right">{formatBytes(c.bytes, i18n.language)}</span>
              <span className="num text-right text-sm text-fg-muted">{(share * 100).toLocaleString(i18n.language, { minimumFractionDigits: 1, maximumFractionDigits: 1 })}%</span>
            </Row>
          </li>
        );
      })}
    </ul>
  );
}
