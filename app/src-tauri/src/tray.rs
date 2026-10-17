//! Menu bar / tray monitor: disk, memory and CPU at a glance, plus
//! shortcuts to open Sapu, run Care or free memory. Values refresh every
//! five seconds. When the main disk runs low the icon turns amber (Low)
//! or red (Critical), the free percentage shows as the title (macOS) or
//! tooltip (Windows), and one system notification is sent per crossing,
//! at most once per six hours per level.
//!
//! Tray strings are Indonesian only for now, like the rest of this file.

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use sapu_core::care::SpaceLevel;
use sapu_core::platform::{DiskInfo, SystemMonitor};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_notification::NotificationExt;

const REFRESH: Duration = Duration::from_secs(5);
/// Minimum gap between two notifications for the same level.
const NOTIFY_EVERY: Duration = Duration::from_secs(6 * 60 * 60);

fn human(n: u64) -> String {
    const U: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1024.0 && i < 4 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{n} B")
    } else {
        format!("{v:.1} {}", U[i])
    }
}

static INSTALLED: AtomicBool = AtomicBool::new(false);

/// Whether the tray icon was installed in this process.
pub fn is_installed() -> bool {
    INSTALLED.load(Ordering::Relaxed)
}

fn severity(level: SpaceLevel) -> u8 {
    match level {
        SpaceLevel::Ok => 0,
        SpaceLevel::Low => 1,
        SpaceLevel::Critical => 2,
    }
}

/// Notify when the disk gets *worse* (first reading included) and this
/// level has not been announced in the last six hours. Getting better
/// (Critical → Low) is not news.
fn should_notify(
    prev: Option<SpaceLevel>,
    next: SpaceLevel,
    last_sent: Option<Instant>,
    now: Instant,
) -> bool {
    if next == SpaceLevel::Ok {
        return false;
    }
    let worse = prev.is_none_or(|p| severity(next) > severity(p));
    worse && last_sent.is_none_or(|t| now.duration_since(t) >= NOTIFY_EVERY)
}

/// Free percentage of a disk, 0 when its size is unknown.
fn free_pct(d: &DiskInfo) -> u64 {
    (d.available_bytes.saturating_mul(100))
        .checked_div(d.total_bytes)
        .unwrap_or(0)
}

fn notification_text(level: SpaceLevel, d: &DiskInfo) -> (&'static str, String) {
    let title = match level {
        SpaceLevel::Critical => "Disk hampir penuh",
        _ => "Ruang disk menipis",
    };
    let body = format!(
        "Ruang disk tinggal {} % ({}). Buka Sapu untuk membersihkan.",
        free_pct(d),
        human(d.available_bytes)
    );
    (title, body)
}

/// Icon, template flag and short title for a level.
fn appearance(level: SpaceLevel, d: &DiskInfo) -> (&'static [u8], bool, Option<String>) {
    match level {
        SpaceLevel::Ok => (include_bytes!("../icons/tray-template.png"), true, None),
        SpaceLevel::Low => (
            include_bytes!("../icons/tray-low.png"),
            false,
            Some(format!("{}%", free_pct(d))),
        ),
        SpaceLevel::Critical => (
            include_bytes!("../icons/tray-critical.png"),
            false,
            Some(format!("{}%", free_pct(d))),
        ),
    }
}

fn apply_level<R: Runtime>(tray: &TrayIcon<R>, level: SpaceLevel, d: &DiskInfo) {
    let (png, template, title) = appearance(level, d);
    match tauri::image::Image::from_bytes(png) {
        Ok(icon) => {
            if let Err(e) = tray.set_icon_with_as_template(Some(icon), template) {
                tracing::warn!(error = %e, "tray icon");
            }
        }
        Err(e) => tracing::warn!(error = %e, "tray icon bytes"),
    }
    // Windows has no title; the tooltip carries the same fact everywhere.
    #[cfg(not(target_os = "windows"))]
    let _ = tray.set_title(title.as_deref());
    let tooltip = match title {
        Some(t) => format!("Sapu · Disk {t} kosong"),
        None => "Sapu".to_string(),
    };
    let _ = tray.set_tooltip(Some(tooltip));
}

pub fn install<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let disk = MenuItem::with_id(app, "disk", "Disk: …", false, None::<&str>)?;
    let mem = MenuItem::with_id(app, "mem", "Memori: …", true, None::<&str>)?;
    let cpu = MenuItem::with_id(app, "cpu", "CPU: …", false, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", "Buka Sapu", true, None::<&str>)?;
    let care = MenuItem::with_id(app, "care", "Rawat sekarang", true, None::<&str>)?;
    let quit = PredefinedMenuItem::quit(app, Some("Keluar"))?;
    let menu = Menu::with_items(
        app,
        &[
            &disk,
            &mem,
            &cpu,
            &PredefinedMenuItem::separator(app)?,
            &open,
            &care,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray-template.png"))?;
    let builder = TrayIconBuilder::with_id("sapu-monitor")
        .icon(icon)
        .menu(&menu)
        .tooltip("Sapu");
    #[cfg(target_os = "macos")]
    let builder = builder.icon_as_template(true);
    let tray = builder
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => show_main(app),
            "care" => {
                show_main(app);
                let _ = app.emit("tray://care", ());
            }
            "mem" => {
                show_main(app);
                let _ = app.emit("tray://memory", ());
            }
            _ => {}
        })
        .build(app)?;

    // Refresh loop; menu items and the icon are cheap to update from
    // another thread (Tauri hops to the main thread itself).
    let monitor = Mutex::new(SystemMonitor::new());
    let handle = app.clone();
    std::thread::spawn(move || {
        let mut prev_level: Option<SpaceLevel> = None;
        let mut last_sent: HashMap<SpaceLevel, Instant> = HashMap::new();
        loop {
            if let Ok(mut m) = monitor.lock() {
                let snap = m.snapshot();
                if let Some(main) = snap
                    .disks
                    .iter()
                    .find(|d| !d.removable)
                    .or(snap.disks.first())
                {
                    let used = main.total_bytes.saturating_sub(main.available_bytes);
                    let pct = (used * 100).checked_div(main.total_bytes).unwrap_or(0);
                    let _ = disk.set_text(format!(
                        "Disk: {} kosong ({pct}% terpakai)",
                        human(main.available_bytes)
                    ));
                    let level = main.level;
                    if prev_level != Some(level) {
                        apply_level(&tray, level, main);
                    }
                    let now = Instant::now();
                    if should_notify(prev_level, level, last_sent.get(&level).copied(), now) {
                        let (title, body) = notification_text(level, main);
                        if let Err(e) = handle
                            .notification()
                            .builder()
                            .title(title)
                            .body(body)
                            .show()
                        {
                            tracing::warn!(error = %e, "disk notification failed");
                        }
                        last_sent.insert(level, now);
                    }
                    prev_level = Some(level);
                }
                let mem_pct = (snap.mem_used_bytes * 100)
                    .checked_div(snap.mem_total_bytes)
                    .unwrap_or(0);
                let _ = mem.set_text(format!("Memori: {mem_pct}% · Bebaskan…"));
                if let Some(c) = snap.cpu_percent {
                    let _ = cpu.set_text(format!("CPU: {c:.0}% dari {} inti", snap.cpu_count));
                }
            }
            std::thread::sleep(REFRESH);
        }
    });
    INSTALLED.store(true, Ordering::Relaxed);
    Ok(())
}

pub fn show_main<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn disk(total: u64, free: u64) -> DiskInfo {
        DiskInfo {
            name: "Macintosh HD".into(),
            mount: "/".into(),
            total_bytes: total,
            available_bytes: free,
            removable: false,
            file_system: "apfs".into(),
            recommended_free_bytes: sapu_core::care::recommended_free_bytes(total),
            level: sapu_core::care::space_level(free, total),
        }
    }

    #[test]
    fn notifies_when_worse_and_rate_limits_per_level() {
        let t0 = Instant::now();
        // First reading already low: tell the user.
        assert!(should_notify(None, SpaceLevel::Low, None, t0));
        // Ok is never news.
        assert!(!should_notify(None, SpaceLevel::Ok, None, t0));
        assert!(!should_notify(
            Some(SpaceLevel::Low),
            SpaceLevel::Ok,
            None,
            t0
        ));
        // Getting worse: yes. Getting better: no.
        assert!(should_notify(
            Some(SpaceLevel::Low),
            SpaceLevel::Critical,
            None,
            t0
        ));
        assert!(!should_notify(
            Some(SpaceLevel::Critical),
            SpaceLevel::Low,
            None,
            t0
        ));
        // Same level again: no.
        assert!(!should_notify(
            Some(SpaceLevel::Low),
            SpaceLevel::Low,
            None,
            t0
        ));
        // Flapping Ok → Low within six hours: once only.
        let sent = Some(t0);
        assert!(!should_notify(
            Some(SpaceLevel::Ok),
            SpaceLevel::Low,
            sent,
            t0 + Duration::from_secs(3600)
        ));
        assert!(should_notify(
            Some(SpaceLevel::Ok),
            SpaceLevel::Low,
            sent,
            t0 + NOTIFY_EVERY
        ));
    }

    #[test]
    fn notification_text_is_short_and_in_indonesian() {
        const GB: u64 = 1024 * 1024 * 1024;
        let d = disk(494 * GB, 18 * GB);
        assert_eq!(d.level, SpaceLevel::Critical);
        let (title, body) = notification_text(d.level, &d);
        assert_eq!(title, "Disk hampir penuh");
        assert_eq!(
            body,
            "Ruang disk tinggal 3 % (18.0 GB). Buka Sapu untuk membersihkan."
        );
        let low = disk(1000 * GB, 180 * GB);
        assert_eq!(notification_text(low.level, &low).0, "Ruang disk menipis");
    }

    #[test]
    fn appearance_follows_the_level() {
        let ok = disk(1000, 500);
        let (_, template, title) = appearance(ok.level, &ok);
        assert!(template);
        assert_eq!(title, None);
        let low = disk(1000, 180);
        let (png, template, title) = appearance(low.level, &low);
        assert!(!template);
        assert_eq!(title.as_deref(), Some("18%"));
        assert_eq!(&png[1..4], b"PNG");
        let crit = disk(1000, 40);
        assert_eq!(appearance(crit.level, &crit).2.as_deref(), Some("4%"));
    }
}
