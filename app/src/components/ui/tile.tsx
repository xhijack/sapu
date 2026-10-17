import { cn } from "@/lib/utils";

interface Props extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  icon: React.ReactNode;
  /** CSS colour for the icon badge, e.g. `var(--cat-video)`. */
  tone: string;
  title: string;
  body: string;
  stat?: React.ReactNode;
  /** Icon beside the text instead of above it; half the height. */
  compact?: boolean;
}

/** Result/action tile: badge, title, one line, optional stat. */
export function Tile({ icon, tone, title, body, stat, compact = false, className, ...rest }: Props) {
  return (
    <button
      type="button"
      className={cn(
        "card group text-left transition-[transform,box-shadow] duration-200 hover:-translate-y-0.5 hover:shadow-[var(--shadow-2)] motion-reduce:transform-none",
        compact ? "flex items-start gap-3 p-4" : "grid gap-3 p-5",
        className,
      )}
      {...rest}
    >
      <span className={cn("grid shrink-0 place-items-center rounded-2xl text-white", compact ? "h-9 w-9 rounded-xl" : "h-11 w-11")} style={{ background: tone }}>
        {icon}
      </span>
      <span className="grid min-w-0 gap-0.5">
        <span className={cn("display", compact ? "text-base" : "text-lg")}>{title}</span>
        <span className={cn("text-fg-muted", compact ? "text-xs" : "text-sm")}>{body}</span>
      </span>
      {stat && <span className="num text-sm text-fg-muted">{stat}</span>}
    </button>
  );
}
