import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { DialogClose } from "@/components/ui/DialogClose";
import { formatBytes } from "@/lib/format";
import { scanApi } from "@/lib/scanApi";
import { useErrorMessage } from "@/lib/errors";
import type { CleanSession } from "@/lib/types";

interface Props {
  paths: string[];
  onClose: (result: CleanSession | null) => void;
}

/** Preview (dry run via safety) → confirm → apply. Nothing moves before the click. */
export function TrashDialog({ paths, onClose }: Props) {
  const { t, i18n } = useTranslation();
  const errorMessage = useErrorMessage();
  const [preview, setPreview] = useState<CleanSession | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const ref = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    ref.current?.showModal();
    let live = true;
    scanApi
      .trash(paths, false)
      .then((s) => live && setPreview(s))
      .catch((e) => live && setError(errorMessage(e)));
    return () => {
      live = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [paths]);

  const movable = preview?.entries.filter((e) => e.outcome.status === "would_dispose") ?? [];
  const size = movable.reduce((a, e) => a + e.size, 0);

  const confirm = async () => {
    setBusy(true);
    try {
      const s = await scanApi.trash(
        movable.map((e) => e.path),
        true,
      );
      onClose(s);
    } catch (e) {
      setError(errorMessage(e));
      setBusy(false);
    }
  };

  return (
    <dialog
      ref={ref}
      onClose={() => onClose(null)}
      aria-labelledby="trash-title"
      className="m-auto w-full max-w-lg card-solid p-0 text-fg shadow-xl backdrop:bg-black/40"
    >
      <div className="grid gap-4 p-6">
        <div className="flex items-start justify-between gap-3">
          <h2 id="trash-title" className="flex items-center gap-2 text-lg font-semibold">
          <Trash2 aria-hidden className="h-5 w-5" />
          {t("scan.trash_title")}
        </h2>
          <DialogClose onClick={() => ref.current?.close()} />
        </div>
        {error ? (
          <p role="alert" className="text-risk-danger-fg">
            {error}
          </p>
        ) : preview ? (
          <>
            <p className="text-fg-muted">{t("scan.trash_body", { count: movable.length, size: formatBytes(size, i18n.language) })}</p>
            <ul className="max-h-56 overflow-y-auto rounded-md border border-border">
              {preview.entries.map((e) => (
                <li key={e.path} className="flex items-center justify-between gap-3 border-b border-border px-3 py-2 last:border-0">
                  <span className="path min-w-0 truncate" title={e.path}>
                    {e.path}
                  </span>
                  {e.outcome.status === "would_dispose" ? (
                    <span className="num shrink-0 text-sm text-fg-muted">{formatBytes(e.size, i18n.language)}</span>
                  ) : (
                    <span className="shrink-0 text-sm text-risk-review-fg">
                      {e.outcome.status === "protected" ? t("scan.trash_protected") : t("scan.trash_skipped")}
                    </span>
                  )}
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
          <Button onClick={() => void confirm()} disabled={busy || movable.length === 0}>
            {t("scan.trash_confirm")}
          </Button>
        </div>
      </div>
    </dialog>
  );
}
