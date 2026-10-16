import { useTranslation } from "react-i18next";
import { AlertCircle, Loader2 } from "lucide-react";
import { Button } from "./ui/button";

/** Empty state: an invitation, not a shrug. */
export function EmptyState({ title, body, action }: { title: string; body: string; action?: React.ReactNode }) {
  return (
    <div className="card grid justify-items-center gap-3 px-8 py-12 text-center">
      <span aria-hidden className="relative grid h-16 w-16 place-items-center">
        <span className="absolute inset-0 rounded-full border-[6px] border-surface-muted" />
        <span className="absolute inset-0 rounded-full border-[6px] border-accent border-r-transparent border-b-transparent opacity-70" />
      </span>
      <h3 className="display text-lg">{title}</h3>
      {body && <p className="max-w-md text-sm text-fg-muted">{body}</p>}
      {action}
    </div>
  );
}

/** Loading: a quiet skeleton rather than a spinner alone. */
export function LoadingState() {
  const { t } = useTranslation();
  return (
    <div role="status" aria-live="polite" className="grid gap-3 p-2">
      <span className="flex items-center gap-2 text-sm text-fg-muted">
        <Loader2 aria-hidden className="h-4 w-4 animate-spin" />
        {t("common.loading")}
      </span>
      <div aria-hidden className="grid gap-2">
        <span className="block h-3 w-2/3 animate-pulse rounded bg-surface-muted" />
        <span className="block h-3 w-1/2 animate-pulse rounded bg-surface-muted" />
        <span className="block h-3 w-3/5 animate-pulse rounded bg-surface-muted" />
      </div>
    </div>
  );
}

/** Error: what happened and what to do, never vague. */
export function ErrorState({ message, onRetry }: { message: string; onRetry?: () => void }) {
  const { t } = useTranslation();
  return (
    <div role="alert" className="card-solid flex flex-col items-start gap-2 border-risk-danger-border p-5">
      <div className="flex items-center gap-2 font-semibold text-risk-danger-fg">
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
