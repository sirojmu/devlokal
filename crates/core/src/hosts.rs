//! Pengelolaan blok devlokal di file hosts. Hanya blok bertanda yang disentuh.

use crate::home::Home;
use anyhow::{bail, Context, Result};
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;

const START: &str = "# >>> devlokal >>>";
const END: &str = "# <<< devlokal <<<";

pub fn hosts_path() -> PathBuf {
    if let Some(p) = std::env::var_os("DEVLOKAL_HOSTS_FILE") {
        return PathBuf::from(p);
    }
    if cfg!(windows) {
        let root = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
        PathBuf::from(root).join("System32/drivers/etc/hosts")
    } else {
        PathBuf::from("/etc/hosts")
    }
}

/// Hasilkan isi hosts baru: buang blok lama, tambahkan blok untuk `domains`.
pub fn render_hosts(existing: &str, domains: &[String]) -> String {
    let nl = if existing.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut kept: Vec<&str> = vec![];
    let mut inside = false;
    for line in existing.lines() {
        let t = line.trim();
        if t == START {
            inside = true;
            continue;
        }
        if t == END {
            inside = false;
            continue;
        }
        if !inside {
            kept.push(line);
        }
    }
    while kept.last().map(|l| l.trim().is_empty()).unwrap_or(false) {
        kept.pop();
    }
    let mut out = kept.join(nl);
    if !out.is_empty() {
        out.push_str(nl);
    }
    if !domains.is_empty() {
        out.push_str(nl);
        out.push_str(START);
        out.push_str(nl);
        let mut ds: Vec<&String> = domains.iter().collect();
        ds.sort();
        ds.dedup();
        for d in ds {
            out.push_str(&format!("127.0.0.1 {d}{nl}"));
        }
        out.push_str(END);
        out.push_str(nl);
    }
    out
}

/// Apakah blok di file hosts sudah sesuai dengan `domains`?
pub fn is_up_to_date(domains: &[String]) -> Result<bool> {
    let cur = fs::read_to_string(hosts_path()).unwrap_or_default();
    Ok(render_hosts(&cur, domains) == cur)
}

/// Terapkan ke file hosts. Mengembalikan `true` bila ada perubahan.
/// Butuh hak administrator/root; cadangan disimpan sekali sebelum perubahan pertama.
pub fn apply(home: &Home, domains: &[String]) -> Result<bool> {
    let path = hosts_path();
    let cur = fs::read_to_string(&path).unwrap_or_default();
    let new = render_hosts(&cur, domains);
    if new == cur {
        return Ok(false);
    }
    let backup = home.backup().join("hosts.original");
    if !backup.exists() && path.exists() {
        fs::create_dir_all(home.backup())?;
        fs::copy(&path, &backup).context("gagal membuat cadangan hosts")?;
    }
    match fs::write(&path, new) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == ErrorKind::PermissionDenied => bail!(
            "tidak punya izin menulis {}. Jalankan sebagai Administrator (Windows) atau dengan sudo (Linux/macOS).",
            path.display()
        ),
        Err(e) => Err(e).with_context(|| format!("gagal menulis {}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn adds_block_and_keeps_existing() {
        let out = render_hosts("127.0.0.1 localhost\n", &d(&["b.test", "a.test"]));
        assert!(out.starts_with("127.0.0.1 localhost\n"));
        assert!(out.contains("127.0.0.1 a.test\n127.0.0.1 b.test\n"));
    }

    #[test]
    fn is_idempotent_and_replaces_old_block() {
        let once = render_hosts("127.0.0.1 localhost\n", &d(&["a.test"]));
        assert_eq!(render_hosts(&once, &d(&["a.test"])), once);
        let changed = render_hosts(&once, &d(&["c.test"]));
        assert!(!changed.contains("a.test"));
        assert!(changed.contains("c.test"));
    }

    #[test]
    fn removes_block_when_no_domains() {
        let once = render_hosts("127.0.0.1 localhost\n", &d(&["a.test"]));
        assert_eq!(render_hosts(&once, &[]), "127.0.0.1 localhost\n");
    }

    #[test]
    fn preserves_crlf() {
        let out = render_hosts("127.0.0.1 localhost\r\n", &d(&["a.test"]));
        assert!(out.contains("a.test\r\n"));
        assert!(!out.replace("\r\n", "").contains('\n'));
    }
}
