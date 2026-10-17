import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { DisclaimerContent } from "./Disclaimer";

/** Full-screen notice for people whose settings predate it. */
export function DisclaimerGate({ onAccept }: { onAccept: () => void }) {
  const { t } = useTranslation();
  const [accepted, setAccepted] = useState(false);
  return (
    <div className="grid h-full place-items-center p-8">
      <div className="drag absolute inset-x-0 top-0 h-10" />
      <section aria-labelledby="disclaimer-title" className="card-solid grid w-full max-w-[560px] gap-4 p-8">
        <DisclaimerContent accepted={accepted} onAccept={setAccepted} />
        <div className="flex justify-end">
          <Button size="lg" disabled={!accepted} onClick={onAccept}>
            {t("common.next")}
          </Button>
        </div>
      </section>
    </div>
  );
}
