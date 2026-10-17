/** Ask the Rapikan screen to open on a folder (from Scan → Browse → Folders). */
export const ORGANIZE_OPEN_EVENT = "sapu:organize-open";

let pending: string | null = null;

export function requestOrganize(root: string) {
  pending = root;
  window.dispatchEvent(new CustomEvent<string>(ORGANIZE_OPEN_EVENT, { detail: root }));
}

/** The folder asked for before the screen existed, once. */
export function takePendingOrganize(): string | null {
  const r = pending;
  pending = null;
  return r;
}
