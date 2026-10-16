# Design: Tahap 0a — Core Foundation & Safety Module

Date: 2026-10-16 · Status: approved (conversation) · Scope: `PRD.md` §8 (S1–S7) and §11 Tahap 0, core/CLI part only. GUI, design system, i18n and onboarding are Tahap 0b.

## Goal

A Cargo workspace where the only code path that can remove a file lives in `sapu-core/src/safety/`, is covered by tests on temporary filesystems, and is exercised end-to-end by a thin CLI that is dry-run by default. CI on Windows, macOS and Linux proves it.

## Non-goals (0a)

Scanner, rules engine, cleaner categories, dedupe pipeline, AI, Tauri app, ETA estimator. Only the hooks they will need (progress event type, error type, `DeletePlan`).

## Workspace layout

```
sapu/
├── Cargo.toml                 # workspace, resolver 3, shared lints
├── clippy.toml                # disallowed-methods: std::fs::remove_* outside safety
├── crates/sapu-core/src/
│   ├── lib.rs                 # pub mod error, progress, safety
│   ├── error.rs               # SapuError (thiserror)
│   ├── progress.rs            # ProgressEvent + ProgressSink trait (core never formats UI)
│   └── safety/
│       ├── mod.rs             # public API: Safety::new(), execute()
│       ├── protected.rs       # ProtectedPaths (append-only, OS defaults via cfg)
│       ├── snapshot.rs        # FileSnapshot captured at scan time
│       ├── validate.rs        # revalidate(snapshot, scope) right before disposal
│       ├── dispose.rs         # ONLY file with remove_*/trash calls; Permanent needs a confirmation token
│       ├── history.rs         # CleanSession JSON store + restore where OS supports it
│       └── execute.rs         # DeletePlan → protected → revalidate → dispose → history, dry-run default
├── crates/sapu-cli/src/main.rs # clap: protected, history, trash <paths> [--apply] [--json]
├── scripts/check-forbidden-fs.sh
├── .github/workflows/ci.yml   # fmt, clippy -D warnings, test, forbidden-fs on 3 OS
├── .githooks/commit-msg       # strips AI attribution
└── .claude/settings.json      # attribution.commit = ""
```

## Safety module contracts

- **S1 Trash by default.** `Disposal::Trash` is the default. `Disposal::Permanent(PermanentConfirmation)` requires a token constructable only via `PermanentConfirmation::i_understand_this_cannot_be_undone()`. Callers (CLI/GUI) must obtain a second user confirmation before constructing it.
- **S2 History.** Every `execute()` that is not dry-run writes a `CleanSession` (id, started/finished unix time, entries with path, size, disposal, outcome) as one JSON file under the app data dir (`dirs::data_local_dir()/sapu/history/`). `restore_session` restores trashed entries via `trash::os_limited` on Windows/Linux; on macOS it returns `RestoreUnsupported` so the UI can point the user to Finder's Trash.
- **S3 Protected paths.** `ProtectedPaths` is append-only: no API removes an entry. OS defaults compiled in via `#[cfg(target_os)]`. A path is protected when it equals or is inside any protected root, compared on both the lexical path and the canonicalized path when it resolves. Default set: OS system dirs, `~/.ssh`, `~/.gnupg`, `~/.aws`, `~/.config/gcloud`, `~/.kube`, `~/.docker/config.json`, keychain dirs, and the user's Documents/Pictures/Desktop roots (deleting *inside* those is allowed only for explicit per-file user actions; the roots themselves never are).
- **S4 Dry-run.** `DeletePlan::dry_run` defaults to `true`. CLI needs `--apply`.
- **S5 Revalidate.** Immediately before disposal: path still exists (via `symlink_metadata`, not following links), size and mtime equal the snapshot, and if it is a symlink its target resolves inside `plan.scope`. Any mismatch skips the item with `Outcome::Skipped(reason)`; it never aborts the whole session.
- **S6** is a scanner concern (Tahap 1) but `FileSnapshot::capture` already uses `symlink_metadata` and records `is_symlink`.
- **S7 Tests.** Every test that disposes anything runs inside `tempfile::tempdir()`. Permanent disposal is tested against tempdir; trash disposal is tested behind a `SAPU_TEST_TRASH=1` guard because the system trash is a real side effect.
- Partial failure: `SessionReport` lists per-item outcomes (`Disposed`, `Skipped`, `Failed`) plus totals; the session is still recorded.

## Dependency direction

`safety` depends on `error`, `progress` only. Future `ai` module must not import `safety` or `cleaner`; enforced in 0b+ by splitting `ai` into its own crate without a dependency on `sapu-core::safety` (recorded here so the workspace is shaped for it: `sapu-core` keeps `safety`, a later `sapu-ai` crate depends only on shared types).

## Lint against stray deletion

`clippy.toml` disallows `std::fs::remove_file`, `remove_dir`, `remove_dir_all`, `trash::delete`, `trash::delete_all`. `dispose.rs` is the only file with `#![allow(clippy::disallowed_methods)]`. `scripts/check-forbidden-fs.sh` greps the tree and fails if the allow attribute or any `remove_*`/`trash::delete` call appears outside `crates/sapu-core/src/safety/dispose.rs`. CI runs both.

## CLI (thin layer)

`sapu protected` lists protected roots. `sapu history [--json]` lists sessions. `sapu trash <path>... [--apply] [--permanent] [--json]` builds a `DeletePlan` and prints the `SessionReport`. `--permanent` additionally asks an interactive "type DELETE" prompt; it is refused when stdin is not a TTY.

## Open items carried forward

- License (PRD Q2): `license` field omitted until decided.
- Dedupe "never delete all copies" guard lives with the dedupe plan type in Tahap 3; `execute()` will accept a `GroupGuard` then.
