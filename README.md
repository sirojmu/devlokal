# devlokal

Lingkungan pengembangan web lokal (nginx + PHP + MariaDB) yang **gratis, ringan, dan hemat bandwidth**,
dibuat dengan memikirkan pengguna di Indonesia: mahasiswa, pengajar, dan developer dengan PC
spesifikasi pas-pasan atau koneksi yang tidak selalu stabil. Alternatif sederhana untuk Laragon/XAMPP.

> **Status: pra-rilis (v0.1, belum ada rilis biner).** Inti dan CLI sudah teruji di Linux; Windows,
> macOS, MariaDB, dan aplikasi desktop belum teruji. Lihat [Status pengujian](docs/ARSITEKTUR.md#status-pengujian)
> sebelum mengandalkannya.

## Pembeda yang dituju

| Tujuan | Wujud saat ini |
|---|---|
| Hemat bandwidth | Unduhan **dapat dilanjutkan** setelah putus, diverifikasi SHA-256, di-cache; pasang **offline** dari arsip/folder (`devlokal import`) |
| Ringan di PC spek rendah | nginx + php-cgi (bukan Apache); layanan bisa dijalankan satu per satu |
| Cocok untuk pendidikan | Pesan dan galat berbahasa Indonesia (mis. penjelasan port bentrok), template proyek, zona waktu `Asia/Jakarta` |
| Satu klik untuk kebutuhan lokal | `devlokal new <nama>` → `sync` → `up`; folder `public/` (Laravel) dikenali otomatis |
| Lintas platform | Inti ditulis lintas OS; GUI memakai Tauri (belum diuji) |

## Mulai cepat

```bash
cargo build --release          # Rust stabil terbaru
./target/release/devlokal doctor
```

Komponen belum dibundel. Pasang dari paket yang kamu unduh sendiri dari **sumber resmi**
(langkah rinci: [docs/SPIKE.md](docs/SPIKE.md)):

```bash
devlokal import nginx-1.26.2 nginx.zip --strip 1
devlokal import php-8.3.12   php.zip
devlokal new toko            # buat proyek di folder www
devlokal sync                # tulis konfigurasi + hosts (Administrator/sudo)
devlokal up                  # jalankan layanan
# buka http://toko.test  (di Linux/macOS: http://toko.test:8080)
```

`devlokal fetch <id>` mengunduh dari manifest, tetapi **manifest sengaja belum berisi checksum**
(lihat [manifest/components.toml](manifest/components.toml)); perintah itu menolak berjalan sampai
URL dan `sha256` diisi dari sumber resmi.

## Perintah

`paths` · `doctor` · `list` · `fetch` · `import` · `use` · `new` · `sync` · `up` · `down` · `status`
(`devlokal --help`). Bahasa Inggris: `DEVLOKAL_LANG=en`.

## Keterbatasan yang perlu diketahui

- Memakai **nginx**, jadi `.htaccess` tidak berlaku.
- Domain per proyek lewat file hosts (butuh Administrator/sudo); wildcard `*.test` belum ada.
- Belum ada HTTPS lokal, ikon tray, pembaruan otomatis, maupun biner rilis bertanda tangan
  (antivirus Windows bisa memberi peringatan pada aplikasi yang mengubah file hosts).
- Argumen inisialisasi MariaDB bisa berbeda antar versi/OS dan belum diverifikasi.

## Peta jalan

1. Spike Windows + Linux dengan biner resmi; isi manifest yang terverifikasi.
2. Rilis CLI v0.1 (Windows/Linux) dengan CI yang membangun biner.
3. Helper berhak istimewa untuk hosts; aplikasi Tauri + tray.
4. HTTPS lokal, resolver wildcard `*.test`, PHP/DB multi-versi, macOS.
5. Materi dan template praktikum bersama komunitas pengajar.

## Berkontribusi

Lihat [CONTRIBUTING.md](CONTRIBUTING.md). Arsitektur: [docs/ARSITEKTUR.md](docs/ARSITEKTUR.md).

## Lisensi

[MIT](LICENSE) untuk kode devlokal. nginx, PHP, dan MariaDB memiliki lisensinya sendiri dan tidak
disertakan dalam repositori ini.
