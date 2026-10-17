//! Memory pressure and the processes behind it, for the "Bebaskan memori"
//! dialog. Pure ranking logic is separated from the `sysinfo` adapter so
//! tests run on fake inputs. Relief actions live in `memory_relief`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Used-memory bands. Only the used percentage decides; swap is reported
/// alongside because on macOS and Windows some swap is normal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PressureLevel {
    /// Below 80 % used.
    Normal,
    /// 80–89 % used.
    Elevated,
    /// 90 % used or more.
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MemoryPressure {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub used_pct: u8,
    pub swap_used_bytes: u64,
    pub level: PressureLevel,
}

/// One row of the "who uses memory" list. On macOS every process of an
/// app bundle is folded into one row (Chrome and its helpers are one
/// app), so `memory_bytes` is the sum and `pid` the main process.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub memory_bytes: u64,
    pub process_count: usize,
    pub exe: Option<PathBuf>,
    /// `CFBundleIdentifier` of the outermost `.app` bundle (macOS).
    pub bundle_id: Option<String>,
    /// Whether Sapu may ask it to quit: not a system or root process,
    /// not Sapu itself.
    pub quittable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MemorySnapshot {
    pub pressure: MemoryPressure,
    /// Largest first.
    pub processes: Vec<ProcessInfo>,
}

/// What the OS adapter hands to the ranking code. Tests build these by hand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawProcess {
    pub pid: u32,
    /// `None` when unknown (Windows); root is `Some(0)`.
    pub uid: Option<u32>,
    pub name: String,
    pub memory_bytes: u64,
    pub exe: Option<PathBuf>,
}

pub fn pressure(
    total_bytes: u64,
    used_bytes: u64,
    available_bytes: u64,
    swap_used_bytes: u64,
) -> MemoryPressure {
    let used_pct = (u128::from(used_bytes) * 100)
        .checked_div(u128::from(total_bytes))
        .unwrap_or(0)
        .min(100) as u8;
    let level = if used_pct >= 90 {
        PressureLevel::High
    } else if used_pct >= 80 {
        PressureLevel::Elevated
    } else {
        PressureLevel::Normal
    };
    MemoryPressure {
        total_bytes,
        used_bytes,
        available_bytes,
        used_pct,
        swap_used_bytes,
        level,
    }
}

/// Binaries the OS owns. Quitting them is never offered.
pub fn is_system_path(p: &Path) -> bool {
    let s = p.to_string_lossy();
    ["/System/", "/usr/", "/sbin/", "/bin/"]
        .iter()
        .any(|pre| s.starts_with(pre))
        || s.get(1..).is_some_and(|rest| {
            let lower = rest.to_ascii_lowercase();
            lower.starts_with(":\\windows\\") || lower.starts_with(":/windows/")
        })
}

/// Low pids, root-owned processes, OS binaries and Sapu itself are off
/// limits. Everything else is the user's to quit.
pub fn quittable(raw: &RawProcess, own_pid: u32) -> bool {
    raw.pid >= 100
        && raw.pid != own_pid
        && raw.uid != Some(0)
        && raw.exe.as_deref().is_none_or(|e| !is_system_path(e))
}

/// Outermost `.app` ancestor of a binary: `Foo.app` for
/// `Foo.app/Contents/Frameworks/Foo Helper.app/Contents/MacOS/Foo Helper`.
pub fn app_bundle_root(exe: &Path) -> Option<PathBuf> {
    exe.ancestors()
        .filter(|a| a.extension().is_some_and(|e| e == "app"))
        .last()
        .map(Path::to_path_buf)
}

fn display_name(raw: &RawProcess) -> String {
    raw.exe
        .as_deref()
        .and_then(Path::file_name)
        .map(|n| n.to_string_lossy().into_owned())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| raw.name.clone())
}

fn bundle_display_name(root: &Path) -> String {
    root.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Groups app-bundle processes, marks what may be quit, sorts largest
/// first and keeps `top` rows. `bundle_id` is filled by `lookup`
/// (plist read on macOS, `|_| None` in tests).
pub fn rank_processes(
    raw: Vec<RawProcess>,
    own_pid: u32,
    top: usize,
    lookup: impl Fn(&Path) -> Option<String>,
) -> Vec<ProcessInfo> {
    let mut groups: HashMap<PathBuf, Vec<RawProcess>> = HashMap::new();
    let mut rows: Vec<ProcessInfo> = Vec::new();
    for p in raw {
        match p.exe.as_deref().and_then(app_bundle_root) {
            Some(root) => groups.entry(root).or_default().push(p),
            None => rows.push(ProcessInfo {
                pid: p.pid,
                name: display_name(&p),
                memory_bytes: p.memory_bytes,
                process_count: 1,
                quittable: quittable(&p, own_pid),
                bundle_id: None,
                exe: p.exe,
            }),
        }
    }
    for (root, members) in groups {
        let main_dir = root.join("Contents").join("MacOS");
        let main = members
            .iter()
            .filter(|m| m.exe.as_deref().and_then(Path::parent) == Some(main_dir.as_path()))
            .min_by_key(|m| m.pid)
            .or_else(|| members.iter().max_by_key(|m| m.memory_bytes));
        let Some(main) = main else {
            continue;
        };
        // A group is only quittable when every member is: a root-owned
        // helper means the app is not plainly the user's to close.
        let all_quittable = members.iter().all(|m| quittable(m, own_pid));
        rows.push(ProcessInfo {
            pid: main.pid,
            name: bundle_display_name(&root),
            memory_bytes: members.iter().map(|m| m.memory_bytes).sum(),
            process_count: members.len(),
            quittable: all_quittable && !is_system_path(&root),
            bundle_id: lookup(&root),
            exe: main.exe.clone(),
        });
    }
    rows.sort_by(|a, b| {
        b.memory_bytes
            .cmp(&a.memory_bytes)
            .then_with(|| a.name.cmp(&b.name))
    });
    rows.truncate(top);
    rows
}

/// Live reading: pressure plus the `top` heaviest apps / processes.
pub fn snapshot(top: usize) -> MemorySnapshot {
    let mut sys = adapter::system_with_processes();
    sys.refresh_memory();
    let pressure = pressure(
        sys.total_memory(),
        sys.used_memory(),
        sys.available_memory(),
        sys.used_swap(),
    );
    let processes = rank_processes(
        adapter::raw_processes(&sys),
        std::process::id(),
        top,
        adapter::bundle_id_of,
    );
    MemorySnapshot {
        pressure,
        processes,
    }
}

/// Fresh facts about the given pids only (re-validated right before a
/// quit, like the safety layer re-checks files before disposal).
pub fn processes_by_pid(pids: &[u32]) -> Vec<ProcessInfo> {
    let sys = adapter::system_with_pids(pids);
    rank_processes(
        adapter::raw_processes(&sys),
        std::process::id(),
        usize::MAX,
        adapter::bundle_id_of,
    )
}

pub(super) mod adapter {
    use super::RawProcess;
    use std::path::Path;
    use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

    fn kind() -> ProcessRefreshKind {
        ProcessRefreshKind::nothing()
            .with_memory()
            .with_exe(UpdateKind::OnlyIfNotSet)
            .with_user(UpdateKind::OnlyIfNotSet)
    }

    pub(super) fn system_with_processes() -> System {
        let mut sys = System::new();
        sys.refresh_processes_specifics(ProcessesToUpdate::All, true, kind());
        sys
    }

    pub(super) fn system_with_pids(pids: &[u32]) -> System {
        let pids: Vec<Pid> = pids.iter().map(|p| Pid::from_u32(*p)).collect();
        let mut sys = System::new();
        sys.refresh_processes_specifics(ProcessesToUpdate::Some(&pids), true, kind());
        sys
    }

    pub(super) fn raw_processes(sys: &System) -> Vec<RawProcess> {
        sys.processes()
            .values()
            .map(|p| RawProcess {
                pid: p.pid().as_u32(),
                uid: uid_of(p),
                name: p.name().to_string_lossy().into_owned(),
                memory_bytes: p.memory(),
                exe: p.exe().map(Path::to_path_buf),
            })
            .collect()
    }

    #[cfg(unix)]
    fn uid_of(p: &sysinfo::Process) -> Option<u32> {
        p.user_id().map(|u| **u)
    }

    #[cfg(not(unix))]
    fn uid_of(_p: &sysinfo::Process) -> Option<u32> {
        None
    }

    /// `CFBundleIdentifier` from the bundle's Info.plist (macOS).
    #[cfg(target_os = "macos")]
    pub(super) fn bundle_id_of(root: &Path) -> Option<String> {
        let info: plist::Dictionary = plist::from_file(root.join("Contents/Info.plist")).ok()?;
        info.get("CFBundleIdentifier")
            .and_then(|v| v.as_string())
            .map(str::to_string)
    }

    #[cfg(not(target_os = "macos"))]
    pub(super) fn bundle_id_of(_root: &Path) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GB: u64 = 1024 * 1024 * 1024;

    fn raw(pid: u32, uid: u32, exe: &str, mem: u64) -> RawProcess {
        RawProcess {
            pid,
            uid: Some(uid),
            name: Path::new(exe)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            memory_bytes: mem,
            exe: Some(PathBuf::from(exe)),
        }
    }

    #[test]
    fn pressure_bands() {
        assert_eq!(
            pressure(16 * GB, 8 * GB, 8 * GB, 0).level,
            PressureLevel::Normal
        );
        assert_eq!(pressure(16 * GB, 8 * GB, 8 * GB, 0).used_pct, 50);
        let e = pressure(100, 80, 20, 5);
        assert_eq!(e.level, PressureLevel::Elevated);
        assert_eq!(e.swap_used_bytes, 5);
        assert_eq!(pressure(100, 89, 11, 0).level, PressureLevel::Elevated);
        assert_eq!(pressure(100, 90, 10, 0).level, PressureLevel::High);
        assert_eq!(pressure(0, 0, 0, 0).used_pct, 0);
        assert_eq!(pressure(10, 50, 0, 0).used_pct, 100);
    }

    #[test]
    fn system_processes_are_never_quittable() {
        let own = 4242;
        assert!(!quittable(
            &raw(50, 501, "/Applications/X.app/Contents/MacOS/X", 1),
            own
        ));
        assert!(!quittable(
            &raw(500, 0, "/Applications/X.app/Contents/MacOS/X", 1),
            own
        ));
        assert!(!quittable(
            &raw(
                500,
                501,
                "/System/Library/CoreServices/Dock.app/Contents/MacOS/Dock",
                1
            ),
            own
        ));
        assert!(!quittable(&raw(500, 501, "/usr/libexec/trustd", 1), own));
        assert!(!quittable(&raw(500, 501, "/sbin/launchd", 1), own));
        assert!(!quittable(
            &raw(500, 501, "C:\\Windows\\System32\\svchost.exe", 1),
            own
        ));
        assert!(!quittable(
            &raw(own, 501, "/Applications/Sapu.app/Contents/MacOS/sapu", 1),
            own
        ));
        assert!(quittable(
            &raw(500, 501, "/Applications/Slack.app/Contents/MacOS/Slack", 1),
            own
        ));
        assert!(quittable(&raw(500, 501, "/opt/homebrew/bin/node", 1), own));
        let unknown = RawProcess {
            pid: 900,
            uid: None,
            name: "x".into(),
            memory_bytes: 1,
            exe: None,
        };
        assert!(quittable(&unknown, own));
    }

    #[test]
    fn bundle_root_is_the_outermost_app() {
        let helper = Path::new(
            "/Applications/Google Chrome.app/Contents/Frameworks/Google Chrome Framework.framework/Versions/1/Helpers/Google Chrome Helper (Renderer).app/Contents/MacOS/Google Chrome Helper (Renderer)",
        );
        assert_eq!(
            app_bundle_root(helper),
            Some(PathBuf::from("/Applications/Google Chrome.app"))
        );
        assert_eq!(app_bundle_root(Path::new("/usr/bin/zsh")), None);
    }

    #[test]
    fn ranks_apps_by_total_memory_and_groups_helpers() {
        let own = 99;
        let chrome = "/Applications/Google Chrome.app";
        let input = vec![
            raw(
                300,
                501,
                &format!("{chrome}/Contents/MacOS/Google Chrome"),
                GB,
            ),
            raw(
                310,
                501,
                &format!(
                    "{chrome}/Contents/Frameworks/Google Chrome Framework.framework/Versions/1/Helpers/Google Chrome Helper (Renderer).app/Contents/MacOS/Google Chrome Helper (Renderer)"
                ),
                2 * GB,
            ),
            raw(
                320,
                501,
                &format!(
                    "{chrome}/Contents/Frameworks/Google Chrome Framework.framework/Versions/1/Helpers/Google Chrome Helper (GPU).app/Contents/MacOS/Google Chrome Helper (GPU)"
                ),
                GB,
            ),
            raw(
                400,
                501,
                "/Applications/Slack.app/Contents/MacOS/Slack",
                3 * GB,
            ),
            raw(150, 0, "/usr/libexec/kernelmanagerd", 5 * GB),
            raw(600, 501, "/opt/homebrew/bin/node", 512 * 1024 * 1024),
            raw(own, 501, "/Applications/Sapu.app/Contents/MacOS/sapu", 100),
        ];
        let rows = rank_processes(input, own, 3, |root| {
            root.ends_with("Slack.app")
                .then(|| "com.tinyspeck.slackmacgap".to_string())
        });
        assert_eq!(rows.len(), 3);
        // kernelmanagerd (5 GB) is first but never quittable.
        assert_eq!(rows[0].name, "kernelmanagerd");
        assert!(!rows[0].quittable);
        assert_eq!(rows[0].process_count, 1);
        // Chrome: 4 GB over three processes, main pid is the Contents/MacOS one.
        assert_eq!(rows[1].name, "Google Chrome");
        assert_eq!(rows[1].memory_bytes, 4 * GB);
        assert_eq!(rows[1].process_count, 3);
        assert_eq!(rows[1].pid, 300);
        assert!(rows[1].quittable);
        assert_eq!(rows[1].bundle_id, None);
        assert_eq!(rows[2].name, "Slack");
        assert_eq!(
            rows[2].bundle_id.as_deref(),
            Some("com.tinyspeck.slackmacgap")
        );
        assert!(rows[2].quittable);
    }

    #[test]
    fn sapu_itself_and_root_helpers_block_the_group() {
        let own = 77;
        let rows = rank_processes(
            vec![
                raw(own, 501, "/Applications/Sapu.app/Contents/MacOS/sapu", 10),
                raw(
                    200,
                    501,
                    "/Applications/Docker.app/Contents/MacOS/Docker",
                    10,
                ),
                raw(
                    201,
                    0,
                    "/Applications/Docker.app/Contents/MacOS/com.docker.backend",
                    10,
                ),
            ],
            own,
            10,
            |_| None,
        );
        let sapu = rows.iter().find(|r| r.name == "Sapu").expect("sapu row");
        assert!(!sapu.quittable);
        let docker = rows
            .iter()
            .find(|r| r.name == "Docker")
            .expect("docker row");
        assert!(!docker.quittable);
        assert_eq!(docker.process_count, 2);
    }

    #[test]
    fn ties_are_sorted_by_name_and_names_fall_back_to_process_name() {
        let own = 1;
        let mut nameless = raw(500, 501, "/opt/x/b", 5);
        nameless.exe = None;
        nameless.name = "zeta".into();
        let rows = rank_processes(
            vec![raw(501, 501, "/opt/x/alpha", 5), nameless],
            own,
            10,
            |_| None,
        );
        assert_eq!(rows[0].name, "alpha");
        assert_eq!(rows[1].name, "zeta");
    }

    #[test]
    fn live_snapshot_has_the_right_shape() {
        let s = snapshot(5);
        assert!(s.pressure.total_bytes > 0);
        assert!(s.processes.len() <= 5);
        assert!(
            s.processes
                .windows(2)
                .all(|w| w[0].memory_bytes >= w[1].memory_bytes)
        );
        assert!(
            s.processes
                .iter()
                .all(|p| p.pid != std::process::id() || !p.quittable)
        );
    }
}
