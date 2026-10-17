import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { AppWindow, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { EmptyState, ErrorState, LoadingState } from "@/components/States";
import { appsApi } from "@/lib/appsApi";
import { useErrorMessage } from "@/lib/errors";
import { formatBytes, formatDate } from "@/lib/format";
import type { CleanSession, InstalledApp, Leftover } from "@/lib/types";
import { cn } from "@/lib/utils";
import { AppDetail } from "./AppDetail";
import { UninstallDialog } from "./UninstallDialog";

type Tab = "installed" | "orphans";

/** Apps: uninstall with leftovers, or clean leftovers of apps already gone. */
export function AppsView({ onSeeHistory }: { onSeeHistory: () => void }) {
  const { t, i18n } = useTranslation();
  const lang = i18n.language;
  const errorMessage = useErrorMessage();
  const [apps, setApps] = useState<InstalledApp[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [tab, setTab] = useState<Tab>("installed");
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState<InstalledApp | null>(null);
  const [orphans, setOrphans] = useState<Leftover[] | null>(null);
  const [orphanPick, setOrphanPick] = useState<Set<string>>(new Set());
  const [pending, setPending] = useState<{ app: InstalledApp | null; leftovers: string[] } | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [removed, setRemoved] = useState<Set<string>>(new Set());

  useEffect(() => {
    let live = true;
    appsApi
      .list()
      .then((a) => live && setApps(a))
      .catch((e) => live && setError(errorMessage(e)));
    return () => {
      live = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (tab !== "orphans" || orphans !== null) return;
    let live = true;
    appsApi
      .orphans()
      .then((o) => live && setOrphans(o))
      .catch(() => live && setOrphans([]));
    return () => {
      live = false;
    };
  }, [tab, orphans]);

  const visible = useMemo(() => (apps ?? []).filter((a) => !removed.has(a.path)).filter((a) => a.name.toLowerCase().includes(query.toLowerCase())), [apps, query, removed]);
  const total = (apps ?? []).reduce((n, a) => n + a.bytes, 0);

  const onDone = (s: CleanSession | null) => {
    setPending(null);
    if (!s) return;
    const gone = new Set(s.entries.filter((e) => e.outcome.status === "disposed").map((e) => e.path));
    setRemoved((r) => new Set([...r, ...gone]));
    if (selected && gone.has(selected.path)) setSelected(null);
    setOrphans((o) => o?.filter((x) => !gone.has(x.path)) ?? null);
    setOrphanPick(new Set());
    setNotice(t("apps.done", { count: s.disposed, size: formatBytes(s.bytes_freed, lang) }));
  };

  if (error) return <ErrorState message={error} />;
  if (apps === null) {
    return (
      <div className="grid gap-3">
        <h1 className="display text-2xl">{t("apps.title")}</h1>
        <p className="text-fg-muted">{t("apps.loading")}</p>
        <LoadingState />
      </div>
    );
  }

  return (
    <div className="grid max-w-[1280px] gap-6">
      <div className="grid gap-1">
        <h1 className="display text-2xl">{t("apps.title")}</h1>
        <p className="max-w-[60ch] text-fg-muted">{t("apps.intro")}</p>
      </div>

      <div className="flex flex-wrap items-center justify-between gap-3">
        <div role="tablist" className="inline-flex rounded-full bg-surface-muted p-1">
          {(["installed", "orphans"] as Tab[]).map((id) => (
            <button key={id} role="tab" type="button" aria-selected={tab === id} onClick={() => setTab(id)} className={cn("rounded-full px-4 py-1.5 text-sm font-medium", tab === id ? "bg-surface-solid shadow-[var(--shadow-1)]" : "text-fg-muted hover:text-fg")}>
              {t(`apps.tab_${id}`)}
            </button>
          ))}
        </div>
        {tab === "installed" && <span className="num text-sm text-fg-muted">{t("apps.count", { count: apps.length, size: formatBytes(total, lang) })}</span>}
      </div>

      {notice && (
        <p role="status" className="card-solid px-4 py-2 text-sm">
          {notice}
        </p>
      )}

      {tab === "installed" &&
        (apps.length === 0 ? (
          <EmptyState title={t("apps.title")} body={t("apps.unsupported")} />
        ) : (
          <div className="grid gap-5 lg:grid-cols-[1.1fr_1fr]">
            <div className="grid content-start gap-3">
              <input type="search" value={query} onChange={(e) => setQuery(e.target.value)} placeholder={t("apps.search")} aria-label={t("apps.search")} className="h-10 rounded-lg border border-border bg-surface px-3 text-sm" />
              <ul className="card-solid max-h-[calc(100vh-330px)] divide-y divide-border overflow-y-auto">
                {visible.map((a) => (
                  <li key={a.path}>
                    <button
                      type="button"
                      onClick={() => setSelected(a)}
                      aria-current={selected?.path === a.path ? "true" : undefined}
                      className={cn("flex w-full items-center gap-3 px-3 py-2.5 text-left hover:bg-surface-muted", selected?.path === a.path && "bg-accent-soft")}
                    >
                      <AppIcon path={a.path} name={a.name} />
                      <span className="min-w-0 flex-1">
                        <span className="flex items-center gap-2">
                          <span className="truncate font-medium">{a.name}</span>
                          {a.is_system && <span className="rounded-full bg-surface-muted px-2 py-0.5 text-[11px] text-fg-muted">{t("apps.system_badge")}</span>}
                        </span>
                        <span className="num block text-xs text-fg-muted">
                          {a.version ? `${t("apps.version", { v: a.version })}` : ""}
                          {a.last_used ? `${a.version ? ", " : ""}${t("apps.last_used", { date: formatDate(a.last_used.secs_since_epoch, lang) })}` : ""}
                        </span>
                      </span>
                      <span className="num shrink-0 text-sm">{formatBytes(a.bytes, lang)}</span>
                    </button>
                  </li>
                ))}
              </ul>
            </div>
            {selected ? (
              <AppDetail key={selected.path} app={selected} onUninstall={(leftovers, includeApp) => setPending({ app: includeApp ? selected : null, leftovers })} />
            ) : (
              <div className="card grid place-items-center p-10 text-center text-sm text-fg-faint">
                <AppWindow aria-hidden className="mb-2 h-8 w-8" />
                {t("apps.search")}
              </div>
            )}
          </div>
        ))}

      {tab === "orphans" && (
        <div className="grid gap-3">
          <p className="text-sm text-fg-muted">{t("apps.orphans_body")}</p>
          {orphans === null ? (
            <LoadingState />
          ) : orphans.length === 0 ? (
            <EmptyState title={t("apps.tab_orphans")} body={t("apps.orphans_none")} />
          ) : (
            <>
              <div className="flex items-center justify-between">
                <label className="flex items-center gap-2 text-sm">
                  <input type="checkbox" checked={orphanPick.size === orphans.length} onChange={(e) => setOrphanPick(e.target.checked ? new Set(orphans.map((o) => o.path)) : new Set())} className="h-4 w-4" />
                  {t("apps.select_all")}
                </label>
                <Button
                  disabled={orphanPick.size === 0}
                  onClick={() => setPending({ app: null, leftovers: [...orphanPick] })}
                >
                  <Trash2 aria-hidden className="h-4 w-4" />
                  {t("apps.remove_leftovers", { count: orphanPick.size, size: formatBytes(orphans.filter((o) => orphanPick.has(o.path)).reduce((n, o) => n + o.bytes, 0), lang) })}
                </Button>
              </div>
              <ul className="card-solid divide-y divide-border">
                {orphans.map((o) => (
                  <li key={o.path}>
                    <label className="flex items-center gap-3 px-3 py-2.5">
                      <input
                        type="checkbox"
                        checked={orphanPick.has(o.path)}
                        onChange={() =>
                          setOrphanPick((s) => {
                            const n = new Set(s);
                            if (n.has(o.path)) n.delete(o.path);
                            else n.add(o.path);
                            return n;
                          })
                        }
                        aria-label={o.path}
                        className="h-4 w-4"
                      />
                      <span className="min-w-0 flex-1">
                        <span className="path block truncate" title={o.path}>
                          {o.path}
                        </span>
                        <span className="text-xs text-fg-muted">{t(`apps.kind_${o.kind}`)}</span>
                      </span>
                      <span className="num shrink-0 text-sm">{formatBytes(o.bytes, lang)}</span>
                    </label>
                  </li>
                ))}
              </ul>
            </>
          )}
        </div>
      )}

      {pending && <UninstallDialog app={pending.app} leftovers={pending.leftovers} onClose={onDone} />}
      <button type="button" onClick={onSeeHistory} className="w-fit text-xs text-accent underline-offset-2 hover:underline">
        {t("clean.see_history")}
      </button>
    </div>
  );
}

export function AppIcon({ path, name }: { path: string; name: string }) {
  const [src, setSrc] = useState<string>("");
  useEffect(() => {
    let live = true;
    appsApi.icon(path).then((s) => live && setSrc(s)).catch(() => undefined);
    return () => {
      live = false;
    };
  }, [path]);
  if (src) return <img src={src} alt="" className="h-9 w-9 shrink-0 rounded-[9px]" />;
  return (
    <span aria-hidden className="grid h-9 w-9 shrink-0 place-items-center rounded-[9px] bg-surface-muted text-sm font-semibold text-fg-muted">
      {name.slice(0, 1).toUpperCase()}
    </span>
  );
}
