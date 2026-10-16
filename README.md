# Sapu

**Smart Analyzer & Pruning Utility.** An open source disk cleaner for Windows, macOS and Linux: scan what fills your disk, clean what is safe to remove, and deduplicate at scale. Built for people who are not technical *and* for developers whose disks are full of Docker images and `node_modules`.

> Status: Tahap 0–3 (safety, scan, clean, dedupe) plus P1 features and the optional AI layer are implemented; not yet released or signed. See [`PRD.md`](PRD.md) for the product spec and [`CLAUDE.md`](CLAUDE.md) for contributor rules.

## Safety first

Every removal goes through one module, `sapu-core/src/safety/`, which:

- moves to the OS trash by default; permanent deletion requires a second confirmation,
- refuses protected paths (OS directories, credentials, your personal folders) no matter what,
- re-validates every file right before removing it (still exists, same size and mtime, symlinks stay in scope),
- is dry-run unless you say `--apply`,
- records every real session so it can be restored.

`clippy.toml` and `scripts/check-forbidden-fs.sh` fail CI if a removal API appears anywhere else.

## Layout

```
crates/sapu-core   engine: safety, scanner, rules, cleaner (incl. dead projects), dedupe, eta, platform
crates/sapu-ai     optional AI layer (providers, metadata-only tasks); never depends on sapu-core
crates/sapu-cli    thin CLI (`sapu`)
app/               Tauri 2 + React app (src-tauri commands only call core)
rules/             declarative TOML rules per OS; user rules live in <config>/sapu/rules
locales/           id.json / en.json
docs/              design specs
```

## Build

```sh
cargo build
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
sh scripts/check-forbidden-fs.sh
```

## CLI

```sh
sapu scan <path> [--json] [--top N]            # what takes up space (allocated size on disk)
sapu clean [--profile dev] [--include-yellow]   # dry run; --apply to execute
sapu dupes <path> [--keep original|oldest|newest] [--hardlink] [--apply]
sapu ai status | whatis <path> | search "<sentence>" <root>   # optional, metadata only
sapu protected                                  # what Sapu will never touch
sapu trash <paths>... [--apply]
sapu history [restore <id>]                     # restore: Windows/Linux; macOS: Finder's Trash
```

## AI mode

Off by default. Local (Ollama or any OpenAI-compatible server) or cloud with your own key (OpenAI-compatible or Anthropic; keys stay in the OS keychain). Only metadata leaves the device, home folder and account name are masked, every request can be previewed first, and an AI answer can never produce a green label.

## Releases

Tagged versions build installers for macOS, Windows and Linux plus CLI archives through `.github/workflows/release.yml`; see `docs/release.md` for signing and package-manager templates in `packaging/`.

## License

Dual-licensed under MIT or Apache-2.0, at your option. You may use, modify, redistribute and sell Sapu or derived works; keep the copyright and license notices.
