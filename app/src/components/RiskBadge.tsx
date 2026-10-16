import { useTranslation } from "react-i18next";
import { CheckCircle2, AlertTriangle, Ban } from "lucide-react";
import { cn } from "@/lib/utils";
import type { RiskLevel } from "@/lib/types";

const STYLE: Record<RiskLevel, { cls: string; Icon: typeof CheckCircle2 }> = {
  safe: { cls: "bg-risk-safe-bg text-risk-safe-fg border-risk-safe-border", Icon: CheckCircle2 },
  review: { cls: "bg-risk-review-bg text-risk-review-fg border-risk-review-border", Icon: AlertTriangle },
  danger: { cls: "bg-risk-danger-bg text-risk-danger-fg border-risk-danger-border", Icon: Ban },
};

/** Risk label: colour + icon + text, never colour alone (PRD §10). */
export function RiskBadge({ level, className }: { level: RiskLevel; className?: string }) {
  const { t } = useTranslation();
  const { cls, Icon } = STYLE[level];
  return (
    <span
      data-risk={level}
      title={t(`risk.${level}_hint`)}
      className={cn("inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-xs font-medium", cls, className)}
    >
      <Icon aria-hidden className="h-3.5 w-3.5" />
      {t(`risk.${level}`)}
    </span>
  );
}
