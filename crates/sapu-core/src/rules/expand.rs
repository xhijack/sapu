//! Path template expansion: `~`, `${VAR}` and globs.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Environment used for expansion. Injectable so tests never depend on the
/// real home directory.
#[derive(Debug, Clone)]
pub struct Env {
    pub home: Option<PathBuf>,
    pub vars: HashMap<String, String>,
}

impl Env {
    pub fn system() -> Self {
        Self {
            home: dirs::home_dir(),
            vars: std::env::vars().collect(),
        }
    }

    pub fn with_home(home: impl Into<PathBuf>) -> Self {
        Self {
            home: Some(home.into()),
            vars: HashMap::new(),
        }
    }

    /// Replace `~` (leading) and `${VAR}`. Unknown variables yield `None`
    /// so the rule is skipped rather than pointing somewhere wrong.
    pub fn expand(&self, template: &str) -> Option<PathBuf> {
        let mut s = String::with_capacity(template.len());
        let mut rest = template;
        if let Some(stripped) = rest.strip_prefix('~') {
            s.push_str(self.home.as_ref()?.to_str()?);
            rest = stripped;
        }
        while let Some(start) = rest.find("${") {
            s.push_str(&rest[..start]);
            let after = &rest[start + 2..];
            let end = after.find('}')?;
            let var = &after[..end];
            s.push_str(self.vars.get(var)?);
            rest = &after[end + 1..];
        }
        s.push_str(rest);
        if s.is_empty() {
            None
        } else {
            Some(PathBuf::from(s))
        }
    }

    /// Expand then glob. A template without wildcards yields itself when it
    /// exists. Symlinks are not followed when matching directories.
    pub fn expand_glob(&self, template: &str) -> Vec<PathBuf> {
        let Some(p) = self.expand(template) else {
            return Vec::new();
        };
        let text = p.to_string_lossy();
        if !text.contains(['*', '?', '[']) {
            return if std::fs::symlink_metadata(&p).is_ok() {
                vec![p]
            } else {
                Vec::new()
            };
        }
        let opts = glob::MatchOptions {
            case_sensitive: !cfg!(windows),
            require_literal_separator: true,
            require_literal_leading_dot: false,
        };
        match glob::glob_with(&text, opts) {
            Ok(paths) => paths.filter_map(Result::ok).collect(),
            Err(_) => Vec::new(),
        }
    }
}

/// True when any component of `path` equals one of `skip` (glob-style, `.*`
/// means hidden).
pub(crate) fn is_skipped_pub(path: &Path, skip: &[String]) -> bool {
    is_skipped(path, skip)
}

pub(crate) fn is_skipped(path: &Path, skip: &[String]) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|name| {
            skip.iter().any(|s| {
                if s == ".*" {
                    name.starts_with('.')
                } else {
                    s == name
                }
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_home_and_vars() {
        let mut env = Env::with_home("/home/u");
        env.vars.insert("LOCALAPPDATA".into(), "/appdata".into());
        assert_eq!(
            env.expand("~/.cache"),
            Some(PathBuf::from("/home/u/.cache"))
        );
        assert_eq!(
            env.expand("${LOCALAPPDATA}/Temp"),
            Some(PathBuf::from("/appdata/Temp"))
        );
        assert_eq!(
            env.expand("${NOPE}/x"),
            None,
            "unknown variable skips the rule"
        );
    }

    #[test]
    fn glob_matches_existing_only() {
        let tmp = tempfile::tempdir().expect("tempdir");
        std::fs::write(tmp.path().join("a.dmg"), b"x").expect("write");
        std::fs::write(tmp.path().join("b.txt"), b"x").expect("write");
        let env = Env::with_home(tmp.path());
        let hits = env.expand_glob("~/*.dmg");
        assert_eq!(hits, vec![tmp.path().join("a.dmg")]);
        assert!(env.expand_glob("~/missing").is_empty());
        assert_eq!(env.expand_glob("~/b.txt"), vec![tmp.path().join("b.txt")]);
    }

    #[test]
    fn skip_patterns() {
        let skip = vec!["Library".to_string(), ".*".to_string()];
        assert!(is_skipped(Path::new("/h/Library"), &skip));
        assert!(is_skipped(Path::new("/h/.git"), &skip));
        assert!(!is_skipped(Path::new("/h/dev"), &skip));
    }
}
