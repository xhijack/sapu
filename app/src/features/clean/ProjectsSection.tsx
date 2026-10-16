import { useTranslation } from "react-i18next";
import { RiskBadge } from "@/components/RiskBadge";
import { formatBytes } from "@/lib/format";
import type { Project } from "@/lib/types";
import { ARTEFACT_PREFIX } from "./rows";
import { AiArtefact } from "@/features/ai/AiArtefact";
import { useAiEnabled } from "@/features/ai/useAiEnabled";

interface Props {
  projects: Project[];
  selected: Set<string>;
  onToggle: (id: string) => void;
}

/** One card per development project: idle time, rebuild command, artefacts. */
export function ProjectsSection({ projects, selected, onToggle }: Props) {
  const { t, i18n } = useTranslation();
  const aiEnabled = useAiEnabled();
  const idle = (p: Project) => {
    if (p.idle_days === null) return t("clean.project_idle_unknown");
    if (p.idle_days < 1) return t("clean.project_active");
    const time = p.idle_days >= 60 ? t("clean.months", { count: Math.round(p.idle_days / 30) }) : t("clean.days", { count: p.idle_days });
    return t("clean.project_idle", { time });
  };
  return (
    <section className="grid gap-2">
      <h2 className="text-sm font-medium text-fg-muted">{t("clean.projects_title")}</h2>
      <p className="text-sm text-fg-muted">{t("clean.projects_body")}</p>
      <ul className="grid gap-3">
        {projects.map((p) => (
          <li key={p.root} className="grid gap-2 rounded-xl border border-border bg-surface p-4">
            <div className="flex flex-wrap items-baseline justify-between gap-2">
              <div className="min-w-0">
                <span className="font-medium">{p.name}</span>
                <span className="ml-2 text-sm text-fg-muted">
                  {t(`clean.eco_${p.ecosystem}`)}, {idle(p)}
                </span>
                <span className="path block truncate text-fg-muted" title={p.root}>
                  {p.root}
                </span>
              </div>
              <span className="num text-lg font-semibold">{formatBytes(p.bytes, i18n.language)}</span>
            </div>
            <ul className="divide-y divide-border rounded-lg border border-border">
              {p.artefacts.map((a) => {
                const id = ARTEFACT_PREFIX + a.path;
                const git = a.tracked === null ? t("clean.artefact_unknown_git") : t("clean.artefact_untracked");
                return (
                  <li key={a.path}>
                    <label className="flex cursor-pointer items-center gap-3 px-3 py-2">
                      <input type="checkbox" checked={selected.has(id)} onChange={() => onToggle(id)} aria-label={`${p.name}/${a.kind}`} className="h-4 w-4 accent-[var(--accent)]" />
                      <span className="path min-w-0 flex-1 truncate">{a.kind}</span>
                      <RiskBadge level={a.risk === "green" ? "safe" : "review"} />
                      <span className="hidden text-xs text-fg-muted sm:inline">
                        {a.gitignored ? t("clean.artefact_ignored") : t("clean.artefact_not_ignored")}, {git}
                      </span>
                      <span className="num w-20 shrink-0 text-right text-sm">{formatBytes(a.bytes, i18n.language)}</span>
                    </label>
                  </li>
                );
              })}
            </ul>
            {aiEnabled && <AiArtefact root={p.root} />}
            {p.rebuild.length > 0 && (
              <p className="text-xs text-fg-muted">
                {t("clean.project_rebuild", { cmd: "" })}
                <code className="path rounded bg-surface-muted px-1.5 py-0.5">{p.rebuild.join(" && ")}</code>
              </p>
            )}
          </li>
        ))}
      </ul>
    </section>
  );
}
