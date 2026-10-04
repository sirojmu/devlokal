//! Pesan dua bahasa. Bahasa Indonesia adalah bawaan; set `DEVLOKAL_LANG=en` untuk Inggris.

use std::sync::OnceLock;

fn english() -> bool {
    static EN: OnceLock<bool> = OnceLock::new();
    *EN.get_or_init(|| {
        std::env::var("DEVLOKAL_LANG")
            .map(|v| v.to_ascii_lowercase().starts_with("en"))
            .unwrap_or(false)
    })
}

/// Pilih teks sesuai bahasa aktif.
pub fn tr(id: &'static str, en: &'static str) -> &'static str {
    if english() {
        en
    } else {
        id
    }
}
