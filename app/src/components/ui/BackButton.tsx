import { useTranslation } from "react-i18next";
import { ArrowLeft } from "lucide-react";

/** "← Kembali" above a step's title. Leaving never cancels work in flight. */
export function BackButton({ onClick, label }: { onClick: () => void; label?: string }) {
  const { t } = useTranslation();
  return (
    <button type="button" onClick={onClick} className="-ml-2 inline-flex w-fit items-center gap-1.5 rounded-full px-2 py-1 text-sm font-medium text-fg-muted hover:bg-surface-muted hover:text-fg">
      <ArrowLeft aria-hidden className="h-4 w-4" />
      {label ?? t("common.back")}
    </button>
  );
}
