//! Duplicate files inside the folder being tidied, through the dedupe engine.

use std::path::Path;

use crate::dedupe::{DedupeOptions, DedupeResult, find_duplicates};
use crate::error::Result;
use crate::progress::ProgressSink;
use crate::scanner::ScanControl;

/// Same-content files under `root` (default dedupe minimum size applies).
pub fn duplicates_in(
    root: &Path,
    control: &ScanControl,
    progress: &mut dyn ProgressSink,
) -> Result<DedupeResult> {
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
