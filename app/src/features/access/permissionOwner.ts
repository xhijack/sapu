import { useEffect, useState } from "react";
import { api } from "@/lib/api";
import type { PermissionOwner } from "@/lib/types";

let cached: Promise<PermissionOwner | null> | null = null;

/** The app macOS asks about for Full Disk Access (fetched once per session). */
export function usePermissionOwner() {
  const [owner, setOwner] = useState<PermissionOwner | null>(null);
  useEffect(() => {
    let live = true;
    cached ??= api
      .appInfo()
      .then((i) => i.permission_owner ?? null)
      .catch(() => null);
    void cached.then((o) => live && setOwner(o));
    return () => {
      live = false;
    };
  }, []);
  return owner;
}

/** For tests: forget the cached owner. */
export function resetPermissionOwner() {
  cached = null;
}
