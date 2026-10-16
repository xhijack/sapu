import { useEffect, useRef, useState } from "react";

/** Step size so digits stay readable: 1 → 10 → 50 → 100 as numbers grow. */
export function counterStep(n: number): number {
  const a = Math.abs(n);
  if (a < 2_000) return 1;
  if (a < 50_000) return 10;
  if (a < 500_000) return 50;
  return 100;
}

/**
 * A live counter that chases a moving target: it never jumps, it rolls
 * toward the real value at a bounded pace and snaps to a step size
 * (odometer feel). Use for values that update many times per second.
 */
export function useSmoothCounter(target: number, chasePerSecond = 0.9): number {
  const [shown, setShown] = useState(target);
  const pos = useRef(target);
  const goal = useRef(target);

  useEffect(() => {
    goal.current = target;
  }, [target]);

  useEffect(() => {
    const reduce = typeof window !== "undefined" && window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
    let raf = 0;
    let last = performance.now();
    const tick = (now: number) => {
      const dt = Math.min(0.1, (now - last) / 1000);
      last = now;
      const target = goal.current;
      if (reduce) {
        pos.current = target;
        setShown((prev) => (prev === target ? prev : target));
        raf = requestAnimationFrame(tick);
        return;
      }
      const diff = target - pos.current;
      // Exponential approach: close `chasePerSecond` of the gap each second,
      // with a floor so the last stretch does not crawl forever.
      const move = diff * (1 - Math.pow(1 - chasePerSecond, dt));
      const floor = Math.sign(diff) * Math.min(Math.abs(diff), counterStep(target) * 60 * dt);
      pos.current += Math.abs(move) > Math.abs(floor) ? move : floor;
      const step = counterStep(target);
      const quantized = Math.abs(target - pos.current) < step ? target : Math.round(pos.current / step) * step;
      setShown((prev) => (prev === quantized ? prev : quantized));
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [chasePerSecond]);

  return shown;
}
