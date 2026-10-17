import { useTranslation } from "react-i18next";
import { FolderOpen, Lock, Sparkles, TriangleAlert } from "lucide-react";
import { Button } from "@/components/ui/button";
import { formatBytes } from "@/lib/format";
import type { OrganizeEntry, OrganizeGroup } from "@/lib/types";
import { cn } from "@/lib/utils";

interface Props {
  group: OrganizeGroup;
  /** Entries that would move (own folder entry and excluded files removed). */
  moving: OrganizeEntry[];
  ticked: boolean;
  name: string;
  onTick: (v: boolean) => void;
  onRename: (name: string) => void;
  onOpen: () => void;
  /** Why the group waits for the person (shown while it is not ticked). */
  waiting?: "folder" | "check" | null;
}

/** One proposed group: tick, destination name, why it was grouped, examples. */
export function PlanGroupCard({ group, moving, ticked, name, onTick, onRename, onOpen, waiting = null }: Props) {
  const { t, i18n } = useTranslation();
  const fixed = group.source === "existing" || group.source === "unit";
  const bytes = moving.reduce((a, e) => a + e.bytes, 0);
  return (
    <section role="group" aria-label={group.name} className={cn("card-solid grid min-w-0 gap-2 p-4", !ticked && waiting && "border-dashed border-risk-review-fg/50")}>
      <div className="flex items-start gap-3">
        <input type="checkbox" checked={ticked} disabled={moving.length === 0} onChange={(e) => onTick(e.target.checked)} aria-label={t("organize.tick", { name: group.name })} className="mt-1.5 h-4 w-4 accent-[var(--accent)]" />
        <div className="grid min-w-0 flex-1 gap-1">
          {fixed ? (
            <span className="flex items-center gap-1.5 truncate font-semibold" title={t("organize.fixed_hint")}>
              {group.name}
              <Lock aria-hidden className="h-3.5 w-3.5 text-fg-faint" />
            </span>
          ) : (
            <input value={name} onChange={(e) => onRename(e.target.value)} aria-label={t("organize.folder_name")} className="h-8 min-w-0 rounded-md border border-border bg-surface px-2 font-semibold" />
          )}
          <span className="flex flex-wrap items-center gap-1.5 text-xs">
            <span className="rounded-full bg-surface-muted px-2 py-0.5">{t(`organize.source.${group.source}`)}</span>
            {group.source === "ai" && <Sparkles aria-hidden className="h-3.5 w-3.5 text-risk-review-fg" />}
            <span className={cn("rounded-full px-2 py-0.5", group.confidence === "sure" ? "bg-risk-safe-bg text-risk-safe-fg" : "bg-risk-review-bg text-risk-review-fg")}>{t(`organize.confidence.${group.confidence}`)}</span>
          </span>
        </div>
        <div className="num shrink-0 text-right">
          <div className="font-medium">{formatBytes(bytes, i18n.language)}</div>
          <div className="text-xs text-fg-muted">{t("organize.items", { count: moving.length })}</div>
        </div>
      </div>
      {!ticked && waiting && (
        <p className="flex items-center gap-1.5 pl-7 text-sm font-medium text-risk-review-fg">
          <TriangleAlert aria-hidden className="h-4 w-4 shrink-0" />
          {t(`organize.waiting_${waiting}`)}
        </p>
      )}
      {moving.length > 0 ? (
        <p className="truncate pl-7 text-xs text-fg-muted" title={moving.map((e) => e.name).join(", ")}>
          {moving
            .slice(0, 3)
            .map((e) => e.name)
            .join(", ")}
          {moving.length > 3 ? ` +${moving.length - 3}` : ""}
        </p>
      ) : (
        <p className="pl-7 text-xs text-fg-faint">{t("organize.nothing_to_move")}</p>
      )}
      {moving.length > 0 && (
        <div className="pl-5">
          <Button variant="ghost" size="sm" onClick={onOpen}>
            <FolderOpen aria-hidden className="h-4 w-4" />
            {t("organize.show_files")}
          </Button>
        </div>
      )}
    </section>
  );
}
