# Rencana: Sapu kualitas premium (redesign + fitur pembanding)

Tanggal: 16 Oktober 2026 · Status: usulan untuk pemilik · Pemicu: penilaian pemilik setelah uji nyata, "tampilannya terlalu biasa, seperti aplikasi open source; UI/UX tidak jalan".

## 1. Apa yang dipelajari dari pesaing

Sumber: artikel Dr.Buho "best Mac cleaner software", halaman produk BuhoCleaner, CleanMyMac, DaisyDisk, dan ulasan CleanMyMac 5 (MacMagazine, TheSweetBits, Macworld).

| Produk | Fitur yang ditonjolkan | Ciri visual yang membuatnya terasa premium |
|---|---|---|
| **CleanMyMac 5** | Smart Care (satu klik, lima tugas), Cleanup, Protection, Performance, Applications (uninstall tanpa sisa), My Clutter (besar, lama, duplikat), Mac Health Score, Assistant dengan pengingat, menu bar monitor, Space Lens | Satu angka hero besar ("5,5 GB"), hasil scan sebagai **ubin ringkasan**, gradien lembut dan ilustrasi berlapis, tombol bulat besar, animasi scan sebagai "momen", skor kesehatan sebagai metafora utama |
| **BuhoCleaner** | Flash Clean satu ketukan, Large Files, Duplicates, Disk Space Analyzer (treemap), App Uninstall + Deep Uninstall 100+ app, Leftover Cleaner, Startup Items, Mac Monitor di menu bar (RAM/CPU/suhu/jaringan), Xcode cache, Purgeable space, screenshot lama, lampiran Mail, Downloads organizer, Shredder | Kartu modul dengan ikon berwarna, dashboard sederhana, pernyataan "Apple notarized", bahasa Apple-aligned |
| **DaisyDisk** | Sunburst interaktif (drill-in, hover, Space untuk Quick Look, tarik ke "collector" lalu hapus), scan 15 detik, admin scan, cloud, snapshot, proteksi file sistem | **Satu visual ikonik** (cincin warna-warni), tema gelap, warna cerah, interaksi yang "menyenangkan, bukan tugas rumah tangga", desain minimal yang dipikirkan |
| **CCleaner / Disk Clean Pro / Disk Doctor** | Pembersihan standar, duplikat, foto, browser | "Profesional dan ramah", tidak ada ciri visual khas |
| **OnyX / AppCleaner** | Kuat tetapi teknis / satu fungsi | Dinilai cocok untuk orang teknis; bukti bahwa fungsi tanpa UX = "aplikasi open source" di mata orang awam |

Kriteria "bagus" menurut artikel: praktis untuk kebutuhan harian, mudah dipakai, cepat, lengkap tanpa rumit, satu klik.

## 2. Diagnosis: kenapa Sapu terasa biasa

1. **Tidak ada momen.** Scan hanya progress bar; selesai scan hanya angka dan daftar. Pesaing menjadikan scan dan hasil sebagai momen visual (cincin bergerak, angka naik, ubin muncul).
2. **Semua berupa daftar dan tabel.** Beranda, Bersih, Duplikat, Riwayat memakai pola yang sama: baris teks + angka kanan. Tidak ada hierarki visual, tidak ada ikon bermakna, tidak ada ilustrasi.
3. **Tidak ada identitas.** Palet pine/mist aman tetapi tidak dikenali. Tidak ada satu visual ikonik seperti sunburst DaisyDisk atau skor CleanMyMac. Disk strip yang direncanakan jadi "elemen ikonik" terlalu tipis dan tenggelam.
4. **Tanpa kedalaman dan gerak.** Permukaan datar, bayangan nol, transisi hampir tidak ada, tidak ada feedback setelah aksi (mis. "12,4 GB dibebaskan" yang dirayakan).
5. **Jendela terasa web.** Tidak memakai sidebar translusen macOS, title bar standar, chrome tidak menyatu dengan OS.
6. **Terlalu banyak teks penjelasan** di tempat yang seharusnya visual, dan terlalu sedikit teks di tempat yang butuh panduan (hasil kosong, langkah berikutnya).
7. **Fitur yang dicari orang awam belum ada**: satu tombol "rawat Mac saya", uninstaller, monitor di menu bar, pembersih khusus (lampiran Mail, screenshot, Downloads, purgeable).

## 3. Arah desain baru (prinsip PRD tetap: jujur, aman, dua audiens)

- **Satu visual ikonik: Cincin Ruang.** Beranda dibuka dengan cincin besar (kapasitas disk) yang terisi segmen kategori; cincin yang sama "berputar" saat scan dan "menipis" saat pembersihan, jadi satu metafora dipakai di seluruh app. Treemap tetap ada sebagai tampilan kedua; tambahkan sunburst sebagai mode eksplorasi karena itu yang dipuji pengguna awam.
- **Angka hero.** Setiap layar punya satu angka besar (ruang terpakai, ruang yang bisa dibebaskan, ruang yang baru dibebaskan) dengan animasi count-up, dan satu tombol utama bulat besar di bawahnya.
- **Hasil sebagai ubin.** Ringkasan scan dan hasil Bersih tampil sebagai ubin dengan ikon kategori custom (bukan lucide generik), warna aksen per kategori, angka, dan satu kalimat. Daftar detail ada di tingkat kedua.
- **Kedalaman dan material.** Latar bergradasi halus, permukaan berlapis dengan bayangan lembut, sidebar translusen macOS (vibrancy) dengan traffic light menyatu, mode gelap sebagai warga kelas satu (gelap default mengikuti sistem).
- **Tipografi baru.** Wajah tampilan untuk angka dan judul (kandidat: Manrope atau Instrument Sans) + teks UI Inter/Plex; tabular numerals; skala 13/15/20/28/44.
- **Palet baru.** Latar gelap biru-hitam dan terang krem dingin; aksen tunggal "sapu hijau-toska" lebih jenuh; kategori memakai spektrum sunburst yang konsisten (8 hue + variasi kecerahan untuk kedalaman folder).
- **Gerak yang berarti.** Satu animasi scan (cincin berputar + partikel "debu" tersapu), transisi ubin saat hasil tiba, animasi "sapuan" saat pembersihan selesai; semua menghormati reduced-motion.
- **Chrome asli.** Hidden title bar + sidebar vibrancy di macOS (`tauri` `titleBarStyle: Overlay` + crate `window-vibrancy`), Mica di Windows 11.
- **Copy.** Judul pendek bernada ajakan ("Mac kamu punya 61 GB ruang kosong", "Sapu bisa membebaskan 22,6 GB"), bukan label teknis.

## 4. Fitur baru yang diambil dari pesaing (dengan batas PRD)

| Fitur | Dasar | Catatan PRD |
|---|---|---|
| **Rawat (Smart Care)**: satu tombol menjalankan scan + evaluasi Bersih + ringkasan duplikat, lalu satu layar hasil dengan **Skor Ruang** (0–100) | CleanMyMac Smart Care + Health Score, Buho Flash Clean | Skor hanya soal ruang dan kebersihan, bukan "performa"/malware (non-goal) |
| **Uninstaller aplikasi + sisa file** | Semua pesaing | PRD P2, dinaikkan ke P1; wajib lewat safety |
| **Monitor menu bar**: disk, RAM, CPU, cepat buka app | Buho Mac Monitor, CleanMyMac menu bar | Informasi saja, tanpa "optimasi RAM" |
| **Pembersih khusus**: lampiran Mail, screenshot lama, Downloads organizer, purgeable/snapshot Time Machine lokal, Trash semua volume | Buho | Semua lewat rules + safety; purgeable via `tmutil` resmi |
| **Quick Look (Space)** di daftar file dan grup duplikat | DaisyDisk | macOS `qlmanage -p`, pratinjau asli |
| **Sunburst interaktif** dengan "keranjang" (collector) lalu satu tombol hapus | DaisyDisk | Keranjang = DeletePlan |
| **Startup/login items** | Buho, CleanMyMac | Opsional, belakangan; perlu kehati-hatian |
| **Shredder** | Buho | Tidak diambil (bertentangan dengan P2) |

## 5. Roadmap

| Sprint | Isi | Kriteria selesai |
|---|---|---|
| **R1 Fondasi visual** (1–2 minggu) | Design system v2: token, tipografi, ikon kategori custom, komponen ubin/angka hero/cincin, vibrancy + hidden title bar, mode gelap; Beranda baru dengan Cincin Ruang | Screenshot Beranda dibandingkan berdampingan dengan CleanMyMac 5 dan dinilai pemilik "setara" |
| **R2 Scan sebagai momen** | Animasi scan, hasil ubin, sunburst + keranjang, Quick Look, angka count-up | Pengguna awam dalam uji 5 orang menemukan folder terbesar < 1 menit |
| **R3 Rawat + Skor Ruang** | Tombol satu klik, layar hasil gabungan, Skor Ruang, pengingat lembut | Dari buka app ke pembersihan pertama < 3 menit (target PRD §14) |
| **R4 Uninstaller + sisa** | Daftar app, ukuran, sisa file (Application Support, Caches, Preferences, Containers), hapus via safety | Nol sisa terlewat untuk 20 app populer |
| **R5 Modul khusus + menu bar** | Mail, screenshot, Downloads, purgeable, monitor menu bar | Semua lewat rules; tray app ringan |
| **R6 Poles + uji** | Onboarding ulang dengan ilustrasi, empty/error state, aksesibilitas, uji kegunaan 5 orang non-teknis, perbaiki | ≥ 90 % berhasil tanpa bantuan (PRD §14) |

## 6. Cara kerja agar hasilnya benar-benar premium

- Desain dulu, kode kemudian: setiap layar dimulai dari mockup HTML statis yang direview lewat screenshot (Playwright) berdampingan dengan tangkapan layar pesaing, baru diimplementasikan.
- Aturan "lepas satu aksesori": setiap layar dievaluasi, apa yang bisa dihapus.
- Satu elemen berani per layar, sisanya tenang. Tidak ada kartu seragam berjajar tanpa hierarki.
- Uji di jendela Tauri nyata di setiap akhir sprint, bukan hanya browser mock.
