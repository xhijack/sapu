//! Duplicate finder commands: run the pipeline on a worker thread, stream
//! progress, serve groups and execute a plan through core.

use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use sapu_core::dedupe::{
    DedupeOptions, DedupePlan, DedupeResult, DedupeRunReport, DedupeSummary, DuplicateGroup,
    FolderPair, KeepStrategy, ReplaceMode, Stage, execute, find_duplicates, folder_pairs,
};
use sapu_core::eta::{Estimator, Eta, OpKind};
use sapu_core::progress::{FnSink, NoProgress, ProgressEvent};
use sapu_core::safety::{HistoryStore, ProtectedPaths, Safety};
use sapu_core::scanner::ScanControl;
use tauri::{AppHandle, Emitter, State};

use crate::support::{notify_done, record_rate};
use crate::{CmdResult, CommandError};

#[derive(Default)]
pub struct DedupeSession {
    result: Mutex<Option<Arc<DedupeResult>>>,
    control: Mutex<Option<ScanControl>>,
    running: AtomicBool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DedupeProgress {
    fraction: Option<f64>,
    items_done: u64,
    items_total: Option<u64>,
    bytes_done: u64,
    bytes_total: Option<u64>,
    elapsed_secs: u64,
    eta: Eta,
    stage: Stage,
}

#[tauri::command]
pub fn dedupe_start(
    app: AppHandle,
    session: State<'_, Arc<DedupeSession>>,
    root: PathBuf,
    min_size: Option<u64>,
) -> CmdResult<()> {
    if session.running.swap(true, Ordering::AcqRel) {
        return Err(CommandError::new(
            "scan_running",
            "a duplicate search is already running",
        ));
    }
    let control = ScanControl::new();
    if let Ok(mut c) = session.control.lock() {
        *c = Some(control.clone());
    }
    let session = Arc::clone(&session);
    std::thread::spawn(move || {
        let mut opts = DedupeOptions::new(&root);
        if let Some(m) = min_size {
            opts.min_size = m;
        }
        let start = Instant::now();
        let stage = Arc::new(Mutex::new(Stage {
            current: 1,
            total: 3,
        }));
        let stage_for_sink = Arc::clone(&stage);
        let app2 = app.clone();
        let mut est = Estimator::default();
        let mut sink = FnSink(move |e: &ProgressEvent| {
            let elapsed = start.elapsed();
            est.observe(e, elapsed);
            let measurable = e.bytes_total.is_some() || e.items_total.is_some();
            let payload = DedupeProgress {
                fraction: measurable.then(|| est.fraction(e)),
                items_done: e.items_done,
                items_total: e.items_total,
                bytes_done: e.bytes_done,
                bytes_total: e.bytes_total,
                elapsed_secs: elapsed.as_secs(),
                eta: est.eta(e, elapsed),
                stage: stage_for_sink.lock().map(|s| *s).unwrap_or(Stage {
                    current: 1,
                    total: 3,
                }),
            };
            let _ = app2.emit("dedupe://progress", payload);
        });
        let stage_for_cb = Arc::clone(&stage);
        let app3 = app.clone();
        let result = find_duplicates(&opts, &control, &mut sink, move |s| {
            if let Ok(mut st) = stage_for_cb.lock() {
                *st = s;
            }
            let _ = app3.emit("dedupe://stage", s);
        });
        match result {
            Ok(r) => {
                let summary = r.summary.clone();
                if !r.summary.cancelled {
                    record_rate(
                        OpKind::Dedupe,
                        r.summary.wasted_bytes.max(r.summary.candidates),
                        start.elapsed(),
                    );
                }
                notify_done(
                    &app,
                    start.elapsed(),
                    "Sapu",
                    &format!("Pencarian duplikat selesai: {} grup", summary.groups),
                );
                if let Ok(mut slot) = session.result.lock() {
                    *slot = Some(Arc::new(r));
                }
                let _ = app.emit("dedupe://done", summary);
            }
            Err(e) => {
                let _ = app.emit("dedupe://error", CommandError::from(e));
            }
        }
        session.running.store(false, Ordering::Release);
    });
    Ok(())
}

/// Folders a lay user thinks of as "the real place": Documents, Pictures,
/// Desktop, Movies, Music. Used by the "keep the copy in my main folder"
/// strategy.
#[tauri::command]
pub async fn dedupe_primary_folders() -> CmdResult<Vec<PathBuf>> {
    crate::off_main(move || {
        Ok({
            [
                dirs::document_dir(),
                dirs::picture_dir(),
                dirs::desktop_dir(),
                dirs::video_dir(),
                dirs::audio_dir(),
            ]
            .into_iter()
            .flatten()
            .collect()
        })
    })
    .await
}

#[tauri::command]
pub fn dedupe_cancel(session: State<'_, Arc<DedupeSession>>) {
    if let Ok(c) = session.control.lock()
        && let Some(c) = c.as_ref()
    {
        c.cancel();
    }
}

fn result(session: &DedupeSession) -> CmdResult<Arc<DedupeResult>> {
    session
        .result
        .lock()
        .ok()
        .and_then(|r| r.clone())
        .ok_or_else(|| CommandError::new("no_scan", "no duplicate search yet"))
}

#[tauri::command]
pub async fn dedupe_summary(session: State<'_, Arc<DedupeSession>>) -> CmdResult<DedupeSummary> {
    let session = session.inner().clone();
    crate::off_main(move || Ok(result(&session)?.summary.clone())).await
}

#[tauri::command]
pub async fn dedupe_groups(
    session: State<'_, Arc<DedupeSession>>,
    offset: usize,
    limit: usize,
) -> CmdResult<Vec<DuplicateGroup>> {
    let session = session.inner().clone();
    crate::off_main(move || {
        Ok(result(&session)?
            .groups
            .iter()
            .skip(offset)
            .take(limit.min(200))
            .cloned()
            .collect())
    })
    .await
}

#[tauri::command]
pub async fn dedupe_folder_pairs(
    session: State<'_, Arc<DedupeSession>>,
    limit: usize,
) -> CmdResult<Vec<FolderPair>> {
    let session = session.inner().clone();
    crate::off_main(move || {
        let mut pairs = folder_pairs(&result(&session)?.groups);
        pairs.truncate(limit.min(200));
        Ok(pairs)
    })
    .await
}

/// Build and run the plan. `keeps` overrides the strategy per group id;
/// `group_ids` restricts the run (empty = all groups).
#[tauri::command]
pub async fn dedupe_execute(
    session: State<'_, Arc<DedupeSession>>,
    strategy: KeepStrategy,
    keeps: HashMap<u64, PathBuf>,
    group_ids: Vec<u64>,
    apply: bool,
    hardlink: bool,
) -> CmdResult<DedupeRunReport> {
    let session = session.inner().clone();
    crate::off_main(move || {
        let r = result(&session)?;
        let mut plan = DedupePlan::build(&r.summary.root, &r.groups, &strategy, |g| {
            keeps.get(&g.id).cloned()
        })
        .map_err(|e| CommandError::new("plan", e.to_string()))?;
        if !group_ids.is_empty() {
            plan = plan.retain(&group_ids);
        }
        let safety = Safety::new(ProtectedPaths::os_defaults(), HistoryStore::open_default()?);
        let mode = if hardlink {
            ReplaceMode::Hardlink
        } else {
            ReplaceMode::Trash
        };
        Ok(execute(&plan, &safety, apply, mode, &mut NoProgress)?)
    })
    .await
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FilePreview {
    Image {
        data_url: String,
    },
    Text {
        excerpt: String,
    },
    /// Office/PDF documents: metadata only, never contents.
    Document {
        doc_kind: String,
        pages: Option<u32>,
    },
    None,
}

const MAX_IMAGE: u64 = 20 * 1024 * 1024;

/// Side-by-side preview data. Images are inlined as a data URL; text files
/// give their first 2 KB; anything else returns `None` (UI shows metadata).
#[tauri::command]
pub async fn file_preview(path: PathBuf) -> CmdResult<FilePreview> {
    crate::off_main(move || {
        Ok((|| {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_ascii_lowercase())
                .unwrap_or_default();
            let mime = match ext.as_str() {
                "jpg" | "jpeg" => Some("image/jpeg"),
                "png" => Some("image/png"),
                "gif" => Some("image/gif"),
                "webp" => Some("image/webp"),
                "bmp" => Some("image/bmp"),
                "svg" => Some("image/svg+xml"),
                _ => None,
            };
            let Ok(meta) = std::fs::symlink_metadata(&path) else {
                return FilePreview::None;
            };
            if let Some(mime) = mime {
                if meta.len() > MAX_IMAGE {
                    return FilePreview::None;
                }
                return match std::fs::read(&path) {
                    Ok(bytes) => FilePreview::Image {
                        data_url: format!("data:{mime};base64,{}", base64(&bytes)),
                    },
                    Err(_) => FilePreview::None,
                };
            }
            if ext == "pdf" {
                return FilePreview::Document {
                    doc_kind: "pdf".into(),
                    pages: pdf_page_count(&path),
                };
            }
            if matches!(
                ext.as_str(),
                "doc"
                    | "docx"
                    | "xls"
                    | "xlsx"
                    | "ppt"
                    | "pptx"
                    | "odt"
                    | "ods"
                    | "odp"
                    | "pages"
                    | "numbers"
                    | "key"
                    | "epub"
            ) {
                return FilePreview::Document {
                    doc_kind: ext.clone(),
                    pages: None,
                };
            }
            if matches!(
                ext.as_str(),
                "txt" | "md" | "csv" | "json" | "log" | "xml" | "html" | "rtf"
            ) {
                let mut buf = vec![0u8; 2048];
                if let Ok(mut f) = std::fs::File::open(&path)
                    && let Ok(n) = f.read(&mut buf)
                {
                    return FilePreview::Text {
                        excerpt: String::from_utf8_lossy(&buf[..n]).into_owned(),
                    };
                }
            }
            FilePreview::None
        })())
    })
    .await
}

/// Reads the page count from the PDF catalog (`/Type /Pages ... /Count N`)
/// by scanning the first and last 512 KB; best effort, no rendering.
fn pdf_page_count(path: &std::path::Path) -> Option<u32> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(path).ok()?;
    let len = f.metadata().ok()?.len();
    let window = 512 * 1024;
    let mut buf = Vec::with_capacity((window * 2) as usize);
    let mut head = vec![0u8; window.min(len) as usize];
    f.read_exact(&mut head).ok()?;
    buf.extend_from_slice(&head);
    if len > window {
        f.seek(SeekFrom::End(-(window as i64))).ok()?;
        let mut tail = vec![0u8; window as usize];
        let n = f.read(&mut tail).ok()?;
        buf.extend_from_slice(&tail[..n]);
    }
    let text = String::from_utf8_lossy(&buf);
    // Prefer /Count that follows a /Type /Pages object; fall back to the largest /Count.
    let mut best: Option<u32> = None;
    for (i, _) in text.match_indices("/Count") {
        let rest = &text[i + 6..];
        let num: String = rest
            .trim_start()
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if let Ok(n) = num.parse::<u32>() {
            best = Some(best.map_or(n, |b| b.max(n)));
        }
    }
    best
}

pub(crate) fn base64_pub(bytes: &[u8]) -> String {
    base64(bytes)
}

fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            T[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            T[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{base64, pdf_page_count};

    #[test]
    fn pdf_page_count_from_catalog() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let p = tmp.path().join("a.pdf");
        std::fs::write(&p, b"%PDF-1.4\n1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n2 0 obj << /Type /Pages /Kids [3 0 R] /Count 12 >> endobj\n%%EOF").expect("write");
        assert_eq!(pdf_page_count(&p), Some(12));
    }

    #[test]
    fn base64_matches_known_vectors() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }
}
