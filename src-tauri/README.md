# Aplikasi desktop (Tauri) — BELUM DIUJI

Folder ini berisi kerangka aplikasi desktop Tauri 2 yang memanggil `devlokal-core`.
**Kerangka ini belum pernah dikompilasi** (lingkungan pembuatnya tidak punya dependensi
WebView/Tauri). Anggap sebagai titik awal, bukan hasil jadi. Inti (`crates/core`) dan CLI
sudah diuji; lihat README utama.

## Menjalankan

1. Pasang prasyarat Tauri untuk OS kamu: https://v2.tauri.app/start/prerequisites/
2. Buat ikon (wajib untuk build): `icons/icon-source.png` hanyalah placeholder 512×512; ganti dengan
   logo kamu (ideal 1024×1024), lalu `cargo install tauri-cli --version "^2"` dan
   `cargo tauri icon icons/icon-source.png` dari folder ini.
3. `cargo tauri dev` untuk mode pengembangan, `cargo tauri build` untuk paket rilis.

## Yang belum ada

- Ikon tray dan perilaku "minimize ke tray".
- Memperbarui file hosts dari UI (butuh helper berhak istimewa; lihat docs/ARSITEKTUR.md).
- Reaping proses anak di Linux/macOS saat aplikasi berjalan lama (proses layanan dijalankan
  terlepas, tetapi `Child` tidak di-`wait`, sehingga bisa menjadi zombie sampai aplikasi keluar).
