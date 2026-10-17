import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { Star } from "lucide-react";
import { Button } from "@/components/ui/button";
import { DialogClose } from "@/components/ui/DialogClose";
import { api } from "@/lib/api";
import { formatBytes } from "@/lib/format";
import { STORE_LINKS } from "@/lib/storeLinks";

/** After a clean pushed the lifetime total past the threshold: one ask, never again. */
export function RatePrompt({ freed, os, onClose }: { freed: number; os: string; onClose: () => void }) {
  const { t, i18n } = useTranslation();
  const ref = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    ref.current?.showModal();
  }, []);
  const link = STORE_LINKS[os] ?? null;
  return (
    <dialog ref={ref} onClose={onClose} aria-labelledby="rate-title" className="m-auto w-full max-w-md card-solid p-0 text-fg shadow-xl backdrop:bg-black/40">
      <div className="grid gap-4 p-6">
        <div className="flex items-start justify-between gap-3">
          <h2 id="rate-title" className="flex items-center gap-2 text-lg font-semibold">
            <Star aria-hidden className="h-5 w-5 text-cat-document" />
            {t("rate.title", { size: formatBytes(freed, i18n.language) })}
          </h2>
          <DialogClose onClick={() => ref.current?.close()} />
        </div>
        <p className="text-fg-muted">{link ? t("rate.body") : t("rate.body_soon")}</p>
        <div className="flex justify-end gap-2">
          <Button variant="secondary" onClick={() => ref.current?.close()}>
            {t("rate.later")}
          </Button>
          {link && (
            <Button
              onClick={() => {
                void api.openUrl(link);
                ref.current?.close();
              }}
            >
              <Star aria-hidden className="h-4 w-4" />
              {t("rate.go")}
            </Button>
          )}
        </div>
      </div>
    </dialog>
  );
}
