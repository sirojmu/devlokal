//! Pemindaian folder proyek dan pembuatan proyek baru dari template.

use anyhow::{bail, Result};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub name: String,
    pub domain: String,
    /// Folder yang dilayani web server (`public/` bila ada, mis. Laravel).
    pub docroot: PathBuf,
}

/// Ubah nama folder menjadi label domain yang valid; `None` bila tidak bisa.
pub fn slug(name: &str) -> Option<String> {
    let s: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let s = s.trim_matches('-').to_string();
    if s.is_empty() || s.len() > 60 {
        None
    } else {
        Some(s)
    }
}

pub fn scan(www: &Path, tld: &str) -> Vec<Project> {
    let mut out = vec![];
    let Ok(rd) = fs::read_dir(www) else {
        return out;
    };
    for e in rd.flatten() {
        let p = e.path();
        let fname = e.file_name().to_string_lossy().to_string();
        if !p.is_dir() || fname.starts_with('.') {
            continue;
        }
        let Some(name) = slug(&fname) else { continue };
        let public = p.join("public");
        let docroot = if public.is_dir() { public } else { p };
        out.push(Project {
            domain: format!("{name}.{tld}"),
            name,
            docroot,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out.dedup_by(|a, b| a.name == b.name);
    out
}

pub fn templates() -> &'static [&'static str] {
    &["php", "html"]
}

pub fn create(www: &Path, name: &str, template: &str) -> Result<PathBuf> {
    let Some(s) = slug(name) else {
        bail!("nama proyek tidak valid: {name}");
    };
    let dir = www.join(&s);
    if dir.exists() {
        bail!("folder sudah ada: {}", dir.display());
    }
    fs::create_dir_all(&dir)?;
    match template {
        "php" => fs::write(dir.join("index.php"), PHP_TEMPLATE.replace("@@NAME@@", &s))?,
        "html" => fs::write(
            dir.join("index.html"),
            HTML_TEMPLATE.replace("@@NAME@@", &s),
        )?,
        other => bail!("template tidak dikenal: {other} (tersedia: php, html)"),
    }
    Ok(dir)
}

const PHP_TEMPLATE: &str = r#"<?php
// Proyek @@NAME@@ — dibuat oleh devlokal.
?><!doctype html>
<html lang="id">
<head><meta charset="utf-8"><title>@@NAME@@</title></head>
<body>
  <h1>Halo dari @@NAME@@!</h1>
  <p>PHP <?= htmlspecialchars(PHP_VERSION) ?> berjalan.</p>
  <?php
  // Contoh koneksi MariaDB (pengguna root tanpa password hanya untuk pengembangan lokal).
  // $pdo = new PDO('mysql:host=127.0.0.1;port=3306;charset=utf8mb4', 'root', '');
  ?>
</body>
</html>
"#;

const HTML_TEMPLATE: &str = r#"<!doctype html>
<html lang="id">
<head><meta charset="utf-8"><title>@@NAME@@</title></head>
<body><h1>Halo dari @@NAME@@!</h1></body>
</html>
"#;

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("devlokal-pr-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn slug_rules() {
        assert_eq!(slug("Toko Online_2"), Some("toko-online-2".into()));
        assert_eq!(slug("---"), None);
    }

    #[test]
    fn scans_and_uses_public() {
        let w = tmp("scan");
        fs::create_dir_all(w.join("blog/public")).unwrap();
        fs::create_dir_all(w.join("Toko Saya")).unwrap();
        fs::create_dir_all(w.join(".hidden")).unwrap();
        fs::write(w.join("file.txt"), "x").unwrap();
        let ps = scan(&w, "test");
        assert_eq!(ps.len(), 2);
        assert_eq!(ps[0].domain, "blog.test");
        assert!(ps[0].docroot.ends_with("public"));
        assert_eq!(ps[1].domain, "toko-saya.test");
    }

    #[test]
    fn creates_from_template() {
        let w = tmp("new");
        let d = create(&w, "Latihan 1", "php").unwrap();
        assert!(d.join("index.php").exists());
        assert!(create(&w, "Latihan 1", "php").is_err());
    }
}
