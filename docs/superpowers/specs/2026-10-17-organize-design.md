# Organize ("Rapikan"): tidy a folder into meaningful groups

Date: 17 October 2026 · Status: implemented (Sprint 1 core + AI + CLI; Sprint 2 Tauri + UI). The UI sends only the selection; the plan stays in the app session. · Owner decision: grouping by topic/project with AI help, file kind inside; proposal first, then tick per group, always undoable.

## 1. Purpose

People have folders such as Documents, Desktop and Downloads that hold hundreds or thousands of loose files. Reading them one by one is not an option. Sapu reads only names, extensions, dates and the folder structure, proposes groups (a project, a topic, a kind of file), lets the person approve each group, moves the files into subfolders, finds duplicates on the way, and can put everything back.

Success: a messy Documents folder becomes a set of named subfolders in a few minutes, nothing is lost, and "Undo" restores the previous layout exactly.

Out of scope for this version: reading file contents, cross-volume moves, renaming files, scheduling, organising inside detected projects or app bundles.

## 2. User flow

1. **Rapikan** in the sidebar, or "Rapikan folder ini" from Scan → Browse → Folders.
2. Idle screen: pick a folder (Documents, Desktop, Downloads, or any folder), see folders tidied before (from history) and tips.
3. The analysis runs as a background task with progress (stages: reading, grouping, asking AI, finding duplicates). The person can leave the screen; the status bar shows "Merapikan".
4. **Plan** screen: one card per proposed group with the destination folder name (editable), file count, size, three example names, a confidence badge (sure / check) and a source badge (existing folder, same name prefix, file kind, AI). "Lihat isi" opens the file table (the existing browser table) where single files can be excluded. Groups are ticked by default when confidence is "sure".
5. **Rapikan N kelompok** starts the move as a background task with progress (file N of M, current group). The dialog can be hidden.
6. Result: "412 files into 9 folders", duplicate summary, **Urungkan**. Undo is also available from History while the files are still where Sapu put them.

## 3. Architecture

New module `crates/sapu-core/src/organize/` with four units; a new AI task in `crates/sapu-ai/src/tasks/tidy.rs`; Tauri commands in `app/src-tauri/src/organize.rs`; frontend `app/src/features/organize/`.

### 3.1 `organize/scan.rs`

Lists the top level of the target folder. Each entry becomes an `OrganizeEntry { path, name, is_dir, kind: FileKind, bytes, mtime, unit: Unit }` where `Unit` is:

- `File` for a regular file,
- `Folder` for an ordinary subfolder,
- `Project` when the folder matches the ecosystem markers already used by `cleaner::projects` (`.git`, `package.json`, `Cargo.toml`, `pyproject.toml`, `go.mod`, `build.gradle`, …),
- `Bundle` for `.app` and other package directories on macOS.

`Project` and `Bundle` entries are moved as one unit and never opened. Symlinks are listed but never moved (`unit: Link`, excluded from every group). Hidden entries (`.DS_Store`, dotfiles) are ignored.

### 3.2 `organize/group.rs` (deterministic)

Builds candidate groups in this order; every entry ends up in exactly one group.

1. **Existing folder**: each ordinary subfolder is a group of its own (name = folder name, confidence sure). Nothing is moved into or out of it unless the AI step merges a prefix group into it with high confidence.
2. **Name prefix**: tokens are the name split on spaces, `-`, `_`, `.` and camelCase boundaries; a group forms when at least three files share the same first token (case-insensitive, digits-only tokens ignored). Name = the shared prefix, confidence sure when the shared run is two tokens or more, otherwise check.
3. **File kind**: remaining files by `FileKind` mapped to folder names (Dokumen, Gambar, Video, Audio, Arsip, Installer, Kode, Lainnya). Confidence sure.
4. **Projects and bundles** are grouped under "Proyek" and "Aplikasi" respectively, moved as units, confidence sure.

Output: `OrganizePlan { root, groups: Vec<Group> }`, `Group { id, name, source: Existing | Prefix | Kind | Unit | Ai, confidence: Sure | Check, entries: Vec<OrganizeEntry>, bytes }`.

### 3.3 AI task `tidy` (sapu-ai, optional)

Runs only when AI mode is on. Input is the candidate list, not the file list: for each group its id, provisional name, source, count, and at most twelve example names (anonymised like every other task). Output schema with `deny_unknown_fields`:

```json
{ "groups": [{ "id": "g3", "name": "Project A", "confidence": 0.8 }],
  "merge": [["g3", "g7"]] }
```

Rules enforced in core after the answer: names are sanitised to a single folder name (no path separators, no leading dot, ≤ 60 chars); merges only join groups that exist; a merge into an existing folder is applied only when confidence ≥ 0.7; any group the AI did not mention keeps its heuristic name. An AI failure or an off setting leaves the heuristic plan untouched. The AI never sees contents and cannot move anything outside the target folder.

### 3.4 `organize/plan.rs`

Turns ticked groups into a `MovePlan { root, moves: Vec<Move { from, to, snapshot: FileSnapshot }> }`:

- destination is always `root/<group name>/<file name>`;
- name clashes get a ` (2)`, ` (3)` suffix;
- a group whose destination folder equals the entry's current parent produces no move;
- the plan refuses any `to` outside `root`, any protected path, and any `from` on another volume than `root`.

### 3.5 `organize/apply.rs`

The only place in the workspace allowed to call `std::fs::rename` (added to `clippy.toml` disallowed methods and `scripts/check-forbidden-fs.sh`, like the removal APIs). Before each move it re-validates the snapshot (exists, same size, same mtime, same type) and creates the destination folder. Progress goes through the existing `ProgressSink`. The run is recorded in `HistoryStore` as an `OrganizeSession { id, root, started_at_unix, finished_at_unix, moves: Vec<{ from, to, bytes }>, moved, skipped, failed }`.

`undo(session)` moves every entry back from `to` to `from` when `to` still exists and `from` is free, in reverse order, and removes destination folders that became empty (through `safety`, as a permanent removal of an empty directory is harmless; the folder was created by Sapu). The session is marked `undone`.

### 3.6 Duplicates

After grouping, `dedupe::find_duplicates` runs on the target folder (top level and existing subfolders, not inside projects). Duplicate groups appear on the plan screen as a separate "Duplikat" section using the existing keep strategy UI (newest / original / per folder). Removing them is a normal dedupe execution through `safety` (Trash), never a move.

### 3.7 Tauri layer

`organize_analyze_start(root)` (events `organize://progress`, `organize://plan`, `organize://error`), `organize_apply_start(plan)` (events `organize://run_progress`, `organize://run_done`, `organize://run_error`), `organize_undo(session_id)`, `organize_history(limit)`, `organize_status()`. Sessions are kept in an `OrganizeSession` state like `CleanSession`.

### 3.8 Frontend

`features/organize/`: `OrganizeView` (phases idle → analysing → plan → applying → done), `PlanGroupCard`, `GroupFilesDialog` (reuses `FileTable`), `UndoBanner`. Sidebar entry "Rapikan" between Duplikat and Aplikasi. The status bar gets `task_organize`. History lists organize sessions with an Undo button. Locales in both languages.

## 4. Safety rules (not negotiable)

- Moves stay inside the target folder; protected paths are refused; symlinks are never moved.
- Detected projects and app bundles move as one unit and are never opened.
- Re-validation before every move; a changed file is skipped, never moved.
- Every real run is recorded and undoable; undo is exact (original paths).
- Duplicates are removed only through `safety`, to the Trash.
- The AI only names and merges groups; its answer is validated against the schema and the sanitising rules above.

## 5. Error handling

Unreadable entries are listed under "Tidak bisa dibaca" and excluded. A move that fails mid-run is recorded as failed, the run continues, and the result says how many failed; undo covers the successful moves. Cross-volume targets (a folder on an external disk whose entries are symlinks elsewhere) are refused with a clear message.

## 6. Testing

Core, on temporary folders under `.test-tmp`:

- prefix grouping needs three files and respects the two-token rule;
- kind grouping covers every `FileKind`;
- a folder with `package.json` is one unit and is not opened;
- symlinks and hidden files are never moved;
- name clashes get suffixes; a destination outside root is refused;
- apply then undo restores the exact tree (compared by path listing);
- changed files are skipped on re-validation;
- AI answers: unknown fields rejected, bad names sanitised, low-confidence merges ignored, provider failure leaves the plan unchanged (fake HTTP server like other tasks).

Frontend: idle → plan → untick a group → apply → result → undo; the status bar shows the task; the Browse entry point opens the right folder.

## 7. Delivery

Sprint 1: core `organize/` + AI task + CLI `sapu organize <folder> [--apply]` (dry run by default, like `clean`). Sprint 2: Tauri commands + UI + history undo. Both end with the usual gates (fmt, clippy, tests, lint, typecheck, forbidden-fs script extended with `rename`).

## 8. Revision 1 (17 October 2026): per-item advice

Owner feedback after a real run on `Documents/Pribadi` (14 folders, 25 loose files): the result kept all 14 folders, put 22 files into one "Documents" group by file kind, and moved an Office lock file. Causes: the AI could only rename or merge whole groups, never split one; nothing could go into or out of an existing folder; the AI never saw what folders contain; the CLI asked for English.

Owner decisions: the AI decides a destination **per item**; it may read **folder listings** (names inside folders, never file contents); it may suggest **better file names**; it may flag **likely junk** and recommend removing it.

Changes to §3:

- **Scan** ignores temporary and lock files (`~$*`, `.~lock*`, `Thumbs.db`, `desktop.ini`); they are never moved. Each folder, project and bundle carries up to 8 child names (`children`), read with `read_dir`, nothing opened.
- **Heuristic** (no AI): a loose file whose normalised name contains the normalised name of an existing folder (letters and digits only, lowercase, folder name at least 4 characters) is proposed into that folder.
- **AI task `tidy_items`** replaces `tidy` for the app and CLI. Input: language from settings, folder name, up to 250 items (`name`, `kind`, `unit`, `year`, `size_kb`, `children`). Output (strict schema): `categories: [{name}]` (4 to 8, in the person's language) and `items: [{name, category | null, rename | null, junk, reason, confidence}]`. Prompt rules: group by purpose (finance, work, identity, business, worship, letters, …), not by file kind; reuse existing folder names; no folder for a single item unless clearly separate; keep extensions; flag junk only for things like temporary exports, duplicates by name, empty-looking or meaningless names.
- **Core `organize::advice`** turns the answer into a plan, validating everything: an item must match a listed entry exactly; one destination per item; category names sanitised; a category equal to an existing folder (case-insensitive) is that folder; renames only for files, sanitised, original extension kept; junk only for files; anything unknown is ignored; an invalid answer leaves the heuristic plan.
- **Existing folders may move** into a category as a whole (one unit, never opened). A group containing a folder is never ticked by default; the person decides.
- **Renames** are offered per file and unticked by default; a renamed file that stays in place moves to `root/<new name>` (the plan check allows `root/<name>` as well as `root/<group>/<name>`). Undo restores the old name.
- **Junk** is a separate "Kemungkinan sampah" list, yellow, unticked, each with the AI's reason. Ticked items go to the Trash through `safety` (never permanent, never moved). AI labels stay yellow at most (CLAUDE.md).
- Groups whose average item confidence is at least 0.75 are "Pasti"; others "Periksa dulu".
