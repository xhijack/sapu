import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Card, CardText, CardTitle } from "@/components/ui/card";
import { api } from "@/lib/api";
import type { ProtectedEntry, Profile, Settings, Theme } from "@/lib/types";
import { AiSettings } from "@/features/ai/AiSettings";

interface Props {
  settings: Settings;
  onChange: (s: Settings) => void;
  onRerunOnboarding: () => void;
}

function Select<T extends string>({ id, label, value, options, onChange }: { id: string; label: string; value: T; options: { value: T; label: string }[]; onChange: (v: T) => void }) {
  return (
    <div className="grid gap-1">
      <label htmlFor={id} className="text-sm font-medium">
        {label}
      </label>
      <select
        id={id}
        value={value}
        onChange={(e) => onChange(e.target.value as T)}
        className="h-10 rounded-lg border border-border bg-surface px-3 text-sm"
      >
        {options.map((o) => (
          <option key={o.value} value={o.value}>
            {o.label}
          </option>
        ))}
      </select>
    </div>
  );
}

export function SettingsView({ settings, onChange, onRerunOnboarding }: Props) {
  const { t } = useTranslation();
  const [protectedPaths, setProtectedPaths] = useState<ProtectedEntry[]>([]);
  useEffect(() => {
    void api.protectedPaths().then(setProtectedPaths).catch(() => undefined);
  }, []);

  const reasonLabel = (reason: string) => {
    const key = `settings.reason_${reason}`;
    const v = t(key);
    return v === key ? reason : v;
  };

  return (
    <div className="grid max-w-2xl gap-6">
      <h1 className="text-2xl font-bold">{t("settings.title")}</h1>
      <Card className="grid gap-4">
        <Select<Profile>
          id="profile"
          label={t("settings.profile")}
          value={settings.profile}
          onChange={(profile) => onChange({ ...settings, profile })}
          options={[
            { value: "general", label: t("onboarding.profile_general") },
            { value: "developer", label: t("onboarding.profile_developer") },
          ]}
        />
        <Select<string>
          id="language"
          label={t("settings.language")}
          value={settings.locale ?? "system"}
          onChange={(v) => onChange({ ...settings, locale: v === "system" ? null : v })}
          options={[
            { value: "system", label: t("settings.language_system") },
            { value: "id", label: "Bahasa Indonesia" },
            { value: "en", label: "English" },
          ]}
        />
        <Select<Theme>
          id="theme"
          label={t("settings.theme")}
          value={settings.theme}
          onChange={(theme) => onChange({ ...settings, theme })}
          options={[
            { value: "system", label: t("settings.theme_system") },
            { value: "light", label: t("settings.theme_light") },
            { value: "dark", label: t("settings.theme_dark") },
          ]}
        />
        <label className="flex items-start gap-3 text-sm">
          <input type="checkbox" checked={settings.notify_when_done} onChange={(e) => onChange({ ...settings, notify_when_done: e.target.checked })} className="mt-0.5 h-4 w-4" />
          <span>
            <span className="block font-medium">{t("settings.notify")}</span>
            <span className="block text-fg-muted">{t("settings.notify_hint")}</span>
          </span>
        </label>
        <div>
          <Button variant="outline" size="sm" onClick={onRerunOnboarding}>
            {t("settings.rerun_onboarding")}
          </Button>
        </div>
      </Card>
      <AiSettings />
      <Card className="grid gap-3">
        <CardTitle>{t("settings.protected_title")}</CardTitle>
        <CardText>{t("settings.protected_body")}</CardText>
        <ul className="divide-y divide-border text-sm">
          {protectedPaths.map((p) => (
            <li key={p.path} className="flex items-center justify-between gap-4 py-2">
              <span className="min-w-0 truncate font-mono text-xs" title={p.path}>
                {p.path}
              </span>
              <span className="shrink-0 text-fg-muted">
                {reasonLabel(p.reason)} · {p.protection === "Exact" ? t("settings.protected_exact") : t("settings.protected_subtree")}
              </span>
            </li>
          ))}
        </ul>
      </Card>
    </div>
  );
}
