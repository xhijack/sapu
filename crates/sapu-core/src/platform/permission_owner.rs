//! Which app macOS asks about for Full Disk Access. A packaged Sapu.app owns
//! its own permission; a development build started from a terminal or an
//! editor inherits the permission of that app, so that is the one to enable.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PermissionOwner {
    /// Name as it appears in System Settings, e.g. "Visual Studio Code".
    pub name: String,
    /// The `.app` bundle to add with the + button.
    pub path: PathBuf,
    /// True when it is not Sapu itself (a development build).
    pub inherited: bool,
}

/// The outermost `.app` bundle containing `exe`, with its display name.
pub fn outermost_app(exe: &Path) -> Option<(String, PathBuf)> {
    let bundle = exe
        .ancestors()
        .filter(|p| p.extension().is_some_and(|e| e == "app"))
        .last()?;
    let name = bundle.file_stem()?.to_string_lossy().into_owned();
    Some((name, bundle.to_path_buf()))
}

/// The first executable in `chain` (self, parent, grandparent, ...) that
/// lives in an app bundle.
pub fn owner_from_chain(chain: &[PathBuf]) -> Option<PermissionOwner> {
    chain.iter().enumerate().find_map(|(i, exe)| {
        outermost_app(exe).map(|(name, path)| PermissionOwner {
            name,
            path,
            inherited: i > 0,
        })
    })
}

/// The current process's permission owner, found by walking parent
/// processes. `None` when no app bundle is found (e.g. started by launchd).
pub fn permission_owner() -> Option<PermissionOwner> {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
    let mut sys = System::new();
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().with_exe(UpdateKind::Always),
    );
    let mut chain = Vec::new();
    let mut pid = sysinfo::get_current_pid().ok();
    // Bounded walk: process trees are shallow; guards against odd cycles.
    for _ in 0..32 {
        let Some(p) = pid.and_then(|id| sys.process(id)) else {
            break;
        };
        if let Some(exe) = p.exe() {
            chain.push(exe.to_path_buf());
        }
        pid = p.parent();
    }
    owner_from_chain(&chain)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_outermost_bundle_is_the_app_people_see() {
        let helper = Path::new(
            "/Applications/Visual Studio Code.app/Contents/Frameworks/Code Helper (Plugin).app/Contents/MacOS/Code Helper (Plugin)",
        );
        assert_eq!(
            outermost_app(helper),
            Some((
                "Visual Studio Code".into(),
                PathBuf::from("/Applications/Visual Studio Code.app")
            ))
        );
        assert_eq!(
            outermost_app(Path::new("/Users/x/sapu/target/debug/sapu-app")),
            None
        );
    }

    #[test]
    fn a_packaged_sapu_owns_its_permission_and_a_dev_build_inherits_it() {
        let packaged = [PathBuf::from(
            "/Applications/Sapu.app/Contents/MacOS/sapu-app",
        )];
        let own = owner_from_chain(&packaged).expect("owner");
        assert_eq!(own.name, "Sapu");
        assert!(!own.inherited);

        let dev = [
            PathBuf::from("/Users/x/sapu/target/debug/sapu-app"),
            PathBuf::from("/opt/homebrew/bin/node"),
            PathBuf::from("/bin/zsh"),
            PathBuf::from(
                "/Applications/Visual Studio Code.app/Contents/Frameworks/Code Helper (Plugin).app/Contents/MacOS/Code Helper (Plugin)",
            ),
            PathBuf::from("/Applications/Visual Studio Code.app/Contents/MacOS/Electron"),
        ];
        let owner = owner_from_chain(&dev).expect("owner");
        assert_eq!(owner.name, "Visual Studio Code");
        assert_eq!(
            owner.path,
            PathBuf::from("/Applications/Visual Studio Code.app")
        );
        assert!(owner.inherited);
        assert_eq!(owner_from_chain(&dev[..3]), None, "no app in the chain");
    }

    #[test]
    fn looking_up_the_running_process_never_panics() {
        let _ = permission_owner();
    }
}
