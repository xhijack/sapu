import { Callout } from "@/components/ui/Callout";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { LoadingState } from "@/components/States";
import { appsApi } from "@/lib/appsApi";
import { formatBytes, formatDate } from "@/lib/format";
import type { InstalledApp, Leftover } from "@/lib/types";
import { AppIcon } from "./AppsView";

interface Props {
  app: InstalledApp;
  onUninstall: (leftovers: string[], includeApp: boolean) => void;
}

export function AppDetail({ app, onUninstall }: Props) {
  const { t, i18n } = useTranslation();
  const lang = i18n.language;
  const [leftovers, setLeftovers] = useState<Leftover[] | null>(null);
  const [picked, setPicked] = useState<Set<string>>(new Set());
  const [running, setRunning] = useState(false);

  useEffect(() => {
    let live = true;
    appsApi
      .leftovers(app)
      .then((l) => {
        if (!live) return;
        setLeftovers(l);
        setPicked(new Set(l.map((x) => x.path)));
      })
      .catch(() => live && setLeftovers([]));
    appsApi.isRunning(app.path).then((r) => live && setRunning(r)).catch(() => undefined);
    return () => {
      live = false;
    };
  }, [app]);

  const pickedBytes = (leftovers ?? []).filter((l) => picked.has(l.path)).reduce((n, l) => n + l.bytes, 0);
  const groups = (leftovers ?? []).reduce<Record<string, Leftover[]>>((acc, l) => ((acc[l.kind] ??= []).push(l), acc), {});

  return (
    <div className="card grid content-start gap-4 p-5">
      <div className="flex items-center gap-3">
        <AppIcon path={app.path} name={app.name} />
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-2">
            <span className="display text-lg">{app.name}</span>
            {app.is_system && <span className="rounded-full bg-surface-muted px-2 py-0.5 text-[11px] text-fg-muted">{t("apps.system_badge")}</span>}
            {running && <span className="rounded-full bg-risk-review-bg px-2 py-0.5 text-[11px] text-risk-review-fg">{t("apps.running_badge")}</span>}
          </div>
          <p className="path truncate text-fg-muted" title={app.path}>
            {app.path}
          </p>
          <p className="num text-xs text-fg-muted">
            {formatBytes(app.bytes, lang)}
            {app.version ? `, ${t("apps.version", { v: app.version })}` : ""}
            {app.last_used ? `, ${t("apps.last_used", { date: formatDate(app.last_used.secs_since_epoch, lang) })}` : ""}
          </p>
        </div>
      </div>

      <div className="grid gap-2">
        <h3 className="text-sm font-semibold text-fg-muted">{t("apps.leftovers_title")}</h3>
        {leftovers === null ? (
          <LoadingState />
        ) : leftovers.length === 0 ? (
          <p className="text-sm text-fg-faint">{t("apps.leftovers_none")}</p>
        ) : (
          <ul className="grid gap-2">
            {Object.entries(groups).map(([kind, items]) => (
              <li key={kind} className="grid gap-1">
                <span className="text-xs text-fg-muted">{t(`apps.kind_${kind}`)}</span>
                {items.map((l) => (
                  <label key={l.path} className="flex items-center gap-2 text-sm">
                    <input
                      type="checkbox"
                      checked={picked.has(l.path)}
                      onChange={() =>
                        setPicked((s) => {
                          const n = new Set(s);
                          if (n.has(l.path)) n.delete(l.path);
                          else n.add(l.path);
                          return n;
                        })
                      }
                      aria-label={l.path}
                      className="h-4 w-4"
                    />
                    <span className="path min-w-0 flex-1 truncate" title={l.path}>
                      {l.path.split("/").pop()}
                    </span>
                    <span className="num shrink-0 text-xs text-fg-muted">{formatBytes(l.bytes, lang)}</span>
                  </label>
                ))}
              </li>
            ))}
          </ul>
        )}
      </div>

      {app.is_system ? (
        <Callout>{t("apps.system_note")}</Callout>
      ) : (
        <div className="grid gap-2">
          {running && <p className="text-sm text-risk-review-fg">{t("apps.running_note")}</p>}
          <Button onClick={() => onUninstall([...picked], true)} disabled={running}>
            <Trash2 aria-hidden className="h-4 w-4" />
            {picked.size > 0 ? t("apps.uninstall", { count: picked.size, size: formatBytes(app.bytes + pickedBytes, lang) }) : t("apps.uninstall_app_only", { size: formatBytes(app.bytes, lang) })}
          </Button>
          {picked.size > 0 && (
            <Button variant="outline" size="sm" onClick={() => onUninstall([...picked], false)}>
              {t("apps.remove_leftovers", { count: picked.size, size: formatBytes(pickedBytes, lang) })}
            </Button>
          )}
        </div>
      )}
    </div>
  );
}
