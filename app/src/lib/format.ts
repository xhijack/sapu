import type { TFunction } from "i18next";
import type { Duration, Eta } from "./types";

export function formatBytes(n: number, locale?: string): string {
  const units = ["B", "KB", "MB", "GB", "TB"];
  let v = n;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i += 1;
  }
  const digits = i === 0 ? 0 : 1;
  return `${v.toLocaleString(locale, { maximumFractionDigits: digits, minimumFractionDigits: digits })} ${units[i]}`;
}

/** Rounded human duration ("2 menit", "1 jam"). Never shows seconds precision above a minute. */
export function formatDuration(t: TFunction, d: Duration | number): string {
  const secs = typeof d === "number" ? d : d.secs;
  if (secs >= 3600) return t("progress.hours", { count: Math.round(secs / 3600) });
  if (secs >= 60) return t("progress.minutes", { count: Math.round(secs / 60) });
  return t("progress.seconds", { count: Math.max(secs, 0) });
}

/** Turns a core `Eta` into one honest sentence (PRD §10.1). */
export function formatEta(t: TFunction, eta: Eta): string {
  switch (eta.kind) {
    case "indeterminate":
      return t("progress.indeterminate");
    case "computing":
      return t("progress.computing");
    case "almost_done":
      return t("progress.almost_done");
    case "range":
      return t("progress.range", {
        low: formatDuration(t, eta.low),
        high: formatDuration(t, eta.high),
      });
    case "about": {
      const d = eta["0"];
      if (d.secs < 60) return t("progress.less_than_minute");
      return t("progress.about", { time: formatDuration(t, d) });
    }
  }
}

export function formatElapsed(t: TFunction, secs: number): string {
  return t("progress.elapsed", { time: formatDuration(t, secs) });
}

export function formatDate(unixSecs: number, locale?: string): string {
  return new Date(unixSecs * 1000).toLocaleString(locale, {
    dateStyle: "medium",
    timeStyle: "short",
  });
}
