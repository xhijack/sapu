import { useTranslation } from "react-i18next";

/** Maps a CommandError code to a translated sentence. */
export function useErrorMessage() {
  const { t } = useTranslation();
  return (e: unknown): string => {
    if (typeof e === "object" && e !== null && "code" in e) {
      const code = String((e as { code: unknown }).code);
      const key = `errors.${code}`;
      const msg = t(key);
      return msg === key ? t("common.unknown_error") : msg;
    }
    return t("common.unknown_error");
  };
}
