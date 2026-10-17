/**
 * One-line way for any surface (status bar, Home, the tray event in App)
 * to ask for the Memory dialog without threading props through the Shell.
 * `MemoryHost` listens and shows the dialog.
 */
export const MEMORY_OPEN_EVENT = "sapu:memory-open";

export function requestMemoryDialog() {
  window.dispatchEvent(new Event(MEMORY_OPEN_EVENT));
}
