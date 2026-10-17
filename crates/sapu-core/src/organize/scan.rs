//! One folder level, classified for organising.

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
    /// Up to eight names inside a folder, project or bundle (never contents).
    #[serde(default)]
    pub children: Vec<String>,
    /// A better file name proposed by the AI (files only, sanitised).
    #[serde(default)]
    pub suggested_name: Option<String>,
    /// Why the AI thinks this file is junk (files only).
    #[serde(default)]
    pub junk: Option<String>,
}

const MAX_CHILDREN: usize = 8;

/// Office lock files, editor temp files and OS thumbnails: never moved.
fn is_temp_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    name.starts_with("~$")
        || name.starts_with(".~lock")
        || lower == "thumbs.db"
        || lower == "desktop.ini"
}

fn child_names(dir: &Path) -> Vec<String> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = rd
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| !n.starts_with('.') && !is_temp_name(n))
        .collect();
    names.sort_by_key(|n| n.to_lowercase());
    names.truncate(MAX_CHILDREN);
    names
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FolderListing {
    pub root: PathBuf,
    pub entries: Vec<OrganizeEntry>,
    pub unreadable: Vec<PathBuf>,
}

const BUNDLE_EXTENSIONS: &[&str] = &["app", "bundle", "framework", "photoslibrary", "pkg"];

/// Package directories macOS shows as one item.
pub(super) fn is_bundle_path(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .is_some_and(|e| BUNDLE_EXTENSIONS.contains(&e.as_str()))
}

/// Can files be placed inside `dir`? Only a plain folder that is not a
/// link, a project or a bundle. A missing path is fine (it will be created).
pub(super) fn plain_folder_or_absent(dir: &Path) -> bool {
    match std::fs::symlink_metadata(dir) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => true,
        Err(_) => false,
        Ok(meta) => {
            !meta.file_type().is_symlink()
                && meta.is_dir()
                && !is_bundle_path(dir)
                && !is_project_root(dir)
        }
    }
}

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
        if name.starts_with('.') || is_temp_name(&name) {
            continue;
        }
        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            unreadable.push(path);
            continue;
        };
        let unit = if meta.file_type().is_symlink() {
            Unit::Link
        } else if meta.is_dir() {
            if is_bundle_path(&path) {
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
        let children = match unit {
            Unit::Folder | Unit::Project | Unit::Bundle => child_names(&path),
            Unit::File | Unit::Link => Vec::new(),
        };
        entries.push(OrganizeEntry {
            path,
            name,
            unit,
            kind,
            bytes,
            mtime: meta.modified().ok(),
            children,
            suggested_name: None,
            junk: None,
        });
    }
    entries.sort_by_key(|e| e.name.to_lowercase());
    Ok(FolderListing {
        root,
        entries,
        unreadable,
    })
}

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
        std::os::unix::fs::symlink(r.join("Invoice-2024.pdf"), r.join("link.pdf"))
            .expect("symlink");

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
        let notes = listing
            .entries
            .iter()
            .find(|e| e.name == "notes")
            .expect("notes");
        let solo = listing
            .entries
            .iter()
            .find(|e| e.name == "solo.txt")
            .expect("solo");
        assert!(
            notes.bytes >= 2 * solo.bytes.min(1),
            "folder size covers its files"
        );
        assert!(solo.bytes > 0);
        assert_eq!(solo.kind, crate::scanner::FileKind::Document);
    }

    #[test]
    fn missing_root_is_an_error() {
        let t = tempdir();
        assert!(scan_folder(&t.path().join("nope")).is_err());
    }

    #[test]
    fn ignores_lock_and_temp_files_and_lists_folder_children() {
        let t = tempdir();
        let r = t.path();
        touch(&r.join("~$ RAMDANI BARU.docx"));
        touch(&r.join(".~lock.report.odt#"));
        touch(&r.join("Thumbs.db"));
        touch(&r.join("desktop.ini"));
        touch(&r.join("keep.pdf"));
        for i in 0..12 {
            touch(&r.join(format!("Saham/file{i:02}.xlsx")));
        }
        let listing = scan_folder(r).expect("scan");
        let names: Vec<_> = listing.entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["keep.pdf", "Saham"]);
        let saham = listing
            .entries
            .iter()
            .find(|e| e.name == "Saham")
            .expect("saham");
        assert_eq!(saham.children.len(), 8, "at most eight names");
        assert_eq!(saham.children[0], "file00.xlsx");
        assert!(
            listing.entries[0].children.is_empty(),
            "files have no children"
        );
    }
}
