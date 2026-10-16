import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Sparkles } from "lucide-react";
import type { AiPreview } from "@/lib/types";

interface Props {
  confidence?: number;
  reason?: string;
  preview: AiPreview | null;
  children: React.ReactNode;
}

/** Frame for every AI answer: label, confidence, reason, payload (PRD AI-4, §12.4). */
export function AiPanel({ confidence, reason, preview, children }: Props) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  return (
    <section data-testid="ai-panel" className="grid gap-3 rounded-xl border border-risk-review-border bg-risk-review-bg/40 p-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <span className="flex items-center gap-2 text-sm font-medium text-risk-review-fg">
          <Sparkles aria-hidden className="h-4 w-4" />
          {t("ai.label")}
        </span>
        {confidence != null && (
          <span className="flex items-center gap-2 text-xs text-fg-muted">
            {t("ai.confidence", { pct: Math.round(confidence * 100) })}
            <span className="h-1.5 w-16 overflow-hidden rounded-full bg-surface-muted">
              <span className="block h-full bg-risk-review-fg" style={{ width: `${Math.round(confidence * 100)}%` }} />
            </span>
          </span>
        )}
      </div>
      <div className="grid gap-2 text-sm">{children}</div>
      {reason && (
        <p className="text-xs text-fg-muted">
          {t("ai.reason")}: {reason}
        </p>
      )}
      {preview && (
        <div className="text-xs">
          <button type="button" className="text-fg-muted underline" onClick={() => setOpen((o) => !o)}>
            {t("ai.show_payload")}
          </button>
          {open && (
            <div className="mt-2 grid gap-1">
              <p className="text-fg-muted">{t("ai.payload_note", { provider: preview.provider, model: preview.model })}</p>
              <pre className="path max-h-48 overflow-auto rounded-md bg-surface-muted p-2 whitespace-pre-wrap">{preview.user}</pre>
            </div>
          )}
        </div>
      )}
    </section>
  );
}
