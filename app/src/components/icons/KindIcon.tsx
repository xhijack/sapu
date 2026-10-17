import { Archive, AppWindow, Box, Code2, Database, Disc, FileText, FileType2, Folder, HardDrive, History, Image, Library, Music, Package, Presentation, ScrollText, Sheet, Video, File } from "lucide-react";
import type { FileKind } from "@/lib/types";

const ICONS: Record<FileKind, typeof File> = {
  video: Video,
  audio: Music,
  image: Image,
  pdf: FileText,
  document: FileText,
  spreadsheet: Sheet,
  presentation: Presentation,
  archive: Archive,
  disk_image: Disc,
  installer: Package,
  application: AppWindow,
  code: Code2,
  log: ScrollText,
  database: Database,
  font: FileType2,
  library: Library,
  virtual_disk: HardDrive,
  backup: History,
  folder: Folder,
  other: Box,
};

/** Small monochrome glyph for a file kind; decorative, label comes from text. */
export function KindIcon({ kind, size = 16, className }: { kind: FileKind; size?: number; className?: string }) {
  const Icon = ICONS[kind] ?? File;
  return <Icon aria-hidden width={size} height={size} className={className ?? "shrink-0 text-fg-muted"} />;
}
