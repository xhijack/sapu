//! Answer cache keyed by task and input, so the same question is not sent twice.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AnswerCache {
    entries: HashMap<String, serde_json::Value>,
}

impl AnswerCache {
    pub fn default_path() -> Option<PathBuf> {
        dirs::data_local_dir().map(|d| d.join("sapu").join("ai-cache.json"))
    }

    pub fn load(path: &Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(
            path,
            serde_json::to_vec(self).map_err(std::io::Error::other)?,
        )
    }

    pub fn key(task: &str, model: &str, input: &serde_json::Value) -> String {
        let mut h = blake3::Hasher::new();
        h.update(task.as_bytes());
        h.update(b"\0");
        h.update(model.as_bytes());
        h.update(b"\0");
        h.update(input.to_string().as_bytes());
        h.finalize().to_hex().to_string()
    }

    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.entries.get(key)
    }

    pub fn put(&mut self, key: String, value: serde_json::Value) {
        if self.entries.len() > 2000 {
            self.entries.clear();
        }
        self.entries.insert(key, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_input_same_key_and_roundtrip() {
        let v = serde_json::json!({"a": 1});
        let k1 = AnswerCache::key("whatis", "m", &v);
        let k2 = AnswerCache::key("whatis", "m", &serde_json::json!({"a": 1}));
        assert_eq!(k1, k2);
        assert_ne!(k1, AnswerCache::key("whatis", "other", &v));
        let tmp = tempfile::tempdir().expect("tempdir");
        let p = tmp.path().join("c.json");
        let mut c = AnswerCache::default();
        c.put(k1.clone(), serde_json::json!({"ok": true}));
        c.save(&p).expect("save");
        assert_eq!(
            AnswerCache::load(&p).get(&k1),
            Some(&serde_json::json!({"ok": true}))
        );
    }
}
