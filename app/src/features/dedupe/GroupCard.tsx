import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { FileText, Image as ImageIcon } from "lucide-react";
import { formatBytes, formatDate } from "@/lib/format";
import { dedupeApi } from "@/lib/dedupeApi";
import type { DuplicateGroup, FilePreview } from "@/lib/types";
import { cn } from "@/lib/utils";

interface Props {
  group: DuplicateGroup;
  keep: string;
  onKeep: (path: string) => void;
}

/** One duplicate group: copies side by side, radio marks the one to keep. */
export function GroupCard({ group, keep, onKeep }: Props) {
  const { t, i18n } = useTranslation();
  return (
    <section aria-label={group.files[0]?.path.split("/").pop()} className="grid gap-3 rounded-xl border border-border bg-surface p-4">
      <p className="num text-sm text-fg-muted">{t("dedupe.copies", { count: group.files.length, size: formatBytes(group.size, i18n.language) })}</p>
      <div role="radiogroup" aria-label={t("dedupe.strategy")} className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
        {group.files.map((f) => {
          const kept = f.path === keep;
          return (
            <label key={f.path} className={cn("grid cursor-pointer gap-2 rounded-lg border p-3", kept ? "border-accent" : "border-border")}>
              <Preview path={f.path} />
              <span className="path block truncate text-xs" title={f.path}>
                {f.path}
              </span>
              <span className="flex items-center justify-between gap-2 text-xs text-fg-muted">
                <span>{f.mtime ? formatDate(f.mtime.secs_since_epoch, i18n.language) : ""}</span>
                <span className={kept ? "font-medium text-accent" : "text-risk-review-fg"}>{kept ? t("dedupe.kept") : t("dedupe.will_remove")}</span>
              </span>
              <span className="flex items-center gap-2 text-sm">
                <input type="radio" name={`keep-${group.id}`} checked={kept} onChange={() => onKeep(f.path)} aria-label={`${t("dedupe.keep_this")}: ${f.path}`} className="h-4 w-4 accent-[var(--accent)]" />
                {t("dedupe.keep_this")}
              </span>
            </label>
          );
        })}
      </div>
    </section>
  );
}

function Preview({ path }: { path: string }) {
  const { t } = useTranslation();
  const [p, setP] = useState<FilePreview | null>(null);
  useEffect(() => {
    let live = true;
    dedupeApi.preview(path).then((x) => live && setP(x)).catch(() => live && setP({ kind: "none" }));
    return () => {
      live = false;
    };
  }, [path]);
  if (p?.kind === "image") return <img src={p.data_url} alt="" className="h-28 w-full rounded-md bg-surface-muted object-cover" />;
  if (p?.kind === "text")
    return (
      <pre className="h-28 overflow-hidden rounded-md bg-surface-muted p-2 text-[10px] leading-tight text-fg-muted">
        {p.excerpt}
      </pre>
    );
  return (
    <div className="flex h-28 items-center justify-center gap-2 rounded-md bg-surface-muted text-xs text-fg-muted">
      {p === null ? <ImageIcon aria-hidden className="h-5 w-5" /> : <FileText aria-hidden className="h-5 w-5" />}
      {p !== null && t("dedupe.no_preview")}
    </div>
  );
}
