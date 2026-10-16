import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { appsApi } from "@/lib/appsApi";
import { useErrorMessage } from "@/lib/errors";
import { formatBytes } from "@/lib/format";
import type { CleanSession, InstalledApp } from "@/lib/types";

interface Props {
  app: InstalledApp | null;
  leftovers: string[];
  onClose: (result: CleanSession | null) => void;
}

/** Dry run through safety first; the Trash move happens on confirm. */
export function UninstallDialog({ app, leftovers, onClose }: Props) {
  const { t, i18n } = useTranslation();
  const errorMessage = useErrorMessage();
  const ref = useRef<HTMLDialogElement>(null);
  const [preview, setPreview] = useState<CleanSession | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    ref.current?.showModal();
    let live = true;
    appsApi
      .uninstall(app, leftovers, false)
      .then((s) => live && setPreview(s))
      .catch((e) => live && setError(errorMessage(e)));
    return () => {
      live = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const confirm = async () => {
    setBusy(true);
    try {
      onClose(await appsApi.uninstall(app, leftovers, true));
    } catch (e) {
      setError(errorMessage(e));
      setBusy(false);
    }
  };

  const movable = preview?.entries.filter((e) => e.outcome.status === "would_dispose") ?? [];
  const size = movable.reduce((n, e) => n + e.size, 0) + (app && movable.some((e) => e.path === app.path) ? app.bytes : 0);

  return (
    <dialog ref={ref} onClose={() => onClose(null)} aria-labelledby="uninstall-title" className="m-auto w-full max-w-lg rounded-xl border border-border bg-surface-solid p-0 text-fg shadow-xl backdrop:bg-black/40">
      <div className="grid gap-4 p-6">
        <h2 id="uninstall-title" className="text-lg font-semibold">
          {t("apps.confirm_title")}
        </h2>
        {error ? (
          <p role="alert" className="text-risk-danger-fg">
            {error}
          </p>
        ) : preview ? (
          <>
            <p className="text-fg-muted">{t("apps.confirm_body", { count: movable.length, size: formatBytes(size, i18n.language) })}</p>
            <ul className="max-h-56 overflow-y-auto rounded-md border border-border text-sm">
              {preview.entries.map((e) => (
                <li key={e.path} className="flex items-center justify-between gap-3 border-b border-border px-3 py-2 last:border-0">
                  <span className="path min-w-0 truncate" title={e.path}>
                    {e.path}
                  </span>
                  <span className={e.outcome.status === "would_dispose" ? "shrink-0 text-xs text-fg-muted" : "shrink-0 text-xs text-risk-review-fg"}>
                    {e.outcome.status === "would_dispose" ? "" : e.outcome.status === "protected" ? t("scan.trash_protected") : t("scan.trash_skipped")}
                  </span>
                </li>
              ))}
            </ul>
          </>
        ) : (
          <p className="text-fg-muted">{t("common.loading")}</p>
        )}
        <div className="flex justify-end gap-2">
          <Button variant="secondary" onClick={() => ref.current?.close()}>
            {t("common.cancel")}
          </Button>
          <Button onClick={() => void confirm()} disabled={busy || movable.length === 0 || !!error}>
            {t("scan.trash_confirm")}
          </Button>
        </div>
      </div>
    </dialog>
  );
}
