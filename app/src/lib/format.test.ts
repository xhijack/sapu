import i18n from "@/i18n";
import { formatBytes, formatDuration, formatEta } from "./format";

const d = (secs: number) => ({ secs, nanos: 0 });

describe("formatBytes", () => {
  it("picks sensible units", () => {
    expect(formatBytes(0, "en")).toBe("0 B");
    expect(formatBytes(1536, "en")).toBe("1.5 KB");
    expect(formatBytes(12.4 * 1024 ** 3, "en")).toBe("12.4 GB");
  });
});

describe("formatEta (PRD 10.1: human language, no false precision)", () => {
  const t = i18n.t.bind(i18n);

  it("never prints mm:ss", () => {
    expect(formatEta(t, { kind: "about", "0": d(107) })).toBe("Sekitar 2 menit lagi");
  });

  it("says less than a minute for short estimates", () => {
    expect(formatEta(t, { kind: "about", "0": d(30) })).toBe("Kurang dari 1 menit lagi");
  });

  it("shows a range when uncertain", () => {
    expect(formatEta(t, { kind: "range", low: d(120), high: d(240) })).toBe("2 menit–4 menit lagi");
  });

  it("has words for computing, indeterminate and almost done", () => {
    expect(formatEta(t, { kind: "computing" })).toBe("Menghitung perkiraan...");
    expect(formatEta(t, { kind: "indeterminate" })).toBe("Sedang bekerja, mohon tunggu");
    expect(formatEta(t, { kind: "almost_done" })).toBe("Hampir selesai");
  });

  it("pluralises in english", async () => {
    await i18n.changeLanguage("en");
    expect(formatDuration(t, d(60))).toBe("1 minute");
    expect(formatDuration(t, d(180))).toBe("3 minutes");
    expect(formatDuration(t, d(7200))).toBe("2 hours");
  });
});
