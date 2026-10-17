import { useTranslation } from "react-i18next";
import { ChevronRight, FolderOpen } from "lucide-react";
import { Button } from "@/components/ui/button";
import { formatBytes } from "@/lib/format";
import { scanApi } from "@/lib/scanApi";
import type { OrganizeRun } from "@/lib/types";

const dirOf = (p: string) => p.slice(0, p.lastIndexOf("/"));
const nameOf = (p: string) => p.slice(p.lastIndexOf("/") + 1);

/**
 * Where everything went: one row per destination folder (count, size),
 * opening to the files that landed there and the folder they came from.
 * Largest folder first; the first one starts open.
 */
export function MoveSummary({ run }: { run: OrganizeRun }) {
  const { t, i18n } = useTranslation();
  const byDest = new Map<string, OrganizeRun["moves"]>();
  for (const m of run.moves) {
    const d = dirOf(m.to);
    byDest.set(d, [...(byDest.get(d) ?? []), m]);
  }
  const folders = [...byDest.entries()]
    .map(([dir, moves]) => ({ dir, moves, bytes: moves.reduce((a, m) => a + m.bytes, 0) }))
    .sort((a, b) => b.moves.length - a.moves.length);
  const rootName = nameOf(run.root) || run.root;

  return (
    <ul aria-label={t("organize.summary_label")} className="grid gap-2">
      {folders.map((f, i) => (
        <li key={f.dir}>
          <details role="group" aria-label={nameOf(f.dir)} open={i === 0} className="group/acc card-solid">
            <summary className="flex cursor-pointer list-none items-center gap-3 px-4 py-2.5 text-sm [&::-webkit-details-marker]:hidden">
              <ChevronRight aria-hidden className="h-4 w-4 text-fg-faint transition-transform group-open/acc:rotate-90" />
              <span className="min-w-0 truncate font-semibold" title={f.dir}>
                {rootName}/{nameOf(f.dir)}
              </span>
              <span className="num text-fg-muted">{t("organize.items", { count: f.moves.length })}</span>
              <span className="num ml-auto font-medium">{formatBytes(f.bytes, i18n.language)}</span>
              <Button
                variant="ghost"
                size="sm"
                aria-label={t("scan.action_reveal")}
                title={t("scan.action_reveal")}
                onClick={(e) => {
                  e.preventDefault();
                  void scanApi.reveal(f.moves[0]!.to);
                }}
              >
                <FolderOpen aria-hidden className="h-4 w-4" />
              </Button>
            </summary>
            <table className="w-full table-fixed border-t border-border text-sm">
              <thead className="text-xs text-fg-muted">
                <tr>
                  <th className="w-[55%] px-4 py-1.5 text-left font-medium">{t("browse.col_name")}</th>
                  <th className="px-4 py-1.5 text-left font-medium">{t("organize.col_from")}</th>
                  <th className="w-24 px-4 py-1.5 text-right font-medium">{t("browse.col_size")}</th>
                </tr>
              </thead>
              <tbody>
                {f.moves.map((m) => (
                  <tr key={m.to} className="border-t border-border/60 odd:bg-surface-muted/40">
                    <td className="truncate px-4 py-1" title={m.to}>
                      {nameOf(m.to)}
                    </td>
                    <td className="path truncate px-4 py-1 text-fg-muted" title={m.from}>
                      {nameOf(dirOf(m.from)) || dirOf(m.from)}
                    </td>
                    <td className="num px-4 py-1 text-right">{formatBytes(m.bytes, i18n.language)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </details>
        </li>
      ))}
    </ul>
  );
}
