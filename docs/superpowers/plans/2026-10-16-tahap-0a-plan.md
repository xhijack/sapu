# Plan: Tahap 0a

Executed inline, TDD per step. Each step ends with `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`.

1. Repo bootstrap: git init, `.gitignore`, `.githooks/commit-msg` + `core.hooksPath`, `.claude/settings.json`, workspace `Cargo.toml`, `clippy.toml`, empty crates compile.
2. `error.rs` + `progress.rs`.
3. `safety/protected.rs` with OS defaults and tests (lexical + canonical containment, append-only).
4. `safety/snapshot.rs` + `safety/validate.rs` with tempdir tests (changed size, changed mtime, missing, symlink out of scope).
5. `safety/dispose.rs` (Trash / Permanent token) + `safety/history.rs` (JSON store, list/get, restore cfg).
6. `safety/execute.rs`: DeletePlan/SessionReport, dry-run default, protected → revalidate → dispose → history, partial failure, progress events. Tests on tempdir.
7. `sapu-cli`: clap commands, dry-run default, JSON output.
8. `scripts/check-forbidden-fs.sh`, CI workflow, README skeleton. First commit(s).
