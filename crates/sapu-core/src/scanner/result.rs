//! Scan output and the queries the UI/CLI run over it.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use super::category::Category;
use super::kind::FileKind;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FileEntry {
    pub path: PathBuf,
    pub size: u64,
    pub mtime: Option<SystemTime>,
    /// Last access, when the filesystem records it.
    pub atime: Option<SystemTime>,
    pub category: Category,
    pub kind: FileKind,
}

/// One row of the folder browser: a direct child of a scanned folder.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BrowseEntry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub bytes: u64,
    pub files: u64,
    pub mtime: Option<SystemTime>,
    pub atime: Option<SystemTime>,
    pub kind: FileKind,
    pub category: Option<Category>,
}

/// The browser's view of one folder.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BrowseFolder {
    pub path: PathBuf,
    pub bytes: u64,
    pub files: u64,
    /// Direct children, largest first; at most the requested limit.
    pub entries: Vec<BrowseEntry>,
    /// Children beyond the limit, folded.
    pub more: usize,
}

/// Per-directory totals. `own_*` counts direct children only; `total_*`
/// includes every descendant.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DirStats {
    pub own_bytes: u64,
    pub own_files: u64,
    pub total_bytes: u64,
    pub total_files: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CategorySummary {
    pub category: Category,
    pub bytes: u64,
    pub files: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DirSummary {
    pub path: PathBuf,
    pub bytes: u64,
    pub files: u64,
}

/// A node of the folder tree for the treemap.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TreeNode {
    pub path: PathBuf,
    pub name: String,
    pub bytes: u64,
    pub files: u64,
    pub is_dir: bool,
    pub category: Option<Category>,
    pub children: Vec<TreeNode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgeFilter {
    #[default]
    Any,
    OlderThanMonth,
    OlderThanSixMonths,
    OlderThanYear,
}

impl AgeFilter {
    fn cutoff(self, now: SystemTime) -> Option<SystemTime> {
        const DAY: u64 = 86_400;
        let days = match self {
            Self::Any => return None,
            Self::OlderThanMonth => 30,
            Self::OlderThanSixMonths => 182,
            Self::OlderThanYear => 365,
        };
        now.checked_sub(Duration::from_secs(days * DAY))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct FileFilter {
    pub min_bytes: u64,
    pub category: Option<Category>,
    pub age: AgeFilter,
    /// Exact day bounds (AI search); combined with `age` when both are set.
    pub min_age_days: Option<u64>,
    pub max_age_days: Option<u64>,
    /// Lowercase, without dot. Empty = any.
    pub extensions: Vec<String>,
    /// Case-insensitive substring of the file name.
    pub name_contains: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScanResult {
    pub root: PathBuf,
    /// Change since the previous scan of this root, when one exists.
    pub diff: Option<super::stats_cache::ScanDiff>,
    /// Capacity of the volume holding `root`, when known.
    pub disk_total_bytes: Option<u64>,
    pub disk_free_bytes: Option<u64>,
    pub started_at_unix: u64,
    pub finished_at_unix: u64,
    pub cancelled: bool,
    pub total_bytes: u64,
    pub file_count: u64,
    pub dir_count: u64,
    pub denied_count: u64,
    /// First few unreadable directories, for the UI's explanation.
    pub denied_sample: Vec<PathBuf>,
    pub files: Vec<FileEntry>,
    pub dirs: HashMap<PathBuf, DirStats>,
}

impl ScanResult {
    /// Bytes and file counts per category, largest first; empty categories kept.
    pub fn categories(&self) -> Vec<CategorySummary> {
        let mut acc: HashMap<Category, (u64, u64)> = HashMap::new();
        for f in &self.files {
            let e = acc.entry(f.category).or_default();
            e.0 += f.size;
            e.1 += 1;
        }
        let mut out: Vec<_> = Category::ALL
            .iter()
            .map(|c| {
                let (bytes, files) = acc.get(c).copied().unwrap_or_default();
                CategorySummary {
                    category: *c,
                    bytes,
                    files,
                }
            })
            .collect();
        out.sort_by_key(|c| std::cmp::Reverse(c.bytes));
        out
    }

    pub fn top_files(&self, filter: &FileFilter, n: usize) -> Vec<FileEntry> {
        let now = SystemTime::now();
        let cutoff = filter.age.cutoff(now);
        let min_cut = filter
            .min_age_days
            .and_then(|d| now.checked_sub(Duration::from_secs(d * 86_400)));
        let max_cut = filter
            .max_age_days
            .and_then(|d| now.checked_sub(Duration::from_secs(d * 86_400)));
        let needle = filter
            .name_contains
            .as_ref()
            .map(|n| n.to_lowercase())
            .filter(|n| !n.is_empty());
        let mut v: Vec<&FileEntry> = self
            .files
            .iter()
            .filter(|f| f.size >= filter.min_bytes)
            .filter(|f| filter.category.is_none_or(|c| f.category == c))
            .filter(|f| cutoff.is_none_or(|cut| f.mtime.is_some_and(|m| m <= cut)))
            .filter(|f| min_cut.is_none_or(|cut| f.mtime.is_some_and(|m| m <= cut)))
            .filter(|f| max_cut.is_none_or(|cut| f.mtime.is_some_and(|m| m >= cut)))
            .filter(|f| {
                filter.extensions.is_empty()
                    || f.path
                        .extension()
                        .and_then(|e| e.to_str())
                        .is_some_and(|e| {
                            filter.extensions.iter().any(|x| x.eq_ignore_ascii_case(e))
                        })
            })
            .filter(|f| {
                needle.as_ref().is_none_or(|n| {
                    f.path
                        .file_name()
                        .and_then(|x| x.to_str())
                        .is_some_and(|x| x.to_lowercase().contains(n.as_str()))
                })
            })
            .collect();
        v.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.path.cmp(&b.path)));
        v.into_iter().take(n).cloned().collect()
    }

    /// Largest folders by total size, excluding the root itself.
    pub fn top_dirs(&self, n: usize) -> Vec<DirSummary> {
        let mut v: Vec<DirSummary> = self
            .dirs
            .iter()
            .filter(|(p, _)| p.as_path() != self.root.as_path())
            .map(|(p, s)| DirSummary {
                path: p.clone(),
                bytes: s.total_bytes,
                files: s.total_files,
            })
            .collect();
        v.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.path.cmp(&b.path)));
        v.truncate(n);
        v
    }

    /// Direct children of `path` for the folder browser, largest first.
    /// `None` when `path` was not part of this scan.
    pub fn browse(&self, path: &Path, limit: usize) -> Option<BrowseFolder> {
        let stats = self.dirs.get(path)?;
        let mut entries: Vec<BrowseEntry> = self
            .dirs
            .iter()
            .filter(|(p, _)| p.parent() == Some(path))
            .map(|(p, s)| BrowseEntry {
                path: p.clone(),
                name: node_name(p),
                is_dir: true,
                bytes: s.total_bytes,
                files: s.total_files,
                mtime: None,
                atime: None,
                kind: if p.extension().is_some_and(|e| e == "app") {
                    FileKind::Application
                } else {
                    FileKind::Folder
                },
                category: None,
            })
            .collect();
        entries.extend(
            self.files
                .iter()
                .filter(|f| f.path.parent() == Some(path))
                .map(|f| BrowseEntry {
                    path: f.path.clone(),
                    name: node_name(&f.path),
                    is_dir: false,
                    bytes: f.size,
                    files: 1,
                    mtime: f.mtime,
                    atime: f.atime,
                    kind: f.kind,
                    category: Some(f.category),
                }),
        );
        entries.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));
        let more = entries.len().saturating_sub(limit);
        entries.truncate(limit);
        Some(BrowseFolder {
            path: path.to_path_buf(),
            bytes: stats.total_bytes,
            files: stats.total_files,
            entries,
            more,
        })
    }

    /// Folder tree under `path`, `depth` levels deep, at most `limit` children
    /// per node (largest first; the rest folded into one "other" node).
    pub fn subtree(&self, path: &Path, depth: usize, limit: usize) -> Option<TreeNode> {
        let stats = self.dirs.get(path)?;
        let mut node = TreeNode {
            path: path.to_path_buf(),
            name: node_name(path),
            bytes: stats.total_bytes,
            files: stats.total_files,
            is_dir: true,
            category: None,
            children: Vec::new(),
        };
        if depth == 0 {
            return Some(node);
        }
        let mut children: Vec<TreeNode> = self
            .dirs
            .iter()
            .filter(|(p, _)| p.parent() == Some(path))
            .filter_map(|(p, _)| self.subtree(p, depth - 1, limit))
            .collect();
        children.extend(
            self.files
                .iter()
                .filter(|f| f.path.parent() == Some(path))
                .map(|f| TreeNode {
                    path: f.path.clone(),
                    name: node_name(&f.path),
                    bytes: f.size,
                    files: 1,
                    is_dir: false,
                    category: Some(f.category),
                    children: Vec::new(),
                }),
        );
        children.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));
        if children.len() > limit {
            let rest = children.split_off(limit);
            let bytes = rest.iter().map(|c| c.bytes).sum();
            let files = rest.iter().map(|c| c.files).sum();
            children.push(TreeNode {
                path: path.join("…"),
                name: String::new(),
                bytes,
                files,
                is_dir: false,
                category: None,
                children: Vec::new(),
            });
        }
        node.children = children;
        Some(node)
    }

    /// Lightweight view for IPC/CLI: everything except the file and dir lists.
    pub fn summary(&self) -> ScanSummary {
        ScanSummary {
            root: self.root.clone(),
            diff: self.diff.clone(),
            disk_total_bytes: self.disk_total_bytes,
            disk_free_bytes: self.disk_free_bytes,
            started_at_unix: self.started_at_unix,
            finished_at_unix: self.finished_at_unix,
            cancelled: self.cancelled,
            total_bytes: self.total_bytes,
            file_count: self.file_count,
            dir_count: self.dir_count,
            denied_count: self.denied_count,
            denied_sample: self.denied_sample.clone(),
            categories: self.categories(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ScanSummary {
    pub root: PathBuf,
    pub diff: Option<super::stats_cache::ScanDiff>,
    pub disk_total_bytes: Option<u64>,
    pub disk_free_bytes: Option<u64>,
    pub started_at_unix: u64,
    pub finished_at_unix: u64,
    pub cancelled: bool,
    pub total_bytes: u64,
    pub file_count: u64,
    pub dir_count: u64,
    pub denied_count: u64,
    pub denied_sample: Vec<PathBuf>,
    pub categories: Vec<CategorySummary>,
}

fn node_name(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| p.to_string_lossy().into_owned())
}
