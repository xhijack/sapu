import { useEffect, useRef, useState } from "react";

/** Publishes `value` at most every `intervalMs`, so the odometer has time to roll. */
export function useThrottledValue<T>(value: T, intervalMs = 250): T {
  const [shown, setShown] = useState(value);
  const latest = useRef(value);
  useEffect(() => {
    latest.current = value;
  }, [value]);
  useEffect(() => {
    const id = setInterval(() => setShown((prev) => (prev === latest.current ? prev : latest.current)), intervalMs);
    return () => clearInterval(id);
  }, [intervalMs]);
  return shown;
}
