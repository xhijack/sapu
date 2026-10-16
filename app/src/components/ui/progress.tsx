import { cn } from "@/lib/utils";

interface Props {
  /** 0..1, or null for an indeterminate bar. */
  value: number | null;
  label: string;
  className?: string;
}

export function ProgressBar({ value, label, className }: Props) {
  const pct = value === null ? null : Math.min(100, Math.max(0, Math.round(value * 100)));
  return (
    <div
      role="progressbar"
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={pct ?? undefined}
      className={cn("h-2 w-full overflow-hidden rounded-full bg-surface-muted", className)}
    >
      {pct === null ? (
        <div className="h-full w-1/3 animate-pulse rounded-full bg-accent" />
      ) : (
        <div className="h-full rounded-full bg-accent transition-[width] duration-500" style={{ width: `${pct}%` }} />
      )}
    </div>
  );
}
