//! Three-stage duplicate detection.

use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

use rayon::prelude::*;

use crate::error::{Result, SapuError};
use crate::progress::{ProgressEvent, ProgressSink, Throttled};
use crate::scanner::ScanControl;

use super::group::{DupFile, DuplicateGroup};
use super::keep::name_score;

#[derive(Debug, Clone)]
pub struct DedupeOptions {
    pub root: PathBuf,
    /// Files smaller than this are ignored (default 4 KiB).
    pub min_size: u64,
}

impl DedupeOptions {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            min_size: 4096,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DedupeSummary {
    pub root: PathBuf,
    pub files_seen: u64,
    pub candidates: u64,
    pub groups: u64,
    pub duplicate_files: u64,
    pub wasted_bytes: u64,
    pub cancelled: bool,
    pub stage_reached: u8,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DedupeResult {
    pub summary: DedupeSummary,
    /// Largest wasted bytes first.
    pub groups: Vec<DuplicateGroup>,
}

/// Progress payload carries the stage so the UI can say "Langkah 2 dari 3".
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct Stage {
    pub current: u8,
    pub total: u8,
}

pub const STAGES: u8 = 3;

struct Entry {
    path: PathBuf,
    size: u64,
    mtime: Option<SystemTime>,
}

/// Runs the pipeline. `on_stage` is told when a new stage starts.
pub fn find_duplicates(
    opts: &DedupeOptions,
    control: &ScanControl,
    progress: &mut dyn ProgressSink,
    mut on_stage: impl FnMut(Stage),
) -> Result<DedupeResult> {
    let root = std::path::absolute(&opts.root).map_err(|e| SapuError::io("resolve root", e))?;
    if !root.is_dir() {
        return Err(SapuError::io(
            "dedupe root",
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "root is not a directory"),
        ));
    }
    let mut sink = Throttled::new(progress, 10);
    let mut summary = DedupeSummary {
        root: root.clone(),
        files_seen: 0,
        candidates: 0,
        groups: 0,
        duplicate_files: 0,
        wasted_bytes: 0,
        cancelled: false,
        stage_reached: 1,
    };

    // Stage 1: walk and bucket by size.
    on_stage(Stage {
        current: 1,
        total: STAGES,
    });
    let mut by_size: HashMap<u64, Vec<Entry>> = HashMap::new();
    let mut seen_inodes = inode::Seen::default();
    let mut cancelled = false;
    for item in jwalk::WalkDir::new(&root)
        .follow_links(false)
        .parallelism(crate::platform::walk_parallelism())
        .skip_hidden(false)
        .sort(false)
    {
        if !control.wait_if_paused() {
            cancelled = true;
            break;
        }
        let Ok(e) = item else { continue };
        if !e.file_type().is_file() {
            continue;
        }
        let path = e.path();
        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        if !meta.is_file() || meta.len() < opts.min_size {
            continue;
        }
        if inode::is_repeat(&meta, &mut seen_inodes) {
            continue;
        }
        summary.files_seen += 1;
        by_size.entry(meta.len()).or_default().push(Entry {
            path,
            size: meta.len(),
            mtime: meta.modified().ok(),
        });
        if summary.files_seen.is_multiple_of(256) {
            sink.report(&ProgressEvent {
                items_done: summary.files_seen,
                items_total: None,
                bytes_done: 0,
                bytes_total: None,
            });
        }
    }
    let mut candidates: Vec<Vec<Entry>> = by_size.into_values().filter(|v| v.len() > 1).collect();
    summary.candidates = candidates.iter().map(|v| v.len() as u64).sum();
    if cancelled {
        summary.cancelled = true;
        return Ok(DedupeResult {
            summary,
            groups: Vec::new(),
        });
    }

    // Stage 2: partial hash within each size bucket.
    on_stage(Stage {
        current: 2,
        total: STAGES,
    });
    summary.stage_reached = 2;
    let total_candidates = summary.candidates;
    let done = AtomicU64::new(0);
    let partial: Vec<Vec<Entry>> = candidates
        .par_drain(..)
        .flat_map_iter(|bucket| {
            let mut by_partial: HashMap<[u8; 32], Vec<Entry>> = HashMap::new();
            for e in bucket {
                if control.is_cancelled() {
                    break;
                }
                if let Ok(h) = partial_hash(&e.path, e.size) {
                    by_partial.entry(h).or_default().push(e);
                }
                done.fetch_add(1, Ordering::Relaxed);
            }
            by_partial
                .into_values()
                .filter(|v| v.len() > 1)
                .collect::<Vec<_>>()
        })
        .collect();
    sink.report(&ProgressEvent {
        items_done: done.load(Ordering::Relaxed),
        items_total: Some(total_candidates),
        bytes_done: 0,
        bytes_total: None,
    });
    if control.is_cancelled() {
        summary.cancelled = true;
        return Ok(DedupeResult {
            summary,
            groups: Vec::new(),
        });
    }

    // Stage 3: full hash; bytes to hash are known now.
    on_stage(Stage {
        current: 3,
        total: STAGES,
    });
    summary.stage_reached = 3;
    let bytes_total: u64 = partial.iter().flatten().map(|e| e.size).sum();
    let bytes_done = AtomicU64::new(0);
    let groups_raw: Vec<(u64, [u8; 32], Vec<Entry>)> = partial
        .into_par_iter()
        .flat_map_iter(|bucket| {
            let mut by_full: HashMap<[u8; 32], Vec<Entry>> = HashMap::new();
            for e in bucket {
                if control.is_cancelled() {
                    break;
                }
                if let Ok(h) = full_hash(&e.path) {
                    bytes_done.fetch_add(e.size, Ordering::Relaxed);
                    by_full.entry(h).or_default().push(e);
                }
            }
            by_full
                .into_iter()
                .filter(|(_, v)| v.len() > 1)
                .map(|(h, v)| (v[0].size, h, v))
                .collect::<Vec<_>>()
        })
        .collect();
    sink.report(&ProgressEvent {
        items_done: groups_raw.len() as u64,
        items_total: Some(groups_raw.len() as u64),
        bytes_done: bytes_done.load(Ordering::Relaxed),
        bytes_total: Some(bytes_total),
    });
    summary.cancelled = control.is_cancelled();

    let mut groups: Vec<DuplicateGroup> = groups_raw
        .into_iter()
        .enumerate()
        .map(|(i, (size, hash, entries))| {
            let files = entries
                .into_iter()
                .map(|e| DupFile {
                    name_score: name_score(&e.path),
                    path: e.path,
                    mtime: e.mtime,
                })
                .collect();
            DuplicateGroup::new(i as u64 + 1, size, hex(&hash), files)
        })
        .collect();
    groups.sort_by(|a, b| {
        b.wasted_bytes
            .cmp(&a.wasted_bytes)
            .then_with(|| a.id.cmp(&b.id))
    });
    summary.groups = groups.len() as u64;
    summary.duplicate_files = groups.iter().map(|g| g.files.len() as u64 - 1).sum();
    summary.wasted_bytes = groups.iter().map(|g| g.wasted_bytes).sum();
    tracing::info!(
        files = summary.files_seen,
        groups = summary.groups,
        wasted = summary.wasted_bytes,
        cancelled = summary.cancelled,
        "dedupe finished"
    );
    Ok(DedupeResult { summary, groups })
}

const PARTIAL: u64 = 4096;

fn partial_hash(path: &Path, size: u64) -> std::io::Result<[u8; 32]> {
    let mut f = std::fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0u8; PARTIAL as usize];
    let n = f.read(&mut buf)?;
    hasher.update(&buf[..n]);
    if size > PARTIAL * 2 {
        f.seek(SeekFrom::End(-(PARTIAL as i64)))?;
        let n = f.read(&mut buf)?;
        hasher.update(&buf[..n]);
    }
    Ok(*hasher.finalize().as_bytes())
}

fn full_hash(path: &Path) -> std::io::Result<[u8; 32]> {
    let mut f = std::fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(*hasher.finalize().as_bytes())
}

fn hex(h: &[u8; 32]) -> String {
    h.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(unix)]
mod inode {
    use std::collections::HashSet;
    use std::fs::Metadata;
    use std::os::unix::fs::MetadataExt;

    #[derive(Default)]
    pub(super) struct Seen(HashSet<(u64, u64)>);

    pub(super) fn is_repeat(meta: &Metadata, seen: &mut Seen) -> bool {
        meta.nlink() > 1 && !seen.0.insert((meta.dev(), meta.ino()))
    }
}

#[cfg(not(unix))]
mod inode {
    use std::fs::Metadata;

    #[derive(Default)]
    pub(super) struct Seen;

    pub(super) fn is_repeat(_meta: &Metadata, _seen: &mut Seen) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::NoProgress;

    fn write(p: &Path, bytes: &[u8]) {
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).expect("mkdir");
        }
        std::fs::write(p, bytes).expect("write");
    }

    fn opts(root: &Path) -> DedupeOptions {
        DedupeOptions {
            root: root.to_path_buf(),
            min_size: 1,
        }
    }

    #[test]
    fn finds_identical_files_only() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let r = tmp.path();
        let big = vec![7u8; 20_000];
        let mut big2 = big.clone();
        big2[10_000] = 8; // same size, same head and tail, different middle
        write(&r.join("a/photo.jpg"), &big);
        write(&r.join("b/photo copy.jpg"), &big);
        write(&r.join("c/other.jpg"), &big2);
        write(&r.join("d/short.txt"), b"hello");
        write(&r.join("e/short2.txt"), b"hellp");
        write(&r.join("f/unique.bin"), &vec![1u8; 300]);

        let mut stages = Vec::new();
        let res = find_duplicates(&opts(r), &ScanControl::new(), &mut NoProgress, |s| {
            stages.push(s.current)
        })
        .expect("dedupe");
        assert_eq!(stages, vec![1, 2, 3]);
        assert_eq!(res.groups.len(), 1, "{:?}", res.groups);
        let g = &res.groups[0];
        assert_eq!(g.files.len(), 2);
        assert_eq!(g.wasted_bytes, 20_000);
        assert_eq!(res.summary.wasted_bytes, 20_000);
        assert_eq!(res.summary.duplicate_files, 1);
        assert!(g.files.iter().any(|f| f.path.ends_with("photo.jpg")));
    }

    #[cfg(unix)]
    #[test]
    fn hardlinks_are_one_file_and_symlinks_ignored() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let r = tmp.path();
        write(&r.join("a.bin"), &vec![3u8; 5000]);
        std::fs::hard_link(r.join("a.bin"), r.join("a-link.bin")).expect("hardlink");
        std::os::unix::fs::symlink(r.join("a.bin"), r.join("a-sym.bin")).expect("symlink");
        let res = find_duplicates(&opts(r), &ScanControl::new(), &mut NoProgress, |_| {})
            .expect("dedupe");
        assert!(res.groups.is_empty(), "{:?}", res.groups);
        assert_eq!(res.summary.files_seen, 1);
    }

    #[test]
    fn min_size_filters() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let r = tmp.path();
        write(&r.join("x"), b"same");
        write(&r.join("y"), b"same");
        let res = find_duplicates(
            &DedupeOptions::new(r),
            &ScanControl::new(),
            &mut NoProgress,
            |_| {},
        )
        .expect("dedupe");
        assert!(res.groups.is_empty());
    }

    #[test]
    fn cancel_returns_partial() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let r = tmp.path();
        for i in 0..50 {
            write(&r.join(format!("d/{i}.bin")), &vec![9u8; 2000]);
        }
        let control = ScanControl::new();
        control.cancel();
        let res = find_duplicates(&opts(r), &control, &mut NoProgress, |_| {}).expect("dedupe");
        assert!(res.summary.cancelled);
        assert!(res.groups.is_empty());
    }
}
