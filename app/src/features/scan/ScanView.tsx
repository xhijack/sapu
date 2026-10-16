import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useErrorMessage } from "@/lib/errors";
import { scanApi } from "@/lib/scanApi";
import type { ScanProgressPayload, ScanSummary } from "@/lib/types";
import { Results } from "./Results";
import { ScanHome } from "./ScanHome";
import { Scanning } from "./Scanning";

type Phase = { kind: "idle" } | { kind: "scanning"; root: string } | { kind: "done"; summary: ScanSummary };

/** Owns the scan lifecycle: idle → scanning (events) → results. */
export function ScanView() {
  const { t } = useTranslation();
  const errorMessage = useErrorMessage();
  const [phase, setPhase] = useState<Phase>({ kind: "idle" });
  const [progress, setProgress] = useState<ScanProgressPayload | null>(null);
  const [paused, setPaused] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const lastRoot = useRef<string | null>(null);

  // Resume a result that already exists in the backend (e.g. after switching screens).
  useEffect(() => {
    let live = true;
    scanApi
      .status()
      .then((s) => {
        if (!live) return;
        if (s.has_result && !s.running) scanApi.summary().then((sum) => live && setPhase({ kind: "done", summary: sum })).catch(() => undefined);
      })
      .catch(() => undefined);
    return () => {
      live = false;
    };
  }, []);

  useEffect(() => {
    const subs = [
      scanApi.onProgress((p) => setProgress(p)),
      scanApi.onDone((summary) => {
        setPhase({ kind: "done", summary });
        setProgress(null);
        setPaused(false);
      }),
      scanApi.onError((e) => {
        setError(errorMessage(e));
        setPhase({ kind: "idle" });
      }),
    ];
    return () => {
      void Promise.all(subs).then((fns) => fns.forEach((f) => f()));
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const start = useCallback(
    (root: string) => {
      setError(null);
      setProgress(null);
      lastRoot.current = root;
      scanApi
        .start(root)
        .then(() => setPhase({ kind: "scanning", root }))
        .catch((e) => setError(errorMessage(e)));
    },
    [errorMessage],
  );

  if (phase.kind === "scanning") {
    return (
      <Scanning
        root={phase.root}
        progress={progress}
        paused={paused}
        onPause={() => void scanApi.pause().then(() => setPaused(true))}
        onResume={() => void scanApi.resume().then(() => setPaused(false))}
        onCancel={() => void scanApi.cancel()}
      />
    );
  }
  if (phase.kind === "done") {
    return <Results summary={phase.summary} onRescan={() => start(phase.summary.root)} onNewScan={() => setPhase({ kind: "idle" })} />;
  }
  return <ScanHome onStart={start} error={error ?? (t("scan.title") && null)} />;
}
