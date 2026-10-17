import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { SpaceRing } from "@/components/viz/SpaceRing";
import { api } from "@/lib/api";
import type { AppInfo, Settings } from "@/lib/types";
import { AccessStep, AiStep, ProfileStep, SafetyStep, ScanStep, WelcomeStep } from "./steps";
import { DisclaimerContent } from "./Disclaimer";

const TOTAL = 7;

interface Props {
  settings: Settings;
  onFinish: (settings: Settings) => void;
}

/** First-run wizard (PRD §9): two columns, the ring fills as steps pass,
 *  every step skippable. */
export function Wizard({ settings, onFinish }: Props) {
  const { t, i18n } = useTranslation();
  const [step, setStep] = useState(1);
  const [draft, setDraft] = useState<Settings>(settings);
  const [info, setInfo] = useState<AppInfo | null>(null);

  useEffect(() => {
    void api
      .appInfo()
      .then((i) => {
        setInfo(i);
        if (i.dev_tools.length > 0) setDraft((d) => (d.profile === "general" ? { ...d, profile: "developer" } : d));
      })
      .catch(() => undefined);
  }, []);

  const [accepted, setAccepted] = useState(settings.disclaimer_accepted);
  // Step 2 (the notice) cannot be skipped or passed without accepting.
  const canProceed = step !== 2 || accepted;
  const finish = () => onFinish({ ...draft, onboarding_done: true, disclaimer_accepted: accepted });
  const next = () => (step === TOTAL ? finish() : setStep(step + 1));
  const back = () => setStep(Math.max(1, step - 1));

  return (
    <div className="grid h-full md:grid-cols-[minmax(280px,38%)_1fr]">
      <aside className="drag relative hidden flex-col justify-between border-r border-border p-10 md:flex" style={{ background: "var(--sidebar)", backdropFilter: "blur(24px)" }}>
        <div className="no-drag flex items-center gap-2.5 pt-2">
          <span aria-hidden className="grid h-8 w-8 place-items-center rounded-xl bg-accent text-accent-fg shadow-[var(--shadow-glow)]">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M14 3l7 7" />
              <path d="M13.5 3.5L4 13l7 7 9.5-9.5" />
              <path d="M7 16l-3 5" />
            </svg>
          </span>
          <span className="grid leading-tight">
            <span className="display text-[17px]">{t("app.name")}</span>
            <span className="text-[10px] text-fg-faint">{t("app.full_name")}</span>
          </span>
        </div>
        <div className="grid place-items-center">
          <SpaceRing size={220} thickness={18} total={TOTAL} segments={[{ key: "steps", value: step, color: "var(--accent)" }]} label={t("progress.step_of", { current: step, total: TOTAL })}>
            <span className="display num text-[40px] leading-none">
              {step}
              <span className="text-xl text-fg-muted">/{TOTAL}</span>
            </span>
          </SpaceRing>
        </div>
        <p className="text-sm text-fg-muted">{t("app.tagline")}</p>
      </aside>

      <main className="flex flex-col">
        <div className="drag h-10" />
        <section aria-labelledby="wizard-step" className="mx-auto flex w-full max-w-[560px] flex-1 flex-col gap-6 px-10 pb-10 pt-4">
          <p id="wizard-step" className="text-sm text-fg-muted">
            {t("progress.step_of", { current: step, total: TOTAL })}
          </p>
          <div className="rise grid flex-1 content-start gap-4" key={step}>
            {step === 1 && (
              <WelcomeStep
                locale={draft.locale}
                onLocale={(locale) => {
                  setDraft({ ...draft, locale });
                  void i18n.changeLanguage(locale);
                  document.documentElement.lang = locale;
                }}
              />
            )}
            {step === 2 && <DisclaimerContent accepted={accepted} onAccept={setAccepted} />}
            {step === 3 && <ProfileStep info={info} value={draft.profile} onChange={(profile) => setDraft({ ...draft, profile })} />}
            {step === 4 && <AccessStep info={info} />}
            {step === 5 && <SafetyStep />}
            {step === 6 && <AiStep value={draft.ai_mode} onChange={(ai_mode) => setDraft({ ...draft, ai_mode })} />}
            {step === 7 && <ScanStep />}
          </div>
          <div className="flex items-center justify-between gap-3 pt-2">
            <Button variant="ghost" onClick={finish} disabled={!accepted}>
              {t("common.skip")}
            </Button>
            <div className="flex gap-2">
              {step > 1 && (
                <Button variant="secondary" onClick={back}>
                  {t("common.back")}
                </Button>
              )}
              <Button size="lg" onClick={next} autoFocus disabled={!canProceed}>
                {step === TOTAL ? t("common.done") : t("common.next")}
              </Button>
            </div>
          </div>
        </section>
      </main>
    </div>
  );
}
