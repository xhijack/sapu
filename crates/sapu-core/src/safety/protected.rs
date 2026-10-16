//! Protected paths (S3): locations Sapu must never remove, whatever rules,
//! user configuration or AI say. The set is append-only by construction.

use std::path::{Component, Path, PathBuf};

/// How a protected entry applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Protection {
    /// Only the path itself is protected; children may be removed by explicit
    /// user action (e.g. the user's Documents folder).
    Exact,
    /// The path and everything below it are protected (e.g. `~/.ssh`).
    Subtree,
}

/// One protected entry.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProtectedEntry {
    pub path: PathBuf,
    pub protection: Protection,
    /// Stable identifier used by the UI for an explanation, never a user path.
    pub reason: &'static str,
}

/// The append-only set of protected paths.
#[derive(Debug, Clone)]
pub struct ProtectedPaths {
    entries: Vec<ProtectedEntry>,
}

impl ProtectedPaths {
    /// The compiled-in defaults for the current OS plus the user's credential
    /// and personal-data roots. This is the only public constructor, so no
    /// caller can start from an empty set.
    pub fn os_defaults() -> Self {
        let mut set = Self {
            entries: Vec::new(),
        };
        platform::add_system_defaults(&mut set);
        add_user_defaults(&mut set);
        set
    }

    /// Append an entry. There is intentionally no way to remove one.
    pub fn add(&mut self, path: impl Into<PathBuf>, protection: Protection, reason: &'static str) {
        let path = path.into();
        if path.as_os_str().is_empty() {
            return;
        }
        self.entries.push(ProtectedEntry {
            path,
            protection,
            reason,
        });
    }

    pub fn entries(&self) -> &[ProtectedEntry] {
        &self.entries
    }

    /// Returns the entry that protects `path`, if any.
    pub fn check(&self, path: &Path) -> Option<&ProtectedEntry> {
        let lexical = lexical_absolute(path);
        let canonical = best_effort_canonical(&lexical);
        self.entries.iter().find(|entry| {
            let root_lexical = lexical_absolute(&entry.path);
            let root_canonical = best_effort_canonical(&root_lexical);
            matches(&lexical, &root_lexical, entry.protection)
                || matches(&canonical, &root_canonical, entry.protection)
        })
    }

    pub fn is_protected(&self, path: &Path) -> bool {
        self.check(path).is_some()
    }
}

fn matches(path: &Path, root: &Path, protection: Protection) -> bool {
    match protection {
        Protection::Exact => path == root,
        Protection::Subtree => path.starts_with(root),
    }
}

/// Absolute path with `.` and `..` components resolved lexically. Does not
/// touch the filesystem.
pub(crate) fn lexical_absolute(path: &Path) -> PathBuf {
    let abs = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let mut out = PathBuf::new();
    for comp in abs.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Canonicalizes the longest existing prefix of `path` and re-appends the
/// rest, so symlinked ancestors are resolved even for paths that do not
/// exist yet. The input must already be absolute.
pub(crate) fn best_effort_canonical(path: &Path) -> PathBuf {
    let mut rest = Vec::new();
    let mut cur = path;
    loop {
        if let Ok(canon) = cur.canonicalize() {
            let mut out = canon;
            for comp in rest.iter().rev() {
                out.push(comp);
            }
            return out;
        }
        match (cur.parent(), cur.file_name()) {
            (Some(parent), Some(name)) => {
                rest.push(name.to_os_string());
                cur = parent;
            }
            _ => return path.to_path_buf(),
        }
    }
}

fn add_user_defaults(set: &mut ProtectedPaths) {
    if let Some(home) = dirs::home_dir() {
        set.add(&home, Protection::Exact, "home");
        for (name, reason) in [
            (".ssh", "credentials"),
            (".gnupg", "credentials"),
            (".aws", "credentials"),
            (".kube", "credentials"),
            (".azure", "credentials"),
            (".netrc", "credentials"),
            (".password-store", "credentials"),
            (".config/gcloud", "credentials"),
            (".docker/config.json", "credentials"),
        ] {
            set.add(home.join(name), Protection::Subtree, reason);
        }
    }
    for (dir, reason) in [
        (dirs::document_dir(), "user-data-root"),
        (dirs::picture_dir(), "user-data-root"),
        (dirs::desktop_dir(), "user-data-root"),
        (dirs::download_dir(), "user-data-root"),
        (dirs::audio_dir(), "user-data-root"),
        (dirs::video_dir(), "user-data-root"),
    ] {
        if let Some(dir) = dir {
            set.add(dir, Protection::Exact, reason);
        }
    }
}

mod platform {
    use super::{ProtectedPaths, Protection};

    #[cfg(target_os = "macos")]
    pub(super) fn add_system_defaults(set: &mut ProtectedPaths) {
        set.add("/", Protection::Exact, "filesystem-root");
        set.add("/Volumes", Protection::Exact, "filesystem-root");
        set.add("/Users", Protection::Exact, "filesystem-root");
        for p in [
            "/System",
            "/Library",
            "/usr",
            "/bin",
            "/sbin",
            "/etc",
            "/private/etc",
            "/private/var/db",
            "/cores",
        ] {
            set.add(p, Protection::Subtree, "os-system");
        }
        // The folder itself stays; individual bundles may be uninstalled
        // through the safety layer. Apple's own apps live in /System/Applications.
        set.add("/Applications", Protection::Exact, "os-system");
        set.add("/Applications/Utilities", Protection::Exact, "os-system");
        if let Some(home) = dirs::home_dir() {
            set.add(
                home.join("Library/Keychains"),
                Protection::Subtree,
                "credentials",
            );
            set.add(home.join("Library"), Protection::Exact, "user-data-root");
        }
    }

    #[cfg(target_os = "linux")]
    pub(super) fn add_system_defaults(set: &mut ProtectedPaths) {
        set.add("/", Protection::Exact, "filesystem-root");
        set.add("/home", Protection::Exact, "filesystem-root");
        set.add("/var", Protection::Exact, "filesystem-root");
        set.add("/opt", Protection::Exact, "filesystem-root");
        set.add("/mnt", Protection::Exact, "filesystem-root");
        set.add("/media", Protection::Exact, "filesystem-root");
        for p in [
            "/bin", "/boot", "/dev", "/etc", "/lib", "/lib32", "/lib64", "/proc", "/run", "/sbin",
            "/sys", "/usr", "/var/lib", "/snap",
        ] {
            set.add(p, Protection::Subtree, "os-system");
        }
    }

    #[cfg(target_os = "windows")]
    pub(super) fn add_system_defaults(set: &mut ProtectedPaths) {
        let var = |k: &str| std::env::var_os(k).map(std::path::PathBuf::from);
        if let Some(drive) = var("SystemDrive") {
            let mut root = drive;
            root.push("\\");
            set.add(root, Protection::Exact, "filesystem-root");
        }
        for key in [
            "SystemRoot",
            "ProgramFiles",
            "ProgramFiles(x86)",
            "ProgramW6432",
        ] {
            if let Some(p) = var(key) {
                set.add(p, Protection::Subtree, "os-system");
            }
        }
        if let Some(p) = var("ProgramData") {
            set.add(p, Protection::Exact, "os-system");
        }
        if let Some(p) = var("USERPROFILE") {
            set.add(p, Protection::Exact, "home");
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    pub(super) fn add_system_defaults(set: &mut ProtectedPaths) {
        set.add("/", Protection::Exact, "filesystem-root");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_never_empty() {
        assert!(!ProtectedPaths::os_defaults().entries().is_empty());
    }

    #[test]
    fn subtree_protects_children_and_self() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let secret = tmp.path().join("secret");
        std::fs::create_dir_all(secret.join("deep")).expect("mkdir");
        let mut set = ProtectedPaths::os_defaults();
        set.add(&secret, Protection::Subtree, "test");
        assert!(set.is_protected(&secret));
        assert!(set.is_protected(&secret.join("deep")));
        assert!(set.is_protected(&secret.join("deep/not-yet-created.txt")));
        assert!(!set.is_protected(&tmp.path().join("other")));
    }

    #[test]
    fn exact_protects_only_the_root() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let docs = tmp.path().join("Documents");
        std::fs::create_dir_all(&docs).expect("mkdir");
        let mut set = ProtectedPaths::os_defaults();
        set.add(&docs, Protection::Exact, "test");
        assert!(set.is_protected(&docs));
        assert!(!set.is_protected(&docs.join("report.docx")));
    }

    #[test]
    fn dot_dot_cannot_escape_protection() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let secret = tmp.path().join("secret");
        std::fs::create_dir_all(&secret).expect("mkdir");
        let mut set = ProtectedPaths::os_defaults();
        set.add(&secret, Protection::Subtree, "test");
        let sneaky = tmp.path().join("other/../secret/key");
        assert!(set.is_protected(&sneaky));
    }

    #[cfg(unix)]
    #[test]
    fn symlink_into_protected_root_is_protected() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let secret = tmp.path().join("secret");
        std::fs::create_dir_all(&secret).expect("mkdir");
        let link = tmp.path().join("link");
        std::os::unix::fs::symlink(&secret, &link).expect("symlink");
        let mut set = ProtectedPaths::os_defaults();
        set.add(&secret, Protection::Subtree, "test");
        assert!(set.is_protected(&link.join("id_rsa")));
    }

    #[test]
    fn home_and_credentials_are_protected_by_default() {
        let set = ProtectedPaths::os_defaults();
        if let Some(home) = dirs::home_dir() {
            assert!(set.is_protected(&home));
            assert!(set.is_protected(&home.join(".ssh/id_ed25519")));
            // A random file directly under home is not protected.
            assert!(!set.is_protected(&home.join("sapu-does-not-exist-xyz.tmp")));
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn applications_folder_protected_but_bundles_removable() {
        let set = ProtectedPaths::os_defaults();
        assert!(set.is_protected(Path::new("/Applications")));
        assert!(!set.is_protected(Path::new("/Applications/SomeApp.app")));
        assert!(set.is_protected(Path::new("/System/Applications/Safari.app")));
    }

    #[test]
    fn empty_path_is_ignored() {
        let mut set = ProtectedPaths::os_defaults();
        let before = set.entries().len();
        set.add("", Protection::Subtree, "test");
        assert_eq!(set.entries().len(), before);
    }
}
