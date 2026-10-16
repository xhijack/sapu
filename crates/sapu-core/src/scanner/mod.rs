//! Disk scanner (Tahap 1): parallel walk, categorisation, queries for the
//! category summary, treemap and largest files/folders.
//!
//! Symlinks are never followed (S6); hardlinks are counted once on Unix.

mod category;
mod control;
mod disk;
mod facts;
mod result;
mod stats_cache;
mod walk;

pub use category::Category;
pub use control::{ScanControl, ScanState};
pub use disk::{capacity, used_bytes};
pub use facts::{PathFacts, describe};
pub use result::{
    AgeFilter, CategorySummary, DirStats, DirSummary, FileEntry, FileFilter, ScanResult,
    ScanSummary, TreeNode,
};
pub use stats_cache::{CategoryDelta, DirDelta, ScanDiff, ScanStats};
pub use walk::{ScanOptions, scan};
