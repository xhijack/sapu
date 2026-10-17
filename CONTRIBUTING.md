# Contributing to Sapu

Thank you for helping. Sapu is a disk cleaner, so every change is judged first by one question: can it make a user lose data? Read `CLAUDE.md` for the architecture and safety rules that are not negotiable, and `PRD.md` for what the product is trying to be.

## Ways to help

- **Report a bug**: open an issue with your OS, the Sapu version (Settings → About), what you did and what happened. Screenshots help.
- **Add a clean-up rule** for an application you know well (see below). This is the most useful contribution a non-Rust developer can make.
- **Translate**: `locales/id.json` and `locales/en.json` must have the same keys; a test enforces it. A new language is a new file plus one line in `app/src/i18n.ts`.
- **Code**: pick an issue labelled `good first issue`, or propose a change in an issue first if it touches `safety/`, `cleaner/` or `dedupe/`.

## Pull requests

1. Fork, branch from `main`, keep one topic per pull request.
2. Run the checks below; CI runs them on macOS, Windows and Linux.
3. Write the commit message in English following Conventional Commits (`feat:`, `fix:`, `rules(macos):`, …). No AI attribution trailers.
4. UI text goes through `locales/`, in both languages. Technical terms belong in detail panels, not headlines.
5. Anything that removes files must go through `safety::DeletePlan`; the build rejects other removal calls.

## Development

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
sh scripts/check-forbidden-fs.sh      # removal APIs only in safety/dispose.rs
sh scripts/check-ai-isolation.sh      # sapu-ai never depends on sapu-core
cd app && pnpm install && pnpm check  # lint + typecheck + tests
```

Commits follow Conventional Commits and carry no AI attribution trailers.

## Contributing a clean-up rule

Rules are TOML files under `rules/<os>/` (or `rules/common/` for commands that exist everywhere). The schema is documented in `rules/README.md`.

1. Start from an existing file in the same folder.
2. Give the rule a kebab-case `id`, a user-facing `name` and `description` in **both** Indonesian and English, a `risk` and a `group`.
3. Only mark a rule `green` when the files are recreated automatically by their application and never hold user data. When in doubt, `yellow`.
4. Never point a rule at user data roots (Documents, Pictures, Desktop) or credential folders. The test suite refuses rules that resolve into protected paths.
5. Run `cargo test -p sapu-core rules`. Two tests must pass for your file: it parses and validates, and a behavioural test that builds a fake home, runs your rule and checks it finds exactly the files under its own paths.
6. Open a pull request titled `rules(<os>): <what it cleans>` and explain which application creates the files and why they are safe to remove. A maintainer verifies the paths on that OS before merging.

Rules produced from an AI answer inside the app are saved locally (yellow) in `<config>/sapu/rules/`. If one proves useful, copy it into this repository and send it through the same review.

## Reporting a bug that lost data

Open an issue with the label `data-loss` and include the Riwayat entry (Settings → Riwayat) if you have it. These issues are handled before anything else.
