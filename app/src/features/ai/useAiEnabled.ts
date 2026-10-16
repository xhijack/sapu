import { useEffect, useState } from "react";
import { aiApi } from "@/lib/aiApi";

/** Whether AI help is on; re-checked on mount. */
export function useAiEnabled() {
  const [enabled, setEnabled] = useState(false);
  useEffect(() => {
    let live = true;
    aiApi
      .status()
      .then((s) => live && setEnabled(s.enabled))
      .catch(() => undefined);
    return () => {
      live = false;
    };
  }, []);
  return enabled;
}
