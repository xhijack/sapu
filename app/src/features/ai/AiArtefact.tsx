import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { aiApi } from "@/lib/aiApi";
import { useErrorMessage } from "@/lib/errors";
import type { AiPreview, ArtefactAnswer } from "@/lib/types";
import { AiPanel } from "./AiPanel";

/** A7: explain one project's artefacts. */
export function AiArtefact({ root }: { root: string }) {
  const { t } = useTranslation();
  const errorMessage = useErrorMessage();
  const [answer, setAnswer] = useState<ArtefactAnswer | null>(null);
  const [preview, setPreview] = useState<AiPreview | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const run = async () => {
    setBusy(true);
    setError(null);
    try {
      const r = await aiApi.artefact(root);
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
      <div className="grid gap-1">
        <div>
          <Button variant="ghost" size="sm" onClick={() => void run()} disabled={busy}>
            {busy ? t("ai.thinking") : t("ai.explain_project")}
          </Button>
        </div>
        {error && (
          <p role="alert" className="text-xs text-risk-danger-fg">
            {error}
          </p>
        )}
      </div>
    );
  }
  return (
    <AiPanel preview={preview}>
      <p>{answer.project_summary}</p>
      <ul className="grid gap-1">
        {answer.artefacts.map((a) => (
          <li key={a.kind} className="grid gap-0.5">
            <span>
              <span className="path">{a.kind}</span>: {a.regenerable ? t("ai.regenerable") : t("ai.not_regenerable")}, <span className={a.verdict === "keep" ? "text-risk-danger-fg" : "text-risk-review-fg"}>{t(`ai.verdict_${a.verdict}`).toLowerCase()}</span>
              <span className="text-xs text-fg-muted"> ({t("ai.confidence", { pct: Math.round(a.confidence * 100) })})</span>
            </span>
            <span className="text-fg-muted">{a.explanation}</span>
            {a.rebuild && <code className="path w-fit rounded bg-surface-muted px-1.5 py-0.5 text-xs">{a.rebuild}</code>}
          </li>
        ))}
      </ul>
    </AiPanel>
  );
}
