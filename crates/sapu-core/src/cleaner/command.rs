//! Runs official clean-up commands (e.g. `docker system prune`). The only
//! place in core that spawns processes for clean-up.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command as Process;
use std::time::Duration;

use crate::rules::Command;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CommandOutcome {
    pub command_line: String,
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

pub fn command_line(cmd: &Command) -> String {
    std::iter::once(cmd.program.as_str())
        .chain(cmd.args.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Is the required binary on PATH (or in a well-known tool folder)?
pub fn is_available(cmd: &Command) -> bool {
    let bin = cmd.requires.as_deref().unwrap_or(&cmd.program);
    resolve_program(bin).is_some()
}

/// Folders searched after `PATH`. GUI apps launched from Finder or a
/// desktop get a minimal `PATH` that misses Homebrew and Docker Desktop.
fn extra_bin_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    #[cfg(target_os = "macos")]
    {
        dirs.push("/usr/local/bin".into());
        dirs.push("/opt/homebrew/bin".into());
        dirs.push("/Applications/Docker.app/Contents/Resources/bin".into());
        if let Some(h) = dirs::home_dir() {
            dirs.push(h.join(".docker/bin"));
            dirs.push(h.join(".cargo/bin"));
        }
    }
    #[cfg(target_os = "linux")]
    {
        dirs.push("/usr/local/bin".into());
        dirs.push("/snap/bin".into());
        if let Some(h) = dirs::home_dir() {
            dirs.push(h.join(".local/bin"));
            dirs.push(h.join(".cargo/bin"));
        }
    }
    #[cfg(windows)]
    if let Some(pf) = std::env::var_os("ProgramFiles") {
        dirs.push(PathBuf::from(pf).join("Docker/Docker/resources/bin"));
    }
    dirs
}

/// Absolute path of `bin`, searching `PATH` then [`extra_bin_dirs`].
pub fn resolve_program(bin: &str) -> Option<PathBuf> {
    if Path::new(bin).is_absolute() {
        return Path::new(bin).is_file().then(|| PathBuf::from(bin));
    }
    let from_path = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect::<Vec<_>>())
        .unwrap_or_default();
    from_path.into_iter().chain(extra_bin_dirs()).find_map(|d| {
        let direct = d.join(bin);
        if direct.is_file() {
            return Some(direct);
        }
        if cfg!(windows) {
            for e in ["exe", "cmd", "bat"] {
                let p = d.join(format!("{bin}.{e}"));
                if p.is_file() {
                    return Some(p);
                }
            }
        }
        None
    })
}

fn program_for(bin: &str) -> PathBuf {
    resolve_program(bin).unwrap_or_else(|| PathBuf::from(bin))
}

/// Runs `cmd` with a timeout; never interprets the output.
pub fn run_command(cmd: &Command, timeout: Duration) -> CommandOutcome {
    let command_line = command_line(cmd);
    let child = Process::new(program_for(&cmd.program))
        .args(&cmd.args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(e) => {
            return CommandOutcome {
                command_line,
                success: false,
                stdout: String::new(),
                stderr: e.to_string(),
            };
        }
    };
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() > timeout => {
                let _ = child.kill();
                break;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(_) => break,
        }
    }
    match child.wait_with_output() {
        Ok(out) => CommandOutcome {
            command_line,
            success: out.status.success(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        },
        Err(e) => CommandOutcome {
            command_line,
            success: false,
            stdout: String::new(),
            stderr: e.to_string(),
        },
    }
}

/// Runs the rule's `estimate` command and sums every size-looking token
/// in its output (e.g. docker's `1.2GB (40%)` lines). Best effort.
/// Output of estimate commands already run during one evaluation, keyed by
/// the full command line, so four Docker rules cost one `docker system df`.
#[derive(Default)]
pub struct EstimateCache(HashMap<Vec<String>, Option<String>>);

/// Like [`estimate_bytes`] but reuses outputs recorded in `cache`.
pub fn estimate_bytes_cached(cmd: &Command, cache: &mut EstimateCache) -> Option<u64> {
    let (program, args) = cmd.estimate.split_first()?;
    let text = cache
        .0
        .entry(cmd.estimate.clone())
        .or_insert_with(|| {
            let out = Process::new(program_for(program))
                .args(args)
                .output()
                .ok()?;
            out.status
                .success()
                .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
        })
        .clone()?;
    let mut total = 0u64;
    let mut any = false;
    for line in text.lines() {
        if cmd
            .estimate_filter
            .as_deref()
            .is_some_and(|f| !line.contains(f))
        {
            continue;
        }
        // Reclaimable column is the last tab-separated field.
        let field = line.rsplit('\t').next().unwrap_or(line).trim();
        if let Some(b) = parse_size(field.split(' ').next().unwrap_or("")) {
            total += b;
            any = true;
        }
    }
    any.then_some(total)
}

/// Parses `1.2GB`, `350MB`, `0B`, `12.5kB`.
pub fn parse_size(s: &str) -> Option<u64> {
    let s = s.trim();
    let idx = s.find(|c: char| c.is_ascii_alphabetic())?;
    let (num, unit) = s.split_at(idx);
    let v: f64 = num.trim().parse().ok()?;
    let mult: f64 = match unit.to_ascii_lowercase().as_str() {
        "b" => 1.0,
        "kb" | "kib" => 1024.0,
        "mb" | "mib" => 1024f64.powi(2),
        "gb" | "gib" => 1024f64.powi(3),
        "tb" | "tib" => 1024f64.powi(4),
        _ => return None,
    };
    Some((v * mult) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_parsing() {
        assert_eq!(parse_size("0B"), Some(0));
        assert_eq!(parse_size("1.5GB"), Some(1_610_612_736));
        assert_eq!(parse_size("350MB"), Some(367_001_600));
        assert_eq!(parse_size("n/a"), None);
    }

    #[test]
    fn runs_a_harmless_command() {
        let cmd = Command {
            program: if cfg!(windows) {
                "cmd".into()
            } else {
                "echo".into()
            },
            args: if cfg!(windows) {
                vec!["/C".into(), "echo hi".into()]
            } else {
                vec!["hi".into()]
            },
            ..Default::default()
        };
        let out = run_command(&cmd, Duration::from_secs(5));
        assert!(out.success);
        assert!(out.stdout.contains("hi"));
    }

    #[test]
    fn missing_program_is_reported_not_panicked() {
        let cmd = Command {
            program: "definitely-not-a-real-binary-sapu".into(),
            ..Default::default()
        };
        let out = run_command(&cmd, Duration::from_secs(1));
        assert!(!out.success);
        assert!(!is_available(&cmd));
    }

    #[test]
    fn estimate_cache_runs_each_command_once() {
        let cmd = Command {
            program: "echo".into(),
            estimate: if cfg!(windows) {
                vec!["cmd".into(), "/C".into(), "echo Images\t2GB".into()]
            } else {
                vec!["printf".into(), "Images\t2GB\n".into()]
            },
            ..Default::default()
        };
        let mut cache = EstimateCache::default();
        let a = estimate_bytes_cached(&cmd, &mut cache);
        let b = estimate_bytes_cached(&cmd, &mut cache);
        assert_eq!(a, b);
        assert_eq!(cache.0.len(), 1);
        assert!(a.is_some_and(|v| v > 0));
    }

    #[test]
    fn resolves_absolute_and_relative_programs() {
        if cfg!(windows) {
            return;
        }
        let sh = resolve_program("sh").expect("sh on PATH");
        assert!(sh.is_absolute());
        assert_eq!(resolve_program(sh.to_str().expect("utf8")), Some(sh));
        assert!(resolve_program("definitely-not-a-real-binary-sapu").is_none());
    }
}
