import { cn } from "@/lib/utils";

interface Props extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  icon: React.ReactNode;
  /** CSS colour for the icon badge, e.g. `var(--cat-video)`. */
  tone: string;
  title: string;
  body: string;
  stat?: React.ReactNode;
}

/** Result/action tile: badge, title, one line, optional stat. */
export function Tile({ icon, tone, title, body, stat, className, ...rest }: Props) {
  return (
    <button
      type="button"
      className={cn(
        "card group grid gap-3 p-5 text-left transition-[transform,box-shadow] duration-200 hover:-translate-y-0.5 hover:shadow-[var(--shadow-2)] motion-reduce:transform-none",
        className,
      )}
      {...rest}
    >
      <span className="grid h-11 w-11 place-items-center rounded-2xl text-white" style={{ background: tone }}>
        {icon}
      </span>
      <span className="grid gap-1">
        <span className="display text-lg">{title}</span>
        <span className="text-sm text-fg-muted">{body}</span>
      </span>
      {stat && <span className="num text-sm text-fg-muted">{stat}</span>}
    </button>
  );
}
