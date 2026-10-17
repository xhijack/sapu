import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { BookOpen, Bug, GitBranch, Scale } from "lucide-react";
import { Card } from "@/components/ui/card";
import { api } from "@/lib/api";
import type { AppInfo } from "@/lib/types";

export const REPO_URL = "https://github.com/xhijack/sapu";
export const AUTHOR = "Ramdani";
export const COMPANY = "PT Sopwer Teknologi Indonesia";

/** Who made Sapu, under which terms, and where to go next. */
export function AboutView() {
  const { t } = useTranslation();
  const [info, setInfo] = useState<AppInfo | null>(null);
  useEffect(() => {
    let live = true;
    api
      .appInfo()
      .then((i) => live && setInfo(i))
      .catch(() => undefined);
    return () => {
      live = false;
    };
  }, []);

  const link = (Icon: typeof BookOpen, label: string, url: string) => (
    <button type="button" onClick={() => void api.openUrl(url)} className="flex items-center gap-2 rounded-lg px-2 py-1.5 text-left text-sm hover:bg-surface-muted" title={url}>
      <Icon aria-hidden className="h-4 w-4 text-fg-muted" />
      <span className="font-medium">{label}</span>
    </button>
  );

  return (
    <div className="grid max-w-[1280px] gap-6">
      <h1 className="display text-2xl">{t("settings.about_title")}</h1>
    <Card className="grid gap-5" data-testid="about">
      <div className="flex items-start gap-4">
        <span aria-hidden className="grid h-14 w-14 shrink-0 place-items-center rounded-2xl bg-accent text-accent-fg shadow-[var(--shadow-glow)]">
          <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <path d="M14 3l7 7" />
            <path d="M13.5 3.5L4 13l7 7 9.5-9.5" />
            <path d="M7 16l-3 5" />
          </svg>
        </span>
        <div className="grid gap-1">
          <h2 className="display text-xl leading-tight">
            {t("app.name")} <span className="text-base font-semibold text-fg-muted">{t("app.full_name")}</span>
          </h2>
          <p className="num text-sm text-fg-muted">{info ? t("settings.about_version", { version: info.version, os: info.os }) : ""}</p>
          <p className="text-sm">{t("settings.about_tagline")}</p>
        </div>
      </div>

      <dl className="grid gap-2 text-sm sm:grid-cols-[auto_1fr] sm:gap-x-6">
        <dt className="text-fg-muted">{t("settings.about_made_by")}</dt>
        <dd>
          <span className="font-medium">{AUTHOR}</span>
          <span className="text-fg-muted"> · {COMPANY}</span>
        </dd>
        <dt className="text-fg-muted">{t("settings.about_license")}</dt>
        <dd>
          <span className="font-medium">MIT OR Apache-2.0</span>
          <span className="block text-fg-muted">{t("settings.about_license_body")}</span>
        </dd>
      </dl>

      <div className="flex flex-wrap gap-1">
        {link(GitBranch, t("settings.about_repo"), REPO_URL)}
        {link(Scale, t("settings.about_license_link"), `${REPO_URL}/blob/main/LICENSE-MIT`)}
        {link(BookOpen, t("settings.about_contribute"), `${REPO_URL}/blob/main/CONTRIBUTING.md`)}
        {link(Bug, t("settings.about_issues"), `${REPO_URL}/issues`)}
      </div>
    </Card>
      <p className="max-w-[80ch] text-xs text-fg-muted">{t("disclaimer.point_liability")}</p>
    </div>
  );
}
