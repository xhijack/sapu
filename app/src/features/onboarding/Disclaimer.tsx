import { Callout } from "@/components/ui/Callout";
import { useTranslation } from "react-i18next";
import { AlertTriangle, ShieldCheck } from "lucide-react";
import { CardTitle } from "@/components/ui/card";

/**
 * The no-warranty notice. Shown as the first wizard step and, for people
 * who set Sapu up before it existed, once as a gate on launch.
 */
export function DisclaimerContent({ accepted, onAccept }: { accepted: boolean; onAccept: (v: boolean) => void }) {
  const { t } = useTranslation();
  return (
    <>
      <CardTitle className="display flex items-center gap-2 text-[28px] leading-tight">
        <AlertTriangle aria-hidden className="h-6 w-6 text-risk-review-fg" />
        {t("disclaimer.title")}
      </CardTitle>
      <Callout tone="review">{t("disclaimer.body")}</Callout>
      <ul className="grid gap-2 text-sm">
        <li className="flex items-start gap-2">
          <ShieldCheck aria-hidden className="mt-0.5 h-4 w-4 shrink-0 text-accent" />
          <span>{t("disclaimer.point_trash")}</span>
        </li>
        <li className="flex items-start gap-2">
          <ShieldCheck aria-hidden className="mt-0.5 h-4 w-4 shrink-0 text-accent" />
          <span>{t("disclaimer.point_review")}</span>
        </li>
        <li className="flex items-start gap-2">
          <AlertTriangle aria-hidden className="mt-0.5 h-4 w-4 shrink-0 text-risk-review-fg" />
          <span className="font-medium">{t("disclaimer.point_liability")}</span>
        </li>
      </ul>
      <label className="flex items-start gap-3 rounded-xl border border-border bg-surface-muted p-3 text-sm">
        <input type="checkbox" checked={accepted} onChange={(e) => onAccept(e.target.checked)} className="mt-0.5 h-4 w-4 accent-[var(--accent)]" />
        <span className="font-medium">{t("disclaimer.accept")}</span>
      </label>
    </>
  );
}
