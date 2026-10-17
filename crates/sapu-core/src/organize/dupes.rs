//! Duplicate files inside the folder being tidied, through the dedupe engine.

use std::path::Path;

use crate::dedupe::{DedupeOptions, DedupeResult, find_duplicates};
use crate::error::Result;
use crate::progress::ProgressSink;
use crate::scanner::ScanControl;

/// Same-content files under `root` (default dedupe minimum size applies),
/// leaving out anything inside a project or a bundle: those are units the
/// organiser never opens.
pub fn duplicates_in(
    root: &Path,
    control: &ScanControl,
    progress: &mut dyn ProgressSink,
) -> Result<DedupeResult> {
    let listing = super::scan::scan_folder(root)?;
    let units: Vec<_> = listing
        .entries
        .iter()
        .filter(|e| {
            matches!(
                e.unit,
                super::scan::Unit::Project | super::scan::Unit::Bundle
            )
        })
        .map(|e| crate::safety::lexical_absolute(&e.path))
        .collect();
    let mut result = find_duplicates(&DedupeOptions::new(root), control, progress, |_| {})?;
    for g in &mut result.groups {
        g.files.retain(|f| {
            let p = crate::safety::lexical_absolute(&f.path);
            !units.iter().any(|u| p.starts_with(u))
        });
        g.wasted_bytes = g.size * (g.files.len().saturating_sub(1)) as u64;
    }
    result.groups.retain(|g| g.files.len() >= 2);
    Ok(result)
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

    #[test]
    fn ignores_files_inside_projects_and_bundles() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.test-tmp");
        std::fs::create_dir_all(&base).expect("mkdir test-tmp");
        let t = tempfile::tempdir_in(base).expect("tempdir");
        let body = vec![7u8; 8192];
        std::fs::create_dir_all(t.path().join("proj")).expect("proj");
        std::fs::write(t.path().join("proj/package.json"), b"{}").expect("marker");
        std::fs::write(t.path().join("proj/a.bin"), &body).expect("write");
        std::fs::write(t.path().join("a.bin"), &body).expect("write");
        let r = duplicates_in(t.path(), &ScanControl::new(), &mut NoProgress).expect("dupes");
        assert!(
            r.groups.is_empty(),
            "the copy inside the project does not count: {:?}",
            r.groups
        );
    }
}
