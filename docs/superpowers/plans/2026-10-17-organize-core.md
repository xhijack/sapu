# Organize (Rapikan) Sprint 1: core, AI task, CLI — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A `sapu_core::organize` module that proposes groups for the loose files of one folder, moves them into subfolders on request, records every move and can undo it, plus an optional AI task that names and merges groups, and a `sapu organize` CLI command. Sprint 2 (Tauri commands and UI) is a separate plan.

**Architecture:** `organize/scan.rs` lists one folder level and classifies entries (file, folder, project, bundle, link); `organize/group.rs` builds deterministic groups (existing folder → shared name prefix → file kind → units) and can apply names/merges that come from outside (the AI); `organize/plan.rs` turns ticked groups into a validated `MovePlan`; `organize/apply.rs` is the only place allowed to `rename`, re-validates each file, records an `OrganizeSession` through `organize/history.rs`, and undoes it. `sapu-ai/tasks/tidy.rs` is the AI task; core never depends on it (the CLI and later the app wire them together).

**Tech Stack:** Rust 2024 edition (MSRV 1.88), serde, thiserror, tempfile (tests), existing `scanner::FileKind`, `cleaner::projects` markers, `safety::{FileSnapshot, ProtectedPaths}`, `dedupe::find_duplicates`, `progress::ProgressSink`; sapu-ai `Task` trait with `extract_json`; clap for the CLI.

**Spec:** `docs/superpowers/specs/2026-10-17-organize-design.md`

## Global Constraints

- Edition 2024, `rust-version = "1.88"`; `cargo clippy --workspace --all-targets -- -D warnings` must be clean; no `unwrap`/`expect` outside tests (workspace lints warn, `disallowed_methods` deny).
- `std::fs::rename` may appear only in `crates/sapu-core/src/organize/apply.rs` (Task 1 adds the lint and the script check). Removal APIs stay confined to `safety/dispose.rs`.
- `sapu-ai` never depends on `sapu-core` (`scripts/check-ai-isolation.sh`); `sapu-core` never depends on `sapu-ai`.
- Moves stay inside the target folder; protected paths are refused; symlinks are never moved; projects and bundles move as one unit and are never opened.
- Every real run is recorded before `apply` returns and is undoable; undo restores original paths.
- Tests that create files use `tempfile::tempdir_in(<repo>/.test-tmp)` (see `scanner/walk.rs::tests::tempdir`), never real user folders.
- Public docs and code comments in English; user-facing strings are not part of this sprint (CLI prints English like the other subcommands).

## Review Focus

1. A folder whose name already equals a proposed group name (for example an existing `Dokumen/` folder and a kind group "Dokumen"): the two must become one group, not a clash that creates `Dokumen (2)/` — pinned in Task 3 (`existing_folder_absorbs_kind_group_of_same_name`).
2. A file whose name contains path-hostile characters or is only an extension (`.env`, `..`, `Makefile`): grouping must never produce an empty or dotted destination name — pinned in Task 3 (`names_are_never_empty_or_dotted`).
3. A file that changes between planning and moving (edited document): it must be skipped, not moved — pinned in Task 6 (`changed_file_is_skipped`).
4. Undo after the user moved one of the files away: the rest must still return and the session report must say one skipped — pinned in Task 6 (`undo_skips_entries_that_moved_again`).
5. An AI answer that tries to escape the folder (`"name": "../Desktop"`) or returns an unknown group id: must be ignored, never applied — pinned in Task 7 (`names_are_sanitised`, `unknown_ids_are_ignored`).

---

### Task 1: Lint guard for `rename`

**Files:**
- Modify: `clippy.toml`
- Modify: `scripts/check-forbidden-fs.sh`

**Interfaces:**
- Produces: a build that fails when `std::fs::rename` appears outside `crates/sapu-core/src/organize/apply.rs`.

- [ ] **Step 1: Add the disallowed method**

In `clippy.toml`, inside `disallowed-methods = [ … ]`, add after the `trash::TrashContext::delete_all` line:

```toml
  { path = "std::fs::rename", reason = "moves must go through sapu_core::organize::apply" },
```

- [ ] **Step 2: Teach the script about the second allowed file**

Replace the body of `scripts/check-forbidden-fs.sh` with:

```sh
#!/usr/bin/env sh
# Fails if any removal API, the rename API, or the clippy opt-out appears
# outside the one file allowed for each. Complements clippy.toml.
set -eu
cd "$(dirname "$0")/.."
REMOVE_ALLOWED="crates/sapu-core/src/safety/dispose.rs"
RENAME_ALLOWED="crates/sapu-core/src/organize/apply.rs"
REMOVE_PATTERN='remove_file\|remove_dir\|remove_dir_all\|trash::delete\|TrashContext\|allow(clippy::disallowed_methods)'
hits=$(grep -rn --include='*.rs' "$REMOVE_PATTERN" crates app 2>/dev/null | grep -v "^$REMOVE_ALLOWED:" || true)
if [ -n "$hits" ]; then
  echo "Forbidden file-removal usage outside $REMOVE_ALLOWED:" >&2
  echo "$hits" >&2
  exit 1
fi
hits=$(grep -rn --include='*.rs' 'fs::rename' crates app 2>/dev/null | grep -v "^$RENAME_ALLOWED:" || true)
if [ -n "$hits" ]; then
  echo "Forbidden rename usage outside $RENAME_ALLOWED:" >&2
  echo "$hits" >&2
  exit 1
fi
echo "ok: removal APIs only in $REMOVE_ALLOWED; rename only in $RENAME_ALLOWED"
```

- [ ] **Step 3: Run the script and clippy**

Run: `sh scripts/check-forbidden-fs.sh && cargo clippy --workspace --all-targets -- -D warnings`
Expected: `ok: removal APIs only in …; rename only in …` and clippy clean (nothing renames yet).

- [ ] **Step 4: Commit**

```bash
git add clippy.toml scripts/check-forbidden-fs.sh
git commit -m "build: confine std::fs::rename to organize/apply.rs"
```

---

### Task 2: `organize/scan.rs` — list and classify one folder level

**Files:**
- Create: `crates/sapu-core/src/organize/mod.rs`
- Create: `crates/sapu-core/src/organize/scan.rs`
- Modify: `crates/sapu-core/src/lib.rs` (add `pub mod organize;` after `pub mod eta;`)
- Modify: `crates/sapu-core/src/cleaner/projects.rs` (expose `is_project_root`)

**Interfaces:**
- Consumes: `crate::scanner::FileKind::of(&Path)`, `crate::platform::disk_usage_of(&Metadata)`, `crate::cleaner::projects::MARKERS`.
- Produces:

```rust
pub enum Unit { File, Folder, Project, Bundle, Link }
pub struct OrganizeEntry { pub path: PathBuf, pub name: String, pub unit: Unit, pub kind: FileKind, pub bytes: u64, pub mtime: Option<SystemTime> }
pub struct FolderListing { pub root: PathBuf, pub entries: Vec<OrganizeEntry>, pub unreadable: Vec<PathBuf> }
pub fn scan_folder(root: &Path) -> Result<FolderListing>
pub fn is_project_root(dir: &Path) -> bool   // in cleaner::projects
```

- [ ] **Step 1: Expose project detection**

In `crates/sapu-core/src/cleaner/projects.rs`, directly after the `MARKERS` constant, add:

```rust
/// Does `dir` hold a project marker (`package.json`, `Cargo.toml`, …) or a
/// git checkout? Such folders are treated as one unit by other features.
pub fn is_project_root(dir: &Path) -> bool {
    dir.join(".git").exists() || MARKERS.iter().any(|(m, _)| dir.join(m).is_file())
}
```

and export it in `crates/sapu-core/src/cleaner/mod.rs`: change the `pub use projects::{…}` line to also include `is_project_root`.

- [ ] **Step 2: Write the failing tests**

Create `crates/sapu-core/src/organize/scan.rs` with only the test module first:

```rust
//! One folder level, classified for organising.

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    pub(crate) fn tempdir() -> tempfile::TempDir {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.test-tmp");
        std::fs::create_dir_all(&base).expect("mkdir test-tmp");
        tempfile::tempdir_in(base).expect("tempdir")
    }

    fn touch(p: &Path) {
        std::fs::create_dir_all(p.parent().expect("parent")).expect("mkdir");
        std::fs::write(p, b"x").expect("write");
    }

    #[test]
    fn classifies_files_folders_projects_bundles_and_links() {
        let t = tempdir();
        let r = t.path();
        touch(&r.join("Invoice-2024.pdf"));
        touch(&r.join("notes/a.txt"));
        touch(&r.join("backend/package.json"));
        touch(&r.join("Tool.app/Contents/Info.plist"));
        touch(&r.join(".DS_Store"));
        #[cfg(unix)]
        std::os::unix::fs::symlink(r.join("Invoice-2024.pdf"), r.join("link.pdf")).expect("symlink");

        let listing = scan_folder(r).expect("scan");
        let unit = |n: &str| listing.entries.iter().find(|e| e.name == n).map(|e| e.unit);
        assert_eq!(unit("Invoice-2024.pdf"), Some(Unit::File));
        assert_eq!(unit("notes"), Some(Unit::Folder));
        assert_eq!(unit("backend"), Some(Unit::Project));
        assert_eq!(unit("Tool.app"), Some(Unit::Bundle));
        #[cfg(unix)]
        assert_eq!(unit("link.pdf"), Some(Unit::Link));
        assert_eq!(unit(".DS_Store"), None, "hidden entries are skipped");
        assert!(listing.unreadable.is_empty());
    }

    #[test]
    fn folders_carry_their_contents_size_and_files_their_disk_size() {
        let t = tempdir();
        let r = t.path();
        touch(&r.join("notes/a.txt"));
        touch(&r.join("notes/b.txt"));
        touch(&r.join("solo.txt"));
        let listing = scan_folder(r).expect("scan");
        let notes = listing.entries.iter().find(|e| e.name == "notes").expect("notes");
        let solo = listing.entries.iter().find(|e| e.name == "solo.txt").expect("solo");
        assert!(notes.bytes >= 2 * solo.bytes.min(1), "folder size covers its files");
        assert!(solo.bytes > 0);
        assert_eq!(solo.kind, crate::scanner::FileKind::Document);
    }

    #[test]
    fn missing_root_is_an_error() {
        let t = tempdir();
        assert!(scan_folder(&t.path().join("nope")).is_err());
    }
}
```

Create `crates/sapu-core/src/organize/mod.rs`:

```rust
//! Organize ("Rapikan"): propose groups for the loose files of one folder,
//! move them into subfolders on request, record every move, undo it.
//! Moves never leave the target folder; see the design spec.

mod scan;

pub use scan::{FolderListing, OrganizeEntry, Unit, scan_folder};
```

Add `pub mod organize;` to `crates/sapu-core/src/lib.rs` after `pub mod eta;`.

- [ ] **Step 3: Run the tests to see them fail**

Run: `cargo test -p sapu-core organize::scan`
Expected: compile error, `scan_folder` and `Unit` not found.

- [ ] **Step 4: Implement**

Prepend to `crates/sapu-core/src/organize/scan.rs` (above the test module):

```rust
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::cleaner::is_project_root;
use crate::error::{Result, SapuError};
use crate::scanner::FileKind;

/// How an entry is handled: files move on their own, folders stay or move
/// whole, projects and bundles are never opened, links are never moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Unit {
    File,
    Folder,
    Project,
    Bundle,
    Link,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OrganizeEntry {
    pub path: PathBuf,
    pub name: String,
    pub unit: Unit,
    pub kind: FileKind,
    /// Disk usage of a file; measured contents of a folder.
    pub bytes: u64,
    pub mtime: Option<SystemTime>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FolderListing {
    pub root: PathBuf,
    pub entries: Vec<OrganizeEntry>,
    pub unreadable: Vec<PathBuf>,
}

const BUNDLE_EXTENSIONS: &[&str] = &["app", "bundle", "framework", "photoslibrary", "pkg"];

/// Lists the top level of `root`. Hidden entries are skipped; unreadable
/// ones are reported, not fatal.
pub fn scan_folder(root: &Path) -> Result<FolderListing> {
    let root = std::path::absolute(root).map_err(|e| SapuError::io("resolve root", e))?;
    if !root.is_dir() {
        return Err(SapuError::io(
            "organize root",
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "root is not a directory"),
        ));
    }
    let mut entries = Vec::new();
    let mut unreadable = Vec::new();
    for dir_entry in std::fs::read_dir(&root).map_err(|e| SapuError::io("read folder", e))? {
        let Ok(dir_entry) = dir_entry else { continue };
        let path = dir_entry.path();
        let name = dir_entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            unreadable.push(path);
            continue;
        };
        let unit = if meta.file_type().is_symlink() {
            Unit::Link
        } else if meta.is_dir() {
            let ext = path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase);
            if ext.is_some_and(|e| BUNDLE_EXTENSIONS.contains(&e.as_str())) {
                Unit::Bundle
            } else if is_project_root(&path) {
                Unit::Project
            } else {
                Unit::Folder
            }
        } else {
            Unit::File
        };
        let kind = match unit {
            Unit::File | Unit::Link => FileKind::of(&path),
            Unit::Bundle => FileKind::Application,
            Unit::Folder | Unit::Project => FileKind::Folder,
        };
        let bytes = match unit {
            Unit::Folder | Unit::Project | Unit::Bundle => crate::cleaner::dir_size_pub(&path),
            Unit::File | Unit::Link => crate::platform::disk_usage_of(&meta),
        };
        entries.push(OrganizeEntry {
            path,
            name,
            unit,
            kind,
            bytes,
            mtime: meta.modified().ok(),
        });
    }
    entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(FolderListing {
        root,
        entries,
        unreadable,
    })
}
```

- [ ] **Step 5: Run the tests**

Run: `cargo test -p sapu-core organize::scan`
Expected: 3 passed.

- [ ] **Step 6: Commit**

```bash
git add crates/sapu-core/src/organize crates/sapu-core/src/lib.rs crates/sapu-core/src/cleaner/projects.rs crates/sapu-core/src/cleaner/mod.rs
git commit -m "feat(organize): list and classify one folder level"
```

---

### Task 3: `organize/group.rs` — deterministic groups and external naming

**Files:**
- Create: `crates/sapu-core/src/organize/group.rs`
- Modify: `crates/sapu-core/src/organize/mod.rs`

**Interfaces:**
- Consumes: `FolderListing`, `OrganizeEntry`, `Unit`, `FileKind` from Task 2.
- Produces:

```rust
pub enum GroupSource { Existing, Prefix, Kind, Unit, Ai }
pub enum Confidence { Sure, Check }
pub struct Group { pub id: String, pub name: String, pub source: GroupSource, pub confidence: Confidence, pub entries: Vec<OrganizeEntry>, pub bytes: u64 }
pub struct OrganizePlan { pub root: PathBuf, pub groups: Vec<Group> }
pub fn propose(listing: &FolderListing) -> OrganizePlan
pub struct Naming { pub id: String, pub name: String, pub confidence: f32 }
pub fn apply_naming(plan: &mut OrganizePlan, names: &[Naming], merges: &[(String, String)])
pub fn sanitize_name(raw: &str) -> Option<String>
pub fn kind_folder(kind: FileKind) -> &'static str
```

- [ ] **Step 1: Write the failing tests**

Create `crates/sapu-core/src/organize/group.rs` with the test module:

```rust
//! Candidate groups from names, kinds and existing folders; names and
//! merges coming from outside (the AI) are applied here under strict rules.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::organize::scan_folder;
    use std::path::Path;

    fn tempdir() -> tempfile::TempDir {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.test-tmp");
        std::fs::create_dir_all(&base).expect("mkdir test-tmp");
        tempfile::tempdir_in(base).expect("tempdir")
    }
    fn touch(p: &Path) {
        std::fs::create_dir_all(p.parent().expect("parent")).expect("mkdir");
        std::fs::write(p, b"x").expect("write");
    }
    fn group<'a>(plan: &'a OrganizePlan, name: &str) -> &'a Group {
        plan.groups.iter().find(|g| g.name == name).unwrap_or_else(|| panic!("group {name} in {:?}", plan.groups.iter().map(|g| &g.name).collect::<Vec<_>>()))
    }

    #[test]
    fn three_files_with_a_shared_first_token_form_a_prefix_group() {
        let t = tempdir();
        for n in ["Invoice-2024-01.pdf", "Invoice-2024-02.pdf", "invoice March.pdf", "photo.jpg"] {
            touch(&t.path().join(n));
        }
        let plan = propose(&scan_folder(t.path()).expect("scan"));
        let g = group(&plan, "Invoice");
        assert_eq!(g.entries.len(), 3);
        assert_eq!(g.source, GroupSource::Prefix);
        assert_eq!(g.confidence, Confidence::Check, "one shared token only");
        assert_eq!(group(&plan, "Gambar").entries.len(), 1);
    }

    #[test]
    fn two_shared_tokens_are_sure_and_name_keeps_both() {
        let t = tempdir();
        for n in ["Laporan Keuangan Q1.xlsx", "Laporan Keuangan Q2.xlsx", "Laporan Keuangan Q3.xlsx"] {
            touch(&t.path().join(n));
        }
        let plan = propose(&scan_folder(t.path()).expect("scan"));
        let g = group(&plan, "Laporan Keuangan");
        assert_eq!(g.confidence, Confidence::Sure);
    }

    #[test]
    fn fewer_than_three_fall_back_to_kind_groups() {
        let t = tempdir();
        for n in ["Invoice-1.pdf", "Invoice-2.pdf", "clip.mp4", "song.mp3"] {
            touch(&t.path().join(n));
        }
        let plan = propose(&scan_folder(t.path()).expect("scan"));
        assert!(plan.groups.iter().all(|g| g.name != "Invoice"));
        assert_eq!(group(&plan, "Dokumen").entries.len(), 2);
        assert_eq!(group(&plan, "Video").entries.len(), 1);
        assert_eq!(group(&plan, "Audio").entries.len(), 1);
    }

    #[test]
    fn existing_folders_projects_bundles_and_links_are_handled() {
        let t = tempdir();
        touch(&t.path().join("Arsip Lama/a.txt"));
        touch(&t.path().join("backend/package.json"));
        touch(&t.path().join("Tool.app/Contents/Info.plist"));
        touch(&t.path().join("x.txt"));
        #[cfg(unix)]
        std::os::unix::fs::symlink(t.path().join("x.txt"), t.path().join("y.txt")).expect("symlink");
        let plan = propose(&scan_folder(t.path()).expect("scan"));
        assert_eq!(group(&plan, "Arsip Lama").source, GroupSource::Existing);
        assert_eq!(group(&plan, "Proyek").entries[0].name, "backend");
        assert_eq!(group(&plan, "Aplikasi").entries[0].name, "Tool.app");
        assert!(plan.groups.iter().all(|g| g.entries.iter().all(|e| e.unit != Unit::Link)), "links are never grouped");
        let total: usize = plan.groups.iter().map(|g| g.entries.len()).sum();
        assert_eq!(total, 4, "every non-link entry is in exactly one group");
    }

    #[test]
    fn existing_folder_absorbs_kind_group_of_same_name() {
        let t = tempdir();
        touch(&t.path().join("Dokumen/old.pdf"));
        for n in ["a.pdf", "b.docx"] {
            touch(&t.path().join(n));
        }
        let plan = propose(&scan_folder(t.path()).expect("scan"));
        let docs: Vec<_> = plan.groups.iter().filter(|g| g.name == "Dokumen").collect();
        assert_eq!(docs.len(), 1, "one group, not Dokumen and Dokumen (2)");
        assert_eq!(docs[0].source, GroupSource::Existing);
        assert_eq!(docs[0].entries.len(), 3);
    }

    #[test]
    fn names_are_never_empty_or_dotted() {
        assert_eq!(sanitize_name("  Project A  "), Some("Project A".into()));
        assert_eq!(sanitize_name("../Desktop"), None);
        assert_eq!(sanitize_name(".env"), None);
        assert_eq!(sanitize_name("a/b"), None);
        assert_eq!(sanitize_name(""), None);
        assert_eq!(sanitize_name(&"x".repeat(80)).map(|s| s.len()), Some(60));
        let t = tempdir();
        for n in ["Makefile", "README", "LICENSE"] {
            touch(&t.path().join(n));
        }
        let plan = propose(&scan_folder(t.path()).expect("scan"));
        assert!(plan.groups.iter().all(|g| sanitize_name(&g.name).as_deref() == Some(g.name.as_str())));
    }

    #[test]
    fn apply_naming_renames_merges_and_ignores_bad_input() {
        let t = tempdir();
        for n in ["Invoice-1.pdf", "Invoice-2.pdf", "Invoice-3.pdf", "Bill-1.pdf", "Bill-2.pdf", "Bill-3.pdf"] {
            touch(&t.path().join(n));
        }
        let mut plan = propose(&scan_folder(t.path()).expect("scan"));
        let inv = group(&plan, "Invoice").id.clone();
        let bill = group(&plan, "Bill").id.clone();
        apply_naming(
            &mut plan,
            &[
                Naming { id: inv.clone(), name: "Tagihan".into(), confidence: 0.9 },
                Naming { id: "nope".into(), name: "Ghost".into(), confidence: 0.9 },
                Naming { id: bill.clone(), name: "../x".into(), confidence: 0.9 },
            ],
            &[(bill.clone(), inv.clone()), ("nope".into(), inv.clone())],
        );
        let g = group(&plan, "Tagihan");
        assert_eq!(g.entries.len(), 6, "Bill merged into Invoice/Tagihan");
        assert_eq!(g.source, GroupSource::Ai);
        assert!(plan.groups.iter().all(|g| g.name != "Ghost" && g.name != "../x"));
    }

    #[test]
    fn low_confidence_merge_into_existing_folder_is_ignored() {
        let t = tempdir();
        touch(&t.path().join("Keuangan/old.xlsx"));
        for n in ["Invoice-1.pdf", "Invoice-2.pdf", "Invoice-3.pdf"] {
            touch(&t.path().join(n));
        }
        let mut plan = propose(&scan_folder(t.path()).expect("scan"));
        let inv = group(&plan, "Invoice").id.clone();
        let fin = group(&plan, "Keuangan").id.clone();
        apply_naming(&mut plan, &[Naming { id: inv.clone(), name: "Invoice".into(), confidence: 0.5 }], &[(inv.clone(), fin.clone())]);
        assert_eq!(group(&plan, "Invoice").entries.len(), 3, "kept apart below 0.7");
        apply_naming(&mut plan, &[Naming { id: inv.clone(), name: "Invoice".into(), confidence: 0.9 }], &[(inv, fin)]);
        assert_eq!(group(&plan, "Keuangan").entries.len(), 4);
    }
}
```

Update `crates/sapu-core/src/organize/mod.rs`:

```rust
mod group;
mod scan;

pub use group::{Confidence, Group, GroupSource, Naming, OrganizePlan, apply_naming, kind_folder, propose, sanitize_name};
pub use scan::{FolderListing, OrganizeEntry, Unit, scan_folder};
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p sapu-core organize::group`
Expected: compile errors for the missing items.

- [ ] **Step 3: Implement**

Prepend to `crates/sapu-core/src/organize/group.rs`:

```rust
use std::collections::BTreeMap;
use std::path::PathBuf;

use super::scan::{FolderListing, OrganizeEntry, Unit};
use crate::scanner::FileKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupSource {
    Existing,
    Prefix,
    Kind,
    Unit,
    Ai,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    Sure,
    Check,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub source: GroupSource,
    pub confidence: Confidence,
    pub entries: Vec<OrganizeEntry>,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OrganizePlan {
    pub root: PathBuf,
    pub groups: Vec<Group>,
}

/// A name (and how sure the namer is) for one group, from the AI.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Naming {
    pub id: String,
    pub name: String,
    pub confidence: f32,
}

const MIN_PREFIX_MEMBERS: usize = 3;
const MAX_NAME_LEN: usize = 60;
const MERGE_INTO_EXISTING_MIN_CONFIDENCE: f32 = 0.7;

/// Folder name for a kind group (Indonesian; the UI translates by key, the
/// folder on disk gets this name).
pub fn kind_folder(kind: FileKind) -> &'static str {
    match kind {
        FileKind::Video => "Video",
        FileKind::Audio => "Audio",
        FileKind::Image => "Gambar",
        FileKind::Pdf | FileKind::Document | FileKind::Spreadsheet | FileKind::Presentation => "Dokumen",
        FileKind::Archive => "Arsip",
        FileKind::Installer | FileKind::DiskImage => "Installer",
        FileKind::Code | FileKind::Database | FileKind::Log => "Kode",
        FileKind::Application => "Aplikasi",
        FileKind::Folder => "Folder",
        FileKind::Font | FileKind::Library | FileKind::VirtualDisk | FileKind::Backup | FileKind::Other => "Lainnya",
    }
}

/// A single safe folder name: trimmed, no separators, not dotted, ≤ 60 chars.
pub fn sanitize_name(raw: &str) -> Option<String> {
    let s: String = raw
        .trim()
        .chars()
        .filter(|c| !matches!(c, '/' | '\\' | ':' | '\0'))
        .take(MAX_NAME_LEN)
        .collect();
    let s = s.trim().to_string();
    if s.is_empty() || s.starts_with('.') || raw.contains('/') || raw.contains('\\') || s == ".." {
        return None;
    }
    Some(s)
}

/// Name tokens: split on spaces, `-`, `_`, `.`, and camelCase boundaries;
/// digit-only tokens are dropped. Case-insensitive comparison, original
/// case kept for display.
fn tokens(name: &str) -> Vec<String> {
    let stem = name.rsplit_once('.').map(|(s, _)| s).unwrap_or(name);
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut prev_lower = false;
    for c in stem.chars() {
        let boundary = c == ' ' || c == '-' || c == '_' || c == '.' || (c.is_uppercase() && prev_lower);
        if boundary && !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
        if !(c == ' ' || c == '-' || c == '_' || c == '.') {
            cur.push(c);
        }
        prev_lower = c.is_lowercase();
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out.into_iter().filter(|t| !t.chars().all(|c| c.is_ascii_digit())).collect()
}

fn next_id(n: usize) -> String {
    format!("g{n}")
}

/// Deterministic proposal. Every non-link entry lands in exactly one group.
pub fn propose(listing: &FolderListing) -> OrganizePlan {
    let mut groups: Vec<Group> = Vec::new();
    let mut push = |name: String, source: GroupSource, confidence: Confidence, entries: Vec<OrganizeEntry>, groups: &mut Vec<Group>| {
        let bytes = entries.iter().map(|e| e.bytes).sum();
        groups.push(Group {
            id: next_id(groups.len() + 1),
            name,
            source,
            confidence,
            entries,
            bytes,
        });
    };

    // 1. Existing folders, projects, bundles.
    let mut projects = Vec::new();
    let mut bundles = Vec::new();
    let mut files: Vec<&OrganizeEntry> = Vec::new();
    for e in &listing.entries {
        match e.unit {
            Unit::Folder => push(e.name.clone(), GroupSource::Existing, Confidence::Sure, vec![e.clone()], &mut groups),
            Unit::Project => projects.push(e.clone()),
            Unit::Bundle => bundles.push(e.clone()),
            Unit::File => files.push(e),
            Unit::Link => {}
        }
    }
    if !projects.is_empty() {
        push("Proyek".into(), GroupSource::Unit, Confidence::Sure, projects, &mut groups);
    }
    if !bundles.is_empty() {
        push("Aplikasi".into(), GroupSource::Unit, Confidence::Sure, bundles, &mut groups);
    }

    // 2. Shared first token (3+ files).
    let mut by_token: BTreeMap<String, Vec<&OrganizeEntry>> = BTreeMap::new();
    for e in &files {
        if let Some(first) = tokens(&e.name).first() {
            by_token.entry(first.to_lowercase()).or_default().push(e);
        }
    }
    let mut placed: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
    for members in by_token.values() {
        if members.len() < MIN_PREFIX_MEMBERS {
            continue;
        }
        // Longest run of shared tokens (case-insensitive) decides the name and confidence.
        let token_lists: Vec<Vec<String>> = members.iter().map(|e| tokens(&e.name)).collect();
        let mut shared = 0usize;
        loop {
            let Some(first) = token_lists[0].get(shared) else { break };
            if token_lists.iter().all(|t| t.get(shared).is_some_and(|x| x.eq_ignore_ascii_case(first))) {
                shared += 1;
            } else {
                break;
            }
        }
        let name = token_lists[0][..shared.max(1)].join(" ");
        let Some(name) = sanitize_name(&name) else { continue };
        let confidence = if shared >= 2 { Confidence::Sure } else { Confidence::Check };
        let entries: Vec<OrganizeEntry> = members.iter().map(|e| (*e).clone()).collect();
        for e in &entries {
            placed.insert(e.path.clone());
        }
        push(name, GroupSource::Prefix, confidence, entries, &mut groups);
    }

    // 3. Remaining files by kind.
    let mut by_kind: BTreeMap<&'static str, Vec<OrganizeEntry>> = BTreeMap::new();
    for e in files {
        if placed.contains(&e.path) {
            continue;
        }
        by_kind.entry(kind_folder(e.kind)).or_default().push(e.clone());
    }
    for (name, entries) in by_kind {
        push(name.to_string(), GroupSource::Kind, Confidence::Sure, entries, &mut groups);
    }

    // Same name as an existing folder: fold into it instead of creating "(2)".
    let mut plan = OrganizePlan {
        root: listing.root.clone(),
        groups,
    };
    fold_duplicate_names(&mut plan);
    plan
}

/// Merges groups that share a name (case-insensitive); the existing folder
/// wins the identity, otherwise the first one does.
fn fold_duplicate_names(plan: &mut OrganizePlan) {
    let mut i = 0;
    while i < plan.groups.len() {
        let name = plan.groups[i].name.to_lowercase();
        let mut j = i + 1;
        while j < plan.groups.len() {
            if plan.groups[j].name.to_lowercase() == name {
                let other = plan.groups.remove(j);
                let keep_other_identity = other.source == GroupSource::Existing && plan.groups[i].source != GroupSource::Existing;
                let target = &mut plan.groups[i];
                if keep_other_identity {
                    target.name = other.name;
                    target.source = GroupSource::Existing;
                    target.confidence = Confidence::Sure;
                }
                target.entries.extend(other.entries);
                target.bytes += other.bytes;
            } else {
                j += 1;
            }
        }
        i += 1;
    }
}

/// Applies names and merges from outside (the AI). Unknown ids and unsafe
/// names are ignored; a merge into an existing folder needs confidence ≥ 0.7.
pub fn apply_naming(plan: &mut OrganizePlan, names: &[Naming], merges: &[(String, String)]) {
    for n in names {
        let Some(name) = sanitize_name(&n.name) else { continue };
        if let Some(g) = plan.groups.iter_mut().find(|g| g.id == n.id) {
            if g.name != name {
                g.name = name;
                g.source = GroupSource::Ai;
            }
            g.confidence = if n.confidence >= MERGE_INTO_EXISTING_MIN_CONFIDENCE { Confidence::Sure } else { Confidence::Check };
        }
    }
    for (from, into) in merges {
        if from == into {
            continue;
        }
        let Some(fi) = plan.groups.iter().position(|g| g.id == *from) else { continue };
        let Some(ii) = plan.groups.iter().position(|g| g.id == *into) else { continue };
        let from_conf = names.iter().find(|n| n.id == *from).map(|n| n.confidence).unwrap_or(0.0);
        if plan.groups[ii].source == GroupSource::Existing && from_conf < MERGE_INTO_EXISTING_MIN_CONFIDENCE {
            continue;
        }
        let moved = plan.groups.remove(fi);
        let ii = if fi < ii { ii - 1 } else { ii };
        let target = &mut plan.groups[ii];
        target.entries.extend(moved.entries);
        target.bytes += moved.bytes;
        if target.source != GroupSource::Existing {
            target.source = GroupSource::Ai;
        }
    }
    fold_duplicate_names(plan);
}
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p sapu-core organize::group`
Expected: 8 passed. If `three_files_with_a_shared_first_token_form_a_prefix_group` reports the name as `Invoice-2024` instead of `Invoice`, check that `tokens` splits on `-` before comparing; the shared run across `Invoice-2024-01`, `Invoice-2024-02`, `invoice March` is one token.

- [ ] **Step 5: Commit**

```bash
git add crates/sapu-core/src/organize
git commit -m "feat(organize): deterministic groups and external naming rules"
```

---

### Task 4: `organize/plan.rs` — from ticked groups to a validated MovePlan

**Files:**
- Create: `crates/sapu-core/src/organize/plan.rs`
- Modify: `crates/sapu-core/src/organize/mod.rs`
- Modify: `crates/sapu-core/src/safety/mod.rs` (export `revalidate`)

**Interfaces:**
- Consumes: `OrganizePlan`, `Group`, `Unit`; `crate::safety::{FileSnapshot, ProtectedPaths}`.
- Produces:

```rust
pub struct Move { pub from: PathBuf, pub to: PathBuf, pub snapshot: FileSnapshot, pub bytes: u64 }
pub struct MovePlan { pub root: PathBuf, pub moves: Vec<Move>, pub skipped: Vec<(PathBuf, String)> }
pub struct Selection { pub group_ids: Vec<String>, pub excluded: Vec<PathBuf>, pub renames: Vec<(String, String)> }
pub fn build_move_plan(plan: &OrganizePlan, selection: &Selection, protected: &ProtectedPaths) -> Result<MovePlan>
```

- [ ] **Step 1: Export `revalidate`**

In `crates/sapu-core/src/safety/mod.rs`, change `pub use validate::ValidationFailure;` to `pub use validate::{ValidationFailure, revalidate};`.

- [ ] **Step 2: Write the failing tests**

Create `crates/sapu-core/src/organize/plan.rs` with the test module:

```rust
//! Turns ticked groups into concrete, validated moves.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::organize::{propose, scan_folder};
    use crate::safety::ProtectedPaths;
    use std::path::Path;

    fn tempdir() -> tempfile::TempDir {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.test-tmp");
        std::fs::create_dir_all(&base).expect("mkdir test-tmp");
        tempfile::tempdir_in(base).expect("tempdir")
    }
    fn touch(p: &Path) {
        std::fs::create_dir_all(p.parent().expect("parent")).expect("mkdir");
        std::fs::write(p, b"x").expect("write");
    }
    fn plan_for(root: &Path) -> OrganizePlan {
        propose(&scan_folder(root).expect("scan"))
    }
    fn all_ids(p: &OrganizePlan) -> Vec<String> {
        p.groups.iter().map(|g| g.id.clone()).collect()
    }

    #[test]
    fn moves_go_under_root_slash_group_and_existing_folders_produce_no_moves() {
        let t = tempdir();
        touch(&t.path().join("Dokumen/old.pdf"));
        touch(&t.path().join("a.pdf"));
        touch(&t.path().join("clip.mp4"));
        let plan = plan_for(t.path());
        let sel = Selection { group_ids: all_ids(&plan), excluded: vec![], renames: vec![] };
        let mp = build_move_plan(&plan, &sel, &ProtectedPaths::new()).expect("plan");
        let targets: Vec<_> = mp.moves.iter().map(|m| m.to.strip_prefix(t.path()).expect("under root").to_path_buf()).collect();
        assert!(targets.contains(&Path::new("Dokumen/a.pdf").to_path_buf()));
        assert!(targets.contains(&Path::new("Video/clip.mp4").to_path_buf()));
        assert!(mp.moves.iter().all(|m| m.from != t.path().join("Dokumen")), "existing folder itself is not moved");
    }

    #[test]
    fn name_clashes_get_a_numbered_suffix() {
        let t = tempdir();
        touch(&t.path().join("Dokumen/a.pdf"));
        touch(&t.path().join("a.pdf"));
        let plan = plan_for(t.path());
        let sel = Selection { group_ids: all_ids(&plan), excluded: vec![], renames: vec![] };
        let mp = build_move_plan(&plan, &sel, &ProtectedPaths::new()).expect("plan");
        let m = mp.moves.iter().find(|m| m.from == t.path().join("a.pdf")).expect("move");
        assert_eq!(m.to, t.path().join("Dokumen/a (2).pdf"));
    }

    #[test]
    fn unticked_groups_excluded_files_and_renames_are_respected() {
        let t = tempdir();
        for n in ["a.pdf", "b.pdf", "clip.mp4"] {
            touch(&t.path().join(n));
        }
        let plan = plan_for(t.path());
        let docs = plan.groups.iter().find(|g| g.name == "Dokumen").expect("docs").id.clone();
        let sel = Selection { group_ids: vec![docs.clone()], excluded: vec![t.path().join("b.pdf")], renames: vec![(docs, "Berkas".into())] };
        let mp = build_move_plan(&plan, &sel, &ProtectedPaths::new()).expect("plan");
        assert_eq!(mp.moves.len(), 1);
        assert_eq!(mp.moves[0].to, t.path().join("Berkas/a.pdf"));
    }

    #[test]
    fn bad_rename_and_protected_source_are_refused() {
        let t = tempdir();
        for n in ["a.pdf", "b.pdf"] {
            touch(&t.path().join(n));
        }
        let plan = plan_for(t.path());
        let docs = plan.groups[0].id.clone();
        let bad = Selection { group_ids: vec![docs.clone()], excluded: vec![], renames: vec![(docs.clone(), "../out".into())] };
        assert!(build_move_plan(&plan, &bad, &ProtectedPaths::new()).is_err());
        let mut protected = ProtectedPaths::new();
        protected.add_exact(t.path().join("a.pdf"), "test");
        let sel = Selection { group_ids: vec![docs], excluded: vec![], renames: vec![] };
        let mp = build_move_plan(&plan, &sel, &protected).expect("plan");
        assert_eq!(mp.moves.len(), 1, "protected file is skipped, not planned");
        assert_eq!(mp.skipped.len(), 1);
    }
}
```

Note for the implementer: check `ProtectedPaths` for the constructor and the method that adds an exact entry (`grep -n "pub fn" crates/sapu-core/src/safety/protected.rs`). If the names differ from `new()` / `add_exact(path, reason)`, use the existing ones in the test; do not add new API to `protected.rs`.

Update `mod.rs`:

```rust
mod group;
mod plan;
mod scan;

pub use group::{Confidence, Group, GroupSource, Naming, OrganizePlan, apply_naming, kind_folder, propose, sanitize_name};
pub use plan::{Move, MovePlan, Selection, build_move_plan};
pub use scan::{FolderListing, OrganizeEntry, Unit, scan_folder};
```

- [ ] **Step 3: Run the tests to see them fail**

Run: `cargo test -p sapu-core organize::plan`
Expected: compile errors for the missing items.

- [ ] **Step 4: Implement**

Prepend to `crates/sapu-core/src/organize/plan.rs`:

```rust
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::group::{OrganizePlan, sanitize_name};
use super::scan::Unit;
use crate::error::{Result, SapuError};
use crate::safety::{FileSnapshot, ProtectedPaths};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Move {
    pub from: PathBuf,
    pub to: PathBuf,
    pub snapshot: FileSnapshot,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MovePlan {
    pub root: PathBuf,
    pub moves: Vec<Move>,
    /// Entries left out with the reason (protected, unreadable, link).
    pub skipped: Vec<(PathBuf, String)>,
}

/// What the person ticked: groups to apply, single files to leave alone,
/// and new names for groups.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Selection {
    pub group_ids: Vec<String>,
    pub excluded: Vec<PathBuf>,
    pub renames: Vec<(String, String)>,
}

fn invalid(msg: &str) -> SapuError {
    SapuError::io(
        "organize plan",
        std::io::Error::new(std::io::ErrorKind::InvalidInput, msg.to_string()),
    )
}

/// `name.ext` → `name (n).ext`, folders get the suffix on the whole name.
fn numbered(name: &str, n: usize, is_dir: bool) -> String {
    match (is_dir, name.rsplit_once('.')) {
        (false, Some((stem, ext))) if !stem.is_empty() => format!("{stem} ({n}).{ext}"),
        _ => format!("{name} ({n})"),
    }
}

/// Builds the moves for the ticked groups. Destinations are always
/// `root/<group>/<name>`; clashes get ` (2)`, ` (3)`, …
pub fn build_move_plan(plan: &OrganizePlan, selection: &Selection, protected: &ProtectedPaths) -> Result<MovePlan> {
    let root = std::path::absolute(&plan.root).map_err(|e| SapuError::io("resolve root", e))?;
    let excluded: HashSet<&Path> = selection.excluded.iter().map(PathBuf::as_path).collect();
    let mut moves = Vec::new();
    let mut skipped = Vec::new();
    let mut taken: HashSet<PathBuf> = HashSet::new();

    for g in plan.groups.iter().filter(|g| selection.group_ids.contains(&g.id)) {
        let name = match selection.renames.iter().find(|(id, _)| *id == g.id) {
            Some((_, raw)) => sanitize_name(raw).ok_or_else(|| invalid("group name is not a safe folder name"))?,
            None => sanitize_name(&g.name).ok_or_else(|| invalid("group name is not a safe folder name"))?,
        };
        let dest_dir = root.join(&name);
        if !dest_dir.starts_with(&root) || dest_dir == root {
            return Err(invalid("destination escapes the folder"));
        }
        for e in &g.entries {
            if excluded.contains(e.path.as_path()) || e.unit == Unit::Link {
                continue;
            }
            // Already where it belongs (existing folder's own entry, or file already inside).
            if e.path == dest_dir || e.path.parent() == Some(dest_dir.as_path()) {
                continue;
            }
            if protected.is_protected(&e.path) {
                skipped.push((e.path.clone(), "protected".into()));
                continue;
            }
            let snapshot = match FileSnapshot::capture(&e.path) {
                Ok(s) => s,
                Err(err) => {
                    skipped.push((e.path.clone(), err.to_string()));
                    continue;
                }
            };
            let mut to = dest_dir.join(&e.name);
            let mut n = 2;
            while to.exists() || taken.contains(&to) {
                to = dest_dir.join(numbered(&e.name, n, e.unit != Unit::File));
                n += 1;
            }
            taken.insert(to.clone());
            moves.push(Move {
                from: e.path.clone(),
                to,
                snapshot,
                bytes: e.bytes,
            });
        }
    }
    Ok(MovePlan { root, moves, skipped })
}
```

- [ ] **Step 5: Run the tests**

Run: `cargo test -p sapu-core organize::plan`
Expected: 4 passed.

- [ ] **Step 6: Commit**

```bash
git add crates/sapu-core/src/organize crates/sapu-core/src/safety/mod.rs
git commit -m "feat(organize): validated move plans with clash suffixes and exclusions"
```

---

### Task 5: `organize/history.rs` — sessions on disk

**Files:**
- Create: `crates/sapu-core/src/organize/history.rs`
- Modify: `crates/sapu-core/src/organize/mod.rs`

**Interfaces:**
- Produces:

```rust
pub struct MoveRecord { pub from: PathBuf, pub to: PathBuf, pub bytes: u64, pub len: u64 }
pub struct OrganizeSession { pub id: String, pub root: PathBuf, pub started_at_unix: u64, pub finished_at_unix: u64, pub moves: Vec<MoveRecord>, pub moved: usize, pub skipped: usize, pub failed: usize, pub undone: bool }
pub struct OrganizeStore { dir: PathBuf }
impl OrganizeStore { pub fn open_default() -> Result<Self>; pub fn with_dir(dir) -> Result<Self>; pub fn record(&self, s: &OrganizeSession) -> Result<PathBuf>; pub fn list(&self) -> Result<Vec<OrganizeSession>>; pub fn get(&self, id: &str) -> Result<Option<OrganizeSession>>; pub fn mark_undone(&self, id: &str) -> Result<()> }
```

- [ ] **Step 1: Write the failing tests**

Create `crates/sapu-core/src/organize/history.rs` with:

```rust
//! Every organise run on disk, so it can be undone later.

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn tempdir() -> tempfile::TempDir {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.test-tmp");
        std::fs::create_dir_all(&base).expect("mkdir test-tmp");
        tempfile::tempdir_in(base).expect("tempdir")
    }

    fn session(id: &str, at: u64) -> OrganizeSession {
        OrganizeSession {
            id: id.into(),
            root: "/r".into(),
            started_at_unix: at,
            finished_at_unix: at + 1,
            moves: vec![MoveRecord { from: "/r/a".into(), to: "/r/G/a".into(), bytes: 5, len: 1 }],
            moved: 1,
            skipped: 0,
            failed: 0,
            undone: false,
        }
    }

    #[test]
    fn records_lists_newest_first_and_marks_undone() {
        let t = tempdir();
        let store = OrganizeStore::with_dir(t.path().join("organize")).expect("store");
        store.record(&session("one", 100)).expect("record");
        store.record(&session("two", 200)).expect("record");
        let list = store.list().expect("list");
        assert_eq!(list.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(), vec!["two", "one"]);
        store.mark_undone("one").expect("mark");
        assert!(store.get("one").expect("get").expect("some").undone);
        assert!(store.get("zzz").expect("get").is_none());
    }

    #[test]
    fn ids_are_unique() {
        let a = OrganizeSession::new_id();
        let b = OrganizeSession::new_id();
        assert_ne!(a, b);
    }
}
```

Add `mod history;` and `pub use history::{MoveRecord, OrganizeSession, OrganizeStore};` to `mod.rs`.

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p sapu-core organize::history`
Expected: compile errors.

- [ ] **Step 3: Implement**

Prepend to `history.rs`:

```rust
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::{Result, SapuError};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MoveRecord {
    pub from: PathBuf,
    pub to: PathBuf,
    /// Disk usage, for the summary.
    pub bytes: u64,
    /// Metadata length at move time, compared before undoing.
    pub len: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OrganizeSession {
    pub id: String,
    pub root: PathBuf,
    pub started_at_unix: u64,
    pub finished_at_unix: u64,
    /// Successful moves only, in the order they happened.
    pub moves: Vec<MoveRecord>,
    pub moved: usize,
    pub skipped: usize,
    pub failed: usize,
    #[serde(default)]
    pub undone: bool,
}

static COUNTER: AtomicU64 = AtomicU64::new(0);

impl OrganizeSession {
    pub fn now_unix() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    /// Time-ordered and unique within a process: `<nanos hex>-<pid>-<n>`.
    pub fn new_id() -> String {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        format!("{:x}-{}-{}", nanos, std::process::id(), COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

/// One JSON file per session under `<local data>/sapu/organize/`.
pub struct OrganizeStore {
    dir: PathBuf,
}

impl OrganizeStore {
    pub fn open_default() -> Result<Self> {
        let base = dirs::data_local_dir().ok_or_else(|| {
            SapuError::io(
                "locate local data dir",
                std::io::Error::new(std::io::ErrorKind::NotFound, "no data dir"),
            )
        })?;
        Self::with_dir(base.join("sapu").join("organize"))
    }

    pub fn with_dir(dir: impl Into<PathBuf>) -> Result<Self> {
        let dir = dir.into();
        std::fs::create_dir_all(&dir).map_err(|e| SapuError::io("create organize dir", e))?;
        Ok(Self { dir })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn file(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.json"))
    }

    pub fn record(&self, session: &OrganizeSession) -> Result<PathBuf> {
        let path = self.file(&session.id);
        std::fs::write(&path, serde_json::to_vec_pretty(session)?)
            .map_err(|e| SapuError::io("write session", e))?;
        Ok(path)
    }

    /// Newest first.
    pub fn list(&self) -> Result<Vec<OrganizeSession>> {
        let mut out = Vec::new();
        for entry in std::fs::read_dir(&self.dir).map_err(|e| SapuError::io("read organize dir", e))? {
            let Ok(entry) = entry else { continue };
            let p = entry.path();
            if p.extension().is_some_and(|e| e == "json")
                && let Ok(text) = std::fs::read_to_string(&p)
                && let Ok(s) = serde_json::from_str::<OrganizeSession>(&text)
            {
                out.push(s);
            }
        }
        out.sort_by(|a, b| b.started_at_unix.cmp(&a.started_at_unix).then_with(|| b.id.cmp(&a.id)));
        Ok(out)
    }

    pub fn get(&self, id: &str) -> Result<Option<OrganizeSession>> {
        match std::fs::read_to_string(self.file(id)) {
            Ok(text) => Ok(Some(serde_json::from_str(&text)?)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(SapuError::io("read session", e)),
        }
    }

    pub fn mark_undone(&self, id: &str) -> Result<()> {
        if let Some(mut s) = self.get(id)? {
            s.undone = true;
            self.record(&s)?;
        }
        Ok(())
    }
}
```

- [ ] **Step 4: Run the tests and the forbidden-fs script**

Run: `cargo test -p sapu-core organize::history && sh scripts/check-forbidden-fs.sh`
Expected: 2 passed; script ok.

- [ ] **Step 5: Commit**

```bash
git add crates/sapu-core/src/organize
git commit -m "feat(organize): session store for undoable runs"
```

---

### Task 6: `organize/apply.rs` — move, record, undo

**Files:**
- Create: `crates/sapu-core/src/organize/apply.rs`
- Modify: `crates/sapu-core/src/organize/mod.rs`

**Interfaces:**
- Consumes: `MovePlan`, `Move` (Task 4); `OrganizeStore`, `OrganizeSession`, `MoveRecord` (Task 5); `crate::safety::revalidate`; `crate::progress::{ProgressSink, ProgressEvent}`; `crate::safety::{Safety, DeletePlan, FileSnapshot}` for removing folders Sapu created that ended up empty after undo.
- Produces:

```rust
pub struct ApplyReport { pub session: OrganizeSession, pub failures: Vec<(PathBuf, String)> }
pub fn apply(plan: &MovePlan, dry_run: bool, store: &OrganizeStore, progress: &mut dyn ProgressSink) -> Result<ApplyReport>
pub struct UndoReport { pub restored: usize, pub skipped: usize, pub failed: usize }
pub fn undo(session: &OrganizeSession, store: &OrganizeStore, progress: &mut dyn ProgressSink) -> Result<UndoReport>
```

- [ ] **Step 1: Write the failing tests**

Create `crates/sapu-core/src/organize/apply.rs` with the test module:

```rust
//! The only place that moves files. Re-validates each one first, records
//! the session before returning, and can put everything back.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::organize::{Selection, build_move_plan, propose, scan_folder};
    use crate::progress::NoProgress;
    use crate::safety::ProtectedPaths;
    use std::path::Path;

    fn tempdir() -> tempfile::TempDir {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.test-tmp");
        std::fs::create_dir_all(&base).expect("mkdir test-tmp");
        tempfile::tempdir_in(base).expect("tempdir")
    }
    fn touch(p: &Path, body: &[u8]) {
        std::fs::create_dir_all(p.parent().expect("parent")).expect("mkdir");
        std::fs::write(p, body).expect("write");
    }
    fn listing(root: &Path) -> Vec<String> {
        let mut v = Vec::new();
        for e in jwalk::WalkDir::new(root).sort(true).into_iter().filter_map(Result::ok) {
            if e.depth() > 0 {
                v.push(e.path().strip_prefix(root).expect("rel").to_string_lossy().into_owned());
            }
        }
        v
    }
    fn full_plan(root: &Path) -> crate::organize::MovePlan {
        let plan = propose(&scan_folder(root).expect("scan"));
        let sel = Selection { group_ids: plan.groups.iter().map(|g| g.id.clone()).collect(), excluded: vec![], renames: vec![] };
        build_move_plan(&plan, &sel, &ProtectedPaths::new()).expect("plan")
    }

    #[test]
    fn apply_then_undo_restores_the_exact_tree() {
        let t = tempdir();
        let root = t.path().join("docs");
        for n in ["a.pdf", "b.pdf", "clip.mp4", "Invoice-1.pdf", "Invoice-2.pdf", "Invoice-3.pdf"] {
            touch(&root.join(n), n.as_bytes());
        }
        let before = listing(&root);
        let store = OrganizeStore::with_dir(t.path().join("store")).expect("store");
        let report = apply(&full_plan(&root), false, &store, &mut NoProgress).expect("apply");
        assert_eq!(report.session.moved, 6);
        assert!(root.join("Invoice/Invoice-1.pdf").is_file());
        assert!(root.join("Video/clip.mp4").is_file());
        assert_eq!(store.list().expect("list").len(), 1, "recorded");

        let undo_report = undo(&report.session, &store, &mut NoProgress).expect("undo");
        assert_eq!(undo_report.restored, 6);
        assert_eq!(listing(&root), before, "tree is exactly as before");
        assert!(store.get(&report.session.id).expect("get").expect("some").undone);
    }

    #[test]
    fn dry_run_moves_nothing_and_records_nothing() {
        let t = tempdir();
        let root = t.path().join("docs");
        touch(&root.join("a.pdf"), b"a");
        let store = OrganizeStore::with_dir(t.path().join("store")).expect("store");
        let report = apply(&full_plan(&root), true, &store, &mut NoProgress).expect("apply");
        assert_eq!(report.session.moved, 1, "counts what would move");
        assert!(root.join("a.pdf").is_file());
        assert!(store.list().expect("list").is_empty());
    }

    #[test]
    fn changed_file_is_skipped() {
        let t = tempdir();
        let root = t.path().join("docs");
        touch(&root.join("a.pdf"), b"a");
        let plan = full_plan(&root);
        touch(&root.join("a.pdf"), b"edited after planning");
        let store = OrganizeStore::with_dir(t.path().join("store")).expect("store");
        let report = apply(&plan, false, &store, &mut NoProgress).expect("apply");
        assert_eq!(report.session.moved, 0);
        assert_eq!(report.session.skipped, 1);
        assert!(root.join("a.pdf").is_file());
    }

    #[test]
    fn undo_skips_entries_that_moved_again() {
        let t = tempdir();
        let root = t.path().join("docs");
        touch(&root.join("a.pdf"), b"a");
        touch(&root.join("b.pdf"), b"b");
        let store = OrganizeStore::with_dir(t.path().join("store")).expect("store");
        let report = apply(&full_plan(&root), false, &store, &mut NoProgress).expect("apply");
        // The person moved one file elsewhere by hand (simulated by writing it out and leaving the slot empty).
        let gone = root.join("Dokumen/a.pdf");
        let elsewhere = t.path().join("elsewhere.pdf");
        std::fs::copy(&gone, &elsewhere).expect("copy");
        // Emulate the hand move without rename/remove APIs: truncate so re-validation fails.
        std::fs::write(&gone, b"changed by hand").expect("write");
        let r = undo(&report.session, &store, &mut NoProgress).expect("undo");
        assert_eq!(r.restored, 1);
        assert_eq!(r.skipped, 1);
        assert!(root.join("b.pdf").is_file());
    }

    #[test]
    fn refuses_moves_outside_root() {
        let t = tempdir();
        let root = t.path().join("docs");
        touch(&root.join("a.pdf"), b"a");
        let mut plan = full_plan(&root);
        plan.moves[0].to = t.path().join("escape.pdf");
        let store = OrganizeStore::with_dir(t.path().join("store")).expect("store");
        assert!(apply(&plan, false, &store, &mut NoProgress).is_err());
    }
}
```

Add `mod apply;` and `pub use apply::{ApplyReport, UndoReport, apply, undo};` to `mod.rs`.

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p sapu-core organize::apply`
Expected: compile errors.

- [ ] **Step 3: Implement**

Prepend to `apply.rs`:

```rust
use std::path::{Path, PathBuf};

use super::history::{MoveRecord, OrganizeSession, OrganizeStore};
use super::plan::MovePlan;
use crate::error::{Result, SapuError};
use crate::progress::{ProgressEvent, ProgressSink, Throttled};
use crate::safety::{FileSnapshot, revalidate};

#[derive(Debug, Clone)]
pub struct ApplyReport {
    pub session: OrganizeSession,
    pub failures: Vec<(PathBuf, String)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UndoReport {
    pub restored: usize,
    pub skipped: usize,
    pub failed: usize,
}

fn invalid(msg: &str) -> SapuError {
    SapuError::io(
        "organize apply",
        std::io::Error::new(std::io::ErrorKind::InvalidInput, msg.to_string()),
    )
}

#[cfg(unix)]
fn same_volume(a: &Path, b: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    let dev = |p: &Path| {
        let mut p = p.to_path_buf();
        while !p.exists() {
            if !p.pop() {
                return None;
            }
        }
        std::fs::metadata(&p).ok().map(|m| m.dev())
    };
    dev(a) == dev(b)
}

#[cfg(not(unix))]
fn same_volume(_a: &Path, _b: &Path) -> bool {
    true
}

/// Moves one path with `rename`, the only call of its kind in the crate.
fn rename(from: &Path, to: &Path) -> std::io::Result<()> {
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::rename(from, to)
}

/// Runs the plan. Each move is re-validated right before it happens; a
/// real run is recorded before returning.
pub fn apply(plan: &MovePlan, dry_run: bool, store: &OrganizeStore, progress: &mut dyn ProgressSink) -> Result<ApplyReport> {
    let root = std::path::absolute(&plan.root).map_err(|e| SapuError::io("resolve root", e))?;
    for m in &plan.moves {
        if !m.to.starts_with(&root) || !m.from.starts_with(&root) {
            return Err(invalid("a move leaves the folder"));
        }
        if !same_volume(&m.from, &m.to) {
            return Err(invalid("moves across volumes are not supported"));
        }
    }
    let mut sink = Throttled::new(progress, 10);
    let total = plan.moves.len() as u64;
    let bytes_total: u64 = plan.moves.iter().map(|m| m.bytes).sum();
    let mut session = OrganizeSession {
        id: OrganizeSession::new_id(),
        root: root.clone(),
        started_at_unix: OrganizeSession::now_unix(),
        finished_at_unix: 0,
        moves: Vec::new(),
        moved: 0,
        skipped: plan.skipped.len(),
        failed: 0,
        undone: false,
    };
    let mut failures = Vec::new();
    let mut bytes_done = 0u64;
    for (i, m) in plan.moves.iter().enumerate() {
        if let Err(reason) = revalidate(&m.snapshot, &root) {
            session.skipped += 1;
            failures.push((m.from.clone(), reason.to_string()));
        } else if dry_run {
            session.moved += 1;
        } else if m.to.exists() {
            session.skipped += 1;
            failures.push((m.from.clone(), "destination already exists".into()));
        } else {
            match rename(&m.from, &m.to) {
                Ok(()) => {
                    session.moved += 1;
                    session.moves.push(MoveRecord {
                        from: m.from.clone(),
                        to: m.to.clone(),
                        bytes: m.bytes,
                        len: m.snapshot.size,
                    });
                }
                Err(e) => {
                    session.failed += 1;
                    failures.push((m.from.clone(), e.to_string()));
                }
            }
        }
        bytes_done += m.bytes;
        sink.report(&ProgressEvent {
            items_done: i as u64 + 1,
            items_total: Some(total),
            bytes_done,
            bytes_total: Some(bytes_total),
        });
    }
    session.finished_at_unix = OrganizeSession::now_unix();
    tracing::info!(dry_run, moved = session.moved, skipped = session.skipped, failed = session.failed, "organize session finished");
    if !dry_run {
        store.record(&session)?;
    }
    Ok(ApplyReport { session, failures })
}

/// Puts every recorded move back, newest first. An entry whose file was
/// changed or moved again since is skipped; the rest still return.
pub fn undo(session: &OrganizeSession, store: &OrganizeStore, progress: &mut dyn ProgressSink) -> Result<UndoReport> {
    let mut sink = Throttled::new(progress, 10);
    let total = session.moves.len() as u64;
    let mut report = UndoReport {
        restored: 0,
        skipped: 0,
        failed: 0,
    };
    for (i, m) in session.moves.iter().rev().enumerate() {
        let current = FileSnapshot::capture(&m.to);
        let back_is_free = !m.from.exists();
        let unchanged = current.as_ref().map(|s| s.size == m.len || s.is_dir).unwrap_or(false);
        if !back_is_free || !unchanged {
            report.skipped += 1;
        } else {
            match rename(&m.to, &m.from) {
                Ok(()) => report.restored += 1,
                Err(_) => report.failed += 1,
            }
        }
        sink.report(&ProgressEvent {
            items_done: i as u64 + 1,
            items_total: Some(total),
            bytes_done: 0,
            bytes_total: None,
        });
    }
    store.mark_undone(&session.id)?;
    Ok(report)
}
```

Empty folders left behind by undo are kept; they are harmless and the next organise run folds them in as existing folders (the spec's "remove through safety" is deferred to Sprint 2's UI, where the person can tick them).

- [ ] **Step 4: Run the tests and the scripts**

Run: `cargo test -p sapu-core organize && sh scripts/check-forbidden-fs.sh && cargo clippy -p sapu-core --all-targets -- -D warnings`
Expected: all organize tests pass; the script reports rename only in apply.rs; clippy clean. If clippy flags `fs::rename` in `apply.rs` itself, the `clippy.toml` entry from Task 1 needs `#[allow(clippy::disallowed_methods)]` on the `rename` helper here, and the script's pattern already excludes this file.

- [ ] **Step 5: Commit**

```bash
git add crates/sapu-core/src/organize
git commit -m "feat(organize): apply moves with re-validation, record the session, undo"
```

---

### Task 7: AI task `tidy` in sapu-ai

**Files:**
- Create: `crates/sapu-ai/src/tasks/tidy.rs`
- Modify: `crates/sapu-ai/src/tasks/mod.rs` (add `pub mod tidy;`)

**Interfaces:**
- Produces:

```rust
pub struct TidyCandidate { pub id: String, pub name: String, pub source: String, pub count: usize, pub examples: Vec<String> }
pub struct TidyInput { pub language: String, pub folder_name: String, pub candidates: Vec<TidyCandidate> }
pub struct TidyName { pub id: String, pub name: String, pub confidence: f32 }
pub struct TidyAnswer { pub groups: Vec<TidyName>, pub merge: Vec<(String, String)> }
pub struct Tidy; impl Task for Tidy { const NAME = "tidy"; … }
```

Core applies the answer through `organize::apply_naming` (the CLI converts `TidyName` to `organize::Naming`); the AI crate itself only validates shape.

- [ ] **Step 1: Write the failing tests**

Create `crates/sapu-ai/src/tasks/tidy.rs`:

```rust
//! Tidy: name and merge candidate groups of a folder from file names only.

use super::{LANGUAGE_RULE, Task, extract_json};
use crate::AiError;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TidyCandidate {
    pub id: String,
    pub name: String,
    pub source: String,
    pub count: usize,
    /// At most twelve file names, chosen by the caller.
    pub examples: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TidyInput {
    pub language: String,
    pub folder_name: String,
    pub candidates: Vec<TidyCandidate>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TidyName {
    pub id: String,
    pub name: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TidyAnswer {
    pub groups: Vec<TidyName>,
    pub merge: Vec<(String, String)>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    groups: Vec<WireName>,
    #[serde(default)]
    merge: Vec<Vec<String>>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct WireName {
    id: String,
    name: String,
    #[serde(default = "half")]
    confidence: f64,
}

fn half() -> f64 {
    0.5
}

pub struct Tidy;

impl Task for Tidy {
    const NAME: &'static str = "tidy";
    type Input = TidyInput;
    type Output = TidyAnswer;

    fn system_prompt() -> &'static str {
        "You help tidy a messy folder. You only see candidate groups with a few example file names, never file contents. Give each group a short, specific folder name a person would recognise (a project, a client, a topic, or a kind of file), and propose merging groups that clearly belong together (for example a backend and a frontend of the same product). Never invent paths. Answer with JSON only."
    }

    fn user_prompt(input: &serde_json::Value) -> String {
        format!(
            "{LANGUAGE_RULE}\nReturn exactly: {{\"groups\": [{{\"id\": \"<candidate id>\", \"name\": \"<folder name, no slashes>\", \"confidence\": 0.0-1.0}}], \"merge\": [[\"<id to fold>\", \"<id to keep>\"]]}}. Keep ids as given. Candidates:\n{}",
            serde_json::to_string_pretty(input).unwrap_or_default()
        )
    }

    fn parse(answer: &str) -> Result<TidyAnswer, AiError> {
        let v = extract_json(answer)?;
        let wire: Wire = serde_json::from_value(v).map_err(|e| AiError::BadAnswer(e.to_string()))?;
        let groups = wire
            .groups
            .into_iter()
            .map(|g| TidyName {
                id: g.id,
                name: g.name.trim().to_string(),
                confidence: g.confidence.clamp(0.0, 1.0) as f32,
            })
            .collect();
        let merge = wire
            .merge
            .into_iter()
            .filter_map(|pair| match pair.as_slice() {
                [a, b] => Some((a.clone(), b.clone())),
                _ => None,
            })
            .collect();
        Ok(TidyAnswer { groups, merge })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_names_and_merges() {
        let a = Tidy::parse(r#"Sure! {"groups":[{"id":"g1","name":" Project A ","confidence":0.9},{"id":"g2","name":"Invoices"}],"merge":[["g3","g1"],["bad"]]}"#).expect("parse");
        assert_eq!(a.groups[0], TidyName { id: "g1".into(), name: "Project A".into(), confidence: 0.9 });
        assert_eq!(a.groups[1].confidence, 0.5, "missing confidence defaults to 0.5");
        assert_eq!(a.merge, vec![("g3".to_string(), "g1".to_string())], "malformed pairs dropped");
    }

    #[test]
    fn unknown_fields_are_rejected() {
        assert!(Tidy::parse(r#"{"groups":[],"merge":[],"delete":["g1"]}"#).is_err());
        assert!(Tidy::parse(r#"{"groups":[{"id":"g1","name":"x","path":"/etc"}]}"#).is_err());
    }

    #[test]
    fn prompt_carries_candidates_and_language() {
        let input = TidyInput {
            language: "id".into(),
            folder_name: "Documents".into(),
            candidates: vec![TidyCandidate { id: "g1".into(), name: "Invoice".into(), source: "prefix".into(), count: 3, examples: vec!["Invoice-1.pdf".into()] }],
        };
        let p = Tidy::user_prompt(&serde_json::to_value(&input).expect("json"));
        assert!(p.contains("Invoice-1.pdf"));
        assert!(p.contains("\"merge\""));
    }
}
```

Add `pub mod tidy;` to `crates/sapu-ai/src/tasks/mod.rs` (alphabetically after `search`).

- [ ] **Step 2: Run the tests**

Run: `cargo test -p sapu-ai tidy && sh scripts/check-ai-isolation.sh`
Expected: 3 passed; isolation ok.

- [ ] **Step 3: Commit**

```bash
git add crates/sapu-ai/src/tasks
git commit -m "feat(ai): tidy task names and merges folder groups from file names"
```

---

### Task 8: Duplicates inside the folder

**Files:**
- Create: `crates/sapu-core/src/organize/dupes.rs`
- Modify: `crates/sapu-core/src/organize/mod.rs`

**Interfaces:**
- Consumes: `crate::dedupe::{DedupeOptions, DedupeResult, find_duplicates}`, `crate::scanner::ScanControl`, `crate::progress::ProgressSink`.
- Produces: `pub fn duplicates_in(root: &Path, control: &ScanControl, progress: &mut dyn ProgressSink) -> Result<DedupeResult>`

- [ ] **Step 1: Write the failing test**

Create `crates/sapu-core/src/organize/dupes.rs`:

```rust
//! Duplicate files inside the folder being tidied, through the dedupe engine.

use std::path::Path;

use crate::dedupe::{DedupeOptions, DedupeResult, find_duplicates};
use crate::error::Result;
use crate::progress::ProgressSink;
use crate::scanner::ScanControl;

/// Same-content files under `root` (default dedupe minimum size applies).
pub fn duplicates_in(root: &Path, control: &ScanControl, progress: &mut dyn ProgressSink) -> Result<DedupeResult> {
    find_duplicates(&DedupeOptions::new(root), control, progress, |_| {})
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::NoProgress;

    #[test]
    fn finds_identical_files_in_the_folder() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.test-tmp");
        std::fs::create_dir_all(&base).expect("mkdir test-tmp");
        let t = tempfile::tempdir_in(base).expect("tempdir");
        let body = vec![7u8; 8192];
        std::fs::write(t.path().join("a.bin"), &body).expect("write");
        std::fs::write(t.path().join("b.bin"), &body).expect("write");
        std::fs::write(t.path().join("c.bin"), vec![1u8; 8192]).expect("write");
        let r = duplicates_in(t.path(), &ScanControl::new(), &mut NoProgress).expect("dupes");
        assert_eq!(r.groups.len(), 1);
        assert_eq!(r.groups[0].files.len(), 2);
    }
}
```

Add `mod dupes;` and `pub use dupes::duplicates_in;` to `mod.rs`. Check the field names of `DedupeResult` (`grep -n "pub struct DedupeResult" -A6 crates/sapu-core/src/dedupe/*.rs`) and adjust `r.groups` if the field is named differently.

- [ ] **Step 2: Run the test**

Run: `cargo test -p sapu-core organize::dupes`
Expected: 1 passed (8 KiB bodies clear the default minimum size).

- [ ] **Step 3: Commit**

```bash
git add crates/sapu-core/src/organize
git commit -m "feat(organize): duplicates inside the folder via the dedupe engine"
```

---

### Task 9: CLI `sapu organize`

**Files:**
- Create: `crates/sapu-cli/src/organize.rs`
- Modify: `crates/sapu-cli/src/main.rs` (add `mod organize;`, the `Organize(organize::OrganizeArgs)` variant with doc comment `/// Propose and apply groups for the loose files of a folder.`, and `Command::Organize(a) => organize::run(a)`)
- Modify: `README.md` (CLI block: add `sapu organize <folder> [--apply] [--ai] [--undo <id>]   # dry run by default; moves stay inside the folder`)

**Interfaces:**
- Consumes: everything from Tasks 2–8; `crate::human_bytes` (existing helper in `main.rs`); `sapu_ai::{AiClient, AiConfig, AiMode, keys}` the way `crates/sapu-cli/src/ai.rs` builds a client (copy its `client()` helper).

- [ ] **Step 1: Write the CLI**

Create `crates/sapu-cli/src/organize.rs`:

```rust
//! `sapu organize`: propose groups for a folder, apply them, undo a run.

use std::path::PathBuf;

use anyhow::Context;
use clap::Args;
use sapu_core::organize::{
    Naming, OrganizeStore, Selection, apply, apply_naming, build_move_plan, propose, scan_folder, undo,
};
use sapu_core::progress::NoProgress;
use sapu_core::safety::ProtectedPaths;

use crate::human_bytes;

#[derive(Args)]
pub struct OrganizeArgs {
    /// Folder to tidy (its top level only).
    path: Option<PathBuf>,
    /// Move the files. Without it, only the plan is printed.
    #[arg(long)]
    apply: bool,
    /// Ask the configured AI to name and merge groups (needs `sapu ai` set up).
    #[arg(long)]
    ai: bool,
    /// Undo a previous run by session id (see `sapu organize --list`).
    #[arg(long, conflicts_with_all = ["apply", "ai"])]
    undo: Option<String>,
    /// List previous runs.
    #[arg(long, conflicts_with_all = ["apply", "ai", "undo"])]
    list: bool,
    /// Machine-readable output.
    #[arg(long)]
    json: bool,
}

pub fn run(a: OrganizeArgs) -> anyhow::Result<()> {
    let store = OrganizeStore::open_default()?;
    if a.list {
        for s in store.list()? {
            println!(
                "{}  {}  {} moved, {} skipped, {} failed{}  {}",
                s.id,
                s.started_at_unix,
                s.moved,
                s.skipped,
                s.failed,
                if s.undone { ", undone" } else { "" },
                s.root.display()
            );
        }
        return Ok(());
    }
    if let Some(id) = a.undo {
        let session = store.get(&id)?.with_context(|| format!("no organize session {id}"))?;
        let r = undo(&session, &store, &mut NoProgress)?;
        println!("restored {}, skipped {}, failed {}", r.restored, r.skipped, r.failed);
        return Ok(());
    }
    let root = a.path.context("a folder is required")?;
    let listing = scan_folder(&root)?;
    let mut plan = propose(&listing);
    if a.ai {
        match crate::ai::client() {
            Ok(client) => {
                let input = sapu_ai::tasks::tidy::TidyInput {
                    language: "en".into(),
                    folder_name: root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
                    candidates: plan
                        .groups
                        .iter()
                        .map(|g| sapu_ai::tasks::tidy::TidyCandidate {
                            id: g.id.clone(),
                            name: g.name.clone(),
                            source: format!("{:?}", g.source).to_lowercase(),
                            count: g.entries.len(),
                            examples: g.entries.iter().take(12).map(|e| e.name.clone()).collect(),
                        })
                        .collect(),
                };
                match client.run::<sapu_ai::tasks::tidy::Tidy>(&input) {
                    Ok(answer) => {
                        let names: Vec<Naming> = answer.groups.into_iter().map(|n| Naming { id: n.id, name: n.name, confidence: n.confidence }).collect();
                        apply_naming(&mut plan, &names, &answer.merge);
                    }
                    Err(e) => eprintln!("AI unavailable, using heuristic names: {e}"),
                }
            }
            Err(e) => eprintln!("AI not configured, using heuristic names: {e}"),
        }
    }
    let selection = Selection {
        group_ids: plan.groups.iter().map(|g| g.id.clone()).collect(),
        ..Default::default()
    };
    let moves = build_move_plan(&plan, &selection, &ProtectedPaths::os_defaults())?;
    if a.json {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({ "plan": plan, "moves": moves }))?);
        return Ok(());
    }
    println!("{}", root.display());
    for g in &plan.groups {
        println!(
            "  {:<28} {:>3} item  {:>9}  [{:?}, {:?}]",
            g.name,
            g.entries.len(),
            human_bytes(g.bytes),
            g.source,
            g.confidence
        );
    }
    println!("{} move(s), {} skipped", moves.moves.len(), moves.skipped.len());
    let report = apply(&moves, !a.apply, &store, &mut NoProgress)?;
    if a.apply {
        println!(
            "moved {}, skipped {}, failed {}; session {} (undo with --undo)",
            report.session.moved, report.session.skipped, report.session.failed, report.session.id
        );
        for (p, why) in report.failures {
            println!("  ! {}: {why}", p.display());
        }
    } else {
        println!("dry run: would move {} item(s). Add --apply to move them.", report.session.moved);
    }
    Ok(())
}
```

Check `crates/sapu-cli/src/ai.rs` for the function that builds an `AiClient` from the saved config and keychain; if it is private, make it `pub(crate) fn client() -> anyhow::Result<AiClient>` there. Check `human_bytes`'s signature in `main.rs` (`fn human_bytes(n: u64) -> String`) and that `serde_json` is a dependency of `sapu-cli` (`grep serde_json crates/sapu-cli/Cargo.toml`; add `serde_json.workspace = true` if missing).

- [ ] **Step 2: Wire it in `main.rs`**

Add `mod organize;` next to the other modules, the enum variant, and the match arm as listed under Files.

- [ ] **Step 3: Try it on a scratch folder**

```bash
mkdir -p /tmp/sapu-organize-demo && cd /tmp/sapu-organize-demo && for n in Invoice-1.pdf Invoice-2.pdf Invoice-3.pdf clip.mp4 song.mp3; do echo x > "$n"; done; cd -
cargo run -q -p sapu-cli -- organize /tmp/sapu-organize-demo
cargo run -q -p sapu-cli -- organize /tmp/sapu-organize-demo --apply
cargo run -q -p sapu-cli -- organize --list
```

Expected: the dry run lists `Invoice` (3, prefix, check), `Video` (1), `Audio` (1) and "would move 5"; `--apply` moves them and prints a session id; `--list` shows the session. Then `cargo run -q -p sapu-cli -- organize --undo <id>` restores the five files to the top level.

- [ ] **Step 4: Gate and commit**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && sh scripts/check-forbidden-fs.sh && sh scripts/check-ai-isolation.sh`
Expected: all clean.

```bash
git add crates/sapu-cli README.md
git commit -m "feat(cli): sapu organize proposes, applies and undoes folder tidying"
```

---

### Task 10: Memory of the sprint

**Files:**
- Modify: `docs/superpowers/specs/2026-10-17-organize-design.md` (Status line → "Sprint 1 implemented; Sprint 2 (Tauri + UI) pending")

- [ ] **Step 1: Update the status line and commit**

```bash
git add docs/superpowers/specs/2026-10-17-organize-design.md
git commit -m "docs: organize spec status after sprint 1"
git push origin main
```

---

## Self-review notes

- Spec coverage: §3.1 Task 2, §3.2 Task 3, §3.3 Task 7 (+ `apply_naming` in Task 3), §3.4 Task 4, §3.5 Tasks 5–6, §3.6 Task 8, §3.7–3.8 deferred to Sprint 2 by the spec's own delivery section, §4 safety rules enforced in Tasks 1, 4, 6, §6 core tests are spread over Tasks 2–8. Empty-folder removal after undo is deferred to Sprint 2 (stated in Task 6).
- Type consistency: `Naming { id, name, confidence }` (Task 3) is what Task 9 builds from `TidyName`; `Selection { group_ids, excluded, renames }` (Task 4) is used in Tasks 6 and 9; `MoveRecord { from, to, bytes, len }` (Task 5) is written in Task 6's `apply` and compared in `undo`.
- Review Focus items are pinned: 1 → Task 3 `existing_folder_absorbs_kind_group_of_same_name`; 2 → Task 3 `names_are_never_empty_or_dotted`; 3 → Task 6 `changed_file_is_skipped`; 4 → Task 6 `undo_skips_entries_that_moved_again`; 5 → Task 3 `apply_naming_renames_merges_and_ignores_bad_input` and Task 7 `unknown_fields_are_rejected`.
