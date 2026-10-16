# Design: Tahap 0b — App Foundation

Date: 2026-10-16 · Status: approved (conversation, "diurut sampai selesai") · Scope: PRD §9 (onboarding), §10 (UI principles, 10.1 progress/ETA), §11 Tahap 0 app part.

## Goal

A Tauri 2 shell whose commands only call `sapu-core`, with the design tokens, i18n, onboarding wizard, and progress/ETA building blocks every later screen will reuse. No scanning yet.

## Core additions (Rust, tested)

- `eta.rs`: `Estimator` (moving-average rate over a 10 s window, exponential smoothing, no number before 3 s / 3 samples, range while < 10 % done, bucketed rounding, never backwards) and `Eta` enum serialized with `kind` tag. UI formats it; core never builds strings.
- `settings.rs`: `Settings { onboarding_done, profile, locale, ai_mode (default Off), theme }` as JSON in the OS config dir. Corrupt file is an error, never a silent reset.
- `platform/`: `detect_dev_tools()` (PATH + home markers) for the profile suggestion; `full_disk_access()` probes a TCC-protected folder on macOS, `NotApplicable` elsewhere.

## Tauri layer (`app/src-tauri`)

Commands: `app_info`, `disk_access_status`, `settings_get/set`, `protected_paths`, `history_list`, `history_restore`, `open_disk_access_settings`. Errors cross IPC as `{code, message}`; the frontend maps `code` to a translated sentence. Workspace member, workspace lints, `tracing` instead of `expect`.

## Frontend (`app/src`)

- `lib/api.ts`: typed invoke wrappers with an in-memory mock when not inside Tauri, so vitest and plain-browser dev work.
- `index.css`: tokens for surface/text/accent and the three risk colours, light + dark (`prefers-color-scheme` and `data-theme`). Tailwind 4 `@theme inline` exposes them as utilities.
- `locales/{id,en}.json` at repo root; a test asserts identical key sets, non-empty values and matching placeholders. No hardcoded UI strings.
- Components: `RiskBadge` (colour + icon + text), `ProgressPanel` (percentage, counts, ETA sentence, elapsed, stage), `ProgressBar` (indeterminate mode), `ChoiceGroup` (keyboard-accessible radio cards), `EmptyState/LoadingState/ErrorState`, `Shell` (sidebar nav).
- `features/onboarding/Wizard`: six steps per PRD §9, every step skippable, developer profile pre-selected when tooling is detected, macOS Full Disk Access status polled every 2 s so it updates without restart, AI defaults to off. Re-openable from Home and Settings.
- `HistoryView` lists sessions with restore (macOS shows the Finder hint); `SettingsView` edits profile/language/theme/AI and lists protected paths.

## Testing

Rust: estimator, settings, platform detection (tempdir). Frontend: locale parity, ETA formatting, RiskBadge, ProgressPanel, Wizard flows, App navigation and language switch. CI gains a `frontend` job and Linux webkit deps for the Tauri crate.

## Deferred

Release signing/notarization (PRD §13, needs Q3), real scan (Tahap 1), system notifications (10.1 P1).
