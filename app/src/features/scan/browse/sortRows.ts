import type { UnixTime } from "@/lib/types";
import type { FileRow } from "./FileTable";

export type SortKey = "name" | "kind" | "accessed" | "size";

const timeOf = (t: UnixTime | null) => t?.secs_since_epoch ?? -1;

export function sortRows(rows: FileRow[], key: SortKey, desc: boolean): FileRow[] {
  const dir = desc ? -1 : 1;
  const cmp: Record<SortKey, (a: FileRow, b: FileRow) => number> = {
    name: (a, b) => a.name.localeCompare(b.name, undefined, { sensitivity: "base", numeric: true }),
    kind: (a, b) => a.kind.localeCompare(b.kind) || b.bytes - a.bytes,
    accessed: (a, b) => timeOf(a.atime) - timeOf(b.atime) || b.bytes - a.bytes,
    size: (a, b) => a.bytes - b.bytes || a.name.localeCompare(b.name),
  };
  return [...rows].sort((a, b) => dir * cmp[key](a, b));
}
