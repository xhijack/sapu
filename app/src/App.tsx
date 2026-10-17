import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Shell, type Screen } from "@/components/Shell";
import { ErrorState, LoadingState } from "@/components/States";
import { useErrorMessage } from "@/lib/errors";
import { Wizard } from "@/features/onboarding/Wizard";
import { ScanView } from "@/features/scan/ScanView";
import { HomeView } from "@/features/home/HomeView";
import { CareView } from "@/features/care/CareView";
import { AppsView } from "@/features/apps/AppsView";
import { CleanView } from "@/features/clean/CleanView";
import { DedupeView } from "@/features/dedupe/DedupeView";
import { HistoryView } from "@/features/history/HistoryView";
import { SettingsView } from "@/features/settings/SettingsView";
import { api, isTauri } from "@/lib/api";
import { resolveLocale } from "@/i18n";
import type { Settings } from "@/lib/types";

function applyTheme(theme: Settings["theme"]) {
  const root = document.documentElement;
  if (theme === "system") root.removeAttribute("data-theme");
  else root.setAttribute("data-theme", theme);
}

export default function App() {
  const { i18n } = useTranslation();
  const errorMessage = useErrorMessage();
  const [settings, setSettings] = useState<Settings | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [screen, setScreen] = useState<Screen>("home");
  // Views stay mounted once visited so a running scan or a selection
  // survives navigation; only the active one is visible. Clean and Apps are
  // mounted from the start so their data is ready before the first click.
  const [visited, setVisited] = useState<Set<Screen>>(new Set(["home", "clean", "apps"]));
  const navigate = (s: Screen) => {
    setVisited((v) => (v.has(s) ? v : new Set([...v, s])));
    setScreen(s);
  };
  const [showWizard, setShowWizard] = useState(false);

  const fetchSettings = useCallback(
    () =>
      api
        .settingsGet()
        .then((s) => {
          setSettings(s);
          setShowWizard(!s.onboarding_done);
        })
        .catch((e) => setError(errorMessage(e))),
    // errorMessage only depends on the active language.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [],
  );
  const retry = () => {
    setError(null);
    void fetchSettings();
  };

  useEffect(() => {
    void fetchSettings();
  }, [fetchSettings]);

  // Tray shortcut: bring Care up and start it.
  const [careKick, setCareKick] = useState(0);
  useEffect(() => {
    if (!isTauri()) return;
    let unlisten: (() => void) | null = null;
    void import("@tauri-apps/api/event").then(({ listen }) =>
      listen("tray://care", () => {
        navigate("care");
        setCareKick((k) => k + 1);
      }).then((u) => {
        unlisten = u;
      }),
    );
    return () => unlisten?.();
  }, []);

  useEffect(() => {
    if (!settings) return;
    const locale = resolveLocale(settings.locale);
    if (i18n.language !== locale) void i18n.changeLanguage(locale);
    document.documentElement.lang = locale;
    applyTheme(settings.theme);
  }, [settings, i18n]);

  const save = (next: Settings) => {
    setSettings(next);
    api.settingsSet(next).catch((e) => setError(errorMessage(e)));
  };

  if (error) return <div className="p-8"><ErrorState message={error} onRetry={retry} /></div>;
  if (!settings) return <LoadingState />;
  if (showWizard) {
    return (
      <Wizard
        settings={settings}
        onFinish={(s) => {
          save(s);
          setShowWizard(false);
        }}
      />
    );
  }

  return (
    <Shell screen={screen} onNavigate={navigate}>
      <Pane active={screen === "home"} mounted>
        <HomeView onNavigate={navigate} onScanNow={() => navigate("care")} />
      </Pane>
      <Pane active={screen === "care"} mounted={visited.has("care")}>
        <CareView onNavigate={navigate} autoStart={careKick} />
      </Pane>
      <Pane active={screen === "scan"} mounted={visited.has("scan")}>
        <ScanView profile={settings.profile} onNavigate={(s) => navigate(s)} />
      </Pane>
      <Pane active={screen === "clean"} mounted={visited.has("clean")}>
        <CleanView profile={settings.profile} onSeeHistory={() => navigate("history")} />
      </Pane>
      <Pane active={screen === "dedupe"} mounted={visited.has("dedupe")}>
        <DedupeView profile={settings.profile} onSeeHistory={() => navigate("history")} />
      </Pane>
      <Pane active={screen === "apps"} mounted={visited.has("apps")}>
        <AppsView onSeeHistory={() => navigate("history")} />
      </Pane>
      <Pane active={screen === "history"} mounted={visited.has("history")}>
        <HistoryView />
      </Pane>
      <Pane active={screen === "settings"} mounted={visited.has("settings")}>
        <SettingsView settings={settings} onChange={save} onRerunOnboarding={() => setShowWizard(true)} />
      </Pane>
    </Shell>
  );
}

function Pane({ active, mounted, children }: { active: boolean; mounted: boolean; children: React.ReactNode }) {
  if (!mounted) return null;
  return (
    <div hidden={!active} aria-hidden={!active}>
      {children}
    </div>
  );
}
