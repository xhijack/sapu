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
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(str::to_ascii_lowercase);
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
}
