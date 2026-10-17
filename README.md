# Sapu

**Smart Analyzer & Pruning Utility.** An open source disk cleaner for macOS, Windows and Linux. Sapu shows what fills your disk, cleans what is safe to remove, finds duplicate files, and keeps every removal reversible. It is built for people who are not technical and for developers whose disks are full of Docker images, build output and `node_modules`.

Made by **Ramdani** at **PT Sopwer Teknologi Indonesia**. Licensed under **MIT or Apache-2.0**, your choice.

> **Bahasa Indonesia.** Sapu adalah pembersih disk sumber terbuka. Ia memindai apa yang memakan ruang, membersihkan yang aman, mencari file duplikat, dan selalu memindahkan ke Trash dulu sehingga bisa dipulihkan. Dibuat oleh Ramdani dari PT Sopwer Teknologi Indonesia, lisensi MIT atau Apache-2.0: bebas dipakai, diubah, disebarkan, dan dijual ulang selama pemberitahuan lisensi ikut disertakan. Cara berkontribusi ada di [CONTRIBUTING.md](CONTRIBUTING.md).

## What it does

| Screen | What you get |
|---|---|
| **Home** | Free space on the main disk with the recommended minimum, one-click Care, how much Sapu has freed since you started using it, other disks. |
| **Care** | One click: scan the home folder, check the clean-up rules, get a Space Score (0–100) and the safest next steps. Previous runs are kept so you can see the trend. |
| **Scan** | Parallel scan of the home folder, the whole disk or any folder, with live progress (files, bytes, ETA, largest folders and files so far). Results as categories, a treemap, an interactive sunburst with a basket, and a Finder-like browser: large files, Downloads organizer, apps built for another CPU, folder by folder. Quick Look with Space. |
| **Clean** | Declarative rules (TOML) for caches, logs, leftovers, installers, Trash on every volume, Mail attachments, old screenshots, device backups, and for developers: Docker, Xcode, package managers, editor caches, and dead projects (`node_modules`, `target`, `build`, …) with their rebuild command. Green rules are preselected; yellow ones need a look. The check runs in the background. |
| **Duplicates** | Size, partial hash, then full BLAKE3 hash. One copy is always kept, by rule. Optional hardlinking instead of removal. |
| **Apps** (macOS) | Installed apps with size and last use, their leftovers in `~/Library`, orphaned leftovers of apps that are gone, and apps that need Rosetta or cannot run at all. |
| **Menu bar** | Disk, memory and CPU at a glance, and Care from the tray. Closing the window keeps Sapu and its tasks running. |
| **AI (optional, off by default)** | "What is this?", explain a project's artefacts, natural-language search, a narrative report. Local (Ollama or any OpenAI-compatible server) or cloud with your own key (OpenAI, Anthropic, DeepSeek, Gemini, Groq, Mistral, OpenRouter, xAI, Together, Moonshot, or a custom address). Only metadata leaves the device, home folder and account name are masked, every request can be previewed, and an AI answer can never mark something green. |

Two languages (Bahasa Indonesia, English), light and dark theme, keyboard navigation, no telemetry.

## Safety, the part that is not negotiable

Every removal goes through one module, [`crates/sapu-core/src/safety/`](crates/sapu-core/src/safety/), which

- moves to the OS trash by default; permanent deletion needs a second, explicit confirmation,
- refuses protected paths (OS folders, credentials, your personal folders) no matter what a rule, a setting or an AI answer says,
- re-validates every file right before removal: still there, same size and modification time, symlinks stay in scope,
- is a dry run unless you say `--apply`,
- records every real session so it can be restored (Windows and Linux restore from Sapu; macOS from Finder's Trash),
- never removes every copy in a duplicate group.

`clippy.toml` and `scripts/check-forbidden-fs.sh` fail the build if a removal API appears anywhere else. The AI crate cannot even depend on the core (`scripts/check-ai-isolation.sh`).

## Install

Nothing is published yet. Build from source (below) or watch the Releases page; tagged versions build a `.dmg`, `.msi`/`.exe`, `.deb`/`.rpm`/AppImage and CLI archives through `.github/workflows/release.yml`. The macOS build is not notarized yet (see `docs/release.md`), so Gatekeeper asks you to confirm the first launch.

To see the whole disk on macOS, grant Sapu **Full Disk Access** in System Settings → Privacy & Security; the onboarding wizard takes you there.

## Build from source

Requirements: Rust 1.88+, Node 20+ with pnpm, and the [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS.

```sh
git clone git@github.com:xhijack/sapu.git
cd sapu
cargo build --release -p sapu-cli          # the `sapu` command
cd app && pnpm install
pnpm tauri dev                             # the desktop app, with hot reload
pnpm tauri build                           # installers under app/src-tauri/target/release/bundle
```

## CLI

```sh
sapu scan <path> [--json] [--top N]             # what takes up space (allocated size on disk)
sapu clean [--profile dev] [--include-yellow]    # dry run; --apply to execute
sapu dupes <path> [--keep original|oldest|newest] [--hardlink] [--apply]
sapu ai status | whatis <path> | search "<sentence>" <root>   # optional, metadata only
sapu protected                                   # what Sapu will never touch
sapu trash <paths>... [--apply]
sapu history [restore <id>]
```

## Repository layout

```
crates/sapu-core   engine: safety, scanner, rules, cleaner (incl. dead projects), dedupe, care, apps, eta, platform
crates/sapu-ai     optional AI layer (providers, metadata-only tasks); never depends on sapu-core
crates/sapu-cli    thin CLI (`sapu`)
app/               Tauri 2 + React/TypeScript app; src-tauri commands only call the core
rules/             declarative TOML rules per OS; user rules live in <config>/sapu/rules
locales/           id.json / en.json (kept in parity by a test)
docs/              design specs, benchmarks, release notes
packaging/         Homebrew, Scoop and Flatpak templates
```

The product spec is [`PRD.md`](PRD.md); the rules every change must follow are in [`CLAUDE.md`](CLAUDE.md).

## Checks

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
sh scripts/check-forbidden-fs.sh
sh scripts/check-ai-isolation.sh
cd app && pnpm check        # lint, typecheck, tests
```

## Contributing

Bug reports, clean-up rules for an application you know well, translations and code are all welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) first: it explains how to add a rule safely, what a pull request needs, and why issues labelled `data-loss` come before everything else.

## No warranty

Sapu removes files. It is built to be safe (Trash first, protected paths, re-validation, history), but it is provided **as is, without warranty of any kind**. The developer and PT Sopwer Teknologi Indonesia are not liable for system damage or data loss resulting from the use of this software. The app shows this notice on first run and asks you to accept it. Look at anything labelled yellow before ticking it.

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option. You may use, modify, redistribute and sell Sapu or works derived from it; keep the copyright and license notices.

Copyright © 2026 Ramdani, PT Sopwer Teknologi Indonesia.
