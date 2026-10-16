# Releasing Sapu

## What a release produces

Pushing a tag `vX.Y.Z` runs `.github/workflows/release.yml`, which creates a **draft** GitHub release with:

- macOS universal `.dmg` and `.app`
- Windows x64 and arm64 `.msi` (WiX) and `.exe` (NSIS, per-user install)
- Linux x64 and arm64 `.AppImage`, `.deb`, `.rpm`
- CLI archives for six targets (`sapu-cli-vX.Y.Z-<target>.tar.gz|zip`)

Review the draft, write the notes, then publish. `cargo install sapu-cli` works once the crate is published to crates.io (`cargo publish -p sapu-core && cargo publish -p sapu-ai && cargo publish -p sapu-cli`; names are reserved per PRD).

## Signing and notarization (PRD §13, blocked on Q3)

Unsigned builds make macOS and Windows show scary warnings, which contradicts the UX principles. Nothing in the workflow requires the secrets, so test builds work today; set these repository secrets to sign:

| Platform | Secrets | Where from |
|---|---|---|
| macOS | `APPLE_CERTIFICATE` (base64 .p12), `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY` ("Developer ID Application: …"), `APPLE_ID`, `APPLE_PASSWORD` (app-specific), `APPLE_TEAM_ID` | Apple Developer Program (USD 99/yr) |
| Windows | Authenticode via SignPath Foundation (free for OSS) or Azure Trusted Signing; add a signing step to the workflow or `bundle.windows.signCommand` in `tauri.conf.json` | SignPath application |
| Updater | `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` from `pnpm tauri signer generate -w ~/.tauri/sapu.key` | self-generated; keep the private key offline |

## Auto-update (PRD §13, "with user confirmation")

Not wired yet because it needs the updater key pair above. When the key exists:

1. `pnpm add @tauri-apps/plugin-updater` and `cargo add tauri-plugin-updater` in `app/src-tauri`.
2. Set `bundle.createUpdaterArtifacts: true` and `plugins.updater.pubkey` + `endpoints` (`https://github.com/xhijack/sapu/releases/latest/download/latest.json`) in `tauri.conf.json`.
3. Add a "Periksa pembaruan" button in Settings that calls `check()` and asks before `downloadAndInstall()`; never update silently.

## Package managers

Templates live in `packaging/`: Homebrew formula (CLI) and cask (app), Scoop manifest, Flatpak manifest. Fill `version`, `url` and `sha256` from the published release; winget uses `wingetcreate` against the NSIS installer URL.

## Checklist

- [ ] `cargo test --workspace`, `cargo clippy -- -D warnings`, `pnpm check` green on CI for the three OSes
- [ ] Version bumped in `Cargo.toml` (workspace) and `app/package.json`, `app/src-tauri/tauri.conf.json`
- [ ] CHANGELOG entry; licence decided (Q2) and `LICENSE` file present before the first public release
- [ ] Tag pushed, draft reviewed, installers smoke-tested on each OS, release published
