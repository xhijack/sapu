//! Which copy to keep.

use std::path::{Path, PathBuf};

use super::group::DuplicateGroup;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum KeepStrategy {
    /// Oldest modification time.
    Oldest,
    Newest,
    /// The name that looks least like a copy, then shortest path.
    OriginalName,
    /// Prefer a copy inside one of these folders (first match wins), then OriginalName.
    InFolder {
        folders: Vec<PathBuf>,
    },
}

/// Penalises typical copy markers in a file name. Higher is more original.
pub fn name_score(path: &Path) -> i32 {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    let mut score = 0;
    for marker in [
        " copy",
        "copy of ",
        "salinan",
        " - copy",
        "(1)",
        "(2)",
        "(3)",
        "_copy",
        "-copy",
        " 2",
        " 3",
        "duplicate",
        "duplikat",
    ] {
        if stem.contains(marker) {
            score -= 10;
        }
    }
    if stem.ends_with(')') {
        score -= 5;
    }
    score - (stem.len() as i32 / 20)
}

/// Index of the file to keep in `group.files`.
pub fn choose_keep(group: &DuplicateGroup, strategy: &KeepStrategy) -> usize {
    let files = &group.files;
    let by_name = |i: usize| {
        (
            files[i].name_score,
            -(files[i].path.as_os_str().len() as i32),
        )
    };
    let best_by_name = (0..files.len()).max_by_key(|&i| by_name(i)).unwrap_or(0);
    match strategy {
        KeepStrategy::Oldest => (0..files.len())
            .filter(|&i| files[i].mtime.is_some())
            .min_by_key(|&i| files[i].mtime)
            .unwrap_or(best_by_name),
        KeepStrategy::Newest => (0..files.len())
            .filter(|&i| files[i].mtime.is_some())
            .max_by_key(|&i| files[i].mtime)
            .unwrap_or(best_by_name),
        KeepStrategy::OriginalName => best_by_name,
        KeepStrategy::InFolder { folders } => {
            for folder in folders {
                let inside: Vec<usize> = (0..files.len())
                    .filter(|&i| files[i].path.starts_with(folder))
                    .collect();
                if let Some(&i) = inside.iter().max_by_key(|&&i| by_name(i)) {
                    return i;
                }
            }
            best_by_name
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    fn file(p: &str, age_days: u64) -> super::super::group::DupFile {
        super::super::group::DupFile {
            path: PathBuf::from(p),
            mtime: Some(
                SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000 - age_days * 86_400),
            ),
            name_score: name_score(Path::new(p)),
        }
    }

    #[test]
    fn copy_markers_lower_the_score() {
        assert!(
            name_score(Path::new("/x/report.pdf")) > name_score(Path::new("/x/report copy.pdf"))
        );
        assert!(name_score(Path::new("/x/IMG_1.jpg")) > name_score(Path::new("/x/IMG_1 (1).jpg")));
        assert!(name_score(Path::new("/x/a.txt")) > name_score(Path::new("/x/Salinan a.txt")));
    }

    #[test]
    fn strategies() {
        let g = DuplicateGroup::new(
            1,
            10,
            "h".into(),
            vec![
                file("/Users/u/Downloads/report copy.pdf", 1),
                file("/Users/u/Documents/report.pdf", 30),
                file("/Users/u/Desktop/report (1).pdf", 10),
            ],
        );
        // files sorted by path: Desktop, Documents, Downloads
        let idx = |p: &str| {
            g.files
                .iter()
                .position(|f| f.path.ends_with(p))
                .expect("idx")
        };
        assert_eq!(
            choose_keep(&g, &KeepStrategy::Oldest),
            idx("Documents/report.pdf")
        );
        assert_eq!(
            choose_keep(&g, &KeepStrategy::Newest),
            idx("report copy.pdf")
        );
        assert_eq!(
            choose_keep(&g, &KeepStrategy::OriginalName),
            idx("Documents/report.pdf")
        );
        assert_eq!(
            choose_keep(
                &g,
                &KeepStrategy::InFolder {
                    folders: vec![PathBuf::from("/Users/u/Desktop")]
                }
            ),
            idx("Desktop/report (1).pdf")
        );
    }
}
