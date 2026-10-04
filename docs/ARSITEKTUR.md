# Arsitektur

```
devlokal/
├─ crates/core      devlokal-core : semua logika, tanpa UI
├─ crates/cli       devlokal      : CLI tipis di atas core
├─ src-tauri        aplikasi desktop (Tauri), juga tipis di atas core  [belum diuji]
├─ ui               antarmuka web statis untuk Tauri
└─ manifest         daftar komponen (nginx, PHP, MariaDB) + URL + sha256
```

Prinsip: **semua logika di `core`**, UI hanya memanggilnya. Dengan begitu perilaku bisa diuji tanpa
GUI, CLI berguna di lab/server tanpa layar, dan Tauri tidak menjadi bottleneck.

## Modul `core`

| Modul | Tugas |
|---|---|
| `home` | Lokasi data (`%LOCALAPPDATA%\devlokal`, `~/.local/share/devlokal`, dst.) dan `devlokal.toml` |
| `components` | Manifest, komponen terpasang, pemilihan versi aktif, pencarian berkas eksekusi |
| `download` | Unduhan HTTPS saja, **resume** (Range), verifikasi SHA-256, cache |
| `extract` | zip/tar.gz, buang folder teratas, tolak path traversal, atomik via folder `.tmp` |
| `install` | Pasang dari arsip atau folder (jalur **offline**) |
| `projects` | Pindai folder `www/`, deteksi `public/`, template proyek |
| `nginx` | Pembuat `nginx.conf`, `php.ini`, `my.ini` |
| `hosts` | Blok bertanda di file hosts, idempoten, cadangan sekali |
| `process` | Jalankan terlepas, cek hidup, hentikan (lintas OS) |
| `env` | Orkestrasi `sync`, `start`, `stop`, `status` |
| `doctor` | Pemeriksaan port, izin hosts, komponen |

## Keputusan desain dan alasannya

- **nginx + php-cgi, bukan Apache.** Lebih ringan; cocok untuk target PC spesifikasi rendah.
  Konsekuensi: `.htaccess` tidak berlaku. Ini batasan nyata bagi pengguna yang terbiasa XAMPP.
- **Domain `.test`** (dicadangkan RFC 6761). `.dev` memaksa HTTPS (HSTS); `.local` bentrok dengan mDNS.
- **Entri hosts per proyek**, bukan wildcard. Wildcard `*.test` butuh resolver DNS lokal (mis. dnsmasq);
  ditunda ke versi berikutnya.
- **Port 80 hanya di Windows secara bawaan**; Linux/macOS memakai 8080 karena port < 1024 butuh hak istimewa.
- **Checksum wajib.** `fetch` menolak komponen tanpa `sha256` kecuali `--trust-first-use`.
- **Tidak mengemas ulang binary pihak ketiga.** Diunduh dari sumber resmi atau dipasang dari paket
  offline yang disiapkan pengguna. Periksa lisensi tiap komponen sebelum membuat paket offline sendiri.
- **Symlink dalam tar diabaikan** saat ekstraksi (keamanan). Ini bisa memengaruhi paket yang memerlukannya.

## Hak istimewa (rencana)

Hanya dua operasi yang butuh hak tinggi: menulis file hosts dan (di Linux/macOS) mengikat port < 1024.
Saat ini `devlokal sync` dijalankan sebagai Administrator/sudo oleh pengguna. Rencana untuk versi GUI:
satu helper kecil yang hanya bisa menulis blok devlokal di hosts, dipanggil lewat UAC/polkit/otorisasi macOS,
sehingga aplikasi utama tidak pernah berjalan sebagai admin.

## Status pengujian

| Area | Status |
|---|---|
| Unduhan (resume, checksum, tolak http), ekstraksi (zip-slip), hosts, pemindai proyek, generator konfigurasi | Tes otomatis lulus (Linux) |
| nginx + php-cgi nyata: vhost, `public/`, eksekusi PHP, hosts | Diuji manual di Linux |
| MariaDB: inisialisasi dan start | **Belum diuji**; argumen `install-db` berbeda antar versi/OS |
| Windows (spawn, `taskkill`, `tasklist`, hosts) | **Belum diuji**; hanya dijaga oleh CI |
| macOS | **Belum diuji** |
| Tauri | **Belum dikompilasi** |
