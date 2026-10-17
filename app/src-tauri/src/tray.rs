//! Menu bar / tray monitor: disk, memory and CPU at a glance, plus
//! shortcuts to open Sapu or run Care. Values refresh every five seconds.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use sapu_core::platform::SystemMonitor;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, Runtime};

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

pub fn install<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let disk = MenuItem::with_id(app, "disk", "Disk: …", false, None::<&str>)?;
    let mem = MenuItem::with_id(app, "mem", "Memori: …", false, None::<&str>)?;
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
    builder
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => show_main(app),
            "care" => {
                show_main(app);
                let _ = app.emit("tray://care", ());
            }
            _ => {}
        })
        .build(app)?;

    // Refresh loop; menu items are cheap to update from another thread.
    let monitor = Mutex::new(SystemMonitor::new());
    std::thread::spawn(move || {
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
                }
                let mem_pct = (snap.mem_used_bytes * 100)
                    .checked_div(snap.mem_total_bytes)
                    .unwrap_or(0);
                let _ = mem.set_text(format!(
                    "Memori: {} dari {} ({mem_pct}%)",
                    human(snap.mem_used_bytes),
                    human(snap.mem_total_bytes)
                ));
                if let Some(c) = snap.cpu_percent {
                    let _ = cpu.set_text(format!("CPU: {c:.0}% dari {} inti", snap.cpu_count));
                }
            }
            std::thread::sleep(Duration::from_secs(5));
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
