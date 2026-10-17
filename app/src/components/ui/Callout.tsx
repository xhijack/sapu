import type { ReactNode } from "react";
import { Info, TriangleAlert } from "lucide-react";
import { cn } from "@/lib/utils";

/**
 * Guidance the person must read before acting. Deliberately unlike
 * secondary text: full-strength colour, a tinted panel, an icon and a bold
 * first line, so it is never mistaken for small print. `review` is the
 * yellow "check first" tone, `info` the neutral one.
 */
export function Callout({ tone = "info", title, children, action }: { tone?: "info" | "review"; title?: ReactNode; children?: ReactNode; action?: ReactNode }) {
  const Icon = tone === "review" ? TriangleAlert : Info;
  return (
    <div
      role="note"
      className={cn(
        "flex flex-wrap items-start gap-3 rounded-xl border-l-4 px-4 py-3",
        tone === "review" ? "border-risk-review-fg bg-risk-review-bg text-risk-review-fg" : "border-accent bg-accent-soft text-fg",
      )}
    >
      <Icon aria-hidden className="mt-0.5 h-5 w-5 shrink-0" />
      <div className="grid min-w-0 flex-1 gap-0.5">
        {title && <p className="font-semibold">{title}</p>}
        {children && <div className="text-sm text-fg">{children}</div>}
      </div>
      {action && <div className="shrink-0 self-center">{action}</div>}
    </div>
  );
}
