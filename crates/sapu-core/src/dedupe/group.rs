use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DupFile {
    pub path: PathBuf,
    pub mtime: Option<SystemTime>,
    /// Higher = looks more like the original name (see `keep::name_score`).
    pub name_score: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DuplicateGroup {
    pub id: u64,
    pub size: u64,
    pub hash: String,
    pub files: Vec<DupFile>,
    /// Bytes recovered when all but one copy go: `size * (files - 1)`.
    pub wasted_bytes: u64,
}

impl DuplicateGroup {
    pub fn new(id: u64, size: u64, hash: String, mut files: Vec<DupFile>) -> Self {
        files.sort_by(|a, b| a.path.cmp(&b.path));
        let wasted_bytes = size * files.len().saturating_sub(1) as u64;
        Self {
            id,
            size,
            hash,
            files,
            wasted_bytes,
        }
    }
}

/// "Folder A and folder B share N files (M bytes)".
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FolderPair {
    pub a: PathBuf,
    pub b: PathBuf,
    pub files: u64,
    pub bytes: u64,
}

pub fn folder_pairs(groups: &[DuplicateGroup]) -> Vec<FolderPair> {
    let mut acc: HashMap<(PathBuf, PathBuf), (u64, u64)> = HashMap::new();
    for g in groups {
        let dirs: Vec<&Path> = g.files.iter().filter_map(|f| f.path.parent()).collect();
        for i in 0..dirs.len() {
            for j in (i + 1)..dirs.len() {
                if dirs[i] == dirs[j] {
                    continue;
                }
                let (a, b) = if dirs[i] < dirs[j] {
                    (dirs[i], dirs[j])
                } else {
                    (dirs[j], dirs[i])
                };
                let e = acc.entry((a.to_path_buf(), b.to_path_buf())).or_default();
                e.0 += 1;
                e.1 += g.size;
            }
        }
    }
    let mut out: Vec<FolderPair> = acc
        .into_iter()
        .map(|((a, b), (files, bytes))| FolderPair { a, b, files, bytes })
        .collect();
    out.sort_by(|x, y| y.bytes.cmp(&x.bytes).then_with(|| x.a.cmp(&y.a)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(p: &str) -> DupFile {
        DupFile {
            path: PathBuf::from(p),
            mtime: None,
            name_score: 0,
        }
    }

    #[test]
    fn wasted_bytes_and_folder_pairs() {
        let g1 = DuplicateGroup::new(1, 100, "h1".into(), vec![f("/a/x"), f("/b/x")]);
        let g2 = DuplicateGroup::new(2, 50, "h2".into(), vec![f("/a/y"), f("/b/y"), f("/c/y")]);
        assert_eq!(g1.wasted_bytes, 100);
        assert_eq!(g2.wasted_bytes, 100);
        let pairs = folder_pairs(&[g1, g2]);
        let ab = pairs
            .iter()
            .find(|p| p.a == Path::new("/a") && p.b == Path::new("/b"))
            .expect("pair");
        assert_eq!((ab.files, ab.bytes), (2, 150));
        assert_eq!(pairs.len(), 3);
    }
}
