import { useEffect, useState } from "react";
import { cn } from "@/lib/utils";

export interface RingSegment {
  key: string;
  value: number;
  color: string;
  label?: string;
}

interface Props {
  segments: RingSegment[];
  /** Full circle value; the remainder is drawn as the "free" track. */
  total: number;
  size?: number;
  thickness?: number;
  /** Indeterminate: a single arc spinning (scanning). */
  spinning?: boolean;
  label: string;
  className?: string;
  children?: React.ReactNode;
}

/** The one iconic element: a disk as a ring of coloured space. */
export function SpaceRing({ segments, total, size = 220, thickness = 18, spinning = false, label, className, children }: Props) {
  const r = (size - thickness) / 2;
  const c = 2 * Math.PI * r;
  const [mounted, setMounted] = useState(false);
  useEffect(() => {
    const id = requestAnimationFrame(() => setMounted(true));
    return () => cancelAnimationFrame(id);
  }, []);

  let offset = 0;
  const arcs = segments
    .filter((s) => s.value > 0 && total > 0)
    .map((s) => {
      const frac = Math.min(1, s.value / total);
      const arc = { ...s, frac, start: offset };
      offset += frac;
      return arc;
    });

  return (
    <div className={cn("relative inline-grid place-items-center", className)} style={{ width: size, height: size }}>
      <svg role="img" aria-label={label} width={size} height={size} viewBox={`0 0 ${size} ${size}`} className={cn("-rotate-90", spinning && "animate-[ring-spin_1.6s_linear_infinite]")}>
        <circle cx={size / 2} cy={size / 2} r={r} fill="none" stroke="var(--cat-free)" strokeWidth={thickness} />
        {spinning ? (
          <circle cx={size / 2} cy={size / 2} r={r} fill="none" stroke="var(--accent)" strokeWidth={thickness} strokeLinecap="round" strokeDasharray={`${c * 0.22} ${c}`} />
        ) : (
          arcs.map((a) => (
            <circle
              key={a.key}
              data-segment={a.key}
              cx={size / 2}
              cy={size / 2}
              r={r}
              fill="none"
              stroke={a.color}
              strokeWidth={thickness}
              strokeDasharray={`${mounted ? a.frac * c : 0} ${c}`}
              strokeDashoffset={-a.start * c}
              className="transition-[stroke-dasharray] duration-[900ms] ease-[cubic-bezier(0.2,0.8,0.2,1)] motion-reduce:transition-none"
            >
              {a.label && <title>{a.label}</title>}
            </circle>
          ))
        )}
      </svg>
      <div className="absolute inset-0 grid place-items-center text-center">{children}</div>
    </div>
  );
}
