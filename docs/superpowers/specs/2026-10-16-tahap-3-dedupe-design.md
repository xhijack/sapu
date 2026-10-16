# Design: Tahap 3 — Deduplikasi ("File apa yang sama persis?")

Date: 2026-10-16 · Status: approved (user: "lanjutkan") · Scope: PRD §11 Tahap 3 P0.

## Core (`sapu-core/src/dedupe/`)

- `pipeline.rs`: `find_duplicates(opts, control, progress) -> DedupeResult`. Stages, each reported with `stage = {current, total}` in the progress payload:
  1. walk the root (no symlinks; hardlinked inodes collapsed to one path) and group by size, dropping sizes below `min_size` (default 4 KiB) and singleton groups;
  2. partial hash (BLAKE3 of the first and last 4 KiB) within each size group, in parallel with rayon;
  3. full BLAKE3 for the survivors. Bytes hashed in stage 3 are the progress denominator (known after stage 2).
  Cancel via `ScanControl`; a cancelled run returns the groups confirmed so far.
- `group.rs`: `DuplicateGroup { id, size, hash, files: Vec<DupFile { path, mtime, name_score }>, wasted_bytes }`; `folder_pairs(groups)` aggregates "folder A and folder B share N files" for the lay user's view.
- `keep.rs`: `KeepStrategy { Oldest, Newest, OriginalName, InFolder(Vec<PathBuf>) }` → `choose_keep(group) -> usize`. `OriginalName` penalises `copy`, `Copy of`, `Salinan`, `(1)`, ` 2` suffixes.
- `plan.rs`: `DedupePlan` can only be built by `DedupePlan::build(groups, keep_for_group)`, which keeps exactly one file per group and refuses groups where the keep index is invalid. `execute(plan, safety, apply, progress)` re-checks that the kept file still exists right before each group (if it vanished the whole group is skipped), then runs one `DeletePlan` per group through `Safety` (scope = the dedupe root). This is the "never delete all copies" guarantee in core, with tests.

## CLI

`sapu dupes <path> [--json] [--min-size 4KiB] [--apply --keep oldest|newest|original]`. Dry-run by default; prints groups (largest wasted first) and folder pairs.

## Tauri + UI

Commands: `dedupe_start(root, min_size)`, `dedupe_cancel`, `dedupe_summary`, `dedupe_groups(offset, limit)`, `dedupe_folder_pairs`, `dedupe_execute(keeps: {group_id → path}, apply)`, `file_preview(path)` (data URL for images ≤ 20 MB, first 2 KB of text for text files, metadata otherwise).

Screen *Duplikat*: root picker → stage progress (ETA from core) → results. Headline "X bisa dibebaskan dari N grup". Section *Per folder* (pairs), section *Grup* with a global keep strategy select and per-group cards: side-by-side previews, radio marking the one to keep, path and date under each. Primary button "Hapus duplikat, bebaskan X" → preview dialog → execute (Trash) → result + History link.

## Acceptance

- Core test: a plan never lists every file of a group; executing with a vanished keep skips the group.
- Core test: identical content with different sizes never groups; same size different content never groups; hardlinks are one file.
- Core test: cancel mid-hash returns partial groups.
- UI test: strategy change moves the radio; preview shows count/size; nothing runs on cancel.
