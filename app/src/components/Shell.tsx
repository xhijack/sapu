import { useTranslation } from "react-i18next";
import { Home, Sparkles, History, Settings as SettingsIcon } from "lucide-react";
import { cn } from "@/lib/utils";

export type Screen = "home" | "clean" | "history" | "settings";

const ITEMS: { id: Screen; Icon: typeof Home }[] = [
  { id: "home", Icon: Home },
  { id: "clean", Icon: Sparkles },
  { id: "history", Icon: History },
  { id: "settings", Icon: SettingsIcon },
];

export function Shell({ screen, onNavigate, children }: { screen: Screen; onNavigate: (s: Screen) => void; children: React.ReactNode }) {
  const { t } = useTranslation();
  return (
    <div className="flex h-full">
      <nav aria-label={t("app.name")} className="flex w-56 shrink-0 flex-col gap-1 border-r border-border bg-surface p-3">
        <div className="px-3 py-2">
          <div className="text-lg font-bold">{t("app.name")}</div>
          <div className="text-xs text-fg-muted">{t("app.tagline")}</div>
        </div>
        {ITEMS.map(({ id, Icon }) => (
          <button
            key={id}
            type="button"
            aria-current={screen === id ? "page" : undefined}
            onClick={() => onNavigate(id)}
            className={cn(
              "flex items-center gap-3 rounded-lg px-3 py-2 text-left text-sm",
              screen === id ? "bg-surface-muted font-medium" : "text-fg-muted hover:bg-surface-muted hover:text-fg",
            )}
          >
            <Icon aria-hidden className="h-4 w-4" />
            {t(`nav.${id}`)}
          </button>
        ))}
      </nav>
      <main className="min-w-0 flex-1 overflow-y-auto p-8">{children}</main>
    </div>
  );
}
