//! Detects developer tooling so onboarding can suggest the Developer profile.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DevTool {
    Docker,
    Node,
    Cargo,
    Python,
    Go,
    Xcode,
    AndroidSdk,
    Git,
}

impl DevTool {
    fn binaries(self) -> &'static [&'static str] {
        match self {
            Self::Docker => &["docker"],
            Self::Node => &["node"],
            Self::Cargo => &["cargo"],
            Self::Python => &["python3", "python", "pip", "pip3"],
            Self::Go => &["go"],
            Self::Xcode => &["xcodebuild"],
            Self::AndroidSdk => &["adb"],
            Self::Git => &["git"],
        }
    }

    fn home_markers(self) -> &'static [&'static str] {
        match self {
            Self::Docker => &[".docker"],
            Self::Node => &[".npm", ".nvm", ".pnpm-store", ".yarn"],
            Self::Cargo => &[".cargo"],
            Self::Python => &[".pyenv", ".cache/pip"],
            Self::Go => &["go/pkg", ".cache/go-build"],
            Self::Xcode => &["Library/Developer/Xcode"],
            Self::AndroidSdk => &["Library/Android/sdk", "Android/Sdk", ".android"],
            Self::Git => &[".gitconfig"],
        }
    }

    const ALL: [DevTool; 8] = [
        Self::Docker,
        Self::Node,
        Self::Cargo,
        Self::Python,
        Self::Go,
        Self::Xcode,
        Self::AndroidSdk,
        Self::Git,
    ];
}

/// Tools found on `PATH` or by well-known folders in the home directory.
pub fn detect_dev_tools() -> Vec<DevTool> {
    let path_dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    detect_dev_tools_in(&path_dirs, dirs::home_dir().as_deref())
}

/// Testable core of [`detect_dev_tools`].
pub fn detect_dev_tools_in(path_dirs: &[PathBuf], home: Option<&Path>) -> Vec<DevTool> {
    DevTool::ALL
        .into_iter()
        .filter(|tool| {
            let on_path = tool
                .binaries()
                .iter()
                .any(|bin| path_dirs.iter().any(|d| binary_exists(d, bin)));
            let in_home =
                home.is_some_and(|h| tool.home_markers().iter().any(|m| h.join(m).exists()));
            on_path || in_home
        })
        .collect()
}

fn binary_exists(dir: &Path, name: &str) -> bool {
    if dir.join(name).is_file() {
        return true;
    }
    cfg!(windows)
        && ["exe", "cmd", "bat"]
            .iter()
            .any(|ext| dir.join(format!("{name}.{ext}")).is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_tools_on_path_and_in_home() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let bin = tmp.path().join("bin");
        std::fs::create_dir_all(&bin).expect("mkdir");
        std::fs::write(
            bin.join(if cfg!(windows) {
                "docker.exe"
            } else {
                "docker"
            }),
            b"",
        )
        .expect("write");
        let home = tmp.path().join("home");
        std::fs::create_dir_all(home.join(".cargo")).expect("mkdir");

        let found = detect_dev_tools_in(&[bin], Some(&home));
        assert!(found.contains(&DevTool::Docker));
        assert!(found.contains(&DevTool::Cargo));
        assert!(!found.contains(&DevTool::Go));
    }

    #[test]
    fn nothing_found_in_empty_env() {
        let tmp = tempfile::tempdir().expect("tempdir");
        assert!(detect_dev_tools_in(&[], Some(tmp.path())).is_empty());
    }
}
