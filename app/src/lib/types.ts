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
  | { kind: "about"; "0": Duration }
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
