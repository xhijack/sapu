//! Safe, explainable memory relief. The only place in core that spawns
//! processes for it (like `cleaner/command.rs` for clean-up commands).
//!
//! Exactly two actions:
//! 1. Ask apps the user picked to quit *gracefully*: a Quit Apple event on
//!    macOS (the app may ask to save), `taskkill /PID` without `/F` on
//!    Windows, SIGTERM on Linux. Never SIGKILL.
//! 2. macOS only: Apple's own `/usr/sbin/purge` through the administrator
//!    prompt. It drops the file cache, which macOS refills; mostly
//!    cosmetic, so the UI says so.
//!
//! Deliberately NOT implemented: the "memory balloon" trick (allocate a
//! large block so the OS pages other apps out, then free it). It makes the
//! used figure drop for a moment while making every other app slower when
//! it swaps back in, and it fights the OS's own memory management. That is
//! a fake optimisation (PRD: performance tuning is a non-goal), so Sapu
//! does not do it.

use std::process::{Command, Stdio};
use std::time::Duration;

use super::memory::{ProcessInfo, processes_by_pid};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuitMethod {
    /// `tell application id "…" to quit` (macOS, app bundles).
    AppleEvent,
    /// `kill -15 <pid>` (macOS without a bundle id, Linux).
    Sigterm,
    /// `taskkill /PID <pid>` without `/F` (Windows).
    Taskkill,
    /// Not quittable any more (gone, system, root, Sapu itself).
    Refused,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct QuitOutcome {
    pub pid: u32,
    pub name: String,
    pub method: QuitMethod,
    /// The request was delivered. Whether the app really quit is up to
    /// it (it may ask about unsaved work first).
    pub requested: bool,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PurgeOutcome {
    pub ok: bool,
    /// The user dismissed the administrator prompt.
    pub cancelled: bool,
    pub used_before_bytes: u64,
    pub used_after_bytes: u64,
    pub message: String,
}

const QUIT_TIMEOUT: Duration = Duration::from_secs(10);
/// The admin prompt waits for a person.
#[cfg(target_os = "macos")]
const PURGE_TIMEOUT: Duration = Duration::from_secs(180);

/// Asks each pid to quit gracefully after re-checking that it is still
/// the user's to quit. Order of the result follows `pids`.
pub fn quit_gracefully(pids: &[u32]) -> Vec<QuitOutcome> {
    let fresh = processes_by_pid(pids);
    pids.iter()
        .map(|pid| match fresh.iter().find(|p| p.pid == *pid) {
            Some(p) if p.quittable => quit_one(p),
            Some(p) => QuitOutcome {
                pid: *pid,
                name: p.name.clone(),
                method: QuitMethod::Refused,
                requested: false,
                message: "not a user app".into(),
            },
            None => QuitOutcome {
                pid: *pid,
                name: String::new(),
                method: QuitMethod::Refused,
                requested: false,
                message: "already gone".into(),
            },
        })
        .collect()
}

fn quit_one(p: &ProcessInfo) -> QuitOutcome {
    let (method, result) = platform::quit(p);
    let (requested, message) = match result {
        Ok(()) => (true, String::new()),
        Err(e) => (false, e),
    };
    QuitOutcome {
        pid: p.pid,
        name: p.name.clone(),
        method,
        requested,
        message,
    }
}

/// macOS: `purge` with the administrator prompt. Elsewhere an error.
pub fn purge_file_cache() -> PurgeOutcome {
    platform::purge()
}

/// Runs a program and reports success or the error text. Never
/// interprets output beyond that.
fn run(program: &str, args: &[String], timeout: Duration) -> Result<String, String> {
    let child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let mut child = child.map_err(|e| e.to_string())?;
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() > timeout => {
                let _ = child.kill();
                return Err("timed out".into());
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(e) => return Err(e.to_string()),
        }
    }
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(if err.is_empty() {
            format!("exit status {}", out.status)
        } else {
            err
        })
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::*;

    pub(super) fn quit(p: &ProcessInfo) -> (QuitMethod, Result<(), String>) {
        match &p.bundle_id {
            // `ignoring application responses` sends the Quit event and
            // returns at once; the app handles it (and may ask to save).
            Some(id) if !id.is_empty() && !id.contains('"') => {
                let script = format!(
                    "ignoring application responses\ntell application id \"{id}\" to quit\nend ignoring"
                );
                let r = run("/usr/bin/osascript", &["-e".into(), script], QUIT_TIMEOUT).map(|_| ());
                (QuitMethod::AppleEvent, r)
            }
            _ => (QuitMethod::Sigterm, sigterm(p.pid)),
        }
    }

    pub(super) fn purge() -> PurgeOutcome {
        let before = super::super::memory::snapshot(0).pressure.used_bytes;
        let script = "do shell script \"/usr/sbin/purge\" with administrator privileges";
        let r = run(
            "/usr/bin/osascript",
            &["-e".into(), script.into()],
            PURGE_TIMEOUT,
        );
        let after = super::super::memory::snapshot(0).pressure.used_bytes;
        match r {
            Ok(_) => PurgeOutcome {
                ok: true,
                cancelled: false,
                used_before_bytes: before,
                used_after_bytes: after,
                message: String::new(),
            },
            Err(e) => PurgeOutcome {
                ok: false,
                cancelled: e.contains("-128"),
                used_before_bytes: before,
                used_after_bytes: after,
                message: e,
            },
        }
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use super::*;

    pub(super) fn quit(p: &ProcessInfo) -> (QuitMethod, Result<(), String>) {
        (QuitMethod::Sigterm, sigterm(p.pid))
    }

    pub(super) fn purge() -> PurgeOutcome {
        unsupported()
    }
}

#[cfg(windows)]
mod platform {
    use super::*;

    /// `taskkill` without `/F` asks the window to close; it does not kill.
    pub(super) fn quit(p: &ProcessInfo) -> (QuitMethod, Result<(), String>) {
        let r = run(
            "taskkill",
            &["/PID".into(), p.pid.to_string()],
            QUIT_TIMEOUT,
        )
        .map(|_| ());
        (QuitMethod::Taskkill, r)
    }

    pub(super) fn purge() -> PurgeOutcome {
        unsupported()
    }
}

#[cfg(unix)]
fn sigterm(pid: u32) -> Result<(), String> {
    run("kill", &["-15".into(), pid.to_string()], QUIT_TIMEOUT).map(|_| ())
}

#[cfg(not(target_os = "macos"))]
fn unsupported() -> PurgeOutcome {
    PurgeOutcome {
        ok: false,
        cancelled: false,
        used_before_bytes: 0,
        used_after_bytes: 0,
        message: "purge is macOS only".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_pids_that_are_not_user_apps() {
        // pid 1 is launchd / init / System: below the pid floor and root.
        let out = quit_gracefully(&[1]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].method, QuitMethod::Refused);
        assert!(!out[0].requested);
        // Sapu's own process is never quittable.
        let own = quit_gracefully(&[std::process::id()]);
        assert_eq!(own[0].method, QuitMethod::Refused);
        // A pid that does not exist.
        let gone = quit_gracefully(&[u32::MAX - 7]);
        assert_eq!(gone[0].method, QuitMethod::Refused);
        assert_eq!(gone[0].message, "already gone");
    }

    #[test]
    fn run_reports_failures_as_text() {
        let r = run("/definitely/not/a/program", &[], Duration::from_secs(1));
        assert!(r.is_err());
    }

    #[cfg(unix)]
    #[test]
    fn run_captures_output_and_exit_status() {
        assert_eq!(run("true", &[], Duration::from_secs(5)), Ok(String::new()));
        assert!(run("false", &[], Duration::from_secs(5)).is_err());
    }
}
