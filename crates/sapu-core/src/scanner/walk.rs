//! The parallel walk.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{Result, SapuError};
use crate::progress::{ProgressEvent, ProgressSink, Throttled};

use super::category::Category;
use super::control::ScanControl;
use super::result::{DirStats, FileEntry, ScanResult};
use super::stats_cache::ScanStats;

#[derive(Debug, Clone)]
pub struct ScanOptions {
    pub root: PathBuf,
    /// Snapshot of the previous scan of this root, for progress and diff.
    pub previous: Option<ScanStats>,
    /// Denominator for progress: bytes/files expected (previous scan or disk
    /// usage). `None` = indeterminate.
    pub expected_bytes: Option<u64>,
    pub expected_files: Option<u64>,
    /// How many unreadable directories to keep as examples.
    pub denied_sample_size: usize,
}

impl ScanOptions {
    /// Chooses the progress denominator per PRD §10.1: previous scan totals
    /// when available (`stats_dir`), otherwise the volume's used bytes.
    pub fn new(root: impl Into<PathBuf>, stats_dir: Option<&Path>) -> Self {
        let root = root.into();
        let prev = stats_dir.and_then(|d| ScanStats::load(d, &root));
        let (expected_bytes, expected_files) = match &prev {
            Some(s) => (Some(s.total_bytes), Some(s.file_count)),
            None => (super::disk::used_bytes(&root), None),
        };
        Self {
            root,
            previous: prev,
            expected_bytes,
            expected_files,
            denied_sample_size: 20,
        }
    }
}

const REPORT_EVERY: u64 = 512;

/// Walks `opts.root` without following symlinks. Unreadable directories are
/// counted and skipped. Returns a partial result when cancelled.
pub fn scan(
    opts: &ScanOptions,
    control: &ScanControl,
    progress: &mut dyn ProgressSink,
) -> Result<ScanResult> {
    let root = std::path::absolute(&opts.root).map_err(|e| SapuError::io("resolve root", e))?;
    let meta = std::fs::symlink_metadata(&root).map_err(|e| SapuError::io("stat root", e))?;
    if !meta.is_dir() {
        return Err(SapuError::io(
            "scan root",
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "root is not a directory"),
        ));
    }

    let started = now_unix();
    let mut sink = Throttled::new(progress, 10);
    let mut files: Vec<FileEntry> = Vec::new();
    let mut dirs: HashMap<PathBuf, DirStats> = HashMap::new();
    dirs.insert(root.clone(), DirStats::default());
    let mut seen_inodes = hardlink::Seen::default();
    let mut total_bytes = 0u64;
    let mut denied_count = 0u64;
    let mut denied_sample = Vec::new();
    let mut cancelled = false;
    let mut since_report = 0u64;

    // Metadata is fetched inside jwalk's worker threads (`process_read_dir`)
    // so the stat calls run in parallel; the consumer loop below only
    // aggregates. This is the difference between ~4 and ~1 minute on a
    // multi-million-file home directory.
    let walker = jwalk::WalkDirGeneric::<((), Option<Stat>)>::new(&root)
        .follow_links(false)
        .skip_hidden(false)
        .sort(false)
        .process_read_dir(|_depth, _path, _state, children| {
            for child in children.iter_mut().flatten() {
                if !child.file_type().is_dir() {
                    child.client_state = std::fs::symlink_metadata(child.path())
                        .ok()
                        .map(|m| Stat::from(&m));
                }
            }
        });

    for item in walker {
        if !control.wait_if_paused() {
            cancelled = true;
            break;
        }
        let entry = match item {
            Ok(e) => e,
            Err(_) => {
                denied_count += 1;
                continue;
            }
        };
        let path = entry.path();
        if entry.depth() == 0 {
            continue;
        }
        if entry.file_type().is_dir() {
            if entry.read_children_error.is_some() {
                denied_count += 1;
                if denied_sample.len() < opts.denied_sample_size {
                    denied_sample.push(path.clone());
                }
            }
            dirs.entry(path).or_default();
            continue;
        }
        // Files and symlinks (never followed): size of the entry itself.
        let Some(meta) = entry.client_state else {
            continue;
        };
        if hardlink::is_repeat(&meta, &mut seen_inodes) {
            continue;
        }
        let size = meta.size;
        total_bytes += size;
        if let Some(parent) = path.parent() {
            let d = dirs.entry(parent.to_path_buf()).or_default();
            d.own_bytes += size;
            d.own_files += 1;
        }
        files.push(FileEntry {
            category: Category::of(&path),
            path,
            size,
            mtime: meta.mtime,
        });

        since_report += 1;
        if since_report >= REPORT_EVERY {
            since_report = 0;
            sink.report(&ProgressEvent {
                items_done: files.len() as u64,
                items_total: opts.expected_files,
                bytes_done: total_bytes,
                bytes_total: opts.expected_bytes,
            });
        }
    }

    aggregate(&root, &mut dirs);
    let finished = now_unix();
    sink.report(&ProgressEvent {
        items_done: files.len() as u64,
        items_total: Some(files.len() as u64),
        bytes_done: total_bytes,
        bytes_total: Some(total_bytes),
    });
    tracing::info!(
        files = files.len(),
        dirs = dirs.len(),
        denied = denied_count,
        cancelled,
        "scan finished"
    );

    let (disk_total_bytes, disk_free_bytes) = super::disk::capacity(&root).unzip();
    let mut result = ScanResult {
        diff: None,
        disk_total_bytes,
        disk_free_bytes,
        root,
        started_at_unix: started,
        finished_at_unix: finished,
        cancelled,
        total_bytes,
        file_count: files.len() as u64,
        dir_count: dirs.len() as u64,
        denied_count,
        denied_sample,
        files,
        dirs,
    };
    if let Some(prev) = &opts.previous
        && !cancelled
    {
        result.diff = Some(prev.diff(&result));
    }
    Ok(result)
}

/// Roll `own_*` totals up into every ancestor inside the scan root.
fn aggregate(root: &Path, dirs: &mut HashMap<PathBuf, DirStats>) {
    let mut paths: Vec<PathBuf> = dirs.keys().cloned().collect();
    // Deepest first so each directory's total is final before its parent adds it.
    paths.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
    for p in paths {
        let (own_bytes, own_files, child_bytes, child_files) = {
            let s = dirs.get(&p).cloned().unwrap_or_default();
            (s.own_bytes, s.own_files, s.total_bytes, s.total_files)
        };
        let total_bytes = own_bytes + child_bytes;
        let total_files = own_files + child_files;
        if let Some(s) = dirs.get_mut(&p) {
            s.total_bytes = total_bytes;
            s.total_files = total_files;
        }
        if p.as_path() == root {
            continue;
        }
        if let Some(parent) = p.parent() {
            let ps = dirs.entry(parent.to_path_buf()).or_default();
            ps.total_bytes += total_bytes;
            ps.total_files += total_files;
        }
    }
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// The few metadata fields the scanner needs, computed in worker threads.
#[derive(Debug, Clone, Copy)]
struct Stat {
    size: u64,
    mtime: Option<SystemTime>,
    #[cfg(unix)]
    dev: u64,
    #[cfg(unix)]
    ino: u64,
    #[cfg(unix)]
    nlink: u64,
}

impl From<&std::fs::Metadata> for Stat {
    fn from(m: &std::fs::Metadata) -> Self {
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt;
        Self {
            size: crate::platform::disk_usage_of(m),
            mtime: m.modified().ok(),
            #[cfg(unix)]
            dev: m.dev(),
            #[cfg(unix)]
            ino: m.ino(),
            #[cfg(unix)]
            nlink: m.nlink(),
        }
    }
}

#[cfg(unix)]
mod hardlink {
    use std::collections::HashSet;

    use super::Stat;

    #[derive(Default)]
    pub(super) struct Seen(HashSet<(u64, u64)>);

    /// True when this inode was already counted (nlink > 1 only).
    pub(super) fn is_repeat(meta: &Stat, seen: &mut Seen) -> bool {
        meta.nlink > 1 && !seen.0.insert((meta.dev, meta.ino))
    }
}

#[cfg(not(unix))]
mod hardlink {
    use super::Stat;

    #[derive(Default)]
    pub(super) struct Seen;

    /// Windows: hardlinks are not deduplicated in P0 (needs by-handle info).
    pub(super) fn is_repeat(_meta: &Stat, _seen: &mut Seen) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::{FnSink, NoProgress};
    use crate::scanner::result::{AgeFilter, FileFilter};

    fn write(p: &Path, n: usize) {
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).expect("mkdir");
        }
        std::fs::write(p, vec![b'x'; n]).expect("write");
    }

    /// The OS temp dir sits under `/var` (macOS) or similar, which the
    /// classifier treats as a system/temporary context by design. Tests
    /// therefore use a throwaway dir inside the repo tree.
    fn tempdir() -> tempfile::TempDir {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.test-tmp");
        std::fs::create_dir_all(&base).expect("mkdir test-tmp");
        tempfile::tempdir_in(base).expect("tempdir")
    }

    /// Sizes are what the disk holds, not the logical length.
    fn on_disk(p: &Path) -> u64 {
        crate::platform::disk_usage_of(&std::fs::symlink_metadata(p).expect("meta"))
    }

    fn opts(root: &Path) -> ScanOptions {
        ScanOptions {
            root: root.to_path_buf(),
            previous: None,
            expected_bytes: None,
            expected_files: None,
            denied_sample_size: 5,
        }
    }

    #[test]
    fn counts_sizes_and_aggregates_dirs() {
        let tmp = tempdir();
        let r = tmp.path();
        write(&r.join("a.txt"), 100);
        write(&r.join("sub/b.mp4"), 20_000);
        write(&r.join("sub/deep/c.jpg"), 5_000);
        let (a, b, c) = (
            on_disk(&r.join("a.txt")),
            on_disk(&r.join("sub/b.mp4")),
            on_disk(&r.join("sub/deep/c.jpg")),
        );
        assert!(
            b > c && c >= a,
            "block sizes must differ for ordering: {a} {b} {c}"
        );

        let res = scan(&opts(r), &ScanControl::new(), &mut NoProgress).expect("scan");
        assert_eq!(res.total_bytes, a + b + c);
        assert_eq!(res.file_count, 3);
        assert!(!res.cancelled);
        assert!(res.disk_total_bytes.is_some_and(|t| t > res.total_bytes));
        let root_abs = std::path::absolute(r).expect("abs");
        assert_eq!(res.dirs[&root_abs].total_bytes, a + b + c);
        assert_eq!(res.dirs[&root_abs.join("sub")].total_bytes, b + c);
        assert_eq!(res.dirs[&root_abs.join("sub")].own_bytes, b);
        assert_eq!(res.dirs[&root_abs.join("sub/deep")].total_files, 1);

        let cats = res.categories();
        assert_eq!(cats[0].category, Category::Video);
        assert_eq!(cats[0].bytes, b);

        let top = res.top_files(&FileFilter::default(), 2);
        assert_eq!(top.len(), 2);
        assert!(top[0].path.ends_with("b.mp4"));

        let dirs = res.top_dirs(10);
        assert!(dirs[0].path.ends_with("sub"));
        assert!(dirs.iter().all(|d| d.path != root_abs));

        let tree = res.subtree(&root_abs, 2, 10).expect("tree");
        assert_eq!(tree.bytes, a + b + c);
        assert_eq!(tree.children[0].name, "sub");
        assert_eq!(tree.children[0].children[0].name, "b.mp4");
    }

    #[test]
    fn filters_by_category_size_and_age() {
        let tmp = tempdir();
        let r = tmp.path();
        write(&r.join("old.mp4"), 50);
        write(&r.join("new.pdf"), 20_000);
        let two_years_ago = SystemTime::now() - std::time::Duration::from_secs(2 * 365 * 86_400);
        std::fs::File::options()
            .write(true)
            .open(r.join("old.mp4"))
            .expect("open")
            .set_modified(two_years_ago)
            .expect("mtime");

        let res = scan(&opts(r), &ScanControl::new(), &mut NoProgress).expect("scan");
        let old = res.top_files(
            &FileFilter {
                age: AgeFilter::OlderThanYear,
                ..Default::default()
            },
            10,
        );
        assert_eq!(old.len(), 1);
        assert!(old[0].path.ends_with("old.mp4"));
        let big = res.top_files(
            &FileFilter {
                min_bytes: 10_000,
                ..Default::default()
            },
            10,
        );
        assert_eq!(big.len(), 1);
        let docs = res.top_files(
            &FileFilter {
                category: Some(Category::Document),
                ..Default::default()
            },
            10,
        );
        assert_eq!(docs.len(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_not_followed_and_hardlinks_counted_once() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let outside = tempfile::tempdir().expect("tempdir");
        let r = tmp.path();
        write(&outside.path().join("big.bin"), 100_000);
        write(&r.join("real.bin"), 1000);
        std::fs::hard_link(r.join("real.bin"), r.join("link.bin")).expect("hardlink");
        std::os::unix::fs::symlink(outside.path(), r.join("to-outside")).expect("symlink dir");
        std::os::unix::fs::symlink(outside.path().join("big.bin"), r.join("to-big"))
            .expect("symlink file");

        let res = scan(&opts(r), &ScanControl::new(), &mut NoProgress).expect("scan");
        assert_eq!(
            res.file_count, 3,
            "real + one symlink-as-file + dir symlink entry"
        );
        assert!(
            res.total_bytes < 100_000,
            "symlink target size must not be counted"
        );
        let expected = on_disk(&r.join("real.bin"))
            + on_disk(&r.join("to-outside"))
            + on_disk(&r.join("to-big"));
        assert_eq!(res.total_bytes, expected, "hardlink counted once");
    }

    #[cfg(unix)]
    #[test]
    fn denied_dir_is_reported_not_fatal() {
        use std::os::unix::fs::PermissionsExt;
        if unsafe_is_root() {
            return; // root can read everything
        }
        let tmp = tempdir();
        let r = tmp.path();
        write(&r.join("ok.txt"), 10);
        let locked = r.join("locked");
        write(&locked.join("secret.txt"), 10);
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).expect("chmod");

        let res = scan(&opts(r), &ScanControl::new(), &mut NoProgress);
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755))
            .expect("chmod back");
        let res = res.expect("scan");
        assert_eq!(res.denied_count, 1);
        assert_eq!(res.denied_sample.len(), 1);
        assert_eq!(res.file_count, 1);
    }

    #[cfg(unix)]
    fn unsafe_is_root() -> bool {
        std::fs::metadata("/root").is_ok_and(|m| {
            use std::os::unix::fs::MetadataExt;
            m.uid() == 0 && std::env::var_os("USER").is_some_and(|u| u == "root")
        })
    }

    #[test]
    fn cancel_returns_partial_result() {
        let tmp = tempdir();
        let r = tmp.path();
        for i in 0..2000 {
            write(&r.join(format!("d{}/f{i}.txt", i % 20)), 1);
        }
        let control = ScanControl::new();
        let c2 = control.clone();
        let mut ticks = 0;
        let res = scan(
            &opts(r),
            &control,
            &mut FnSink(|_: &ProgressEvent| {
                ticks += 1;
                c2.cancel();
            }),
        )
        .expect("scan");
        assert!(res.cancelled);
        assert!(res.file_count < 2000);
    }

    #[test]
    fn reports_progress_with_final_event() {
        let tmp = tempdir();
        write(&tmp.path().join("a"), 5);
        let mut last = None;
        scan(
            &opts(tmp.path()),
            &ScanControl::new(),
            &mut FnSink(|e: &ProgressEvent| last = Some(e.clone())),
        )
        .expect("scan");
        let last = last.expect("event");
        assert_eq!(last.items_done, 1);
        assert_eq!(last.bytes_total, Some(on_disk(&tmp.path().join("a"))));
    }

    #[test]
    fn second_scan_reports_the_difference() {
        let tmp = tempdir();
        let r = tmp.path();
        write(&r.join("keep/a.txt"), 100);
        let first = scan(&opts(r), &ScanControl::new(), &mut NoProgress).expect("scan");
        assert!(first.diff.is_none());
        let prev = ScanStats::from_result(&first);

        write(&r.join("grow/big.mp4"), 50_000);
        let mut o = opts(r);
        o.previous = Some(prev.clone());
        let second = scan(&o, &ScanControl::new(), &mut NoProgress).expect("scan");
        let diff = second.diff.expect("diff");
        assert_eq!(diff.since_unix, prev.finished_at_unix);
        assert_eq!(diff.delta_files, 1);
        assert_eq!(diff.delta_bytes, on_disk(&r.join("grow/big.mp4")) as i64);
        assert!(diff.dirs[0].path.ends_with("grow") && diff.dirs[0].is_new);
        assert_eq!(diff.categories[0].category, Category::Video);
    }

    #[test]
    fn non_directory_root_is_an_error() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write(&tmp.path().join("f"), 1);
        assert!(
            scan(
                &opts(&tmp.path().join("f")),
                &ScanControl::new(),
                &mut NoProgress
            )
            .is_err()
        );
    }
}
