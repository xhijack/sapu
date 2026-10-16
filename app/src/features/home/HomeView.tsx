import { useTranslation } from "react-i18next";
import { EmptyState } from "@/components/States";
import { Button } from "@/components/ui/button";

export function HomeView({ onRerunOnboarding }: { onRerunOnboarding: () => void }) {
  const { t } = useTranslation();
  return (
    <div className="grid gap-6">
      <h1 className="text-2xl font-bold">{t("home.title")}</h1>
      <EmptyState
        title={t("home.empty_title")}
        body={t("home.empty_body")}
        action={
          <Button variant="outline" size="sm" onClick={onRerunOnboarding}>
            {t("home.rerun_onboarding")}
          </Button>
        }
      />
    </div>
  );
}
