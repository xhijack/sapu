# Design: Mode AI (PRD §12)

Date: 2026-10-16 · Status: approved (goal "lanjutkan sampai selesai") · Scope: A7, A1, A2, A3, A4 + §12.3–12.5 safety and privacy. A5/A6/A8 later.

## Structural guarantees

- New crate `crates/sapu-ai` **does not depend on `sapu-core`**. It cannot reach `safety`, `cleaner` or the filesystem walker (rule 3 of CLAUDE.md, AI-1). The app and CLI translate core data into metadata-only inputs.
- AI verdicts are typed as `Verdict { Keep, CheckFirst }`: the type has no "safe" variant, so an AI answer can never become a green label (AI-2). Protected paths are checked by core as always (AI-3).
- Filters produced by A2 are deserialised into `SearchFilter` with bounded fields and rejected on unknown keys (AI-5); core runs them deterministically.
- Every answer carries `confidence` and `reason` and is shown under a "Saran AI" label (AI-4).

## Privacy (§12.4)

- `Anonymizer` replaces the home folder with `~` and the account name with `<user>` in every string before it leaves the device (default on, toggle in Settings).
- `preview(kind, input)` returns the exact JSON that would be sent, so the UI can show "data yang akan dikirim" before each request.
- Only metadata travels: paths, sizes, ages, extensions, counts, framework/script/folder names. Never file contents.
- Answers are cached per `(task, input hash)` in `ai-cache.json`.

## Providers (§12.5)

`Provider` trait with one method `complete(system, user) -> text`. Implementations: Ollama (`/api/chat`, also lists models from `/api/tags`), OpenAI-compatible (`/v1/chat/completions`, any base URL), Anthropic (`/v1/messages`). API keys live in the OS keychain via `keyring` (service `sapu`). Q7 stays open: the local model is chosen from what Ollama reports as installed; nothing is hard-coded.

## Tasks

| Task | Input (metadata) | Output |
|---|---|---|
| A1 `whatis` | path, is_dir, bytes, file count, top extensions, newest/oldest mtime age | summary, likely_owner, verdict, consequence, confidence, reason |
| A7 `artefact` | project name, ecosystem, framework deps, script names, artefact kind, gitignored, tracked, idle days, bytes, rebuild | per-artefact: regenerable, verdict, explanation, rebuild, confidence |
| A2 `search` | sentence, available categories | `SearchFilter { category?, min_bytes?, max_age_days?/min_age_days?, extensions?, name_contains? }` |
| A3 `report` | totals, categories, top folders, diff | three short paragraphs: what fills the disk, where to start, what to leave alone |
| A4 `save rule` | confirmed A1 answer | TOML rule file in `<config>/sapu/rules/` (yellow, `dir_contents`), loaded with the embedded rules |

## App

Settings → "Bantuan AI": mode, local URL + model picker, cloud provider + URL + model + key (keychain), anonymise toggle, "Uji koneksi". Results → folder rows get "Apa ini?"; summary tab gets "Buat ringkasan"; largest tab gets a natural-language search box. Bersih → project cards get "Jelaskan dengan AI". Every AI panel: label, confidence bar, reason, "Lihat data yang dikirim", and for A1 "Simpan sebagai aturan".

CLI: `sapu ai status`, `sapu ai whatis <path>`, `sapu ai search "<sentence>" <root>`.
