import { useState } from "react";
import { useTranslation } from "react-i18next";
import { FileText, PenLine, Trash2, TriangleAlert } from "lucide-react";
import { Button } from "@/components/ui/button";
import { formatBytes } from "@/lib/format";
import type { OrganizeEntry } from "@/lib/types";

interface RenameProps {
  entries: OrganizeEntry[];
  /** path → accepted new name; absent means not ticked. */
  accepted: Record<string, string>;
  drafts: Record<string, string>;
  onTick: (e: OrganizeEntry, v: boolean) => void;
  onEdit: (e: OrganizeEntry, name: string) => void;
}

/** AI suggestions for clearer file names. Nothing is ticked by default. */
export function RenameSuggestions({ entries, accepted, drafts, onTick, onEdit }: RenameProps) {
  const { t } = useTranslation();
  if (entries.length === 0) return null;
  return (
    <section aria-label={t("organize.renames_title")} className="card-solid grid gap-2 p-4">
      <h2 className="flex items-center gap-2 font-semibold">
        <PenLine aria-hidden className="h-4 w-4 text-accent" />
        {t("organize.renames_title")}
      </h2>
      <p className="text-xs text-fg-muted">{t("organize.renames_hint")}</p>
      <ul className="grid gap-1.5">
        {entries.map((e) => (
          <li key={e.path} className="grid items-center gap-2 sm:grid-cols-[auto_minmax(0,1fr)_minmax(0,1fr)]">
            <input type="checkbox" checked={e.path in accepted} onChange={(ev) => onTick(e, ev.target.checked)} aria-label={t("organize.rename_tick", { name: e.name })} className="h-4 w-4 accent-[var(--accent)]" />
            <span className="path min-w-0 truncate text-sm text-fg-muted" title={e.path}>
              {e.name}
            </span>
            <input value={drafts[e.path] ?? e.suggested_name ?? ""} onChange={(ev) => onEdit(e, ev.target.value)} aria-label={t("organize.rename_new", { name: e.name })} className="h-8 min-w-0 rounded-md border border-border bg-surface px-2 text-sm" />
          </li>
        ))}
      </ul>
    </section>
  );
}

/**
 * Files the AI thinks are junk. AI labels are yellow at most: nothing is
 * ticked, each shows the reason, and ticked files go to the Trash through
 * the safety layer (recorded, restorable), never deleted.
 */
export function JunkSuggestions({ entries, onTrash }: { entries: OrganizeEntry[]; onTrash: (paths: string[]) => Promise<number> }) {
  const { t, i18n } = useTranslation();
  const [picked, setPicked] = useState<Set<string>>(new Set());
  const [busy, setBusy] = useState(false);
  const [trashed, setTrashed] = useState<number | null>(null);
  if (entries.length === 0 && trashed === null) return null;
  const go = async () => {
    setBusy(true);
    try {
      setTrashed(await onTrash([...picked]));
      setPicked(new Set());
    } finally {
      setBusy(false);
    }
  };
  return (
    <section aria-label={t("organize.junk_title")} className="grid gap-2 rounded-xl border border-border bg-risk-review-bg p-4">
      <h2 className="flex items-center gap-2 font-semibold text-risk-review-fg">
        <TriangleAlert aria-hidden className="h-4 w-4" />
        {t("organize.junk_title")}
      </h2>
      <p className="text-xs">{t("organize.junk_hint")}</p>
      {entries.length > 0 && (
        <ul className="grid gap-1.5">
          {entries.map((e) => (
            <li key={e.path} className="flex items-start gap-2 text-sm">
              <input
                type="checkbox"
                checked={picked.has(e.path)}
                onChange={(ev) =>
                  setPicked((s) => {
                    const n = new Set(s);
                    if (ev.target.checked) n.add(e.path);
                    else n.delete(e.path);
                    return n;
                  })
                }
                aria-label={t("organize.junk_tick", { name: e.name })}
                className="mt-0.5 h-4 w-4 accent-[var(--accent)]"
              />
              <span className="min-w-0 flex-1">
                <span className="path block truncate" title={e.path}>
                  {e.name}
                </span>
                <span className="block text-xs text-fg-muted">{e.junk}</span>
              </span>
              <span className="num shrink-0 text-xs text-fg-muted">{formatBytes(e.bytes, i18n.language)}</span>
            </li>
          ))}
        </ul>
      )}
      <div className="flex flex-wrap items-center gap-3">
        {entries.length > 0 && (
          <Button variant="outline" size="sm" disabled={picked.size === 0 || busy} onClick={() => void go()}>
            <Trash2 aria-hidden className="h-4 w-4" />
            {t("organize.junk_trash", { count: picked.size })}
          </Button>
        )}
        {trashed !== null && (
          <span role="status" className="text-sm">
            {t("organize.junk_trashed", { count: trashed })}
          </span>
        )}
      </div>
    </section>
  );
}

/** Items no group claims: they stay where they are. */
export function Unplaced({ entries }: { entries: OrganizeEntry[] }) {
  const { t } = useTranslation();
  if (entries.length === 0) return null;
  return (
    <section aria-label={t("organize.unplaced_title")} className="card-solid grid gap-2 p-4">
      <h2 className="font-semibold">
        {t("organize.unplaced_title")} <span className="num text-sm font-normal text-fg-muted">· {t("organize.items", { count: entries.length })}</span>
      </h2>
      <p className="text-xs text-fg-muted">{t("organize.unplaced_hint")}</p>
      <ul className="flex flex-wrap gap-1.5 text-xs">
        {entries.map((e) => (
          <li key={e.path} className="flex max-w-full items-center gap-1 rounded-full bg-surface-muted px-2 py-0.5" title={e.path}>
            <FileText aria-hidden className="h-3 w-3 shrink-0 text-fg-faint" />
            <span className="truncate">{e.name}</span>
          </li>
        ))}
      </ul>
    </section>
  );
}
