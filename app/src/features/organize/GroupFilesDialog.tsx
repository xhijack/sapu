import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { DialogClose } from "@/components/ui/DialogClose";
import { KindIcon } from "@/components/icons/KindIcon";
import { formatBytes } from "@/lib/format";
import type { OrganizeGroup } from "@/lib/types";

interface Props {
  group: OrganizeGroup;
  /** Entries that move (the folder's own entry is left out by the caller). */
  entries: OrganizeGroup["entries"];
  excluded: Set<string>;
  onToggle: (path: string) => void;
  onClose: () => void;
}

/** The files of one group; untick a file to leave it where it is. */
export function GroupFilesDialog({ group, entries, excluded, onToggle, onClose }: Props) {
  const { t, i18n } = useTranslation();
  const ref = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    ref.current?.showModal();
  }, []);
  return (
    <dialog ref={ref} onClose={onClose} aria-labelledby="group-files-title" className="m-auto w-full max-w-xl card-solid p-0 text-fg shadow-xl backdrop:bg-black/40">
      <div className="grid max-h-[80vh] gap-3 p-6">
        <div className="flex items-start justify-between gap-3">
          <h2 id="group-files-title" className="min-w-0 truncate text-lg font-semibold">
            {group.name}
          </h2>
          <DialogClose onClick={() => ref.current?.close()} />
        </div>
        <p className="text-sm text-fg-muted">{t("organize.files_hint")}</p>
        <ul className="grid min-h-0 gap-0.5 overflow-y-auto rounded-lg border border-border">
          {entries.map((e) => (
            <li key={e.path}>
              <label className="flex items-center gap-3 px-3 py-1.5 text-sm hover:bg-surface-muted">
                <input type="checkbox" checked={!excluded.has(e.path)} onChange={() => onToggle(e.path)} aria-label={e.name} className="h-4 w-4 accent-[var(--accent)]" />
                <KindIcon kind={e.kind} />
                <span className="min-w-0 flex-1 truncate" title={e.path}>
                  {e.name}
                </span>
                <span className="num shrink-0 text-fg-muted">{formatBytes(e.bytes, i18n.language)}</span>
              </label>
            </li>
          ))}
        </ul>
      </div>
    </dialog>
  );
}
