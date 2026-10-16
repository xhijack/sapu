import { useTranslation } from "react-i18next";
import { Pause, Play, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { ProgressPanel } from "@/components/ProgressPanel";
import type { ScanProgressPayload } from "@/lib/types";

interface Props {
  root: string;
  progress: ScanProgressPayload | null;
  paused: boolean;
  onPause: () => void;
  onResume: () => void;
  onCancel: () => void;
}

export function Scanning({ root, progress, paused, onPause, onResume, onCancel }: Props) {
  const { t } = useTranslation();
  const state = progress ?? { fraction: null, items_done: 0, items_total: null, bytes_done: 0, bytes_total: null, elapsed_secs: 0, eta: { kind: "computing" as const } };
  return (
    <div className="grid max-w-2xl gap-8">
      <ProgressPanel title={t("scan.scanning_title", { path: root })} state={state} />
      <div className="flex gap-2">
        {paused ? (
          <Button onClick={onResume}>
            <Play aria-hidden className="h-4 w-4" />
            {t("scan.resume")}
          </Button>
        ) : (
          <Button variant="secondary" onClick={onPause}>
            <Pause aria-hidden className="h-4 w-4" />
            {t("scan.pause")}
          </Button>
        )}
        <Button variant="ghost" onClick={onCancel}>
          <X aria-hidden className="h-4 w-4" />
          {t("scan.cancel")}
        </Button>
      </div>
    </div>
  );
}
