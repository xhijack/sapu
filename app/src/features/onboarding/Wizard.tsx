import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { api } from "@/lib/api";
import type { AppInfo, Settings } from "@/lib/types";
import { AccessStep, AiStep, ProfileStep, SafetyStep, ScanStep, WelcomeStep } from "./steps";

const TOTAL = 6;

interface Props {
  settings: Settings;
  onFinish: (settings: Settings) => void;
}

/** First-run wizard (PRD §9). Every step can be skipped; "skip" finishes
 *  the wizard with whatever has been chosen so far. */
export function Wizard({ settings, onFinish }: Props) {
  const { t } = useTranslation();
  const [step, setStep] = useState(1);
  const [draft, setDraft] = useState<Settings>(settings);
  const [info, setInfo] = useState<AppInfo | null>(null);

  useEffect(() => {
    void api.appInfo().then((i) => {
      setInfo(i);
      // Suggest the developer profile when tooling is detected (PRD §9.2).
      if (i.dev_tools.length > 0) setDraft((d) => (d.profile === "general" ? { ...d, profile: "developer" } : d));
    }).catch(() => undefined);
  }, []);

  const finish = () => onFinish({ ...draft, onboarding_done: true });
  const next = () => (step === TOTAL ? finish() : setStep(step + 1));
  const back = () => setStep(Math.max(1, step - 1));

  return (
    <div className="flex h-full items-center justify-center bg-bg p-6">
      <Card className="flex w-full max-w-xl flex-col gap-5" aria-labelledby="wizard-step">
        <p id="wizard-step" className="text-xs font-medium uppercase tracking-wide text-fg-muted">
          {t("progress.step_of", { current: step, total: TOTAL })}
        </p>
        {step === 1 && <WelcomeStep />}
        {step === 2 && <ProfileStep info={info} value={draft.profile} onChange={(profile) => setDraft({ ...draft, profile })} />}
        {step === 3 && <AccessStep info={info} />}
        {step === 4 && <SafetyStep />}
        {step === 5 && <AiStep value={draft.ai_mode} onChange={(ai_mode) => setDraft({ ...draft, ai_mode })} />}
        {step === 6 && <ScanStep />}
        <div className="mt-2 flex items-center justify-between gap-3">
          <Button variant="ghost" onClick={finish}>
            {t("common.skip")}
          </Button>
          <div className="flex gap-2">
            {step > 1 && (
              <Button variant="secondary" onClick={back}>
                {t("common.back")}
              </Button>
            )}
            <Button onClick={next} autoFocus>
              {step === TOTAL ? t("common.done") : t("common.next")}
            </Button>
          </div>
        </div>
      </Card>
    </div>
  );
}
