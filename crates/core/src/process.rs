//! Utilitas proses lintas OS: jalankan terlepas dari terminal, cek hidup, hentikan.

use anyhow::{Context, Result};
use std::fs::OpenOptions;
use std::net::TcpListener;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Port bebas bila kita bisa mengikatnya di 127.0.0.1.
pub fn port_free(port: u16) -> bool {
    TcpListener::bind(("127.0.0.1", port)).is_ok()
}

/// Jalankan `cmd` di latar belakang, log stdout/stderr ke `log`. Mengembalikan PID.
pub fn spawn_detached(cmd: &mut Command, log: &Path) -> Result<u32> {
    if let Some(p) = log.parent() {
        std::fs::create_dir_all(p)?;
    }
    let f = OpenOptions::new().create(true).append(true).open(log)?;
    cmd.stdin(Stdio::null())
        .stdout(Stdio::from(f.try_clone()?))
        .stderr(Stdio::from(f));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW | DETACHED_PROCESS
        cmd.creation_flags(0x0800_0000 | 0x0000_0008);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    let child = cmd.spawn().context("gagal menjalankan proses")?;
    Ok(child.id())
}

pub fn is_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .creation_flags(0x0800_0000)
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains(&pid.to_string()))
            .unwrap_or(false)
    }
}

/// Hentikan proses beserta anak-anaknya. Menunggu hingga 5 detik sebelum memaksa.
pub fn terminate(pid: u32) -> Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .creation_flags(0x0800_0000)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
    }
    #[cfg(unix)]
    {
        let _ = Command::new("kill").arg(pid.to_string()).status();
        let t = Instant::now();
        while is_alive(pid) && t.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(100));
        }
        if is_alive(pid) {
            let _ = Command::new("kill").args(["-9", &pid.to_string()]).status();
        }
    }
    let _ = (Instant::now(), Duration::from_secs(0));
    Ok(())
}
