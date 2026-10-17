import { useTranslation } from "react-i18next";
import { Trophy } from "lucide-react";
import { formatBytes, formatDate } from "@/lib/format";

import { MILESTONES, milestoneIndex } from "@/lib/milestones";

/** Lifetime space freed as an achievement: a badge, the number, the road to the next one. */
export function Achievement({ freed, sessions, firstUnix, days, className }: { freed: number; sessions: number; firstUnix: number | null; days: number | null; className?: string }) {
  const { t, i18n } = useTranslation();
  const lang = i18n.language;
  const level = milestoneIndex(freed);
  const next = MILESTONES[level] ?? null;
  const prev = level > 0 ? MILESTONES[level - 1]! : 0;
  const pct = next ? Math.min(100, Math.round(((freed - prev) / (next - prev)) * 100)) : 100;
  const tone = level >= 4 ? "var(--cat-document)" : level >= 2 ? "var(--accent)" : "var(--cat-system)";
  return (
    <section aria-label={t("achievement.title")} className={className ?? "card rise grid content-start gap-3 p-4"}>
      <h2 className="flex items-center gap-2 text-sm font-semibold text-fg-muted">
        <Trophy aria-hidden className="h-4 w-4" />
        {t("achievement.title")}
      </h2>
      {sessions > 0 ? (
        <>
          <div className="flex items-center gap-3">
            <span aria-hidden className="grid h-12 w-12 shrink-0 place-items-center rounded-2xl text-white shadow-[var(--shadow-1)]" style={{ background: tone }}>
              <Trophy className="h-6 w-6" />
            </span>
            <div className="min-w-0">
              <p className="display num text-2xl leading-tight">{formatBytes(freed, lang)}</p>
              <p className="text-sm font-medium">{t(`achievement.level_${level}`)}</p>
            </div>
          </div>
          <div className="grid gap-1">
            <div role="meter" aria-label={t("achievement.next_label")} aria-valuemin={0} aria-valuemax={100} aria-valuenow={pct} className="h-1.5 w-full overflow-hidden rounded-full bg-surface-muted">
              <span className="block h-full rounded-full" style={{ width: `${pct}%`, background: tone }} />
            </div>
            <p className="num text-xs text-fg-muted">{next ? t("achievement.next", { size: formatBytes(next - freed, lang), goal: formatBytes(next, lang) }) : t("achievement.max")}</p>
          </div>
          <p className="num text-xs text-fg-muted">
            {firstUnix && days ? t("home.freed_since", { date: formatDate(firstUnix, lang), days }) + ", " : ""}
            {t("achievement.sessions", { count: sessions })}
          </p>
        </>
      ) : (
        <p className="text-sm text-fg-faint">{t("achievement.none")}</p>
      )}
    </section>
  );
}
