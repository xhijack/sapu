# CLAUDE.md — Panduan Kerja di Repo SAPU (Smart Analyzer & Pruning Utility)

> Dokumen ini untuk Claude Code (dan developer manusia) yang bekerja di repo ini.
> **Baca `PRD.md` terlebih dahulu** untuk konteks produk lengkap. File ini berisi aturan kerja, bukan spesifikasi produk.

## Apa ini

Sapu: utilitas pembersih disk open source lintas platform (Windows, macOS, Linux). Tiga fungsi bertahap: **scan → bersih → deduplikasi**, plus mode AI opsional. Target pengguna: orang awam **dan** developer.

## Stack

- **Core & CLI:** Rust (stable terbaru), workspace Cargo
- **GUI:** Tauri 2 + React + TypeScript + Tailwind CSS + shadcn/ui
- **Visualisasi:** d3 / visx
- **Crate kunci:** `jwalk`, `blake3`, `trash`, `serde`, `clap`, `thiserror`, `tracing`

## Struktur repo (target)

```
sapu/
├── crates/
│   ├── sapu-core/        # SEMUA logika: scanner, rules, cleaner, dedupe, safety, ai
│   │   └── src/
│   │       ├── scanner/
│   │       ├── rules/
│   │       ├── cleaner/
│   │       ├── dedupe/
│   │       ├── safety/   # trash, protected paths, riwayat, validasi ulang
│   │       └── ai/       # abstraksi provider, filter schema
│   └── sapu-cli/         # lapisan tipis di atas core
├── app/                  # Tauri 2 app
│   ├── src-tauri/        # command Tauri, hanya memanggil sapu-core
│   └── src/              # React frontend
├── rules/                # file rules deklaratif per OS (windows/, macos/, linux/, common/)
├── locales/              # id.json, en.json
├── docs/
├── PRD.md
└── CLAUDE.md
```

## Aturan arsitektur (wajib)

1. **Semua logika di `sapu-core`.** GUI (`src-tauri`) dan CLI hanya memanggil fungsi core. Jangan menulis logika bisnis di frontend React atau di command Tauri.
2. **Satu-satunya jalan untuk menghapus file adalah lewat modul `safety`.** Tidak boleh ada `std::fs::remove_file`, `remove_dir_all`, atau sejenisnya di luar `sapu-core/src/safety/`. Tambahkan lint/CI check untuk ini.
3. **Modul `ai` tidak punya akses ke modul `safety` atau `cleaner`.** AI hanya mengembalikan saran, penjelasan, atau filter terstruktur. Ini harus dijaga oleh struktur dependensi modul, bukan hanya konvensi.
4. **Core tidak tahu soal UI.** Core mengembalikan data terstruktur dan event progress (channel/callback); format tampilan adalah urusan GUI/CLI.
5. **Kode spesifik OS** diisolasi di balik trait/modul platform dengan `#[cfg(target_os = ...)]`, bukan tersebar di mana-mana.

## Aturan keamanan (tidak bisa ditawar)

Pelanggaran aturan ini adalah bug kritis, apa pun alasannya.

- **Default pindah ke Trash.** Hapus permanen hanya lewat opsi eksplisit dengan konfirmasi kedua.
- **Protected paths mutlak.** Tidak bisa ditimpa oleh rules, konfigurasi pengguna biasa, maupun AI.
- **Validasi ulang tepat sebelum menghapus:** file masih ada, ukuran & mtime sama dengan saat scan, bukan symlink keluar scope.
- **Jangan ikuti symlink** saat scan secara default. Hardlink tidak dihitung ganda.
- **Deduplikasi tidak pernah menghapus semua salinan** dalam satu grup. Dijamin di level core.
- **AI maksimal memberi label kuning.** Label hijau hanya dari rules deterministik.
- **CLI default dry-run.** Eksekusi butuh `--apply`.
- **Setiap sesi pembersihan dicatat** di riwayat agar bisa dipulihkan.

## Testing

- Logika di `safety/`, `cleaner/`, dan `dedupe/` wajib memiliki test paling lengkap di repo.
- Test penghapusan berjalan di **filesystem sementara** (`tempfile`), tidak pernah di path nyata.
- Setiap rule di `rules/` wajib punya test yang membuktikan rule tersebut tidak menyentuh protected paths.
- CI menjalankan test di Windows, macOS, dan Linux.
- Sebelum menyatakan pekerjaan selesai: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, serta lint & typecheck frontend.

## UI/UX

Prinsip lengkap ada di `PRD.md` bagian 9–10. Ringkasan yang wajib diingat saat menulis komponen:

- **Anggap pengguna tidak punya pengetahuan teknis.** Teks UI pakai bahasa manusia; istilah teknis hanya di panel detail.
- **Tidak ada string hardcoded di UI.** Semua teks lewat `locales/` (Bahasa Indonesia + Inggris).
- **Warna risiko konsisten:** hijau = aman, kuning = periksa dulu, merah = jangan disentuh (tidak bisa dicentang). Selalu disertai ikon/teks, tidak hanya warna.
- **Satu aksi utama per layar.**
- **Rancang empty, loading, dan error state** dengan sengaja.
- **UI tidak boleh freeze.** Operasi berat berjalan di core secara async dengan progress dan bisa dibatalkan.
- **Setiap operasi panjang wajib punya progress + estimasi waktu selesai** sesuai `PRD.md` bagian 10.1: moving average, smoothing, bahasa manusia ("sekitar 2 menit lagi"), dan status indeterminate bila progress tidak bisa diukur. Jangan pernah menampilkan persentase atau ETA palsu.
- Aksesibilitas: navigasi keyboard penuh, kontras WCAG AA.

## Konvensi kode

- Rust: error pakai `thiserror` di core, `anyhow` boleh di CLI. Hindari `unwrap()`/`expect()` di jalur produksi.
- Logging pakai `tracing`. **Jangan log path lengkap pengguna** di level info ke atas tanpa alasan.
- TypeScript strict mode. Komponen fungsional + hooks.
- Commit message: Conventional Commits (`feat:`, `fix:`, `refactor:`, ...). Lihat bagian "Aturan commit" di bawah.
- Dokumentasi publik dan komentar kode dalam bahasa Inggris (proyek open source); teks UI dua bahasa.

## Aturan commit (wajib)

- **Jangan pernah memakai Claude sebagai author atau co-author commit.** Author commit adalah identitas dari `git config user.name` / `user.email` milik maintainer. Jangan mengubah konfigurasi git tersebut.
- **Jangan menambahkan trailer atau baris atribusi AI** di commit message maupun deskripsi PR, termasuk:
  - `Co-Authored-By: Claude ...`
  - `Generated with Claude Code` (atau variasinya, termasuk emoji)
  - `Claude-Session: ...`
- Commit message hanya berisi ringkasan perubahan sesuai Conventional Commits, ditulis dalam bahasa Inggris.

Pengamanan berlapis (karena instruksi saja bisa terlewat):

1. **Setting Claude Code** di repo, file `.claude/settings.json`:

   ```json
   {
     "attribution": {
       "commit": "",
       "pr": ""
     }
   }
   ```

2. **Git hook `commit-msg`** sebagai jaring pengaman terakhir. Simpan di `.githooks/commit-msg` lalu aktifkan dengan `git config core.hooksPath .githooks`:

   ```sh
   #!/bin/sh
   # Hapus baris atribusi AI dari commit message
   sed -i.bak \
     -e '/^Co-[Aa]uthored-[Bb]y: Claude/d' \
     -e '/Generated with \[\{0,1\}Claude Code/d' \
     -e '/^Claude-Session:/d' \
     "$1" && rm -f "$1.bak"
   ```

   Pastikan file executable: `chmod +x .githooks/commit-msg`.

## Mode AI

- Provider diabstraksi di `sapu-core/src/ai/`: lokal (Ollama/OpenAI-compatible) dan cloud BYOK.
- **Hanya metadata yang boleh dikirim** (path, ukuran, umur, ekstensi). Isi file tidak pernah dikirim ke provider cloud.
- Fitur berbasis konten file (mis. deteksi foto mirip) **hanya untuk model lokal**.
- Output AI yang berupa filter wajib divalidasi terhadap skema sebelum dieksekusi.
- API key disimpan di keychain OS, bukan file teks.

## Urutan kerja

Ikuti roadmap di `PRD.md` bagian 11:
**Tahap 0 (Fondasi) → Tahap 1 (Scan) → Tahap 2 (Bersih) → Tahap 3 (Deduplikasi)**, dengan Mode AI sebagai lapisan setelah fondasi stabil.

Jangan mulai tahap berikutnya sebelum modul safety di Tahap 0 selesai dan teruji.

## Saat ragu

- Kalau sebuah keputusan menyangkut **penghapusan file**, pilih opsi yang paling konservatif dan tanyakan ke maintainer.
- Kalau ada konflik antara kemudahan implementasi dan kejelasan bagi pengguna awam, **prioritaskan pengguna awam**.
- Pertanyaan yang belum terjawab tercatat di `PRD.md` bagian 15. Jangan mengasumsikan jawabannya.
