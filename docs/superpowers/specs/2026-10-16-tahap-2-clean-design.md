# Design: Tahap 2 — Bersih ("Apa yang aman dihapus?")

Date: 2026-10-16 · Status: approved (user: "lanjutkan", TOML per recommendation) · Scope: PRD §11 Tahap 2 P0, §8 S1–S5 reuse.

## Rules (declarative, TOML, embedded)

`rules/{common,macos,linux,windows}/*.toml`, embedded at build time with `include_dir` and filtered by OS at load. Community rules later load from a user directory with the same schema.

```toml
[rule]
id = "chrome-cache"                 # kebab-case, unique
name = { id = "Cache browser Chrome", en = "Chrome browser cache" }
description = { id = "…", en = "…" }
risk = "green"                      # green | yellow (never red; red is protected paths)
profile = "general"                 # general | developer
group = "browser"                   # UI grouping key, translated in locales
permanent = false                   # true only for items already in the Trash

[[match]]
path = "~/Library/Caches/Google/Chrome"   # ~ and ${VAR} expanded, glob * ? ** allowed
kind = "dir_contents"               # dir_contents | dir | files | dir_name
min_age_days = 0

# dir_name variant (bounded walk from roots, never descends into a match):
# name = "node_modules", roots = ["~"], max_depth = 6, unused_days = 30, skip = ["Library", ".*"]

[command]                           # alternative to [[match]]: an official tool does the work
program = "docker"
args = ["image", "prune", "-f"]
requires = "docker"
estimate = ["docker", "system", "df", "--format", "{{.Type}}\t{{.Reclaimable}}"]
confirm_separately = false
```

Validation at load (and in a test over every shipped file): unique ids, both languages non-empty, exactly one of `match`/`command`, `kind`-specific fields present.

## Core

- `rules/`: `schema.rs` (serde + `validate`), `load.rs` (embedded set, OS filter), `expand.rs` (`~`, `${VAR}`, glob; injectable `Env` for tests).
- `cleaner/evaluate.rs`: `evaluate(rules, profile, protected, Env) -> CleanReport { findings: Vec<Finding { rule, items: Vec<Candidate{path,size,is_dir,mtime}>, bytes, skipped_protected }>, commands: Vec<CommandFinding { rule, available, estimate_bytes }> }`. Sizes computed with a symlink-safe walk. Protected paths are filtered here *and* again by safety on execution.
- `cleaner/execute.rs`: `execute(report, selection, apply, disposal, progress) -> CleanRunReport`. One `DeletePlan` per finding (scope = the match base dir) through `Safety`; sessions recorded in history. Commands run via `std::process::Command` only in `cleaner/command.rs`; dry run prints the command line; output captured in the report.
- Green findings are pre-selected; yellow require a tick; `permanent` rules need `PermanentConfirmation` from the caller.

## CLI

`sapu clean [--profile general|dev] [--apply] [--include-yellow] [--json]`: lists findings with risk, sizes and the total; `--apply` executes the selection (green by default). Trash rules (permanent) are only run with `--include-yellow --apply` and an interactive DELETE prompt.

## Tauri + UI

Commands: `clean_evaluate(profile)`, `clean_execute(selection, apply, permanent_ok)`. Screen *Bersih*: findings grouped by rule, each row with `RiskBadge`, checkbox (green ticked), size and item count; disabled rows for unavailable commands. One primary button "Bersihkan X GB yang aman" where X follows the selection. Preview dialog (what, how many, where, the command lines) → confirm → result with per-rule outcome and a link to Riwayat. Docker volume prune has its own confirmation (PRD).

## Acceptance

- Test: every shipped rule expands to paths outside `ProtectedPaths::os_defaults()`.
- Test: every rule has `name`/`description` in id and en, valid kind fields, unique id.
- Test on tempdir: evaluation finds candidates, honours `min_age_days`, skips protected; execution dry-run touches nothing, apply removes only selected items, partial failure is reported.
- UI test: green pre-selected, yellow not; total follows selection; preview before apply.
