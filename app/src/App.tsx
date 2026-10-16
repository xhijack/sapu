import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Shell, type Screen } from "@/components/Shell";
import { ErrorState, LoadingState } from "@/components/States";
import { useErrorMessage } from "@/lib/errors";
import { Wizard } from "@/features/onboarding/Wizard";
import { ScanView } from "@/features/scan/ScanView";
import { CleanView } from "@/features/clean/CleanView";
import { DedupeView } from "@/features/dedupe/DedupeView";
import { HistoryView } from "@/features/history/HistoryView";
import { SettingsView } from "@/features/settings/SettingsView";
import { api } from "@/lib/api";
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
    <Shell screen={screen} onNavigate={setScreen}>
      {screen === "home" && <ScanView />}
      {screen === "clean" && <CleanView profile={settings.profile} onSeeHistory={() => setScreen("history")} />}
      {screen === "dedupe" && <DedupeView onSeeHistory={() => setScreen("history")} />}
      {screen === "history" && <HistoryView />}
      {screen === "settings" && (
        <SettingsView settings={settings} onChange={save} onRerunOnboarding={() => setShowWizard(true)} />
      )}
    </Shell>
  );
}
