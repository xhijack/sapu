import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { EmptyState, LoadingState } from "@/components/States";
import { appsApi } from "@/lib/appsApi";
import { formatBytes } from "@/lib/format";
import type { InstalledApp } from "@/lib/types";
import { FileTable, type FileRow } from "./FileTable";

interface Props {
  onTrash: (paths: string[]) => void;
  removed: Set<string>;
}

/**
 * Apps whose binary does not fit this Mac: Intel-only ones run through
 * Rosetta (slower), 32-bit ones cannot run at all. Trashing the bundle goes
 * through the same safety path as everything else; leftovers stay in Apps.
 */
export function UnsupportedApps({ onTrash, removed }: Props) {
  const { t, i18n } = useTranslation();
  const [apps, setApps] = useState<InstalledApp[] | null>(null);
  useEffect(() => {
    let live = true;
    appsApi
      .list()
      .then((a) => live && setApps(a))
      .catch(() => live && setApps([]));
    return () => {
      live = false;
    };
  }, []);

  const flagged = useMemo(() => (apps ?? []).filter((a) => a.support === "rosetta" || a.support === "unsupported").filter((a) => !a.is_system), [apps]);
  const rows: FileRow[] = flagged.map((a) => ({ path: a.path, name: a.name, kind: "application", is_dir: true, bytes: a.bytes, atime: a.last_used, mtime: null }));

  if (apps === null) return <LoadingState />;
  if (flagged.length === 0) return <EmptyState title={t("browse.unsupported_empty_title")} body={t("browse.unsupported_empty_body")} />;
  const bytes = flagged.reduce((a, x) => a + x.bytes, 0);
  return (
    <div className="grid gap-4">
      <p className="text-sm text-fg-muted">{t("browse.unsupported_intro", { count: flagged.length, size: formatBytes(bytes, i18n.language) })}</p>
      <FileTable
        rows={rows}
        onTrash={onTrash}
        removed={removed}
        nameExtra={(r) => {
          const a = flagged.find((x) => x.path === r.path);
          if (!a) return null;
          const bad = a.support === "unsupported";
          return <span className={bad ? "rounded-full bg-risk-danger-bg px-2 py-0.5 text-xs text-risk-danger-fg" : "rounded-full bg-risk-review-bg px-2 py-0.5 text-xs text-risk-review-fg"}>{t(`browse.support.${a.support}`)}</span>;
        }}
      />
    </div>
  );
}
