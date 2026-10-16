import id from "../../locales/id.json";
import en from "../../locales/en.json";
import i18n, { resolveLocale } from "./i18n";

function flatten(obj: unknown, prefix = ""): Record<string, string> {
  if (typeof obj !== "object" || obj === null) return { [prefix]: String(obj) };
  return Object.entries(obj as Record<string, unknown>).reduce(
    (acc, [k, v]) => ({ ...acc, ...flatten(v, prefix ? `${prefix}.${k}` : k) }),
    {} as Record<string, string>,
  );
}

describe("locales", () => {
  const idKeys = Object.keys(flatten(id)).sort();
  const enKeys = Object.keys(flatten(en)).sort();

  it("id and en have exactly the same keys", () => {
    expect(enKeys).toEqual(idKeys);
  });

  it("no value is empty", () => {
    for (const [k, v] of Object.entries({ ...flatten(id), ...flatten(en) })) {
      expect(v.trim(), k).not.toBe("");
    }
  });

  it("interpolation placeholders match between languages", () => {
    const fid = flatten(id);
    const fen = flatten(en);
    for (const k of idKeys) {
      const ph = (s: string) => (s.match(/{{\w+}}/g) ?? []).sort();
      expect(ph(fen[k] ?? ""), k).toEqual(ph(fid[k] ?? ""));
    }
  });

  it("switching language changes output", async () => {
    await i18n.changeLanguage("en");
    expect(i18n.t("common.next")).toBe("Next");
    await i18n.changeLanguage("id");
    expect(i18n.t("common.next")).toBe("Lanjut");
  });

  it("resolveLocale falls back to a supported locale", () => {
    expect(resolveLocale("en")).toBe("en");
    expect(["id", "en"]).toContain(resolveLocale("fr"));
    expect(["id", "en"]).toContain(resolveLocale(null));
  });
});
