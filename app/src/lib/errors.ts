import { useTranslation } from "react-i18next";

/** Maps a CommandError code to a translated sentence. */
export function useErrorMessage() {
  const { t } = useTranslation();
  return (e: unknown): string => {
    if (typeof e === "object" && e !== null && "code" in e) {
      const code = String((e as { code: unknown }).code);
      const key = `errors.${code}`;
      const msg = t(key);
      const base = msg === key ? t("common.unknown_error") : msg;
      // The provider's own explanation ("invalid x-api-key", "model not
      // found") is what lets someone fix an AI setup; show it.
      const detail = code === "ai_provider" && "message" in e ? String((e as { message: unknown }).message) : "";
      return detail ? `${base} (${detail.replace(/^provider error: /, "")})` : base;
    }
    return t("common.unknown_error");
  };
}
