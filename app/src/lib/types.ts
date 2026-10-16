// Mirrors of the serde shapes in sapu-core. Keep in sync with Rust.

export type Profile = "general" | "developer";
export type AiMode = "off" | "local" | "cloud";
export type Theme = "system" | "light" | "dark";

export interface Settings {
  onboarding_done: boolean;
  profile: Profile;
  locale: string | null;
  ai_mode: AiMode;
  theme: Theme;
}

export type DiskAccess = "not_applicable" | "granted" | "denied" | "unknown";
export type DevTool =
  | "docker"
  | "node"
  | "cargo"
  | "python"
  | "go"
  | "xcode"
  | "android_sdk"
  | "git";

export interface AppInfo {
  version: string;
  os: string;
  full_disk_access: DiskAccess;
  dev_tools: DevTool[];
}

export type Protection = "Exact" | "Subtree";
export interface ProtectedEntry {
  path: string;
  protection: Protection;
  reason: string;
}

export type DisposalKind = "trash" | "permanent";
export type Outcome =
  | { status: "disposed" }
  | { status: "would_dispose" }
  | { status: "protected"; reason: string }
  | { status: "skipped"; reason: { kind: string } }
  | { status: "failed"; message: string };

export interface SessionEntry {
  path: string;
  size: number;
  is_dir: boolean;
  outcome: Outcome;
}

export interface CleanSession {
  id: string;
  started_at_unix: number;
  finished_at_unix: number;
  dry_run: boolean;
  disposal: DisposalKind;
  scope: string;
  entries: SessionEntry[];
  bytes_freed: number;
  disposed: number;
  skipped: number;
  failed: number;
}

export interface RestoreReport {
  restored: string[];
  not_found_in_trash: string[];
}

export interface Duration {
  secs: number;
  nanos: number;
}

export type Eta =
  | { kind: "indeterminate" }
  | { kind: "computing" }
  | { kind: "range"; low: Duration; high: Duration }
  | { kind: "about"; secs: number; nanos: number }
  | { kind: "almost_done" };

export interface ProgressState {
  fraction: number | null;
  items_done: number;
  items_total: number | null;
  bytes_done: number;
  bytes_total: number | null;
  elapsed_secs: number;
  eta: Eta;
  stage?: { current: number; total: number; label: string };
}

export interface CommandError {
  code: string;
  message: string;
}

export type RiskLevel = "safe" | "review" | "danger";

// ---- Scan (Tahap 1) ----

export type Category =
  | "video"
  | "photo"
  | "document"
  | "application"
  | "system"
  | "temporary"
  | "development"
  | "other";

export const CATEGORIES: Category[] = [
  "video",
  "photo",
  "document",
  "application",
  "system",
  "temporary",
  "development",
  "other",
];

export interface CategorySummary {
  category: Category;
  bytes: number;
  files: number;
}

export interface ScanSummary {
  root: string;
  started_at_unix: number;
  finished_at_unix: number;
  cancelled: boolean;
  total_bytes: number;
  file_count: number;
  dir_count: number;
  denied_count: number;
  denied_sample: string[];
  categories: CategorySummary[];
}

export interface FileEntry {
  path: string;
  size: number;
  mtime: { secs_since_epoch: number; nanos_since_epoch: number } | null;
  category: Category;
}

export interface DirSummary {
  path: string;
  bytes: number;
  files: number;
}

export interface TreeNode {
  path: string;
  name: string;
  bytes: number;
  files: number;
  is_dir: boolean;
  category: Category | null;
  children: TreeNode[];
}

export type AgeFilter = "any" | "older_than_month" | "older_than_six_months" | "older_than_year";

export interface FileFilter {
  min_bytes: number;
  category: Category | null;
  age: AgeFilter;
}

export type ScanRunState = "running" | "paused" | "cancelled";

export interface ScanStatus {
  running: boolean;
  state: ScanRunState | null;
  has_result: boolean;
}

export interface ScanTargets {
  home: string | null;
  disk: string;
}

export interface ScanProgressPayload {
  fraction: number | null;
  items_done: number;
  items_total: number | null;
  bytes_done: number;
  bytes_total: number | null;
  elapsed_secs: number;
  eta: Eta;
}
