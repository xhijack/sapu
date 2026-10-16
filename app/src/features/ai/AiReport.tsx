import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { aiApi } from "@/lib/aiApi";
import { useErrorMessage } from "@/lib/errors";
import type { AiPreview, ReportAnswer } from "@/lib/types";
import { AiPanel } from "./AiPanel";

/** A3: narrative summary on demand. */
export function AiReport() {
  const { t } = useTranslation();
  const errorMessage = useErrorMessage();
  const [answer, setAnswer] = useState<ReportAnswer | null>(null);
  const [preview, setPreview] = useState<AiPreview | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const run = async () => {
    setBusy(true);
    setError(null);
    try {
      const r = await aiApi.report();
      setAnswer(r.answer);
      setPreview(r.preview);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  if (!answer) {
    return (
      <div className="grid gap-2">
        <div>
          <Button variant="outline" size="sm" onClick={() => void run()} disabled={busy}>
            {busy ? t("ai.thinking") : t("ai.report")}
          </Button>
        </div>
        {error && (
          <p role="alert" className="text-sm text-risk-danger-fg">
            {error}
          </p>
        )}
      </div>
    );
  }
  return (
    <AiPanel preview={preview}>
      <p>
        <span className="font-medium">{t("ai.report_what_fills")}.</span> {answer.what_fills}
      </p>
      <p>
        <span className="font-medium">{t("ai.report_start_here")}.</span> {answer.start_here}
      </p>
      <p>
        <span className="font-medium">{t("ai.report_leave_alone")}.</span> {answer.leave_alone}
      </p>
    </AiPanel>
  );
}
