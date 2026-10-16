import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import id from "../../locales/id.json";
import en from "../../locales/en.json";

export const SUPPORTED = ["id", "en"] as const;
export type Locale = (typeof SUPPORTED)[number];

export function systemLocale(): Locale {
  const nav = typeof navigator !== "undefined" ? navigator.language : "id";
  return nav.toLowerCase().startsWith("en") ? "en" : "id";
}

export function resolveLocale(preferred: string | null | undefined): Locale {
  if (preferred && (SUPPORTED as readonly string[]).includes(preferred)) return preferred as Locale;
  return systemLocale();
}

void i18n.use(initReactI18next).init({
  resources: { id: { translation: id }, en: { translation: en } },
  lng: systemLocale(),
  fallbackLng: "id",
  interpolation: { escapeValue: false },
  returnNull: false,
});

export default i18n;
