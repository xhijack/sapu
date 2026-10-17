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
  menu_bar_monitor: boolean;
  /** The no-warranty notice was read and accepted. */
  disclaimer_accepted: boolean;
  /** The rating thank-you was shown (or dismissed) once. */
  rate_prompt_done: boolean;
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

export type UnixTime = { secs_since_epoch: number; nanos_since_epoch: number };

export type FileKind =
  | "video"
  | "audio"
  | "image"
  | "pdf"
  | "document"
  | "spreadsheet"
  | "presentation"
  | "archive"
  | "disk_image"
  | "installer"
  | "application"
  | "code"
  | "log"
  | "database"
  | "font"
  | "library"
  | "virtual_disk"
  | "backup"
  | "folder"
  | "other";

export interface FileEntry {
  path: string;
  size: number;
  mtime: UnixTime | null;
  atime: UnixTime | null;
  category: Category;
  kind: FileKind;
}

// ---- File browser (Jelajah) ----

export interface BrowseEntry {
  path: string;
  name: string;
  is_dir: boolean;
  bytes: number;
  files: number;
  mtime: UnixTime | null;
  atime: UnixTime | null;
  kind: FileKind;
  category: Category | null;
}

export interface BrowseFolder {
  path: string;
  bytes: number;
  files: number;
  entries: BrowseEntry[];
  more: number;
}

export type DownloadGroup = "installers" | "archives" | "video" | "audio" | "images" | "documents" | "folders" | "other";

export const DOWNLOAD_GROUPS: DownloadGroup[] = ["installers", "archives", "video", "audio", "images", "documents", "folders", "other"];

export interface DownloadEntry {
  path: string;
  name: string;
  is_dir: boolean;
  bytes: number;
  kind: FileKind;
  group: DownloadGroup;
  mtime: UnixTime | null;
  atime: UnixTime | null;
}

export interface DownloadsReport {
  dir: string;
  bytes: number;
  count: number;
  groups: { group: DownloadGroup; bytes: number; count: number }[];
  entries: DownloadEntry[];
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
  top_files: [string, number][];
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

export interface CleanCheckProgress {
  stage: "projects" | "rules";
  done: number;
  total: number;
  fraction: number | null;
  elapsed_secs: number;
  eta: Eta;
}

export type RunStep = { kind: "rule"; rule_id: string; items: number } | { kind: "command"; rule_id: string; command_line: string };

export interface CleanRunProgress {
  items_done: number;
  items_total: number | null;
  bytes_done: number;
  bytes_total: number | null;
  fraction: number | null;
  step: RunStep | null;
  elapsed_secs: number;
  eta: Eta;
}

export interface TrashStatus {
  bytes: number;
  items: number;
}

export interface CleanRunStatus {
  running: boolean;
  has_result: boolean;
}

export interface CleanCheckStatus {
  running: boolean;
  has_result: boolean;
  finished_at_unix: number | null;
}

export interface CleanEvaluation {
  report: CleanReport;
  rules: RuleInfo[];
  preselected: string[];
  preselected_artefacts: string[];
  profile: Profile;
  finished_at_unix: number;
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

/** Free-space band shared with the tray (core `care::space_level`). */
export type SpaceLevel = "ok" | "low" | "critical";

export interface DiskInfo {
  name: string;
  mount: string;
  total_bytes: number;
  available_bytes: number;
  removable: boolean;
  file_system: string;
  recommended_free_bytes: number;
  level: SpaceLevel;
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

// ---- Memory ("Bebaskan memori") ----

export type PressureLevel = "normal" | "elevated" | "high";

export interface MemoryPressure {
  total_bytes: number;
  used_bytes: number;
  available_bytes: number;
  used_pct: number;
  swap_used_bytes: number;
  level: PressureLevel;
}

export interface MemoryProcess {
  pid: number;
  name: string;
  memory_bytes: number;
  process_count: number;
  exe: string | null;
  bundle_id: string | null;
  quittable: boolean;
}

export interface MemorySnapshot {
  pressure: MemoryPressure;
  processes: MemoryProcess[];
}

export type QuitMethod = "apple_event" | "sigterm" | "taskkill" | "refused";

export interface QuitOutcome {
  pid: number;
  name: string;
  method: QuitMethod;
  requested: boolean;
  message: string;
}

export interface PurgeOutcome {
  ok: boolean;
  cancelled: boolean;
  used_before_bytes: number;
  used_after_bytes: number;
  message: string;
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
  /** "…abcd": the stored key's last characters, never the whole key. */
  key_hint: string | null;
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
  first_session_unix: number | null;
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

export interface CareLogEntry {
  finished_at_unix: number;
  score: number;
  free_pct: number;
  safe_bytes: number;
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

// ---- Apps (R4) ----

export interface InstalledApp {
  name: string;
  bundle_id: string | null;
  version: string | null;
  path: string;
  bytes: number;
  last_used: { secs_since_epoch: number; nanos_since_epoch: number } | null;
  is_system: boolean;
  /** Whether the binary fits this machine's CPU (macOS). */
  support: AppSupport;
}

export type AppSupport = "native" | "rosetta" | "unsupported" | "unknown";

export type LeftoverKind = "support" | "cache" | "preferences" | "logs" | "container" | "group_container" | "saved_state" | "web_kit" | "launch_agent" | "cookies";

export interface Leftover {
  path: string;
  kind: LeftoverKind;
  bytes: number;
}

// ---- Organize (Rapikan) ----

export type OrganizeUnit = "file" | "folder" | "project" | "bundle" | "link";
export type GroupSource = "existing" | "prefix" | "kind" | "unit" | "ai";
export type GroupConfidence = "sure" | "check";

export interface OrganizeEntry {
  path: string;
  name: string;
  unit: OrganizeUnit;
  kind: FileKind;
  bytes: number;
  mtime: UnixTime | null;
  /** Up to 8 names inside a folder (never file contents). */
  children?: string[];
  /** AI suggestion for a clearer file name, extension kept. */
  suggested_name?: string | null;
  /** Why the AI thinks this file is junk; null when it is not. */
  junk?: string | null;
}

export interface OrganizeGroup {
  id: string;
  name: string;
  source: GroupSource;
  confidence: GroupConfidence;
  entries: OrganizeEntry[];
  bytes: number;
}

export interface OrganizeAnalysis {
  /** `unplaced`: items that stay where they are. */
  plan: { root: string; groups: OrganizeGroup[]; unplaced?: OrganizeEntry[] };
  unreadable: string[];
  duplicate_groups: number;
  duplicate_files: number;
  duplicate_bytes: number;
  /** Office lock files and extra exact copies, found without AI. */
  leftovers?: OrganizeLeftover[];
  ai_used: boolean;
  ai_note: string | null;
}

export type LeftoverReason = { kind: "office_lock" } | { kind: "duplicate"; of: string };

export interface OrganizeLeftover {
  path: string;
  name: string;
  bytes: number;
  reason: LeftoverReason;
}

export interface OrganizeSelection {
  group_ids: string[];
  excluded: string[];
  renames: [string, string][];
  /** Accepted new names for single files: [path, new name]. */
  file_renames: [string, string][];
}

export interface OrganizeRun {
  id: string;
  root: string;
  started_at_unix: number;
  finished_at_unix: number;
  moves: { from: string; to: string; bytes: number; len: number }[];
  created_dirs: string[];
  moved: number;
  skipped: number;
  failed: number;
  undone: boolean;
}

export interface OrganizeRunResult {
  session: OrganizeRun;
  failures: [string, string][];
}

export interface OrganizeUndoReport {
  restored: number;
  skipped: number;
  failed: number;
}

export interface OrganizeTarget {
  key: "documents" | "desktop" | "downloads";
  path: string;
}

export interface OrganizeAnalyseProgress {
  stage: "reading" | "grouping" | "ai" | "duplicates";
  elapsed_secs: number;
}

export interface OrganizeRunProgress {
  items_done: number;
  items_total: number | null;
  fraction: number | null;
  elapsed_secs: number;
  eta: Eta;
}
