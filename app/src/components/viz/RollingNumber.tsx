import { cn } from "@/lib/utils";

/**
 * Odometer: every digit slides vertically to its new value. Pair with a
 * value that updates a few times per second, not every frame.
 */
export function RollingNumber({ value, className }: { value: string; className?: string }) {
  const reduce = typeof window !== "undefined" && window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
  return (
    <span className={cn("inline-flex items-baseline overflow-hidden tabular-nums", className)} aria-label={value} role="text">
      {Array.from(value).map((ch, i) => (/\d/.test(ch) ? <Digit key={`${value.length}-${i}`} digit={Number(ch)} animate={!reduce} /> : <span key={`${value.length}-${i}-s`}>{ch}</span>))}
    </span>
  );
}

function Digit({ digit, animate }: { digit: number; animate: boolean }) {
  return (
    <span aria-hidden className="relative inline-block h-[1em] w-[0.62em] overflow-hidden align-baseline leading-none">
      <span
        className={cn("absolute left-0 top-0 flex flex-col items-center", animate && "transition-transform duration-[260ms] ease-[cubic-bezier(0.2,0.8,0.2,1)]")}
        style={{ transform: `translateY(-${digit}em)` }}
      >
        {Array.from({ length: 10 }, (_, d) => (
          <span key={d} className="block h-[1em] leading-none">
            {d}
          </span>
        ))}
      </span>
    </span>
  );
}
