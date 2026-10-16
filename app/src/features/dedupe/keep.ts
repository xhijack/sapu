import type { DuplicateGroup } from "@/lib/types";

export type StrategyKey = "original_name" | "oldest" | "newest" | "in_folder";

/** Default keep per strategy, mirroring core's `choose_keep` for display;
 *  the backend recomputes on execute, explicit picks win. */
export function defaultKeep(g: DuplicateGroup, strategy: StrategyKey, folders: string[] = []): string {
  const byName = [...g.files].sort((a, b) => b.name_score - a.name_score || a.path.length - b.path.length)[0]!;
  if (strategy === "original_name") return byName.path;
  if (strategy === "in_folder") {
    for (const folder of folders) {
      const inside = g.files.filter((f) => f.path.startsWith(folder.endsWith("/") ? folder : `${folder}/`));
      if (inside.length > 0) return [...inside].sort((a, b) => b.name_score - a.name_score || a.path.length - b.path.length)[0]!.path;
    }
    return byName.path;
  }
  const withTime = g.files.filter((f) => f.mtime);
  if (withTime.length === 0) return byName.path;
  const sorted = [...withTime].sort((a, b) => a.mtime!.secs_since_epoch - b.mtime!.secs_since_epoch);
  return (strategy === "oldest" ? sorted[0] : sorted[sorted.length - 1])!.path;
}
