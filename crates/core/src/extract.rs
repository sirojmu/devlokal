//! Ekstraksi zip dan tar.gz dengan perlindungan path traversal (zip-slip).

use crate::components::ArchiveKind;
use anyhow::{bail, Context, Result};
use std::fs::{self, File};
use std::io;
use std::path::{Component, Path, PathBuf};

pub fn kind_from_name(name: &str) -> Option<ArchiveKind> {
    let n = name.to_ascii_lowercase();
    if n.ends_with(".zip") {
        Some(ArchiveKind::Zip)
    } else if n.ends_with(".tar.gz") || n.ends_with(".tgz") {
        Some(ArchiveKind::TarGz)
    } else {
        None
    }
}

/// Ekstrak ke `dest` lewat folder sementara, lalu ganti nama (atomik).
pub fn extract(archive: &Path, kind: ArchiveKind, dest: &Path, strip: usize) -> Result<()> {
    let tmp = PathBuf::from(format!("{}.tmp", dest.display()));
    if tmp.exists() {
        fs::remove_dir_all(&tmp)?;
    }
    fs::create_dir_all(&tmp)?;
    let res = match kind {
        ArchiveKind::Zip => extract_zip(archive, &tmp, strip),
        ArchiveKind::TarGz => extract_tar_gz(archive, &tmp, strip),
    };
    if let Err(e) = res {
        fs::remove_dir_all(&tmp).ok();
        return Err(e);
    }
    if dest.exists() {
        fs::remove_dir_all(dest)?;
    }
    fs::rename(&tmp, dest)?;
    Ok(())
}

/// Buang `strip` komponen awal dan tolak path yang keluar dari folder tujuan.
fn clean_rel(p: &Path, strip: usize) -> Result<Option<PathBuf>> {
    let mut out = PathBuf::new();
    let mut seen = 0usize;
    for c in p.components() {
        match c {
            Component::Normal(s) => {
                if seen < strip {
                    seen += 1;
                } else {
                    out.push(s);
                }
            }
            Component::CurDir => {}
            _ => bail!("path berbahaya di dalam arsip: {}", p.display()),
        }
    }
    Ok(if out.as_os_str().is_empty() {
        None
    } else {
        Some(out)
    })
}

fn extract_zip(archive: &Path, dest: &Path, strip: usize) -> Result<()> {
    let mut z = zip::ZipArchive::new(File::open(archive)?).context("zip tidak bisa dibaca")?;
    for i in 0..z.len() {
        let mut f = z.by_index(i)?;
        let Some(name) = f.enclosed_name().map(|p| p.to_path_buf()) else {
            bail!("path berbahaya di dalam zip: {}", f.name());
        };
        let Some(rel) = clean_rel(&name, strip)? else {
            continue;
        };
        let out = dest.join(&rel);
        if f.is_dir() {
            fs::create_dir_all(&out)?;
            continue;
        }
        if let Some(p) = out.parent() {
            fs::create_dir_all(p)?;
        }
        let mut w = File::create(&out)?;
        io::copy(&mut f, &mut w)?;
        #[cfg(unix)]
        if let Some(mode) = f.unix_mode() {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&out, fs::Permissions::from_mode(mode & 0o777))?;
        }
    }
    Ok(())
}

fn extract_tar_gz(archive: &Path, dest: &Path, strip: usize) -> Result<()> {
    let gz = flate2::read::GzDecoder::new(File::open(archive)?);
    let mut tar = tar::Archive::new(gz);
    for entry in tar.entries()? {
        let mut entry = entry?;
        let ty = entry.header().entry_type();
        // Hanya berkas biasa dan folder; symlink/hardlink sengaja diabaikan demi keamanan.
        if !(ty.is_file() || ty.is_dir()) {
            continue;
        }
        let path = entry.path()?.to_path_buf();
        let Some(rel) = clean_rel(&path, strip)? else {
            continue;
        };
        let out = dest.join(rel);
        if ty.is_dir() {
            fs::create_dir_all(&out)?;
        } else {
            if let Some(p) = out.parent() {
                fs::create_dir_all(p)?;
            }
            entry.unpack(&out)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("devlokal-ex-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn make_zip(path: &Path, files: &[(&str, &str)]) {
        let mut w = zip::ZipWriter::new(File::create(path).unwrap());
        for (n, c) in files {
            w.start_file(*n, zip::write::FileOptions::default())
                .unwrap();
            w.write_all(c.as_bytes()).unwrap();
        }
        w.finish().unwrap();
    }

    #[test]
    fn extracts_with_strip() {
        let d = tmp("strip");
        let z = d.join("a.zip");
        make_zip(
            &z,
            &[("nginx-1.0/nginx.exe", "x"), ("nginx-1.0/conf/a.conf", "y")],
        );
        let out = d.join("out");
        extract(&z, ArchiveKind::Zip, &out, 1).unwrap();
        assert!(out.join("nginx.exe").exists());
        assert!(out.join("conf/a.conf").exists());
    }

    #[test]
    fn rejects_zip_slip() {
        let d = tmp("slip");
        let z = d.join("evil.zip");
        make_zip(&z, &[("../evil.txt", "x")]);
        let out = d.join("out");
        assert!(extract(&z, ArchiveKind::Zip, &out, 0).is_err());
        assert!(!d.join("evil.txt").exists());
    }

    #[test]
    fn detects_kind() {
        assert_eq!(kind_from_name("a.TAR.GZ"), Some(ArchiveKind::TarGz));
        assert_eq!(kind_from_name("a.zip"), Some(ArchiveKind::Zip));
        assert_eq!(kind_from_name("a.7z"), None);
    }
}
