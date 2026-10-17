import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ArrowRight, ChevronRight, File, Folder, FolderOpen } from "lucide-react";
import { Button } from "@/components/ui/button";
import { formatBytes } from "@/lib/format";
import { scanApi } from "@/lib/scanApi";
import type { OrganizeRun } from "@/lib/types";
import { cn } from "@/lib/utils";

const dirOf = (p: string) => p.slice(0, p.lastIndexOf("/"));
const nameOf = (p: string) => p.slice(p.lastIndexOf("/") + 1);

/**
 * The folder before and after organizing, side by side. Left: the loose
 * items as they were. Right: the new folders with their contents. Picking a
 * folder on the right highlights where its items were on the left.
 */
export function BeforeAfter({ run }: { run: OrganizeRun }) {
  const { t, i18n } = useTranslation();
  const lang = i18n.language;
  const rootName = nameOf(run.root) || run.root;

  const byDest = new Map<string, OrganizeRun["moves"]>();
  for (const m of run.moves) {
    const d = dirOf(m.to);
    byDest.set(d, [...(byDest.get(d) ?? []), m]);
  }
  const folders = [...byDest.entries()]
    .map(([dir, moves]) => ({ dir, moves, bytes: moves.reduce((a, m) => a + m.bytes, 0) }))
    .sort((a, b) => b.moves.length - a.moves.length);
  const before = [...run.moves].sort((a, b) => nameOf(a.from).localeCompare(nameOf(b.from), undefined, { numeric: true, sensitivity: "base" }));

  const [open, setOpen] = useState<string | null>(folders[0]?.dir ?? null);
  const highlighted = new Set((open ? byDest.get(open) : [])?.map((m) => m.from));

  return (
    <div aria-label={t("organize.summary_label")} className="grid items-start gap-3 lg:grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)]">
      <section aria-label={t("organize.before")} className="card-solid grid min-w-0 gap-1 p-3">
        <h2 className="flex items-center gap-2 px-1 text-sm font-semibold text-fg-muted">
          {t("organize.before")}
          <span className="num font-normal">· {t("organize.loose_items", { count: before.length, folder: rootName })}</span>
        </h2>
        <ul className="grid max-h-[52vh] gap-px overflow-y-auto text-sm">
          {before.map((m) => {
            const lit = highlighted.has(m.from);
            return (
              <li key={m.from} data-highlight={lit ? "true" : "false"} className={cn("flex items-center gap-2 rounded-md px-2 py-1", lit ? "bg-accent-soft" : "")}>
                <File aria-hidden className="h-3.5 w-3.5 shrink-0 text-fg-faint" />
                <span className="min-w-0 flex-1 truncate" title={m.from}>
                  {nameOf(m.from)}
                </span>
                <span className="num shrink-0 text-xs text-fg-muted">{formatBytes(m.bytes, lang)}</span>
              </li>
            );
          })}
        </ul>
      </section>

      <ArrowRight aria-hidden className="mx-auto hidden h-5 w-5 self-center text-fg-faint lg:block" />

      <section aria-label={t("organize.after")} className="card-solid grid min-w-0 gap-1 p-3">
        <h2 className="flex items-center gap-2 px-1 text-sm font-semibold text-fg-muted">
          {t("organize.after")}
          <span className="num font-normal">· {t("organize.new_folders", { count: folders.length, folder: rootName })}</span>
        </h2>
        <ul className="grid max-h-[52vh] gap-1 overflow-y-auto text-sm">
          {folders.map((f) => {
            const isOpen = open === f.dir;
            return (
              <li key={f.dir} role="group" aria-label={nameOf(f.dir)} className={cn("rounded-lg", isOpen && "bg-surface-muted/60")}>
                <div className="flex items-center gap-2 px-2 py-1">
                  <button type="button" aria-expanded={isOpen} onClick={() => setOpen(isOpen ? null : f.dir)} className="flex min-w-0 flex-1 items-center gap-2 text-left">
                    <ChevronRight aria-hidden className={cn("h-4 w-4 shrink-0 text-fg-faint transition-transform", isOpen && "rotate-90")} />
                    <Folder aria-hidden className="h-4 w-4 shrink-0 text-accent" />
                    <span className="min-w-0 truncate font-medium" title={f.dir}>
                      {nameOf(f.dir)}
                    </span>
                    <span className="num shrink-0 text-xs text-fg-muted">{t("organize.items", { count: f.moves.length })}</span>
                  </button>
                  <span className="num shrink-0 text-xs text-fg-muted">{formatBytes(f.bytes, lang)}</span>
                  <Button variant="ghost" size="sm" aria-label={t("scan.action_reveal")} title={t("scan.action_reveal")} onClick={() => void scanApi.reveal(f.moves[0]!.to)}>
                    <FolderOpen aria-hidden className="h-4 w-4" />
                  </Button>
                </div>
                {isOpen && (
                  <ul className="grid gap-px pb-1 pl-9 pr-2">
                    {f.moves.map((m) => (
                      <li key={m.to} className="flex items-center gap-2 py-0.5">
                        <File aria-hidden className="h-3.5 w-3.5 shrink-0 text-fg-faint" />
                        <span className="min-w-0 flex-1 truncate" title={m.to}>
                          {nameOf(m.to)}
                        </span>
                        <span className="num shrink-0 text-xs text-fg-muted">{formatBytes(m.bytes, lang)}</span>
                      </li>
                    ))}
                  </ul>
                )}
              </li>
            );
          })}
        </ul>
      </section>
    </div>
  );
}
