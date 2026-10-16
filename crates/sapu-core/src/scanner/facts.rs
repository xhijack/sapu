//! Metadata facts about one path, for explaining it (AI "what is this?")
//! without ever sending contents anywhere.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PathFacts {
    pub path: PathBuf,
    pub is_dir: bool,
    pub bytes: u64,
    pub file_count: u64,
    /// Most common extensions with counts, largest first (at most 8).
    pub top_extensions: Vec<(String, u64)>,
    pub newest_age_days: Option<u64>,
    pub oldest_age_days: Option<u64>,
}

/// Bounded walk (at most `max_files` files) that never follows symlinks.
pub fn describe(path: &Path, max_files: usize) -> std::io::Result<PathFacts> {
    let meta = std::fs::symlink_metadata(path)?;
    let now = SystemTime::now();
    let age = |t: SystemTime| now.duration_since(t).ok().map(|d| d.as_secs() / 86_400);
    if !meta.is_dir() {
        let a = meta.modified().ok().and_then(age);
        return Ok(PathFacts {
            path: path.to_path_buf(),
            is_dir: false,
            bytes: crate::platform::disk_usage_of(&meta),
            file_count: 1,
            top_extensions: path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| vec![(e.to_ascii_lowercase(), 1)])
                .unwrap_or_default(),
            newest_age_days: a,
            oldest_age_days: a,
        });
    }
    let mut bytes = 0u64;
    let mut count = 0u64;
    let mut exts: HashMap<String, u64> = HashMap::new();
    let (mut newest, mut oldest): (Option<SystemTime>, Option<SystemTime>) = (None, None);
    for e in jwalk::WalkDir::new(path)
        .follow_links(false)
        .skip_hidden(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        if !e.file_type().is_file() {
            continue;
        }
        if count as usize >= max_files {
            break;
        }
        let Ok(m) = std::fs::symlink_metadata(e.path()) else {
            continue;
        };
        count += 1;
        bytes += crate::platform::disk_usage_of(&m);
        if let Some(ext) = e.path().extension().and_then(|x| x.to_str()) {
            *exts.entry(ext.to_ascii_lowercase()).or_default() += 1;
        }
        if let Ok(t) = m.modified() {
            newest = Some(newest.map_or(t, |n| n.max(t)));
            oldest = Some(oldest.map_or(t, |o| o.min(t)));
        }
    }
    let mut top: Vec<(String, u64)> = exts.into_iter().collect();
    top.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    top.truncate(8);
    Ok(PathFacts {
        path: path.to_path_buf(),
        is_dir: true,
        bytes,
        file_count: count,
        top_extensions: top,
        newest_age_days: newest.and_then(age),
        oldest_age_days: oldest.and_then(age),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describes_a_folder() {
        let tmp = tempfile::tempdir().expect("tempdir");
        for n in ["a.jpg", "b.JPG", "c.mov", "sub/d.jpg"] {
            let p = tmp.path().join(n);
            std::fs::create_dir_all(p.parent().expect("p")).expect("mkdir");
            std::fs::write(&p, b"xx").expect("write");
        }
        let f = describe(tmp.path(), 10_000).expect("facts");
        assert!(f.is_dir);
        assert_eq!(f.file_count, 4);
        assert_eq!(f.top_extensions[0], ("jpg".into(), 3));
        assert_eq!(f.newest_age_days, Some(0));
    }
}
