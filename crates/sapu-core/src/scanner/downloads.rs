//! Downloads organizer: the top level of the Downloads folder grouped by
//! what the files are (installers, archives, media, documents, folders),
//! independent of a full scan so it is instant and always available.
//! Listing only; removal goes through `safety`.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::FileKind;
use crate::error::{Result, SapuError};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum DownloadGroup {
    /// dmg, pkg, exe, msi, deb, apk… usually useless once installed.
    Installers,
    Archives,
    Video,
    Audio,
    Images,
    Documents,
    Folders,
    Other,
}

impl DownloadGroup {
    pub const ALL: [DownloadGroup; 8] = [
        Self::Installers,
        Self::Archives,
        Self::Video,
        Self::Audio,
        Self::Images,
        Self::Documents,
        Self::Folders,
        Self::Other,
    ];

    fn of(kind: FileKind, is_dir: bool) -> Self {
        if is_dir {
            return Self::Folders;
        }
        match kind {
            FileKind::Installer | FileKind::DiskImage => Self::Installers,
            FileKind::Archive => Self::Archives,
            FileKind::Video => Self::Video,
            FileKind::Audio => Self::Audio,
            FileKind::Image => Self::Images,
            FileKind::Pdf | FileKind::Document | FileKind::Spreadsheet | FileKind::Presentation => {
                Self::Documents
            }
            _ => Self::Other,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DownloadEntry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub bytes: u64,
    pub kind: FileKind,
    pub group: DownloadGroup,
    pub mtime: Option<SystemTime>,
    pub atime: Option<SystemTime>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DownloadGroupSummary {
    pub group: DownloadGroup,
    pub bytes: u64,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DownloadsReport {
    pub dir: PathBuf,
    pub bytes: u64,
    pub count: usize,
    /// Non-empty groups, largest first.
    pub groups: Vec<DownloadGroupSummary>,
    /// Every top-level entry, largest first.
    pub entries: Vec<DownloadEntry>,
}

/// The user's Downloads folder, when it exists.
pub fn downloads_dir() -> Option<PathBuf> {
    dirs::download_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join("Downloads")))
        .filter(|p| p.is_dir())
}

/// Lists the top level of `dir`. Folders are sized by walking them (no
/// symlink following); hidden entries such as `.DS_Store` are skipped.
pub fn scan_downloads(dir: &Path) -> Result<DownloadsReport> {
    let mut entries = Vec::new();
    for e in std::fs::read_dir(dir).map_err(|e| SapuError::io("read Downloads", e))? {
        let Ok(e) = e else { continue };
        let path = e.path();
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        if meta.file_type().is_symlink() {
            continue;
        }
        let is_dir = meta.is_dir();
        let kind = if is_dir {
            if name.ends_with(".app") {
                FileKind::Application
            } else {
                FileKind::Folder
            }
        } else {
            FileKind::of(&path)
        };
        let bytes = if is_dir {
            crate::cleaner::dir_size_pub(&path)
        } else {
            crate::platform::disk_usage_of(&meta)
        };
        entries.push(DownloadEntry {
            group: DownloadGroup::of(kind, is_dir && kind != FileKind::Application),
            path,
            name,
            is_dir,
            bytes,
            kind,
            mtime: meta.modified().ok(),
            atime: meta.accessed().ok(),
        });
    }
    entries.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));
    let mut groups: Vec<DownloadGroupSummary> = DownloadGroup::ALL
        .iter()
        .map(|g| {
            let in_group = entries.iter().filter(|e| e.group == *g);
            DownloadGroupSummary {
                group: *g,
                bytes: in_group.clone().map(|e| e.bytes).sum(),
                count: in_group.count(),
            }
        })
        .filter(|g| g.count > 0)
        .collect();
    groups.sort_by_key(|g| std::cmp::Reverse(g.bytes));
    Ok(DownloadsReport {
        dir: dir.to_path_buf(),
        bytes: entries.iter().map(|e| e.bytes).sum(),
        count: entries.len(),
        groups,
        entries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_top_level_entries() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.test-tmp");
        std::fs::create_dir_all(&base).expect("mkdir test-tmp");
        let dir = tempfile::tempdir_in(&base).expect("tempdir");
        let tmp = dir.path().to_path_buf();
        std::fs::write(tmp.join("Claude.dmg"), vec![0u8; 4096]).expect("dmg");
        std::fs::write(tmp.join("photo.jpg"), vec![0u8; 8192]).expect("jpg");
        std::fs::write(tmp.join(".DS_Store"), b"x").expect("ds");
        std::fs::create_dir(tmp.join("project")).expect("dir");
        std::fs::write(tmp.join("project/a.txt"), vec![0u8; 4096]).expect("txt");
        let r = scan_downloads(&tmp).expect("report");
        assert_eq!(r.count, 3);
        assert_eq!(r.entries[0].name, "photo.jpg");
        let g: Vec<_> = r.groups.iter().map(|g| g.group).collect();
        assert!(g.contains(&DownloadGroup::Installers));
        assert!(g.contains(&DownloadGroup::Images));
        assert!(g.contains(&DownloadGroup::Folders));
        assert!(r.entries.iter().any(|e| e.is_dir && e.bytes >= 4096));
    }
}
