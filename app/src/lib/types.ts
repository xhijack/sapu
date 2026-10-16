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
  notify_when_done: boolean;
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

export interface ScanDiff {
  since_unix: number;
  delta_bytes: number;
  delta_files: number;
  categories: { category: Category; delta_bytes: number }[];
  dirs: { path: string; delta_bytes: number; is_new: boolean }[];
}

export interface PreviousScan {
  finished_at_unix: number;
  total_bytes: number;
  file_count: number;
  estimated_secs: number | null;
}

export type OpKind = "scan" | "clean" | "dedupe";

export interface ScanSummary {
  root: string;
  diff: ScanDiff | null;
  disk_total_bytes: number | null;
  disk_free_bytes: number | null;
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
  root: string | null;
}

export interface ScanTargets {
  home: string | null;
  disk: string;
}

export interface LiveSnapshot {
  current_dir: string;
  categories: [Category, number][];
  top_dirs: [string, number][];
  files: number;
  bytes: number;
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

// ---- Clean (Tahap 2) ----

export type Risk = "green" | "yellow";

export interface Localized {
  id: string;
  en: string;
}

export interface RuleInfo {
  id: string;
  name: Localized;
  description: Localized;
  risk: Risk;
  group: string;
  permanent: boolean;
}

export interface Candidate {
  path: string;
  size: number;
  is_dir: boolean;
  mtime: { secs_since_epoch: number; nanos_since_epoch: number } | null;
}

export interface Finding {
  rule_id: string;
  risk: Risk;
  profile: Profile;
  group: string;
  permanent: boolean;
  scope: string;
  items: Candidate[];
  bytes: number;
  skipped_protected: number;
}

export interface CommandFinding {
  rule_id: string;
  risk: Risk;
  profile: Profile;
  group: string;
  command_line: string;
  available: boolean;
  confirm_separately: boolean;
  estimate_bytes: number | null;
}

export type Ecosystem = "node" | "rust" | "python" | "go" | "dart" | "gradle" | "maven";

export interface Artefact {
  path: string;
  kind: string;
  bytes: number;
  gitignored: boolean;
  tracked: boolean | null;
  risk: Risk;
}

export interface Project {
  root: string;
  name: string;
  ecosystem: Ecosystem;
  last_activity: { secs_since_epoch: number; nanos_since_epoch: number } | null;
  idle_days: number | null;
  artefacts: Artefact[];
  bytes: number;
  rebuild: string[];
}

export interface CleanReport {
  findings: Finding[];
  commands: CommandFinding[];
  projects: Project[];
}

export interface CleanEvaluation {
  report: CleanReport;
  rules: RuleInfo[];
  preselected: string[];
  preselected_artefacts: string[];
}

export type RuleOutcome =
  | { kind: "files"; rule_id: string; session: CleanSession }
  | { kind: "project"; root: string; session: CleanSession }
  | { kind: "command"; rule_id: string; outcome: { command_line: string; success: boolean; stdout: string; stderr: string } }
  | { kind: "skipped"; rule_id: string; reason: string };

export interface CleanRunReport {
  dry_run: boolean;
  outcomes: RuleOutcome[];
  bytes_freed: number;
  disposed: number;
  skipped: number;
  failed: number;
}

// ---- Dedupe (Tahap 3) ----

export interface DupFile {
  path: string;
  mtime: { secs_since_epoch: number; nanos_since_epoch: number } | null;
  name_score: number;
}

export interface DuplicateGroup {
  id: number;
  size: number;
  hash: string;
  files: DupFile[];
  wasted_bytes: number;
}

export interface FolderPair {
  a: string;
  b: string;
  files: number;
  bytes: number;
}

export interface DedupeSummary {
  root: string;
  files_seen: number;
  candidates: number;
  groups: number;
  duplicate_files: number;
  wasted_bytes: number;
  cancelled: boolean;
  stage_reached: number;
}

export type KeepStrategy = { kind: "oldest" } | { kind: "newest" } | { kind: "original_name" } | { kind: "in_folder"; folders: string[] };

export interface DedupeProgressPayload extends ScanProgressPayload {
  stage: { current: number; total: number };
}

export type GroupOutcome =
  | { kind: "done"; group_id: number; session: CleanSession; linked: number; link_failed: string[] }
  | { kind: "keep_missing"; group_id: number; keep: string };

export interface DedupeRunReport {
  dry_run: boolean;
  outcomes: GroupOutcome[];
  bytes_freed: number;
  disposed: number;
  skipped: number;
  failed: number;
}

export type FilePreview = { kind: "image"; data_url: string } | { kind: "text"; excerpt: string } | { kind: "document"; doc_kind: string; pages: number | null } | { kind: "none" };

// ---- System (home dashboard) ----

export interface DiskInfo {
  name: string;
  mount: string;
  total_bytes: number;
  available_bytes: number;
  removable: boolean;
  file_system: string;
}

export interface SystemSnapshot {
  cpu_percent: number | null;
  cpu_per_core: number[];
  cpu_count: number;
  mem_total_bytes: number;
  mem_used_bytes: number;
  swap_used_bytes: number;
  disks: DiskInfo[];
  os_name: string;
}

// ---- AI (PRD §12) ----

export type AiModeKind = "off" | "local" | "cloud";
export type CloudKind = "openai_compatible" | "anthropic";

export interface AiConfig {
  mode: AiModeKind;
  local_url: string;
  local_model: string;
  cloud_kind: CloudKind;
  cloud_url: string;
  cloud_model: string;
  anonymize: boolean;
}

export interface AiStatus {
  enabled: boolean;
  config: AiConfig;
  has_key: boolean;
}

export interface AiPreview {
  provider: string;
  model: string;
  system: string;
  user: string;
}

export type Verdict = "keep" | "check_first";

export interface WhatIsAnswer {
  summary: string;
  likely_owner: string;
  verdict: Verdict;
  consequence: string;
  confidence: number;
  reason: string;
}

export interface ArtefactVerdict {
  kind: string;
  regenerable: boolean;
  verdict: Verdict;
  explanation: string;
  rebuild: string;
  confidence: number;
}

export interface ArtefactAnswer {
  project_summary: string;
  artefacts: ArtefactVerdict[];
}

export interface SearchFilter {
  category: string | null;
  min_bytes: number | null;
  min_age_days: number | null;
  max_age_days: number | null;
  extensions: string[];
  name_contains: string | null;
  description: string;
}

export interface ReportAnswer {
  what_fills: string;
  start_here: string;
  leave_alone: string;
}

export interface AiResult<T> {
  answer: T | null;
  preview: AiPreview;
}

export interface AiSearchResult {
  filter: SearchFilter;
  files: FileEntry[];
  preview: AiPreview;
}

export interface ScanLogEntry {
  root: string;
  finished_at_unix: number;
  total_bytes: number;
  file_count: number;
}

export interface HistoryTotals {
  sessions: number;
  bytes_freed: number;
  items_disposed: number;
  last_session_unix: number | null;
}

// ---- Care (R3) ----

export interface ScoreComponent {
  key: "free_space" | "safe_clutter" | "review_clutter" | "stale_projects";
  points: number;
  max: number;
}

export type Recommendation =
  | { kind: "clean_safe"; bytes: number }
  | { kind: "review_clutter"; bytes: number }
  | { kind: "stale_projects"; count: number; bytes: number }
  | { kind: "largest_folder"; path: string; bytes: number }
  | { kind: "disk_nearly_full"; free_pct: number };

export interface SpaceScore {
  score: number;
  components: ScoreComponent[];
  free_pct: number;
  safe_bytes: number;
  review_bytes: number;
  stale_project_bytes: number;
  stale_projects: number;
  recommendations: Recommendation[];
}

export interface CareReport {
  score: SpaceScore;
  scan: ScanSummary;
  preselected: string[];
  preselected_artefacts: string[];
  finished_at_unix: number;
}

export interface CareProgressPayload {
  stage: number;
  total: number;
  fraction: number | null;
  items_done: number;
  bytes_done: number;
  elapsed_secs: number;
  eta: Eta;
}
