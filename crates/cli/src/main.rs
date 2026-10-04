use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use devlokal_core::components::{self, Manifest};
use devlokal_core::download;
use devlokal_core::i18n::tr;
use devlokal_core::{doctor, install, projects, Env, Service};
use std::io::Write;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "devlokal",
    version,
    about = "Lingkungan pengembangan web lokal yang ringan dan hemat bandwidth"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Tampilkan lokasi folder dan pengaturan
    Paths,
    /// Periksa komponen, port, dan izin hosts
    Doctor,
    /// Daftar komponen di manifest dan yang sudah terpasang
    List,
    /// Unduh dan pasang komponen dari manifest (dapat dilanjutkan, diverifikasi SHA-256)
    Fetch {
        id: String,
        /// Terima berkas tanpa checksum di manifest; checksum hasil unduhan ditampilkan untuk dicocokkan manual
        #[arg(long)]
        trust_first_use: bool,
    },
    /// Pasang komponen dari arsip (.zip/.tar.gz) atau folder lokal, tanpa internet
    Import {
        id: String,
        path: PathBuf,
        /// Jumlah folder teratas arsip yang dibuang
        #[arg(long, default_value_t = 0)]
        strip: usize,
    },
    /// Pilih komponen aktif untuk satu layanan (nginx, php, mariadb)
    Use { service: String, id: String },
    /// Buat proyek baru dari template (php, html)
    New {
        name: String,
        #[arg(long, default_value = "php")]
        template: String,
    },
    /// Tulis ulang konfigurasi dan perbarui file hosts (hosts butuh Administrator/sudo)
    Sync {
        /// Jangan ubah file hosts
        #[arg(long)]
        no_hosts: bool,
    },
    /// Jalankan layanan (semua bila tidak disebut)
    Up { services: Vec<String> },
    /// Hentikan layanan (semua bila tidak disebut)
    Down { services: Vec<String> },
    /// Tampilkan status layanan dan daftar proyek
    Status,
}

fn services(names: &[String]) -> Result<Vec<Service>> {
    if names.is_empty() {
        return Ok(Service::ALL.to_vec());
    }
    names
        .iter()
        .map(|n| {
            Service::parse(n)
                .with_context(|| format!("layanan tidak dikenal: {n} (nginx, php, mariadb)"))
        })
        .collect()
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{} {e:#}", tr("Galat:", "Error:"));
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let mut env = Env::load()?;
    match cli.cmd {
        Cmd::Paths => {
            println!("home : {}", env.home.root.display());
            println!("www  : {}", env.settings.www_dir(&env.home).display());
            println!("cfg  : {}", env.home.settings_file().display());
            println!("hosts: {}", devlokal_core::hosts::hosts_path().display());
            println!("platform: {}", components::platform_key());
        }
        Cmd::Doctor => {
            let mut bad = 0;
            for c in doctor::run(&env) {
                println!(
                    "[{}] {:<22} {}",
                    if c.ok { " ok " } else { "perlu" },
                    c.name,
                    c.detail
                );
                if !c.ok {
                    bad += 1;
                }
            }
            if bad > 0 {
                println!(
                    "\n{bad} {}",
                    tr("hal perlu perhatian.", "item(s) need attention.")
                );
            }
        }
        Cmd::List => {
            let m = Manifest::bundled()?;
            let have = components::installed(&env.home)?;
            println!("{}", tr("Manifest:", "Manifest:"));
            for c in &m.components {
                let src = c.sources.get(&components::platform_key());
                let state = match src {
                    None => tr("tidak tersedia untuk OS ini", "not available for this OS"),
                    Some(s) if s.url.is_empty() => tr(
                        "URL belum diisi (gunakan import)",
                        "URL not filled (use import)",
                    ),
                    Some(s) if s.sha256.is_empty() => {
                        tr("checksum belum diverifikasi", "checksum unverified")
                    }
                    Some(_) => tr("siap diunduh", "ready to fetch"),
                };
                println!("  {:<16} {}", c.id, state);
            }
            println!("{}", tr("Terpasang:", "Installed:"));
            for id in have {
                println!("  {id}");
            }
        }
        Cmd::Fetch {
            id,
            trust_first_use,
        } => {
            let m = Manifest::bundled()?;
            let c = m
                .find(&id)
                .with_context(|| format!("{id} tidak ada di manifest"))?;
            let key = components::platform_key();
            let s = c
                .sources
                .get(&key)
                .with_context(|| format!("{id} belum tersedia untuk {key}"))?;
            if s.url.is_empty() {
                bail!("URL untuk {id} belum diisi di manifest. Unduh manual lalu: devlokal import {id} <berkas>");
            }
            if s.sha256.is_empty() && !trust_first_use {
                bail!("checksum {id} belum diverifikasi di manifest. Isi sha256 dari sumber resmi, atau gunakan --trust-first-use dan cocokkan hasilnya secara manual.");
            }
            let fname = s.url.rsplit('/').next().unwrap_or("download.bin");
            let dest = env.home.cache().join(fname);
            let expected = if s.sha256.is_empty() {
                None
            } else {
                Some(s.sha256.as_str())
            };
            let hash = download::fetch(&s.url, &dest, expected, |done, total| {
                match total {
                    Some(t) if t > 0 => eprint!(
                        "\r  {:>3}%  {} / {} KB",
                        done * 100 / t,
                        done / 1024,
                        t / 1024
                    ),
                    _ => eprint!("\r  {} KB", done / 1024),
                }
                let _ = std::io::stderr().flush();
            })?;
            eprintln!();
            println!("sha256: {hash}");
            install::install_archive(&env.home, &id, &dest, Some(s.archive), s.strip)?;
            println!("{} {id}", tr("Terpasang:", "Installed:"));
        }
        Cmd::Import { id, path, strip } => {
            if path.is_dir() {
                install::install_dir(&env.home, &id, &path)?;
            } else {
                install::install_archive(&env.home, &id, &path, None, strip)?;
            }
            println!("{} {id}", tr("Terpasang:", "Installed:"));
        }
        Cmd::Use { service, id } => {
            let svc = Service::parse(&service)
                .with_context(|| format!("layanan tidak dikenal: {service}"))?;
            if components::kind_of(&id) != Some(svc.kind()) {
                bail!("{id} bukan komponen {service}");
            }
            if !components::component_dir(&env.home, &id).exists() {
                bail!("{id} belum terpasang");
            }
            env.settings.active.insert(service.clone(), id.clone());
            env.settings.save(&env.home)?;
            println!("{service} -> {id}");
        }
        Cmd::New { name, template } => {
            let dir = projects::create(&env.settings.www_dir(&env.home), &name, &template)?;
            println!(
                "{} {}",
                tr("Proyek dibuat:", "Project created:"),
                dir.display()
            );
            println!(
                "{}",
                tr(
                    "Jalankan `devlokal sync` agar domainnya aktif.",
                    "Run `devlokal sync` to activate its domain."
                )
            );
        }
        Cmd::Sync { no_hosts } => {
            let r = env.sync(!no_hosts)?;
            for p in &r.projects {
                println!(
                    "  http://{}{}",
                    p.domain,
                    port_suffix(env.settings.http_port)
                );
            }
            match r.hosts_changed {
                Some(true) => println!("{}", tr("File hosts diperbarui.", "Hosts file updated.")),
                Some(false) => println!(
                    "{}",
                    tr("File hosts sudah sesuai.", "Hosts file already up to date.")
                ),
                None => {}
            }
        }
        Cmd::Up { services: names } => {
            let list = services(&names)?;
            for svc in list {
                match env.start(svc) {
                    Ok(Some(pid)) => println!(
                        "  {:<8} {} (pid {pid})",
                        svc.name(),
                        tr("berjalan", "started")
                    ),
                    Ok(None) => println!(
                        "  {:<8} {}",
                        svc.name(),
                        tr("sudah berjalan", "already running")
                    ),
                    Err(e) => {
                        eprintln!("  {:<8} {} {e:#}", svc.name(), tr("GAGAL:", "FAILED:"));
                    }
                }
            }
            if !env.hosts_up_to_date() {
                println!(
                    "\n{}",
                    tr(
                        "Catatan: file hosts belum memuat domain proyek. Jalankan `devlokal sync` sebagai Administrator/sudo.",
                        "Note: hosts file lacks your project domains. Run `devlokal sync` as Administrator/sudo.",
                    )
                );
            }
        }
        Cmd::Down { services: names } => {
            let mut list = services(&names)?;
            list.reverse();
            for svc in list {
                let stopped = env.stop(svc)?;
                println!(
                    "  {:<8} {}",
                    svc.name(),
                    if stopped {
                        tr("dihentikan", "stopped")
                    } else {
                        tr("tidak berjalan", "not running")
                    }
                );
            }
        }
        Cmd::Status => {
            for s in env.status() {
                println!(
                    "  {:<8} {:<16} port {:<5} {}",
                    s.service.name(),
                    s.component.unwrap_or_else(|| "-".into()),
                    s.port,
                    match s.pid {
                        Some(p) => format!("{} (pid {p})", tr("berjalan", "running")),
                        None => tr("berhenti", "stopped").to_string(),
                    }
                );
            }
            let ps = env.projects();
            println!("\n{} ({}):", tr("Proyek", "Projects"), ps.len());
            for p in ps {
                println!(
                    "  http://{}{}  ->  {}",
                    p.domain,
                    port_suffix(env.settings.http_port),
                    p.docroot.display()
                );
            }
        }
    }
    Ok(())
}

fn port_suffix(port: u16) -> String {
    if port == 80 {
        String::new()
    } else {
        format!(":{port}")
    }
}
