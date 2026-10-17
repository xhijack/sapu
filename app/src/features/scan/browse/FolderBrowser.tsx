import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { ChevronRight, CornerLeftUp, FolderInput } from "lucide-react";
import { requestOrganize } from "@/features/organize/organizeBus";
import { Button } from "@/components/ui/button";
import { EmptyState, LoadingState } from "@/components/States";
import { formatBytes } from "@/lib/format";
import { scanApi } from "@/lib/scanApi";
import type { BrowseFolder } from "@/lib/types";
import { FileTable, type FileRow } from "./FileTable";

interface Props {
  root: string;
  onTrash: (paths: string[]) => void;
  removed: Set<string>;
  onWhatIs?: (path: string) => void;
}

/** Column-less Finder: one folder at a time, largest first, with a crumb trail. */
export function FolderBrowser({ root, onTrash, removed, onWhatIs }: Props) {
  const { t, i18n } = useTranslation();
  const [path, setPath] = useState(root);
  const [loaded, setLoaded] = useState<{ path: string; folder: BrowseFolder | null } | null>(null);
  const folder: BrowseFolder | null | undefined = loaded && loaded.path === path ? loaded.folder : undefined;

  useEffect(() => {
    let live = true;
    scanApi
      .browse(path)
      .then((f) => live && setLoaded({ path, folder: f }))
      .catch(() => live && setLoaded({ path, folder: null }));
    return () => {
      live = false;
    };
  }, [path]);

  const crumbs = useMemo(() => {
    const rel = path.startsWith(root) ? path.slice(root.length) : path;
    const parts = rel.split("/").filter(Boolean);
    return [{ label: root.split("/").filter(Boolean).pop() || root, path: root }, ...parts.map((p, i) => ({ label: p, path: `${root}/${parts.slice(0, i + 1).join("/")}` }))];
  }, [path, root]);

  const rows: FileRow[] = (folder?.entries ?? []).filter((e) => !removed.has(e.path)).map((e) => ({ path: e.path, name: e.name, kind: e.kind, is_dir: e.is_dir, bytes: e.bytes, atime: e.atime, mtime: e.mtime, category: e.category }));
  const up = () => {
    if (path === root) return;
    const parent = path.slice(0, path.lastIndexOf("/")) || root;
    setPath(parent.length < root.length ? root : parent);
  };

  return (
    <div className="grid gap-3">
      <nav aria-label={t("browse.crumbs")} className="flex flex-wrap items-center gap-1 text-sm">
        <Button variant="ghost" size="sm" aria-label={t("scan.map_up")} title={t("scan.map_up")} disabled={path === root} onClick={up}>
          <CornerLeftUp aria-hidden className="h-4 w-4" />
        </Button>
        {crumbs.map((c, i) => (
          <span key={c.path} className="flex items-center gap-1">
            {i > 0 && <ChevronRight aria-hidden className="h-3.5 w-3.5 text-fg-faint" />}
            {i === crumbs.length - 1 ? (
              <span className="font-medium" aria-current="location">
                {c.label}
              </span>
            ) : (
              <button type="button" className="text-fg-muted hover:text-fg hover:underline" onClick={() => setPath(c.path)}>
                {c.label}
              </button>
            )}
          </span>
        ))}
        {folder && <span className="num ml-auto text-fg-muted">{t("browse.folder_total", { size: formatBytes(folder.bytes, i18n.language), count: folder.files })}</span>}
        <Button variant="outline" size="sm" onClick={() => requestOrganize(path)}>
          <FolderInput aria-hidden className="h-4 w-4" />
          {t("browse.organize_here")}
        </Button>
      </nav>
      {folder === undefined ? <LoadingState /> : folder === null || rows.length === 0 ? <EmptyState title={t("browse.folder_empty")} body="" /> : <FileTable rows={rows} onOpenDir={setPath} onTrash={onTrash} onWhatIs={onWhatIs} removed={removed} />}
      {folder && folder.more > 0 && <p className="text-sm text-fg-muted">{t("browse.folder_more", { count: folder.more })}</p>}
    </div>
  );
}
