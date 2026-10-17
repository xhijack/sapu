import { useEffect, useRef, useState } from "react";
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
  const [open, setOpen] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const run = async () => {
    setBusy(true);
    setError(null);
    try {
      const r = await aiApi.artefact(root);
      setAnswer(r.answer);
      setPreview(r.preview);
      setOpen(true);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="grid gap-1">
      <div className="flex items-center gap-2">
        <Button variant="ghost" size="sm" onClick={() => (answer ? setOpen(true) : void run())} disabled={busy}>
          {busy ? t("ai.thinking") : answer ? t("ai.explain_show") : t("ai.explain_project")}
        </Button>
      </div>
      {error && (
        <p role="alert" className="text-xs text-risk-danger-fg">
          {error}
        </p>
      )}
      {answer && open && <ArtefactDialog root={root} answer={answer} preview={preview} onClose={() => setOpen(false)} />}
    </div>
  );
}

/** The explanation in its own window, scrolling inside, so the list behind it keeps its height. */
function ArtefactDialog({ root, answer, preview, onClose }: { root: string; answer: ArtefactAnswer; preview: AiPreview | null; onClose: () => void }) {
  const { t } = useTranslation();
  const ref = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    ref.current?.showModal();
  }, []);
  return (
    <dialog ref={ref} onClose={onClose} aria-labelledby="ai-artefact-title" className="m-auto w-full max-w-2xl card-solid p-0 text-fg shadow-xl backdrop:bg-black/40">
      <div className="grid max-h-[80vh] gap-4 overflow-y-auto p-6">
        <div className="flex items-start justify-between gap-3">
          <h2 id="ai-artefact-title" className="min-w-0 text-lg font-semibold">
            {t("ai.explain_title", { name: root.split("/").pop() ?? root })}
          </h2>
          <Button variant="ghost" size="sm" onClick={() => ref.current?.close()}>
            {t("common.close")}
          </Button>
        </div>
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
      </div>
    </dialog>
  );
}
