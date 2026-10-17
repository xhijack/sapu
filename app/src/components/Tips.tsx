import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Lightbulb } from "lucide-react";

const TIP_KEYS = ["quicklook", "trash", "downloads", "dev", "tray", "close", "care", "dupes"] as const;

/**
 * Three short tips about using Sapu. `seed` picks which three, so screens
 * show different ones; they rotate with the day so a returning user sees
 * new ones without any state.
 */
export function Tips({ seed = 0, className }: { seed?: number; className?: string }) {
  const { t } = useTranslation();
  const [day] = useState(() => Math.floor(Date.now() / 86_400_000));
  const start = (day + seed * 3) % TIP_KEYS.length;
  const picks = [0, 1, 2].map((i) => TIP_KEYS[(start + i) % TIP_KEYS.length]);
  return (
    <section aria-label={t("tips.title")} className={className ?? "card grid gap-2 p-4"}>
      <h2 className="flex items-center gap-2 text-sm font-semibold text-fg-muted">
        <Lightbulb aria-hidden className="h-4 w-4" />
        {t("tips.title")}
      </h2>
      <ul className="grid gap-2 text-sm">
        {picks.map((k) => (
          <li key={k} className="grid gap-0.5">
            <span className="font-medium">{t(`tips.${k}_title`)}</span>
            <span className="text-fg-muted">{t(`tips.${k}_body`)}</span>
          </li>
        ))}
      </ul>
    </section>
  );
}
