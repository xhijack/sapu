import { useEffect, useState } from "react";
import { api } from "@/lib/api";
import { useTranslation } from "react-i18next";
import type { Category } from "@/lib/types";
import { cn } from "@/lib/utils";
import { DownloadsPanel } from "./DownloadsPanel";
import { FolderBrowser } from "./FolderBrowser";
import { LargeFiles } from "./LargeFiles";
import { UnsupportedApps } from "./UnsupportedApps";

export type BrowseTab = "large" | "downloads" | "unsupported" | "folders";

interface Props {
  root: string;
  initialTab?: BrowseTab;
  initialCategory?: Category | null;
  onTrash: (paths: string[]) => void;
  removed: Set<string>;
  onWhatIs?: (path: string) => void;
  aiEnabled?: boolean;
}

/** The "Jelajah" tab: four ways into the same scan, one table design. */
export function BrowseView({ root, initialTab = "large", initialCategory = null, onTrash, removed, onWhatIs, aiEnabled }: Props) {
  const { t } = useTranslation();
  const [tab, setTab] = useState<BrowseTab>(initialTab);
  const [os, setOs] = useState<string>("macos");
  useEffect(() => {
    let live = true;
    api
      .appInfo()
      .then((i) => live && setOs(i.os))
      .catch(() => undefined);
    return () => {
      live = false;
    };
  }, []);
  const tabs: BrowseTab[] = os === "macos" ? ["large", "downloads", "unsupported", "folders"] : ["large", "downloads", "folders"];
  return (
    <div className="grid gap-5">
      <div role="tablist" aria-label={t("scan.tab_browse")} className="inline-flex w-fit rounded-xl bg-surface-muted p-1">
        {tabs.map((id) => (
          <button key={id} role="tab" type="button" aria-selected={tab === id} onClick={() => setTab(id)} className={cn("rounded-lg px-4 py-1.5 text-sm font-medium transition-colors", tab === id ? "bg-surface-solid text-fg shadow-[var(--shadow-1)]" : "text-fg-muted hover:text-fg")}>
            {t(`browse.tab_${id}`)}
          </button>
        ))}
      </div>
      <div role="tabpanel">
        {tab === "large" && <LargeFiles key={initialCategory ?? "all"} initialCategory={initialCategory} onTrash={onTrash} removed={removed} onWhatIs={onWhatIs} aiEnabled={aiEnabled} />}
        {tab === "downloads" && <DownloadsPanel onTrash={onTrash} removed={removed} onWhatIs={onWhatIs} refreshKey={removed.size} />}
        {tab === "unsupported" && <UnsupportedApps onTrash={onTrash} removed={removed} />}
        {tab === "folders" && <FolderBrowser root={root} onTrash={onTrash} removed={removed} onWhatIs={onWhatIs} />}
      </div>
    </div>
  );
}
