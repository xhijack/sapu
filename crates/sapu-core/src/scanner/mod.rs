//! Disk scanner (Tahap 1): parallel walk, categorisation, queries for the
//! category summary, treemap and largest files/folders.
//!
//! Symlinks are never followed (S6); hardlinks are counted once on Unix.

mod category;
mod control;
mod disk;
mod downloads;
mod facts;
mod kind;
mod result;
mod stats_cache;
mod walk;

pub use category::Category;
pub use control::{ScanControl, ScanState};
pub use disk::{capacity, used_bytes};
pub use downloads::{
    DownloadEntry, DownloadGroup, DownloadGroupSummary, DownloadsReport, downloads_dir,
    scan_downloads,
};
pub use facts::{PathFacts, describe};
pub use kind::FileKind;
pub use result::{
    AgeFilter, BrowseEntry, BrowseFolder, CategorySummary, DirStats, DirSummary, FileEntry,
    FileFilter, ScanResult, ScanSummary, TreeNode,
};
pub use stats_cache::{CategoryDelta, DirDelta, ScanDiff, ScanLogEntry, ScanStats};
pub use walk::{LiveSnapshot, ScanOptions, scan, scan_with_live};
