import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { FolderSearch, HardDrive, Home } from "lucide-react";
import { Button } from "@/components/ui/button";
import { scanApi } from "@/lib/scanApi";
import type { ScanTargets } from "@/lib/types";
import { cn } from "@/lib/utils";

type Choice = "home" | "disk" | "folder";

interface Props {
  onStart: (root: string) => void;
  error?: string | null;
}

/** First screen: an invitation to scan. One primary action. */
export function ScanHome({ onStart, error }: Props) {
  const { t } = useTranslation();
  const [targets, setTargets] = useState<ScanTargets | null>(null);
  const [choice, setChoice] = useState<Choice>("home");
  const [folder, setFolder] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    scanApi.targets().then((tg) => live && setTargets(tg)).catch(() => undefined);
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
    <div className="grid max-w-2xl gap-8">
      <div>
        <h1 className="text-2xl font-semibold">{t("scan.title")}</h1>
        <p className="mt-1 text-fg-muted">{t("scan.intro")}</p>
      </div>
      <div role="radiogroup" aria-label={t("scan.title")} className="grid divide-y divide-border rounded-xl border border-border bg-surface">
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
  );
}
