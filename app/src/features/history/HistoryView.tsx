import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { RotateCcw } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { EmptyState, ErrorState, LoadingState } from "@/components/States";
import { useErrorMessage } from "@/lib/errors";
import { api, isCommandError } from "@/lib/api";
import { formatBytes, formatDate } from "@/lib/format";
import type { CleanSession } from "@/lib/types";

export function HistoryView() {
  const { t, i18n } = useTranslation();
  const errorMessage = useErrorMessage();
  const [sessions, setSessions] = useState<CleanSession[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const fetchHistory = useCallback(
    () => api.historyList().then(setSessions).catch((e) => setError(errorMessage(e))),
    // errorMessage only depends on the active language.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [],
  );
  const retry = () => {
    setError(null);
    void fetchHistory();
  };

  useEffect(() => {
    void fetchHistory();
  }, [fetchHistory]);

  const restore = async (id: string) => {
    setNotice(null);
    try {
      const r = await api.historyRestore(id);
      setNotice(
        [
          t("history.restored", { count: r.restored.length }),
          r.not_found_in_trash.length ? t("history.not_in_trash", { count: r.not_found_in_trash.length }) : null,
        ]
          .filter(Boolean)
          .join(" · "),
      );
    } catch (e) {
      setNotice(isCommandError(e) && e.code === "restore_unsupported" ? t("history.restore_unsupported") : errorMessage(e));
    }
  };

  return (
    <div className="grid gap-6">
      <h1 className="display text-2xl">{t("history.title")}</h1>
      {notice && (
        <p role="status" className="rounded-lg bg-surface-muted px-3 py-2 text-sm">
          {notice}
        </p>
      )}
      {error ? (
        <ErrorState message={error} onRetry={retry} />
      ) : sessions === null ? (
        <LoadingState />
      ) : sessions.length === 0 ? (
        <EmptyState title={t("history.empty_title")} body={t("history.empty_body")} />
      ) : (
        <ul className="grid gap-3">
          {sessions.map((s) => (
            <li key={s.id}>
              <Card className="flex items-center justify-between gap-4">
                <div className="min-w-0">
                  <div className="font-medium">{formatDate(s.started_at_unix, i18n.language)}</div>
                  <div className="truncate text-sm text-fg-muted" title={s.scope}>
                    {s.scope}
                  </div>
                  <div className="mt-1 flex flex-wrap gap-x-3 text-sm text-fg-muted">
                    <span>{s.dry_run ? t("history.dry_run") : t(`history.${s.disposal}`)}</span>
                    <span>{t("history.freed", { size: formatBytes(s.bytes_freed, i18n.language) })}</span>
                    <span>{t("history.removed", { count: s.disposed })}</span>
                    <span>{t("history.skipped", { count: s.skipped })}</span>
                    {s.failed > 0 && <span className="text-risk-danger-fg">{t("history.failed", { count: s.failed })}</span>}
                  </div>
                </div>
                {!s.dry_run && s.disposal === "trash" && (
                  <Button variant="outline" size="sm" onClick={() => void restore(s.id)}>
                    <RotateCcw aria-hidden className="h-4 w-4" />
                    {t("history.restore")}
                  </Button>
                )}
              </Card>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
