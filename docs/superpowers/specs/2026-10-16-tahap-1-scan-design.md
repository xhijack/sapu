# Design: Tahap 1 — Scan ("Apa yang memakan ruang disk saya?")

Date: 2026-10-16 · Status: approved (user: no designer, use a professional, non-generic design direction) · Scope: PRD §11 Tahap 1 P0 + §10.1 progress.

## Visual direction

Subject: a disk is a finite shelf of space. The design's one memorable element is the **disk strip**: a single full-width stacked bar that shows the whole disk divided by category, exactly like looking at a shelf. Everything else is quiet: numbers set in tabular figures, dense lists, one primary action per screen.

- **Palette** (base): mist `#F1F4F2` background, white `#FFFFFF` surfaces, ink `#17201C`, muted ink `#5B655F`, **pine `#0F6E5F`** as the single accent (buttons, focus, selected). Dark: `#121715` / `#1B2320` / ink `#E9EDEB`, pine lifts to `#3FB39F`.
- **Category hues** (data, not decoration): video `#355C9E`, photo `#B4466B`, document `#C27A1F`, application `#6B4FB3`, system `#5E6A75`, temporary `#9A9A2E`, development `#1C8C6B`, other `#A3ACA7`. Each category always has its hue + a label; never hue alone.
- **Risk colours** stay as defined in Tahap 0b.
- **Type**: IBM Plex Sans (bundled via `@fontsource`, offline), weights 400/500/600; `font-variant-numeric: tabular-nums` on every number; IBM Plex Mono only for file paths. Type scale: 13 / 15 / 18 / 24 / 36 (the biggest size reserved for the one headline number on the results screen).
- **Layout**: left-aligned, 56 px sidebar rhythm, content max 960 px. No card grid: results are one column of sections separated by whitespace and a single hairline where the data changes kind.
- **Motion**: only the progress bar and the disk strip filling in once when results arrive. Respect `prefers-reduced-motion`.
- **Copy**: sentence case, plain verbs. "Pindai folder Beranda", "Pindahkan ke Trash", "Buka di Finder/Explorer/File manager" (name resolved per OS).

Rejected defaults: cream + serif + terracotta, dark + acid green, uppercase eyebrows, numbered markers, middle-dot meta strings, identical rounded cards with soft shadows.

## Screens

1. **Pindai (Home)** — empty state is an invitation: three targets as large rows (Beranda, Seluruh disk, Pilih folder…) with the last scan date if any. One primary button.
2. **Sedang memindai** — `ProgressPanel` (percentage from bytes scanned ÷ disk used, or previous scan totals), honest ETA, Pause / Lanjutkan / Batal.
3. **Hasil** — headline: total scanned in the 36 px figure with the root path under it. Disk strip. Category list (hue swatch, name, size, bar, share). Tabs: *Ringkasan* (categories), *Peta* (treemap), *Terbesar* (files / folders with filters size, type, age). Row actions: open in file manager, copy path, move to Trash (safety flow with preview + confirm; trash counts shown in History).

## Core (`sapu-core/src/scanner/`)

- `ScanOptions { root, top_files }`, `ScanControl` (run / paused / cancelled, shared atomics), `scan(opts, control, progress) -> ScanResult`.
- Walk with `jwalk`, `follow_links(false)`; unreadable dirs counted in `denied` and never abort. Hardlinks: on Unix a `(dev, ino)` set skips repeated inodes with `nlink > 1`; on Windows not deduplicated in P0 (documented).
- `Category` classifier: directory context first (caches/temp, node_modules/target/.git etc., OS dirs, app bundles), then extension. Tested.
- `ScanResult` keeps all `FileEntry`s and a flat `DirStats` map; queries: `categories()`, `top_files(filter, n)`, `top_dirs(n)`, `subtree(path, depth, limit)` for the treemap.
- Progress denominators (PRD §10.1): first scan uses `disk::used_bytes(root)` (statvfs / GetDiskFreeSpaceEx via `fs4`); later scans use the stored totals of the previous scan (`ScanStats` cache in the data dir).
- Events throttled to 10 Hz inside core.

## CLI

`sapu scan <path> [--json] [--top N]`: summary by category and the N largest files/folders; a plain progress line on a TTY, silent otherwise; `--json` emits the summary object.

## Tauri

State holds the last `ScanResult`. Commands: `scan_start(target)`, `scan_pause/resume/cancel`, `scan_status`, `scan_summary`, `scan_top_files(filter)`, `scan_top_dirs`, `scan_subtree(path)`, `reveal_path`, `trash_paths(paths, apply)` (safety). Progress arrives on the `scan://progress` event with fraction + `Eta`.

## Acceptance (from PRD)

Hardlinks not double counted (Unix test); symlinks not followed (test); denied folders reported, scan continues (test); cancel stops within one progress tick (test); UI stays responsive (scan runs on its own thread).
