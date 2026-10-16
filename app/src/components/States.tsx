import { useTranslation } from "react-i18next";
import { Inbox, AlertCircle, Loader2 } from "lucide-react";
import { Button } from "./ui/button";

export function EmptyState({ title, body, action }: { title: string; body: string; action?: React.ReactNode }) {
  return (
    <div className="flex flex-col items-center gap-3 rounded-xl border border-dashed border-border p-10 text-center">
      <Inbox aria-hidden className="h-8 w-8 text-fg-muted" />
      <h3 className="font-semibold">{title}</h3>
      <p className="max-w-md text-sm text-fg-muted">{body}</p>
      {action}
    </div>
  );
}

export function LoadingState() {
  const { t } = useTranslation();
  return (
    <div role="status" className="flex items-center gap-2 p-6 text-sm text-fg-muted">
      <Loader2 aria-hidden className="h-4 w-4 animate-spin" />
      {t("common.loading")}
    </div>
  );
}

export function ErrorState({ message, onRetry }: { message: string; onRetry?: () => void }) {
  const { t } = useTranslation();
  return (
    <div role="alert" className="flex flex-col items-start gap-2 rounded-xl border border-risk-danger-border bg-risk-danger-bg p-5 text-risk-danger-fg">
      <div className="flex items-center gap-2 font-semibold">
        <AlertCircle aria-hidden className="h-5 w-5" />
        {t("common.error_title")}
      </div>
      <p className="text-sm">{message}</p>
      {onRetry && (
        <Button variant="outline" size="sm" onClick={onRetry}>
          {t("common.retry")}
        </Button>
      )}
    </div>
  );
}
