import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { aiApi } from "@/lib/aiApi";
import { useErrorMessage } from "@/lib/errors";
import { formatBytes } from "@/lib/format";
import type { AiSearchResult } from "@/lib/types";
import { AiPanel } from "./AiPanel";

/** A2: sentence → filter (validated in core) → deterministic result list. */
export function AiSearch({ onTrash }: { onTrash: (paths: string[]) => void }) {
  const { t, i18n } = useTranslation();
  const errorMessage = useErrorMessage();
  const [sentence, setSentence] = useState("");
  const [result, setResult] = useState<AiSearchResult | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const run = async () => {
    if (!sentence.trim()) return;
    setBusy(true);
    setError(null);
    try {
      setResult(await aiApi.search(sentence.trim()));
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="grid gap-3">
      <form
        className="flex gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          void run();
        }}
      >
        <input
          type="search"
          value={sentence}
          onChange={(e) => setSentence(e.target.value)}
          placeholder={t("ai.search_placeholder")}
          aria-label={t("ai.search")}
          className="h-10 min-w-0 flex-1 rounded-lg border border-border bg-surface px-3 text-sm"
        />
        <Button type="submit" variant="outline" disabled={busy || !sentence.trim()}>
          {busy ? t("ai.thinking") : t("ai.search")}
        </Button>
      </form>
      {error && (
        <p role="alert" className="text-sm text-risk-danger-fg">
          {error}
        </p>
      )}
      {result && (
        <AiPanel preview={result.preview}>
          <p className="text-fg-muted">{t("ai.search_understood", { text: result.filter.description })}</p>
          {result.files.length === 0 ? (
            <p>{t("ai.search_empty")}</p>
          ) : (
            <ul className="divide-y divide-border">
              {result.files.map((f) => (
                <li key={f.path} className="flex items-center gap-3 py-1.5">
                  <span className="path min-w-0 flex-1 truncate" title={f.path}>
                    {f.path}
                  </span>
                  <span className="num w-20 shrink-0 text-right">{formatBytes(f.size, i18n.language)}</span>
                  <Button variant="ghost" size="sm" onClick={() => onTrash([f.path])}>
                    {t("scan.action_trash")}
                  </Button>
                </li>
              ))}
            </ul>
          )}
        </AiPanel>
      )}
    </div>
  );
}
