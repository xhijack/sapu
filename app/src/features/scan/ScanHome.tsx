import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { FolderSearch, HardDrive, Home } from "lucide-react";
import { Button } from "@/components/ui/button";
import { scanApi } from "@/lib/scanApi";
import type { PreviousScan, ScanLogEntry, ScanTargets } from "@/lib/types";
import { Tips } from "@/components/Tips";
import { formatBytes, formatDate, formatDuration } from "@/lib/format";
import { cn } from "@/lib/utils";

type Choice = "home" | "disk" | "folder";

interface Props {
  onStart: (root: string) => void;
  error?: string | null;
}

/** First screen: an invitation to scan. One primary action. */
export function ScanHome({ onStart, error }: Props) {
  const { t, i18n } = useTranslation();
  const [targets, setTargets] = useState<ScanTargets | null>(null);
  const [choice, setChoice] = useState<Choice>("home");
  const [folder, setFolder] = useState<string | null>(null);
  const [previous, setPrevious] = useState<Record<string, PreviousScan | null>>({});
  const [recent, setRecent] = useState<ScanLogEntry[]>([]);

  useEffect(() => {
    const paths = [targets?.home, targets?.disk, folder].filter((p): p is string => !!p);
    let live = true;
    for (const p of paths) {
      if (p in previous) continue;
      scanApi.previous(p).then((prev) => live && setPrevious((m) => ({ ...m, [p]: prev }))).catch(() => undefined);
    }
    return () => {
      live = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [targets, folder]);

  useEffect(() => {
    let live = true;
    scanApi.targets().then((tg) => live && setTargets(tg)).catch(() => undefined);
    scanApi.history(6).then((h) => live && setRecent(h)).catch(() => undefined);
    return () => {
      live = false;
    };
  }, []);

  const rows: { id: Choice; Icon: typeof Home; label: string; hint: string; path: string | null }[] = [
    { id: "home", Icon: Home, label: t("scan.target_home"), hint: t("scan.target_home_hint"), path: targets?.home ?? null },
    { id: "disk", Icon: HardDrive, label: t("scan.target_disk"), hint: t("scan.target_disk_hint"), path: targets?.disk ?? null },
    { id: "folder", Icon: FolderSearch, label: t("scan.target_folder"), hint: t("scan.target_folder_hint"), path: folder },
  ];
  const selected = rows.find((r) => r.id === choice);
  const root = selected?.path ?? null;

  const pick = async () => {
    const p = await scanApi.pickFolder().catch(() => null);
    if (p) {
      setFolder(p);
      setChoice("folder");
    }
  };

  return (
    <div className="grid max-w-[1280px] gap-6">
      <div>
        <h1 className="display text-2xl">{t("scan.title")}</h1>
        <p className="mt-1 text-fg-muted">{t("scan.intro")}</p>
      </div>
      <div className="grid gap-6 xl:grid-cols-[minmax(0,1.3fr)_minmax(300px,1fr)]">
      <div className="grid content-start gap-6">
      <div role="radiogroup" aria-label={t("scan.title")} className="grid divide-y divide-border card-solid">
        {rows.map(({ id, Icon, label, hint, path }) => {
          const active = choice === id;
          return (
            <label
              key={id}
              className={cn("flex cursor-pointer items-center gap-4 px-5 py-4 first:rounded-t-xl last:rounded-b-xl", active ? "bg-surface-muted" : "hover:bg-surface-muted/60")}
              onClick={id === "folder" ? () => void pick() : undefined}
            >
              <input type="radio" name="target" value={id} checked={active} onChange={() => setChoice(id)} className="sr-only" />
              <Icon aria-hidden className={cn("h-6 w-6 shrink-0", active ? "text-accent" : "text-fg-muted")} />
              <span className="min-w-0 flex-1">
                <span className="block font-medium">{label}</span>
                <span className="block text-sm text-fg-muted">{hint}</span>
                {path && id === "folder" && (
                  <span className="path mt-1 block truncate text-fg-muted" title={path}>
                    {path}
                  </span>
                )}
                {path && previous[path] && (
                  <span className="num mt-1 block text-xs text-fg-muted">
                    {t("scan.last_scan_line", { date: formatDate(previous[path]!.finished_at_unix, i18n.language), size: formatBytes(previous[path]!.total_bytes, i18n.language) })}
                    {previous[path]!.estimated_secs != null && `. ${t("scan.estimate_line", { time: formatDuration(t, previous[path]!.estimated_secs!) })}`}
                  </span>
                )}
              </span>
            </label>
          );
        })}
      </div>
      {error && (
        <p role="alert" className="text-risk-danger-fg">
          {error}
        </p>
      )}
      <div>
        <Button size="lg" disabled={!root} onClick={() => root && onStart(root)}>
          {t("scan.start")}
        </Button>
      </div>
      </div>
      <div className="grid content-start gap-4">
        <section className="card-solid grid gap-2 p-4" aria-label={t("home.recent_scans")}>
          <h2 className="text-sm font-semibold text-fg-muted">{t("home.recent_scans")}</h2>
          {recent.length === 0 ? (
            <p className="text-sm text-fg-faint">{t("scan.recent_empty")}</p>
          ) : (
            <ul className="grid gap-1.5 text-sm">
              {recent.map((r) => (
                <li key={`${r.root}-${r.finished_at_unix}`} className="grid gap-0.5">
                  <span className="flex items-baseline justify-between gap-3">
                    <span className="path min-w-0 truncate" title={r.root}>
                      {r.root.split("/").filter(Boolean).pop() || r.root}
                    </span>
                    <span className="num shrink-0 font-medium">{formatBytes(r.total_bytes, i18n.language)}</span>
                  </span>
                  <span className="num text-xs text-fg-muted">
                    {formatDate(r.finished_at_unix, i18n.language)}, {t("scan.files_count", { count: r.file_count.toLocaleString(i18n.language) })}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </section>
        <Tips seed={2} />
      </div>
      </div>
    </div>
  );
}
