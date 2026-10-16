# Sapu

**Smart Analyzer & Pruning Utility.** An open source disk cleaner for Windows, macOS and Linux: scan what fills your disk, clean what is safe to remove, and deduplicate at scale. Built for people who are not technical *and* for developers whose disks are full of Docker images and `node_modules`.

> Status: Tahap 0a (core foundation + safety layer). No GUI yet. See [`PRD.md`](PRD.md) for the product spec and [`CLAUDE.md`](CLAUDE.md) for contributor rules.

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
crates/sapu-core   engine: error, progress, safety (scanner/rules/cleaner/dedupe/ai come next)
crates/sapu-cli    thin CLI (`sapu`)
docs/              design specs and plans
rules/, locales/   declarative rules per OS and UI strings (Tahap 0b+)
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
sapu protected                 # what Sapu will never touch
sapu trash <paths>...          # dry run
sapu trash <paths>... --apply  # move to trash
sapu history                   # past sessions
sapu history restore <id>      # Windows/Linux; macOS: use Finder's Trash
```

## License

Not decided yet (see PRD §15 Q2).
