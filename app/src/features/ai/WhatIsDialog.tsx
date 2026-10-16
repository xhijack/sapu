import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { aiApi } from "@/lib/aiApi";
import { useErrorMessage } from "@/lib/errors";
import type { AiPreview, WhatIsAnswer } from "@/lib/types";
import { AiPanel } from "./AiPanel";

/** A1 + A4: explain a path, optionally keep the answer as a local rule. */
export function WhatIsDialog({ path, onClose }: { path: string; onClose: () => void }) {
  const { t } = useTranslation();
  const errorMessage = useErrorMessage();
  const ref = useRef<HTMLDialogElement>(null);
  const [answer, setAnswer] = useState<WhatIsAnswer | null>(null);
  const [preview, setPreview] = useState<AiPreview | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    ref.current?.showModal();
    let live = true;
    aiApi
      .whatis(path)
      .then((r) => {
        if (!live) return;
        setAnswer(r.answer);
        setPreview(r.preview);
      })
      .catch((e) => live && setError(errorMessage(e)));
    return () => {
      live = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [path]);

  const save = async () => {
    if (!answer) return;
    try {
      await aiApi.saveRule(path, answer.summary, answer.summary, answer.consequence, answer.consequence);
      setSaved(true);
    } catch (e) {
      setError(errorMessage(e));
    }
  };

  const copyToml = async () => {
    if (!answer) return;
    const esc = (x: string) => x.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
    const toml = `[rule]\nid = "community-rule"\nname = { id = "${esc(answer.summary)}", en = "${esc(answer.summary)}" }\ndescription = { id = "${esc(answer.consequence)}", en = "${esc(answer.consequence)}" }\nrisk = "yellow"\nprofile = "general"\ngroup = "user"\n\n[[match]]\npath = "${esc(path)}"\nkind = "dir_contents"\n`;
    try {
      await navigator.clipboard.writeText(toml);
      setCopied(true);
    } catch {
      /* clipboard unavailable */
    }
  };

  return (
    <dialog ref={ref} onClose={onClose} aria-labelledby="whatis-title" className="m-auto w-full max-w-lg card-solid p-0 text-fg shadow-xl backdrop:bg-black/40">
      <div className="grid gap-4 p-6">
        <h2 id="whatis-title" className="text-lg font-semibold">
          {t("ai.whatis_title")}
        </h2>
        <p className="path truncate text-fg-muted" title={path}>
          {path}
        </p>
        {error ? (
          <p role="alert" className="text-risk-danger-fg">
            {error}
          </p>
        ) : answer ? (
          <AiPanel confidence={answer.confidence} reason={answer.reason} preview={preview}>
            <p className="font-medium">{answer.summary}</p>
            {answer.likely_owner && <p>{t("ai.whatis_owner", { owner: answer.likely_owner })}</p>}
            <p className={answer.verdict === "keep" ? "text-risk-danger-fg" : "text-risk-review-fg"}>{t(`ai.verdict_${answer.verdict}`)}</p>
            {answer.consequence && <p className="text-fg-muted">{t("ai.whatis_consequence", { text: answer.consequence })}</p>}
          </AiPanel>
        ) : (
          <p className="text-fg-muted">{t("ai.thinking")}</p>
        )}
        {answer?.verdict === "check_first" && !saved && (
          <div className="grid gap-1">
            <Button variant="outline" size="sm" onClick={() => void save()}>
              {t("ai.save_rule")}
            </Button>
            <span className="text-xs text-fg-muted">{t("ai.save_rule_hint")}</span>
          </div>
        )}
        {saved && (
          <div className="grid gap-1">
            <p role="status" className="text-sm text-risk-safe-fg">
              {t("ai.rule_saved")}
            </p>
            <div>
              <Button variant="ghost" size="sm" onClick={() => void copyToml()}>
                {t("ai.copy_rule")}
              </Button>
            </div>
            {copied && <p className="text-xs text-fg-muted">{t("ai.copied_rule")}</p>}
          </div>
        )}
        <div className="flex justify-end">
          <Button variant="secondary" onClick={() => ref.current?.close()}>
            {t("common.done")}
          </Button>
        </div>
      </div>
    </dialog>
  );
}
