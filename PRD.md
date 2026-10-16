# PRD — SAPU: Smart Analyzer & Pruning Utility

> **Status:** Draft v0.2 · **Pemilik:** Ramdani · **Tanggal:** 16 Oktober 2026
> **SAPU** = **S**mart **A**nalyzer & **P**runing **U**tility. Juga bermakna "sapu" (alat bersih-bersih) dalam Bahasa Indonesia.
> Nama `sapu`, `sapu-core`, `sapu-cli` tersedia di crates.io dan npm per 16 Oktober 2026.

---

## 1. Ringkasan

Sapu adalah aplikasi desktop **open source** untuk Windows, macOS, dan Linux yang membantu pengguna memahami apa yang memakan ruang disk, membersihkan file yang aman dihapus, dan menghapus file duplikat dalam skala besar. Sapu dirancang untuk **dua audiens sekaligus**: pengguna umum tanpa pengetahuan teknis, dan developer yang disknya dipenuhi Docker, `node_modules`, build cache, dan sejenisnya.

Pembeda utama Sapu:

1. **UI/UX setara aplikasi komersial.** Open source tidak berarti boleh jelek.
2. **Keamanan sebagai fondasi.** Semua penghapusan bisa dibatalkan.
3. **Mode AI opsional yang menghormati privasi.** Hanya metadata yang dikirim, bisa memakai model lokal, dan kodenya bisa diaudit.

## 2. Latar Belakang dan Masalah

Disk penuh adalah masalah universal, tetapi solusi yang ada terbelah dua:

- **Tool komersial** (CleanMyMac, DaisyDisk) mudah dipakai, tetapi berbayar, sebagian hanya untuk satu OS, dan tertutup soal apa yang sebenarnya dihapus.
- **Tool open source** (WinDirStat, BleachBit, Czkawka, ncdu, dust, gdu) kuat secara fungsi, tetapi membingungkan bagi orang awam: istilah teknis, UI tidak ramah, dan tidak ada panduan.

Akibatnya, pengguna awam takut menghapus apa pun karena tidak tahu mana yang aman, sementara developer membersihkan secara manual dengan perintah yang berbeda per tool (`docker system prune`, `npm cache clean`, menghapus DerivedData, dan seterusnya).

## 3. Prinsip Produk (Tidak Bisa Ditawar)

| # | Prinsip | Arti dalam praktik |
|---|---------|--------------------|
| P1 | **Anggap pengguna tidak punya pengetahuan teknis** | Bahasa manusia di UI. Istilah teknis hanya di panel detail. |
| P2 | **Tidak ada yang hilang tanpa bisa dipulihkan** | Default = pindah ke Trash/Recycle Bin. Hapus permanen hanya lewat opsi eksplisit. |
| P3 | **Pratinjau sebelum aksi** | Setiap pembersihan menampilkan ringkasan ("akan membebaskan 12,4 GB dari 3.210 file") sebelum dieksekusi. |
| P4 | **AI memberi saran, manusia memutuskan** | AI tidak pernah bisa memicu penghapusan sendiri. |
| P5 | **Privasi yang bisa dibuktikan** | AI default mati. Isi file tidak pernah keluar dari perangkat. |
| P6 | **Dua audiens, dua lapisan** | Mode sederhana untuk pengguna umum, mode developer dengan detail penuh. Bisa ditukar kapan saja. |
| P7 | **Satu aksi utama per layar** | Tombol besar yang jelas, opsi lain sekunder. |

## 4. Tujuan

1. Pengguna awam bisa membebaskan ruang disk dengan aman **tanpa memahami istilah teknis**, dalam kurang dari 3 menit sejak aplikasi pertama dibuka.
2. Developer bisa membersihkan seluruh artefak development (Docker, cache package manager, build output) dari **satu tempat**, lewat GUI maupun CLI.
3. **Nol laporan kehilangan data** yang tidak bisa dipulihkan akibat bug Sapu.
4. Deduplikasi bisa menangani **ratusan ribu file** tanpa membuat sistem tidak responsif.
5. Menjadi proyek open source yang **menarik kontributor**: kode terstruktur, dokumentasi jelas, rules mudah ditambahkan.

## 5. Bukan Tujuan (Non-Goals)

| Bukan tujuan | Alasan |
|--------------|--------|
| Antivirus / deteksi malware | Domain berbeda, tanggung jawab dan risikonya jauh lebih besar. |
| "Optimasi" RAM, registry cleaner, speed booster | Manfaatnya diragukan dan sering merusak sistem. Bertentangan dengan prinsip P2. |
| Uninstaller aplikasi lengkap | Bisa dipertimbangkan nanti (lihat P2 requirement), bukan fokus awal. |
| Sinkronisasi cloud / backup | Di luar masalah inti. Sapu membersihkan, bukan menyimpan. |
| Komersialisasi / fitur berbayar | Proyek ini sepenuhnya open source dan non-komersial. |
| Berjalan otomatis di background tanpa izin | Pembersihan selalu atas inisiatif pengguna (penjadwalan opsional bisa dipertimbangkan nanti). |

## 6. Persona

**Bu Rina — pengguna umum.** Staf admin, laptop Windows 256 GB selalu penuh. Tidak tahu apa itu "cache". Takut menghapus sesuatu yang salah. Yang dia butuhkan: tombol yang jelas dan jaminan bahwa semuanya bisa dibatalkan.

**Dimas — developer.** Pakai MacBook untuk web dan mobile development. Disk 512 GB habis oleh Docker image, `node_modules` di 40 proyek lama, Xcode DerivedData, dan simulator iOS. Ingin satu tool yang paham semua itu, lebih suka CLI, tetapi sesekali butuh visual untuk melihat gambaran besar.

**Andi — sysadmin server.** Mengelola server Linux via SSH. Disk server penuh oleh log, backup lama, dan Docker volume yatim. Butuh CLI headless dengan output JSON yang bisa diotomasi.

## 7. Arsitektur Tingkat Tinggi

```
┌─────────────────────────────────────────────────────────┐
│  GUI (Tauri 2 + React + TypeScript)   │   CLI (Rust)    │
├─────────────────────────────────────────────────────────┤
│                    sapu-core (Rust)                     │
│  scanner │ rules engine │ cleaner │ dedupe │ safety     │
│                  ai (provider abstraction)              │
├─────────────────────────────────────────────────────────┤
│        Platform layer: Windows │ macOS │ Linux          │
└─────────────────────────────────────────────────────────┘
```

**Keputusan teknologi:**

| Area | Pilihan | Alasan |
|------|---------|--------|
| Core | **Rust** | Performa, kontrol low-level (mis. pembacaan MFT NTFS di masa depan), ekosistem tool utilitas yang kuat dan menarik kontributor. |
| GUI | **Tauri 2 + React + TypeScript** | Binary kecil (pakai webview sistem), satu ekosistem Rust, visualisasi mudah di web. |
| Styling | Tailwind CSS + shadcn/ui | Design system konsisten, light/dark mode. |
| Visualisasi | d3 / visx | Treemap dan sunburst. |
| Crate kunci | `jwalk` (walk paralel), `blake3` (hash), `trash` (pindah ke Trash lintas OS), `serde`, `clap` (CLI) | Matang dan teruji. |
| i18n | Bahasa Indonesia + Inggris sejak hari pertama | Menambahkan belakangan jauh lebih mahal. |

**Aturan arsitektur:** semua logika ada di `sapu-core`. GUI dan CLI hanya lapisan tipis yang memanggil core. Tidak ada logika penghapusan di frontend.

## 8. Fondasi Keamanan (Berlaku di Semua Tahap)

Ini adalah bagian paling kritis. Satu bug penghapusan bisa menghancurkan reputasi proyek.

- **S1 — Trash by default.** Semua penghapusan memindahkan file ke Trash/Recycle Bin. Hapus permanen hanya lewat opsi eksplisit dengan konfirmasi kedua.
- **S2 — Riwayat & pemulihan.** Setiap sesi pembersihan dicatat (apa, kapan, berapa besar, dari mana). Halaman "Riwayat Pembersihan" bisa memulihkan per sesi atau per file selama masih ada di Trash.
- **S3 — Protected paths.** Daftar lokasi yang **tidak bisa dihapus dalam kondisi apa pun**, dan tidak bisa ditimpa oleh AI maupun rules komunitas. Contoh: direktori sistem OS, `~/.ssh`, `~/.gnupg`, folder Dokumen/Foto pengguna (untuk modul Bersih), keychain, dan file konfigurasi kredensial.
- **S4 — Dry-run.** CLI default ke dry-run; perlu flag eksplisit (`--apply`) untuk eksekusi. GUI selalu menampilkan pratinjau.
- **S5 — Validasi ulang sebelum hapus.** Tepat sebelum menghapus, cek ulang bahwa file masih ada, ukuran/mtime tidak berubah sejak scan, dan bukan symlink yang mengarah ke luar scope.
- **S6 — Jangan ikuti symlink** saat scanning secara default. Tangani hardlink agar ukuran tidak dihitung ganda.
- **S7 — Test suite khusus penghapusan.** Logika penghapusan wajib memiliki cakupan test tertinggi di proyek, termasuk test di filesystem sementara untuk setiap OS di CI.

## 9. Onboarding Wizard

Wizard muncul saat aplikasi pertama kali dibuka. Setiap langkah punya tombol **"Lewati, saya sudah paham"**.

1. **Sambutan.** Satu kalimat: apa yang dilakukan Sapu.
2. **Pilih profil.** "Pengguna biasa" atau "Developer". Dideteksi otomatis: kalau Docker, Node, Xcode, dsb. terpasang, sarankan profil Developer.
3. **Izin akses.** Panduan langkah demi langkah bergambar, spesifik per OS:
   - macOS: Full Disk Access (titik gesekan terbesar), dengan deteksi otomatis bahwa izin sudah diberikan.
   - Windows: opsi menjalankan sebagai admin untuk scan lebih cepat dan lengkap.
   - Linux: penjelasan scope akses.
4. **Jaminan keamanan.** Jelaskan Trash, riwayat, dan pemulihan. Tujuannya menurunkan rasa takut.
5. **Mode AI (opsional, default mati).** Jelaskan dengan jujur data apa yang dikirim, lalu pilihan: mati / model lokal / cloud dengan API key sendiri.
6. **Scan pertama.** Langsung jalankan scan agar pengguna segera melihat hasil, bukan layar kosong.

**Kriteria penerimaan:**
- [ ] Pengguna baru bisa menyelesaikan wizard dan melihat hasil scan pertama dalam kurang dari 2 menit (di luar durasi scan).
- [ ] Setiap langkah bisa dilewati dan wizard bisa dibuka ulang dari Pengaturan.
- [ ] Di macOS, aplikasi mendeteksi status Full Disk Access tanpa restart.

## 10. Prinsip UI/UX

- **Bahasa manusia:** "File sementara aplikasi" bukan "cache"; "Sisa instalasi" bukan "orphaned packages". Istilah teknis tersedia di panel detail.
- **Warna risiko konsisten di seluruh aplikasi:**
  - 🟢 **Hijau — Aman dihapus.** Akan dibuat ulang otomatis oleh aplikasinya, tidak ada data pengguna.
  - 🟡 **Kuning — Periksa dulu.** Mungkin masih dipakai atau berisi data pengguna.
  - 🔴 **Merah — Jangan disentuh.** Ditampilkan untuk informasi saja, tidak bisa dicentang.
- **Empty state, loading state, dan error state** dirancang dengan sengaja, bukan default.
- **Progress yang jujur dengan estimasi waktu selesai:** lihat bagian 10.1.
- **Aksesibilitas:** navigasi keyboard penuh, kontras memenuhi WCAG AA, warna risiko selalu disertai ikon/teks (tidak hanya warna).
- **Uji kegunaan dengan orang non-teknis** sebelum setiap rilis tahap.

### 10.1 Progress dan Estimasi Waktu Selesai (Lintas Tahap)

Setiap operasi yang bisa berjalan lebih dari beberapa detik (scan, bersih, deduplikasi) wajib menampilkan progress dan **estimasi waktu selesai (ETA)**. Bagi pengguna awam, layar yang "diam" terasa seperti aplikasi macet.

**Tantangan teknis per operasi:**

| Operasi | Penyebut progress | Tingkat kesulitan |
|---------|-------------------|-------------------|
| **Scan pertama** | Total file **tidak diketahui** di awal. Pakai **ruang disk terpakai** dari OS (statvfs / GetDiskFreeSpaceEx) sebagai penyebut: byte terscan ÷ byte terpakai. | Sulit; hasil berupa perkiraan. |
| **Scan berikutnya** | Jumlah file dan ukuran dari **scan sebelumnya** sebagai penyebut. | Sedang; jauh lebih akurat. |
| **Bersih** | Jumlah file dan total byte sudah diketahui dari pratinjau. | Mudah. |
| **Deduplikasi (hashing)** | Total byte kandidat yang perlu di-hash diketahui setelah tahap pengelompokan ukuran. | Mudah, per sub-tahap. |
| **Perintah eksternal** (mis. `docker system prune`) | Tidak ada progress dari luar. | Tampilkan status tak tentu (indeterminate) + waktu berjalan, tanpa ETA palsu. |

**Requirements P0:**
- Progress menampilkan: persentase, item diproses (jumlah file, ukuran), **ETA**, dan waktu berjalan.
- ETA dihitung dari **laju bergerak (moving average)**, bukan rata-rata total, agar menyesuaikan saat laju berubah (mis. pindah dari SSD ke folder berisi jutaan file kecil).
- **Tidak ada presisi palsu.** ETA dibulatkan dan ditampilkan dalam bahasa manusia: "sekitar 2 menit lagi", "kurang dari 1 menit", bukan "1:47". Saat ketidakpastian tinggi (awal operasi, scan pertama), tampilkan "Menghitung perkiraan..." atau rentang ("2–4 menit lagi").
- ETA **tidak melompat-lompat liar**: perubahan dihaluskan (smoothing) dan tidak boleh naik-turun drastis setiap detik.
- Operasi multi-tahap (mis. deduplikasi: kelompokkan → hash sebagian → hash penuh) menampilkan **tahap saat ini** ("Langkah 2 dari 3: membandingkan isi file") beserta progress per tahap dan total.
- Progress tidak pernah mundur; persentase tidak pernah melewati 100% sebelum selesai.
- Event progress dikirim dari core dengan frekuensi terbatas (mis. maksimal 10×/detik) agar UI tidak terbebani.
- CLI: progress bar di terminal interaktif; diam atau JSON lines saat output di-pipe (`--json`).

**P1:**
- Estimasi **sebelum** operasi dimulai, di layar pratinjau: "Pembersihan ini diperkirakan memakan waktu sekitar 1 menit", berdasarkan riwayat laju di perangkat ini.
- Notifikasi sistem saat operasi panjang selesai, agar pengguna bisa meninggalkan aplikasi.

**Kriteria penerimaan (contoh):**
- [ ] Untuk scan berikutnya (bukan pertama), ETA di titik 50% berada dalam ±30% dari waktu aktual pada kondisi disk normal.
- [ ] ETA tidak pernah ditampilkan sebagai angka saat belum ada cukup data (mis. 3 detik pertama).
- [ ] Operasi tanpa progress terukur menampilkan status indeterminate, bukan persentase palsu.

---

## 11. Roadmap Bertahap

### Tahap 0 — Fondasi

Struktur proyek (workspace Rust + Tauri), `sapu-core` dengan modul safety (S1–S7), design system, i18n, onboarding wizard, pipeline CI lintas OS, dan rilis (signing/notarization).

### Tahap 1 — Scan: "Apa yang memakan ruang disk saya?"

**User stories:**
- Sebagai Bu Rina, saya ingin melihat ringkasan sederhana apa yang memenuhi disk saya, supaya saya tahu harus mulai dari mana.
- Sebagai Dimas, saya ingin menjelajahi folder terbesar secara visual, supaya saya bisa menemukan sumber masalah dengan cepat.
- Sebagai Andi, saya ingin menjalankan scan via CLI dengan output JSON, supaya saya bisa mengotomasinya di server.

**Requirements P0:**
- Scan seluruh disk, folder home, atau folder pilihan, dengan walk paralel.
- **Tampilan default: ringkasan kategori** (Video, Foto, Dokumen, Aplikasi, File Sistem, File Sementara, Development, Lainnya) dengan bar sederhana dan ukuran.
- Tampilan kedua: treemap/sunburst untuk eksplorasi.
- Daftar file terbesar dan folder terbesar, dengan filter ukuran, tipe, dan umur (terakhir diubah/diakses).
- Aksi per item: buka di file manager, salin path, pindah ke Trash (melalui modul safety).
- Scan bisa dibatalkan dan dijeda. UI tetap responsif selama scan.
- CLI: `sapu scan <path> [--json] [--top N]`.

**P1:**
- Cache hasil scan dan pembaruan inkremental (tidak scan ulang dari nol).
- Perbandingan antar scan ("disk bertambah 8 GB sejak minggu lalu, terutama dari folder X").

**P2 (rancang agar memungkinkan):**
- Pembacaan MFT NTFS langsung di Windows untuk scan sangat cepat.
- Scan disk eksternal dan network drive.

**Kriteria penerimaan (contoh):**
- [ ] Scan 1 juta file di SSD selesai dalam waktu yang kompetitif dengan gdu/dust (target dibenchmark di Tahap 0).
- [ ] Ukuran tidak dihitung ganda untuk hardlink.
- [ ] Symlink tidak diikuti secara default.
- [ ] Folder tanpa izin akses ditampilkan dengan keterangan, bukan error yang menghentikan scan.

### Tahap 2 — Bersih: "Apa yang aman dihapus?"

**User stories:**
- Sebagai Bu Rina, saya ingin satu tombol untuk membersihkan semua yang aman, supaya saya tidak perlu memahami detailnya.
- Sebagai Dimas, saya ingin membersihkan Docker, cache npm/pip/cargo, dan build output dari satu tempat.
- Sebagai siapa pun, saya ingin bisa memulihkan apa yang sudah dibersihkan kalau ternyata saya salah.

**Requirements P0:**
- **Rules engine deterministik** berbasis file deklaratif (mis. TOML/YAML) per OS. Setiap rule berisi: nama ramah pengguna, deskripsi, path/pola, tingkat risiko (hijau/kuning), profil (umum/developer), dan cara pembersihan (hapus file atau jalankan perintah resmi, mis. `docker system prune`).
- Kategori awal:
  - **Umum:** cache browser, file temporary sistem, Recycle Bin/Trash, log lama, thumbnail cache, file installer yang sudah diunduh.
  - **Developer:** Docker (image dangling, container berhenti, build cache, volume yatim, dengan konfirmasi terpisah untuk volume), `node_modules` pada proyek yang tidak disentuh lebih dari N hari, cache npm/yarn/pnpm/pip/cargo/go/gradle/maven, Xcode DerivedData dan simulator lama, Homebrew cache.
- Tampilan daftar kategori dengan warna risiko; kategori hijau dicentang otomatis.
- Tombol utama: **"Bersihkan [X] GB yang aman"**.
- Pratinjau sebelum eksekusi, riwayat, dan pemulihan (S2).
- Untuk Docker, gunakan perintah/API resmi Docker, bukan menghapus file internal Docker secara langsung.
- CLI: `sapu clean [--profile dev] [--apply]`.

**P1:**
- Proyek development "mati": deteksi proyek yang tidak di-commit/diubah dalam N bulan dan tawarkan pembersihan artefak build-nya.
- Rules kontribusi komunitas dengan proses review di repo.

**P2:**
- Uninstaller aplikasi beserta sisa filenya.
- Penjadwalan pembersihan opsional.

**Kriteria penerimaan (contoh):**
- [ ] Tidak ada rule yang bisa menyentuh protected paths (dibuktikan dengan test).
- [ ] Setiap rule baru wajib punya test dan deskripsi dalam dua bahasa.
- [ ] Pembersihan yang gagal sebagian (mis. file terkunci) melaporkan dengan jelas apa yang berhasil dan apa yang tidak.

### Tahap 3 — Deduplikasi: "File apa yang sama persis?"

**User stories:**
- Sebagai Bu Rina, saya ingin menghapus foto dan dokumen ganda tanpa harus memilih satu per satu, dan yakin yang tersisa adalah yang benar.
- Sebagai Dimas, saya ingin mengganti duplikat dengan hardlink/reflink, supaya ruang kembali tanpa ada file yang hilang dari strukturnya.

**Requirements P0:**
- **Pipeline bertingkat:** kelompokkan berdasarkan ukuran → hash sebagian (mis. 4 KB awal dan akhir) → hash penuh BLAKE3 hanya untuk kandidat tersisa.
- Mampu menangani ratusan ribu file dengan pemakaian memori terkendali dan bisa dibatalkan.
- **Pemilihan salinan otomatis yang cerdas**, dengan strategi yang bisa dipilih: simpan yang di folder utama (Dokumen/Foto), yang tertua, atau yang namanya paling asli (bukan "copy (2)", "(1)", dsb.).
- **Pratinjau berdampingan** untuk gambar dan dokumen.
- Pengelompokan hasil per folder ("folder A dan folder B berisi 2.300 file yang sama"), karena ini cara orang awam memahami duplikasi.
- CLI: `sapu dupes <path> [--json] [--apply --keep=oldest]`.

**P1:**
- Ganti dengan hardlink atau reflink (APFS, btrfs, ReFS) di mode developer.
- Abaikan file kecil di bawah ambang ukuran (konfigurabel).

**P2:**
- Deteksi file **mirip** (bukan identik), lihat Mode AI: foto burst, foto blur, versi resolusi berbeda.

**Kriteria penerimaan (contoh):**
- [ ] Tidak pernah menghapus **semua** salinan dalam satu grup duplikat (dijamin di level core, dengan test).
- [ ] Hardlink yang sudah ada tidak dianggap duplikat yang memakan ruang.
- [ ] Hasil tetap benar jika file berubah selama proses scan (validasi ulang S5).

---

## 12. Mode AI (Lapisan Lintas Tahap)

### 12.1 Filosofi

AI di Sapu **bukan pengambil keputusan**, melainkan penerjemah dan penjelas. Rules deterministik menangani kasus yang sudah dikenal; AI menangani "ekor panjang": folder dan file yang tidak dikenali rules.

### 12.2 Ide Fungsi AI

| # | Fungsi | Tahap | Audiens | Penjelasan |
|---|--------|-------|---------|------------|
| A1 | **"Apa ini?"** | 1–2 | Semua | Klik kanan folder tak dikenal → AI menjelaskan kemungkinan aplikasi pemiliknya, apakah aman dihapus, dan apa konsekuensinya. Contoh: "Folder ini kemungkinan cache dari aplikasi Spotify. Aman dihapus; lagu offline perlu diunduh ulang." |
| A2 | **Pencarian bahasa natural** | 1 | Semua | "Video lebih dari 1 GB yang tidak dibuka setahun", "build artifact dari proyek yang sudah lama tidak disentuh". AI menerjemahkan kalimat menjadi **filter terstruktur (JSON)**, lalu filter dieksekusi secara lokal dan deterministik oleh core. |
| A3 | **Laporan naratif** | 1 | Pengguna umum | Setelah scan: "Disk kamu penuh terutama karena 3 hal: video rekaman Zoom (48 GB), cache Docker (22 GB), dan installer lama (9 GB). Mulai dari yang kedua, paling aman." |
| A4 | **AI → rules (flywheel komunitas)** | 2 | Semua | Jawaban AI yang dikonfirmasi pengguna bisa disimpan sebagai rule lokal, dan opsional diajukan sebagai rule komunitas ke repo. Seiring waktu, semakin banyak kasus ditangani rules deterministik dan semakin sedikit butuh AI. |
| A5 | **Penentu salinan yang disimpan** | 3 | Semua | Untuk grup duplikat yang ambigu, AI menyarankan salinan mana yang sebaiknya disimpan berdasarkan konteks path, beserta alasannya. |
| A6 | **File mirip (bukan identik)** | 3+ | Pengguna umum | Foto burst, foto blur, screenshot lama, versi resolusi berbeda. Memakai **model vision/embedding lokal** (mis. CLIP-like, perceptual hash), tidak butuh cloud. Fitur ini sangat bernilai bagi pengguna umum yang galeri fotonya penuh. |
| A7 | **Analisis kontekstual artefak build** ⭐ | 2 | Developer | Fitur unggulan untuk developer. Rules biasa hanya tahu nama folder (`dist`, `build`, `.next`), tetapi tidak tahu **seberapa penting** folder itu. AI menganalisis konteks proyek: framework apa, folder ini output dari perintah apa, apakah di-gitignore, kapan proyek terakhir disentuh, dan bisa dibuat ulang atau tidak. Hasilnya penjelasan seperti: "`dist/` ini output build Vite, di-gitignore, proyek tidak disentuh 7 bulan, bisa dibuat ulang dengan `npm run build`. Aman dihapus." Detail di bagian 12.6. |
| A8 | **Tanya disk kamu (chat)** | 2+ | Semua | Panel chat untuk bertanya bebas tentang hasil scan. Hanya bisa membaca hasil scan dan menyarankan aksi; tidak punya akses untuk menghapus. |

**Prioritas yang disarankan:** A7 dan A1 dulu (nilai terbesar dan paling membedakan Sapu dari tool lain), lalu A2, A3, dan A4, kemudian A5, A6, dan A8.

### 12.3 Aturan Keamanan AI

- **AI-1:** AI tidak pernah memanggil fungsi penghapusan. Output AI hanya berupa saran, penjelasan, atau filter terstruktur.
- **AI-2:** Klasifikasi dari AI **maksimal berwarna kuning**. Hanya rules deterministik yang boleh memberi label hijau.
- **AI-3:** Protected paths tetap berlaku mutlak, apa pun kata AI.
- **AI-4:** Setiap saran AI menampilkan tingkat keyakinan dan alasan, serta label jelas bahwa ini saran AI.
- **AI-5:** Output AI yang berupa filter divalidasi terhadap skema sebelum dieksekusi.

### 12.4 Privasi

- AI **default mati**.
- Hanya **metadata** yang dikirim: path, ukuran, umur, ekstensi, jumlah file. **Isi file tidak pernah dikirim.**
- Opsi **anonimisasi path**: nama pengguna dan segmen path pribadi diganti token sebelum dikirim.
- Pratinjau "data yang akan dikirim" bisa dibuka sebelum setiap permintaan (mode transparansi).
- Fungsi berbasis konten (A6) **hanya boleh berjalan dengan model lokal**.

### 12.5 Provider

Lapisan abstraksi provider di `sapu-core`:
- **Lokal:** Ollama (atau server OpenAI-compatible lokal lainnya).
- **Cloud, BYOK:** OpenAI-compatible API, Anthropic, dan lainnya, memakai API key milik pengguna. Key disimpan di keychain OS, bukan file teks.
- Cache jawaban AI per path/pola agar tidak bertanya berulang.

### 12.6 Analisis Kontekstual Artefak Build (A7)

**Masalah.** Developer frontend (dan developer lain) punya puluhan proyek, masing-masing dengan `node_modules`, `dist`, `build`, `.next`, `.nuxt`, `.turbo`, `.parcel-cache`, `coverage`, `storybook-static`, dan seterusnya. Tool utilitas biasa hanya bisa mencocokkan nama folder, sehingga tidak bisa membedakan `build/` hasil kompilasi (aman dihapus) dari `build/` yang berisi script build buatan tangan (jangan dihapus).

**Cara kerja: dua langkah.**

**Langkah 1, ekstraksi sinyal secara lokal dan deterministik** (oleh core, tanpa AI):

| Sinyal | Sumber | Kegunaan |
|--------|--------|----------|
| Root proyek & ekosistem | `package.json`, `Cargo.toml`, `pyproject.toml`, `go.mod`, `pubspec.yaml`, `pom.xml`, dll. | Tahu ini proyek apa. |
| Framework & build tool | Dependencies (`next`, `vite`, `nuxt`, `@angular/core`, `react-scripts`, dll.) | Tahu folder output standarnya. |
| Folder output yang dikonfigurasi | `vite.config.*` (`outDir`), `next.config.*` (`distDir`), `tsconfig.json` (`outDir`), `angular.json`, dll. | Cocokkan folder dengan output nyata, bukan tebakan nama. |
| Perintah pembuat ulang | `scripts` di `package.json` (`build`, `dev`), lockfile | Bisa memberi tahu cara membuat ulang (`npm run build`, `pnpm install`). |
| Status gitignore | `.gitignore` proyek | Sinyal kuat: folder yang di-ignore biasanya hasil generate. |
| Status di git | Apakah folder berisi file yang di-track git | Kalau ada file yang di-track, **bukan** artefak; jangan disarankan. |
| Umur aktivitas | Commit terakhir, mtime file sumber terakhir | Proyek aktif vs. ditinggalkan. |
| Ukuran & jumlah file | Hasil scan | Prioritas dampak. |

**Langkah 2, penalaran oleh AI** atas sinyal tersebut (bukan atas kode sumber):
- Menyimpulkan apakah folder adalah artefak yang bisa dibuat ulang, beserta alasannya dalam bahasa manusia.
- Memberi tahu **perintah untuk membuat ulang** bila nanti dibutuhkan.
- Menangani framework atau setup yang tidak dikenali rules (monorepo custom, build tool baru, konfigurasi tidak standar).
- Mengelompokkan saran per proyek: "Proyek `landing-page-2024`: tidak disentuh 7 bulan, 1,8 GB bisa dibebaskan (node_modules 1,4 GB, .next 400 MB)."

**Hubungan dengan rules deterministik:**
- Kombinasi sinyal yang sangat jelas (mis. folder sesuai `outDir` yang dikonfigurasi, di-gitignore, tidak ada file yang di-track git) boleh ditangani **rule deterministik** dan mendapat label hijau.
- Kasus yang lebih ambigu ditangani AI dan maksimal mendapat label kuning (aturan AI-2).
- Jawaban AI yang dikonfirmasi pengguna bisa menjadi rule baru (A4), sehingga makin banyak framework tertangani tanpa AI.

**Privasi khusus A7:**
- File konfigurasi dibaca **secara lokal**. Yang dikirim ke AI hanya **sinyal turunan** (nama framework, nama script, nama folder output, status gitignore, umur), bukan isi file utuh dan bukan kode sumber.
- Nama paket privat atau nama proyek dari `package.json` ikut dianonimkan bila opsi anonimisasi aktif.

**Kriteria penerimaan (contoh):**
- [ ] Folder yang berisi file yang di-track git **tidak pernah** disarankan untuk dihapus, apa pun kata AI.
- [ ] Setiap saran menampilkan alasan dan perintah pembuat ulang bila diketahui.
- [ ] Tanpa AI aktif, kasus standar (Next.js, Vite, CRA, Nuxt, Angular) tetap terdeteksi oleh rules.

---

## 13. Distribusi dan Rilis

- Build otomatis untuk Windows (x64, arm64), macOS (Intel, Apple Silicon), Linux (x64, arm64) lewat CI.
- Format: `.msi`/`.exe` (Windows), `.dmg` (macOS), AppImage + `.deb` + `.rpm` + Flatpak (Linux). CLI juga via Homebrew, Scoop/winget, dan `cargo install`.
- **Code signing dan notarization** wajib untuk Windows dan macOS. Tanpa ini, pengguna awam akan melihat peringatan keamanan yang menakutkan, yang bertentangan langsung dengan prinsip UX proyek ini.
- Auto-update dengan konfirmasi pengguna.

## 14. Metrik Keberhasilan

Karena proyek open source tanpa telemetri default, metrik diukur dari sinyal publik dan uji kegunaan:

**Indikator cepat:**
- Pengguna non-teknis dalam uji kegunaan berhasil membersihkan disk tanpa bantuan: target ≥ 90%.
- Waktu dari buka aplikasi hingga pembersihan pertama: target < 3 menit.
- Laporan bug terkait kehilangan data: target 0.

**Indikator jangka panjang:**
- Jumlah unduhan per rilis, bintang GitHub, dan jumlah kontributor aktif.
- Jumlah rules komunitas yang diterima.
- Sentimen issue dan diskusi (keluhan UX vs. permintaan fitur).

Telemetri, jika ada, harus **opt-in**, anonim, dan dijelaskan di onboarding.

## 15. Pertanyaan Terbuka

| # | Pertanyaan | Siapa | Blocking? |
|---|------------|-------|-----------|
| Q1 | Kepanjangan final SAPU (usulan saat ini: Smart Analyzer & Pruning Utility) dan ketersediaan domain. | Ramdani | Ya, sebelum rilis publik |
| Q2 | Lisensi: MIT/Apache-2.0 (dual, konvensi Rust, ramah adopsi) atau GPL-3.0 (memastikan turunan tetap open source)? | Ramdani | Ya, sebelum repo publik |
| Q3 | Pendanaan sertifikat code signing (Apple Developer, sertifikat Windows) untuk proyek non-komersial: dana pribadi, sponsor, atau program seperti SignPath untuk OSS? | Ramdani | Ya, sebelum rilis publik |
| Q4 | Siapa desainer UI/UX proyek ini? Desain sebaiknya dikerjakan sebelum implementasi GUI Tahap 1. | Ramdani | Ya, untuk Tahap 1 |
| Q5 | Telemetri: tidak ada sama sekali, atau opt-in? | Ramdani | Tidak |
| Q6 | Format file rules: TOML atau YAML? Bagaimana alur review rules komunitas? | Engineering | Tidak (sebelum Tahap 2) |
| Q7 | Model lokal mana yang direkomendasikan untuk A1/A2 (ukuran kecil, berjalan di laptop biasa)? | Engineering | Tidak (sebelum Mode AI) |
| Q8 | Dukungan Linux: distro mana yang menjadi target utama pengujian? | Engineering | Tidak |

## 16. Glosarium

- **Rules engine:** kumpulan aturan deterministik yang mendefinisikan lokasi dan cara membersihkan file yang dikenal.
- **Protected paths:** lokasi yang tidak boleh dihapus dalam kondisi apa pun.
- **Dry-run:** simulasi pembersihan tanpa benar-benar menghapus.
- **Reflink:** salinan yang berbagi blok data di level filesystem (APFS, btrfs) sehingga tidak memakan ruang tambahan.
- **BYOK:** *Bring Your Own Key*, pengguna memakai API key AI miliknya sendiri.
