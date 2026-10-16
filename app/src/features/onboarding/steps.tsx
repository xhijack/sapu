import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ShieldCheck, Trash2, History, Lock, Eye, ExternalLink, CheckCircle2, AlertTriangle } from "lucide-react";
import { Button } from "@/components/ui/button";
import { CardText, CardTitle } from "@/components/ui/card";
import { ChoiceGroup } from "@/components/ui/choice";
import { api } from "@/lib/api";
import type { AiMode, AppInfo, DiskAccess, Profile } from "@/lib/types";

/** Language comes first: everything after it is read in that language. */
export function WelcomeStep({ locale, onLocale }: { locale: string | null; onLocale: (l: "id" | "en") => void }) {
  const { t, i18n } = useTranslation();
  const current = locale ?? (i18n.language.startsWith("en") ? "en" : "id");
  return (
    <>
      <CardTitle>{t("onboarding.language_title")}</CardTitle>
      <CardText>{t("onboarding.language_body")}</CardText>
      <div role="radiogroup" aria-label={t("onboarding.language_title")} className="grid grid-cols-2 gap-3">
        {(
          [
            { v: "id", label: "Bahasa Indonesia" },
            { v: "en", label: "English" },
          ] as const
        ).map(({ v, label }) => (
          <label key={v} className={current === v ? "flex cursor-pointer items-center justify-center rounded-xl border border-accent bg-surface px-4 py-3 font-medium" : "flex cursor-pointer items-center justify-center card-solid px-4 py-3 hover:bg-surface-muted"}>
            <input type="radio" name="language" value={v} checked={current === v} onChange={() => onLocale(v)} className="sr-only" />
            {label}
          </label>
        ))}
      </div>
      <CardTitle className="mt-2">{t("onboarding.welcome_title")}</CardTitle>
      <CardText>{t("onboarding.welcome_body")}</CardText>
    </>
  );
}

export function ProfileStep({ info, value, onChange }: { info: AppInfo | null; value: Profile; onChange: (p: Profile) => void }) {
  const { t } = useTranslation();
  const tools = info?.dev_tools ?? [];
  return (
    <>
      <CardTitle>{t("onboarding.profile_title")}</CardTitle>
      <CardText>{t("onboarding.profile_body")}</CardText>
      {tools.length > 0 && (
        <p className="rounded-lg bg-surface-muted px-3 py-2 text-sm">
          {t("onboarding.profile_suggested", { tools: tools.join(", ") })}
        </p>
      )}
      <ChoiceGroup<Profile>
        name={t("onboarding.profile_title")}
        value={value}
        onChange={onChange}
        options={[
          { value: "general", label: t("onboarding.profile_general"), hint: t("onboarding.profile_general_hint") },
          { value: "developer", label: t("onboarding.profile_developer"), hint: t("onboarding.profile_developer_hint") },
        ]}
      />
    </>
  );
}

export function AccessStep({ info }: { info: AppInfo | null }) {
  const { t } = useTranslation();
  const os = info?.os ?? "linux";
  const [status, setStatus] = useState<DiskAccess>(info?.full_disk_access ?? "not_applicable");

  // macOS: poll so the status updates without a restart (PRD §9 criterion).
  useEffect(() => {
    if (os !== "macos" || status === "granted") return;
    const id = setInterval(() => {
      void api.diskAccessStatus().then(setStatus).catch(() => undefined);
    }, 2000);
    return () => clearInterval(id);
  }, [os, status]);

  const bodyKey = os === "macos" ? "access_body_macos" : os === "windows" ? "access_body_windows" : "access_body_linux";
  return (
    <>
      <CardTitle>{t("onboarding.access_title")}</CardTitle>
      <CardText>{t(`onboarding.${bodyKey}`)}</CardText>
      {os === "windows" && (
        <div className="grid gap-1">
          <div>
            <Button variant="outline" onClick={() => void api.relaunchAsAdmin()}>
              {t("onboarding.access_run_admin")}
            </Button>
          </div>
          <span className="text-xs text-fg-muted">{t("onboarding.access_run_admin_hint")}</span>
        </div>
      )}
      {os === "macos" && (
        <div className="grid gap-3">
          <Button variant="outline" onClick={() => void api.openDiskAccessSettings()}>
            <ExternalLink aria-hidden className="h-4 w-4" />
            {t("onboarding.access_open_settings")}
          </Button>
          <ol className="list-decimal space-y-1 pl-5 text-sm text-fg-muted">
            <li>{t("onboarding.access_steps_macos_1")}</li>
            <li>{t("onboarding.access_steps_macos_2")}</li>
            <li>{t("onboarding.access_steps_macos_3")}</li>
          </ol>
          <div>
            <Button variant="secondary" size="sm" onClick={() => void api.revealAppBinary()}>
              {t("onboarding.access_reveal_app")}
            </Button>
          </div>
          {import.meta.env.DEV && <p className="text-xs text-fg-muted">{t("onboarding.access_dev_note")}</p>}
          <AccessStatus status={status} />
        </div>
      )}
    </>
  );
}

function AccessStatus({ status }: { status: DiskAccess }) {
  const { t } = useTranslation();
  if (status === "not_applicable") return null;
  const granted = status === "granted";
  const key = granted ? "access_granted" : status === "denied" ? "access_denied" : "access_unknown";
  return (
    <p
      role="status"
      data-status={status}
      className={granted ? "flex items-center gap-2 text-sm text-risk-safe-fg" : "flex items-center gap-2 text-sm text-risk-review-fg"}
    >
      {granted ? <CheckCircle2 aria-hidden className="h-4 w-4" /> : <AlertTriangle aria-hidden className="h-4 w-4" />}
      {t(`onboarding.${key}`)}
    </p>
  );
}

export function SafetyStep() {
  const { t } = useTranslation();
  const rows = [
    { Icon: Trash2, key: "safety_trash" },
    { Icon: History, key: "safety_history" },
    { Icon: Lock, key: "safety_protected" },
    { Icon: Eye, key: "safety_preview" },
  ];
  return (
    <>
      <CardTitle className="flex items-center gap-2">
        <ShieldCheck aria-hidden className="h-5 w-5 text-risk-safe-fg" />
        {t("onboarding.safety_title")}
      </CardTitle>
      <ul className="grid gap-3">
        {rows.map(({ Icon, key }) => (
          <li key={key} className="flex items-start gap-3 text-sm">
            <Icon aria-hidden className="mt-0.5 h-4 w-4 shrink-0 text-fg-muted" />
            <span>{t(`onboarding.${key}`)}</span>
          </li>
        ))}
      </ul>
    </>
  );
}

export function AiStep({ value, onChange }: { value: AiMode; onChange: (m: AiMode) => void }) {
  const { t } = useTranslation();
  return (
    <>
      <CardTitle>{t("onboarding.ai_title")}</CardTitle>
      <CardText>{t("onboarding.ai_body")}</CardText>
      <ChoiceGroup<AiMode>
        name={t("onboarding.ai_title")}
        value={value}
        onChange={onChange}
        options={[
          { value: "off", label: t("onboarding.ai_off"), hint: t("onboarding.ai_off_hint") },
          { value: "local", label: t("onboarding.ai_local"), hint: t("onboarding.ai_local_hint") },
          { value: "cloud", label: t("onboarding.ai_cloud"), hint: t("onboarding.ai_cloud_hint") },
        ]}
      />
    </>
  );
}

export function ScanStep() {
  const { t } = useTranslation();
  return (
    <>
      <CardTitle>{t("onboarding.scan_title")}</CardTitle>
      <CardText>{t("onboarding.scan_body")}</CardText>
      <p className="rounded-lg bg-surface-muted px-3 py-2 text-sm text-fg-muted">{t("onboarding.scan_unavailable")}</p>
    </>
  );
}
