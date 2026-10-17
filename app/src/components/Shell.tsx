import { useTranslation } from "react-i18next";
import { Home, HeartPulse, Search, Sparkles, Copy, FolderInput, AppWindow, History, Info, Settings as SettingsIcon } from "lucide-react";
import { cn } from "@/lib/utils";
import { StatusBar } from "./StatusBar";

export type Screen = "home" | "care" | "scan" | "clean" | "dedupe" | "organize" | "apps" | "history" | "settings" | "about";

const MAIN: { id: Screen; Icon: typeof Home }[] = [
  { id: "home", Icon: Home },
  { id: "care", Icon: HeartPulse },
  { id: "scan", Icon: Search },
  { id: "clean", Icon: Sparkles },
  { id: "dedupe", Icon: Copy },
  { id: "organize", Icon: FolderInput },
  { id: "apps", Icon: AppWindow },
];
const SECONDARY: { id: Screen; Icon: typeof Home }[] = [
  { id: "history", Icon: History },
  { id: "settings", Icon: SettingsIcon },
  { id: "about", Icon: Info },
];

/** Native-feeling frame: translucent sidebar, hidden title bar, draggable top strip. */
export function Shell({ screen, onNavigate, children }: { screen: Screen; onNavigate: (s: Screen) => void; children: React.ReactNode }) {
  const { t } = useTranslation();
  const item = ({ id, Icon }: { id: Screen; Icon: typeof Home }) => (
    <button
      key={id}
      type="button"
      aria-current={screen === id ? "page" : undefined}
      onClick={() => onNavigate(id)}
      className={cn(
        "no-drag flex w-full items-center gap-3 rounded-xl px-3 py-2 text-left text-[14px] font-medium transition-colors",
        screen === id ? "bg-accent-soft text-fg shadow-[inset_0_0_0_1px_var(--border)]" : "text-fg-muted hover:bg-surface-muted hover:text-fg",
      )}
    >
      <Icon aria-hidden className={cn("h-[18px] w-[18px]", screen === id ? "text-accent" : "")} />
      {t(`nav.${id}`)}
    </button>
  );
  return (
    <div className="flex h-full">
      <nav aria-label={t("app.name")} className="drag flex w-[232px] shrink-0 flex-col gap-1 border-r border-border px-3 pb-4 pt-12" style={{ background: "var(--sidebar)", backdropFilter: "blur(24px)" }}>
        <button type="button" onClick={() => onNavigate("about")} aria-label={t("nav.about")} className="no-drag mb-4 flex items-center gap-2.5 rounded-xl px-2 py-1 text-left hover:bg-surface-muted">
          <span aria-hidden className="grid h-8 w-8 place-items-center rounded-xl bg-accent text-accent-fg shadow-[var(--shadow-glow)]">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M14 3l7 7" />
              <path d="M13.5 3.5L4 13l7 7 9.5-9.5" />
              <path d="M7 16l-3 5" />
            </svg>
          </span>
          <span className="grid leading-tight">
            <span className="display text-[17px]">{t("app.name")}</span>
            <span className="text-[10px] leading-tight text-fg-faint">{t("app.full_name")}</span>
          </span>
        </button>
        <div className="grid gap-1">{MAIN.map(item)}</div>
        <div className="mt-auto grid gap-1">{SECONDARY.map(item)}</div>
      </nav>
      <div className="flex min-w-0 flex-1 flex-col">
        <main className="min-w-0 flex-1 overflow-y-auto">
          <div className="drag h-10" />
          <div className="px-8 pb-10">{children}</div>
        </main>
        <StatusBar onNavigate={onNavigate} />
      </div>
    </div>
  );
}
