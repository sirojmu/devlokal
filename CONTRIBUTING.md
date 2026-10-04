# Berkontribusi

Terima kasih sudah tertarik! Proyek ini masih awal, jadi kontribusi kecil pun sangat berarti.

## Cara mulai

```bash
git clone https://github.com/sirojmu/devlokal
cd devlokal
cargo test --workspace
cargo run -p devlokal -- doctor
```

Butuh Rust stabil terbaru (versi minimum belum ditetapkan; dikembangkan dengan 1.91).

## Aturan singkat

- Pesan untuk pengguna ditulis dalam bahasa Indonesia, dengan terjemahan Inggris lewat `tr("id", "en")`.
- Setiap logika baru di `crates/core` sebaiknya punya tes. Hindari tes yang butuh internet.
- **Jangan mengisi URL/checksum di `manifest/components.toml` dari ingatan atau sumber pihak ketiga.**
  Ambil dari situs resmi, unduh sendiri, hitung `sha256`, dan sertakan tautan sumber di deskripsi PR.
- Perubahan yang menyentuh file hosts, hak istimewa, atau pengunduhan binary perlu ekstra hati-hati:
  jelaskan model ancamannya di PR.

## Melapor masalah keamanan

Untuk celah keamanan (mis. path traversal saat ekstraksi, penulisan hosts yang tidak aman),
jangan buka issue publik; gunakan fitur *Private vulnerability reporting* di tab Security repositori.
