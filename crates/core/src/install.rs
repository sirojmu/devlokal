//! Pemasangan komponen dari arsip (hasil unduhan atau paket offline) atau dari folder.

use crate::components::{component_dir, kind_of, ArchiveKind};
use crate::extract;
use crate::home::Home;
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::Path;

fn check_id(id: &str) -> Result<()> {
    if kind_of(id).is_none() {
        bail!("id harus diawali nginx-, php-, atau mariadb- (contoh: php-8.3.12)");
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.' || c == '_')
    {
        bail!("id hanya boleh berisi huruf, angka, '-', '.', '_'");
    }
    Ok(())
}

pub fn install_archive(
    home: &Home,
    id: &str,
    archive: &Path,
    kind: Option<ArchiveKind>,
    strip: usize,
) -> Result<()> {
    check_id(id)?;
    let name = archive.file_name().and_then(|s| s.to_str()).unwrap_or("");
    let kind = kind
        .or_else(|| extract::kind_from_name(name))
        .context("format arsip tidak dikenali (didukung: .zip, .tar.gz)")?;
    extract::extract(archive, kind, &component_dir(home, id), strip)
}

pub fn install_dir(home: &Home, id: &str, src: &Path) -> Result<()> {
    check_id(id)?;
    let dest = component_dir(home, id);
    if dest.exists() {
        fs::remove_dir_all(&dest)?;
    }
    copy_dir(src, &dest).with_context(|| format!("gagal menyalin {}", src.display()))
}

fn copy_dir(src: &Path, dest: &Path) -> Result<()> {
    fs::create_dir_all(dest)?;
    for e in fs::read_dir(src)? {
        let e = e?;
        let to = dest.join(e.file_name());
        if e.file_type()?.is_dir() {
            copy_dir(&e.path(), &to)?;
        } else {
            fs::copy(e.path(), &to)?;
        }
    }
    Ok(())
}

pub fn uninstall(home: &Home, id: &str) -> Result<()> {
    check_id(id)?;
    let d = component_dir(home, id);
    if d.exists() {
        fs::remove_dir_all(d)?;
    }
    Ok(())
}
