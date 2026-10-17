import { useEffect, useState } from "react";
import { api } from "@/lib/api";
import { MemoryDialog } from "./MemoryDialog";
import { MEMORY_OPEN_EVENT } from "./memoryBus";

/**
 * Mounts once in App and shows the Memory dialog whenever anything calls
 * `requestMemoryDialog()` (status bar, Home, the tray's "Memori" item).
 */
export function MemoryHost() {
  const [open, setOpen] = useState(false);
  const [os, setOs] = useState("macos");

  useEffect(() => {
    let live = true;
    api
      .appInfo()
      .then((i) => live && setOs(i.os))
      .catch(() => undefined);
    const show = () => setOpen(true);
    window.addEventListener(MEMORY_OPEN_EVENT, show);
    return () => {
      live = false;
      window.removeEventListener(MEMORY_OPEN_EVENT, show);
    };
  }, []);

  if (!open) return null;
  return <MemoryDialog os={os} onClose={() => setOpen(false)} />;
}
