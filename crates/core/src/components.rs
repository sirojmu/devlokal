//! Manifest komponen (nginx, PHP, MariaDB) dan komponen yang sudah terpasang.

use crate::home::{Home, Settings};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Nginx,
    Php,
    Mariadb,
}

impl Kind {
    pub fn prefix(self) -> &'static str {
        match self {
            Kind::Nginx => "nginx-",
            Kind::Php => "php-",
            Kind::Mariadb => "mariadb-",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum ArchiveKind {
    #[serde(rename = "zip")]
    Zip,
    #[serde(rename = "tar.gz")]
    TarGz,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Source {
    /// Kosong berarti belum diisi; gunakan `devlokal import`.
    #[serde(default)]
    pub url: String,
    /// Checksum SHA-256 (hex). Kosong berarti belum diverifikasi.
    #[serde(default)]
    pub sha256: String,
    pub archive: ArchiveKind,
    /// Jumlah folder teratas yang dibuang saat ekstraksi.
    #[serde(default)]
    pub strip: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Component {
    pub id: String,
    pub kind: Kind,
    pub version: String,
    #[serde(default)]
    pub note: String,
    /// Kunci: `windows-x86_64`, `linux-x86_64`, `macos-aarch64`, dst.
    #[serde(default)]
    pub sources: BTreeMap<String, Source>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    #[serde(default)]
    pub components: Vec<Component>,
}

const BUNDLED: &str = include_str!("../../../manifest/components.toml");

impl Manifest {
    pub fn bundled() -> Result<Self> {
        Self::parse(BUNDLED)
    }

    pub fn parse(text: &str) -> Result<Self> {
        toml::from_str(text).context("manifest komponen tidak valid")
    }

    pub fn find(&self, id: &str) -> Option<&Component> {
        self.components.iter().find(|c| c.id == id)
    }
}

pub fn platform_key() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

pub fn kind_of(id: &str) -> Option<Kind> {
    [Kind::Nginx, Kind::Php, Kind::Mariadb]
        .into_iter()
        .find(|k| id.starts_with(k.prefix()))
}

pub fn component_dir(home: &Home, id: &str) -> PathBuf {
    home.components().join(id)
}

/// Daftar id komponen yang sudah terpasang, terurut.
pub fn installed(home: &Home) -> Result<Vec<String>> {
    let dir = home.components();
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut out = vec![];
    for e in std::fs::read_dir(dir)? {
        let e = e?;
        if e.file_type()?.is_dir() {
            let name = e.file_name().to_string_lossy().to_string();
            if !name.ends_with(".tmp") {
                out.push(name);
            }
        }
    }
    out.sort_by_key(|id| version_key(id));
    Ok(out)
}

/// Kunci urut sederhana: angka dalam id dibandingkan secara numerik (8.10 > 8.9).
pub fn version_key(id: &str) -> Vec<u64> {
    id.split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect()
}

/// Komponen yang dipakai untuk satu jenis layanan: pilihan pengguna, atau versi terbaru.
pub fn resolve(home: &Home, settings: &Settings, kind: Kind) -> Option<String> {
    let name = match kind {
        Kind::Nginx => "nginx",
        Kind::Php => "php",
        Kind::Mariadb => "mariadb",
    };
    if let Some(id) = settings.active.get(name) {
        return Some(id.clone());
    }
    installed(home)
        .ok()?
        .into_iter()
        .filter(|id| id.starts_with(kind.prefix()))
        .last()
}

/// Cari berkas eksekusi (hingga kedalaman 4), yang paling dangkal didahulukan.
pub fn find_exe(root: &Path, names: &[&str]) -> Option<PathBuf> {
    let mut level = vec![root.to_path_buf()];
    for _ in 0..5 {
        let mut next = vec![];
        let mut dirs = vec![];
        for d in &level {
            let Ok(rd) = std::fs::read_dir(d) else {
                continue;
            };
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    dirs.push(p);
                } else if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                    let ext_ok = match p.extension().and_then(|s| s.to_str()) {
                        None => !cfg!(windows),
                        Some(x) => x.eq_ignore_ascii_case("exe"),
                    };
                    if ext_ok
                        && names.iter().any(|n| stem.eq_ignore_ascii_case(n))
                        && is_executable(&p)
                    {
                        return Some(p);
                    }
                }
            }
        }
        next.extend(dirs);
        if next.is_empty() {
            break;
        }
        level = next;
    }
    None
}

#[cfg(unix)]
fn is_executable(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(p)
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(_p: &Path) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_manifest_parses() {
        let m = Manifest::bundled().unwrap();
        assert!(!m.components.is_empty());
    }

    #[test]
    fn version_order_is_numeric() {
        assert!(version_key("php-8.10.0") > version_key("php-8.9.1"));
    }

    #[test]
    fn kind_from_prefix() {
        assert_eq!(kind_of("php-8.3.12"), Some(Kind::Php));
        assert_eq!(kind_of("mariadb-11.4.2"), Some(Kind::Mariadb));
        assert_eq!(kind_of("foo-1"), None);
    }
}
