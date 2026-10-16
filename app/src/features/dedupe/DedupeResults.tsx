import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { EmptyState, LoadingState } from "@/components/States";
import { formatBytes } from "@/lib/format";
import { dedupeApi } from "@/lib/dedupeApi";
import type { DedupeRunReport, DedupeSummary, DuplicateGroup, FolderPair, KeepStrategy } from "@/lib/types";
import { GroupCard } from "./GroupCard";
import { DedupePreview } from "./DedupePreview";
import { defaultKeep, type StrategyKey } from "./keep";

const PAGE = 20;

interface Props {
  summary: DedupeSummary;
  onSeeHistory: () => void;
  onNewSearch: () => void;
  onRerun: () => void;
}

export function DedupeResults({ summary, onSeeHistory, onNewSearch, onRerun }: Props) {
  const { t, i18n } = useTranslation();
  const [groups, setGroups] = useState<DuplicateGroup[] | null>(null);
  const [pairs, setPairs] = useState<FolderPair[]>([]);
  const [strategy, setStrategy] = useState<StrategyKey>("original_name");
  const [picks, setPicks] = useState<Record<number, string>>({});
  const [preview, setPreview] = useState(false);
  const [result, setResult] = useState<DedupeRunReport | null>(null);

  const loadMore = useCallback(
    (offset: number) =>
      dedupeApi
        .groups(offset, PAGE)
        .then((g) => setGroups((prev) => (offset === 0 ? g : [...(prev ?? []), ...g])))
        .catch(() => setGroups((prev) => prev ?? [])),
    [],
  );

  useEffect(() => {
    void loadMore(0);
    dedupeApi.folderPairs(10).then(setPairs).catch(() => undefined);
  }, [loadMore, summary.root]);

  const keepFor = useCallback((g: DuplicateGroup) => picks[g.id] ?? defaultKeep(g, strategy), [picks, strategy]);
  const strategyValue: KeepStrategy = useMemo(() => ({ kind: strategy }), [strategy]);

  if (result) {
    return (
      <div className="grid max-w-3xl gap-6">
        <h1 className="text-2xl font-semibold">{t("dedupe.done_title")}</h1>
        <p className="num text-lg">
          {t("dedupe.done_body", { size: formatBytes(result.bytes_freed, i18n.language), ok: result.disposed, skipped: result.skipped, failed: result.failed })}
        </p>
        {result.outcomes.some((o) => o.kind === "keep_missing") && <p className="text-sm text-risk-review-fg">{t("dedupe.keep_missing")}</p>}
        <div className="flex gap-2">
          <Button variant="outline" onClick={onSeeHistory}>
            {t("clean.see_history")}
          </Button>
          <Button variant="ghost" onClick={onRerun}>
            {t("scan.rescan")}
          </Button>
        </div>
      </div>
    );
  }

  if (groups === null) return <LoadingState />;

  if (summary.groups === 0) {
    return (
      <div className="grid max-w-3xl gap-6">
        <h1 className="text-2xl font-semibold">{t("dedupe.title")}</h1>
        <EmptyState title={t("dedupe.empty_title")} body={t("dedupe.empty_body")} action={<Button variant="outline" size="sm" onClick={onNewSearch}>{t("dedupe.new_search")}</Button>} />
      </div>
    );
  }

  return (
    <div className="grid max-w-4xl gap-8">
      <header className="grid gap-1">
        <p className="num text-4xl font-semibold tracking-tight">{t("dedupe.headline", { size: formatBytes(summary.wasted_bytes, i18n.language) })}</p>
        <p className="text-sm text-fg-muted">
          {t("dedupe.headline_sub", { groups: summary.groups.toLocaleString(i18n.language), files: summary.duplicate_files.toLocaleString(i18n.language), seen: summary.files_seen.toLocaleString(i18n.language) })}
        </p>
        <p className="path truncate text-fg-muted" title={summary.root}>
          {summary.root}
        </p>
        {summary.cancelled && <p className="text-sm text-risk-review-fg">{t("dedupe.cancelled_note")}</p>}
      </header>

      {pairs.length > 0 && (
        <section className="grid gap-2">
          <h2 className="text-lg font-semibold">{t("dedupe.pairs_title")}</h2>
          <p className="text-sm text-fg-muted">{t("dedupe.pairs_body")}</p>
          <ul className="divide-y divide-border rounded-xl border border-border bg-surface">
            {pairs.map((p) => (
              <li key={`${p.a}|${p.b}`} className="grid gap-0.5 px-4 py-3 text-sm">
                <span className="path truncate" title={p.a}>
                  {p.a}
                </span>
                <span className="path truncate" title={p.b}>
                  {p.b}
                </span>
                <span className="num text-fg-muted">{t("dedupe.pair_line", { count: p.files, size: formatBytes(p.bytes, i18n.language) })}</span>
              </li>
            ))}
          </ul>
        </section>
      )}

      <section className="grid gap-3">
        <div className="flex flex-wrap items-end justify-between gap-3">
          <h2 className="text-lg font-semibold">{t("dedupe.groups_title")}</h2>
          <label className="grid gap-1 text-sm">
            <span className="text-fg-muted">{t("dedupe.strategy")}</span>
            <select
              value={strategy}
              onChange={(e) => {
                setStrategy(e.target.value as StrategyKey);
                setPicks({});
              }}
              className="h-9 rounded-md border border-border bg-surface px-2"
            >
              <option value="original_name">{t("dedupe.strategy_original")}</option>
              <option value="oldest">{t("dedupe.strategy_oldest")}</option>
              <option value="newest">{t("dedupe.strategy_newest")}</option>
            </select>
          </label>
        </div>
        <ul className="grid gap-4">
          {groups.map((g) => (
            <li key={g.id}>
              <GroupCard group={g} keep={keepFor(g)} onKeep={(path) => setPicks((p) => ({ ...p, [g.id]: path }))} />
            </li>
          ))}
        </ul>
        {groups.length < summary.groups && (
          <div>
            <Button variant="outline" size="sm" onClick={() => void loadMore(groups.length)}>
              {t("dedupe.show_more", { count: Math.min(PAGE, summary.groups - groups.length) })}
            </Button>
          </div>
        )}
      </section>

      <div className="sticky bottom-0 -mx-8 flex items-center justify-between gap-4 border-t border-border bg-bg/95 px-8 py-4 backdrop-blur">
        <Button variant="ghost" size="sm" onClick={onNewSearch}>
          {t("dedupe.new_search")}
        </Button>
        <Button size="lg" onClick={() => setPreview(true)}>
          {t("dedupe.primary", { size: formatBytes(summary.wasted_bytes, i18n.language) })}
        </Button>
      </div>

      {preview && (
        <DedupePreview
          summary={summary}
          strategy={strategyValue}
          picks={picks}
          onClose={() => setPreview(false)}
          onDone={(r) => {
            setPreview(false);
            setResult(r);
          }}
        />
      )}
    </div>
  );
}
