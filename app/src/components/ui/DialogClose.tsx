import { useTranslation } from "react-i18next";
import { X } from "lucide-react";

/** The X in a dialog's corner. Closing is obvious; the label is for screen readers. */
export function DialogClose({ onClick }: { onClick: () => void }) {
  const { t } = useTranslation();
  return (
    <button type="button" onClick={onClick} aria-label={t("common.close")} title={t("common.close")} className="grid h-8 w-8 shrink-0 place-items-center rounded-full text-fg-muted hover:bg-surface-muted hover:text-fg">
      <X aria-hidden className="h-4 w-4" />
    </button>
  );
}
