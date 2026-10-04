//! Unduhan hemat bandwidth: dapat dilanjutkan (resume), diverifikasi SHA-256, dan di-cache.

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let mut f = File::open(path)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(hex(&h.finalize()))
}

/// Unduh `url` ke `dest`. Hanya HTTPS. Mengembalikan SHA-256 berkas.
pub fn fetch(
    url: &str,
    dest: &Path,
    expected: Option<&str>,
    progress: impl FnMut(u64, Option<u64>),
) -> Result<String> {
    fetch_opts(url, dest, expected, false, progress)
}

/// Seperti [`fetch`], dengan opsi `allow_http` (dipakai pengujian lokal saja).
pub fn fetch_opts(
    url: &str,
    dest: &Path,
    expected: Option<&str>,
    allow_http: bool,
    mut progress: impl FnMut(u64, Option<u64>),
) -> Result<String> {
    if !(url.starts_with("https://") || (allow_http && url.starts_with("http://"))) {
        bail!("hanya URL https:// yang diizinkan: {url}");
    }
    // Cache: berkas yang sudah ada dan cocok checksum-nya tidak diunduh ulang.
    if let (true, Some(exp)) = (dest.exists(), expected) {
        let h = sha256_file(dest)?;
        if h.eq_ignore_ascii_case(exp.trim()) {
            return Ok(h);
        }
    }
    if let Some(p) = dest.parent() {
        fs::create_dir_all(p)?;
    }
    let part = part_path(dest);
    let existing = fs::metadata(&part).map(|m| m.len()).unwrap_or(0);

    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout_read(Duration::from_secs(30))
        .build();
    let mut req = agent.get(url);
    if existing > 0 {
        req = req.set("Range", &format!("bytes={existing}-"));
    }
    let resp = match req.call() {
        Ok(r) => r,
        // 416: bagian yang tersimpan sudah lengkap atau tidak cocok dengan berkas di server.
        Err(ureq::Error::Status(416, _)) => {
            if expected.is_some() {
                return finish(&part, dest, expected);
            }
            fs::remove_file(&part).ok();
            bail!("berkas parsial tidak cocok dengan server; ulangi perintah untuk mengunduh dari awal");
        }
        Err(e) => return Err(e).with_context(|| format!("gagal mengunduh {url}")),
    };

    let (mut file, mut done) = if resp.status() == 206 {
        (OpenOptions::new().append(true).open(&part)?, existing)
    } else {
        // Server mengabaikan Range: mulai dari awal.
        (File::create(&part)?, 0)
    };
    let total = resp
        .header("Content-Length")
        .and_then(|v| v.parse::<u64>().ok())
        .map(|n| n + done);

    let mut reader = resp.into_reader();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = reader
            .read(&mut buf)
            .context("koneksi terputus; jalankan ulang untuk melanjutkan unduhan")?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])?;
        done += n as u64;
        progress(done, total);
    }
    file.flush()?;
    drop(file);

    if let Some(t) = total {
        if done != t {
            bail!("unduhan belum lengkap ({done}/{t} byte); jalankan ulang untuk melanjutkan");
        }
    }
    finish(&part, dest, expected)
}

fn part_path(dest: &Path) -> PathBuf {
    PathBuf::from(format!("{}.part", dest.display()))
}

fn finish(part: &Path, dest: &Path, expected: Option<&str>) -> Result<String> {
    let h = sha256_file(part)?;
    if let Some(exp) = expected {
        if !h.eq_ignore_ascii_case(exp.trim()) {
            fs::remove_file(part).ok();
            bail!("checksum tidak cocok (didapat {h}, diharapkan {exp}); berkas dibuang");
        }
    }
    fs::rename(part, dest)?;
    Ok(h)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    fn tmp(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("devlokal-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    /// Server HTTP mini yang mendukung header Range. Mengembalikan (url, Range yang terlihat).
    fn serve(data: Vec<u8>) -> (String, Arc<Mutex<Option<String>>>) {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/file.bin", l.local_addr().unwrap());
        let seen = Arc::new(Mutex::new(None));
        let seen2 = seen.clone();
        std::thread::spawn(move || {
            for stream in l.incoming() {
                let mut s = stream.unwrap();
                let mut r = BufReader::new(s.try_clone().unwrap());
                let mut range = None;
                loop {
                    let mut line = String::new();
                    r.read_line(&mut line).unwrap();
                    if line == "\r\n" || line.is_empty() {
                        break;
                    }
                    if let Some(v) = line.to_ascii_lowercase().strip_prefix("range: bytes=") {
                        range = Some(v.trim().trim_end_matches('-').to_string());
                    }
                }
                let start: usize = range.as_ref().and_then(|v| v.parse().ok()).unwrap_or(0);
                *seen2.lock().unwrap() = range.clone();
                let body = &data[start..];
                let head = if range.is_some() {
                    format!(
                        "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {}-{}/{}\r\nConnection: close\r\n\r\n",
                        body.len(), start, data.len() - 1, data.len()
                    )
                } else {
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    )
                };
                s.write_all(head.as_bytes()).unwrap();
                s.write_all(body).unwrap();
            }
        });
        (url, seen)
    }

    fn sample() -> Vec<u8> {
        (0..300_000u32).map(|i| (i % 251) as u8).collect()
    }

    #[test]
    fn downloads_and_verifies() {
        let data = sample();
        let want = hex(&Sha256::digest(&data));
        let (url, _) = serve(data);
        let dir = tmp("full");
        let dest = dir.join("a.bin");
        let got = fetch_opts(&url, &dest, Some(&want), true, |_, _| {}).unwrap();
        assert_eq!(got, want);
        assert!(dest.exists());
    }

    #[test]
    fn resumes_partial_download() {
        let data = sample();
        let want = hex(&Sha256::digest(&data));
        let (url, seen) = serve(data.clone());
        let dir = tmp("resume");
        let dest = dir.join("a.bin");
        // Simulasikan unduhan yang terputus di 100.000 byte.
        fs::write(part_path(&dest), &data[..100_000]).unwrap();
        let got = fetch_opts(&url, &dest, Some(&want), true, |_, _| {}).unwrap();
        assert_eq!(got, want);
        assert_eq!(seen.lock().unwrap().as_deref(), Some("100000"));
    }

    #[test]
    fn rejects_bad_checksum_and_http() {
        let data = sample();
        let (url, _) = serve(data);
        let dir = tmp("bad");
        let dest = dir.join("a.bin");
        assert!(fetch_opts(&url, &dest, Some("00"), true, |_, _| {}).is_err());
        assert!(!dest.exists());
        assert!(fetch(&url, &dest, None, |_, _| {}).is_err());
    }
}
