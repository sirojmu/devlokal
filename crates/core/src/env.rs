//! Orkestrasi: sinkronisasi konfigurasi, start/stop layanan, status.

use crate::components::{self, component_dir, find_exe, Kind};
use crate::home::{Home, Settings};
use crate::{hosts, nginx, process, projects};
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Service {
    Mariadb,
    Php,
    Nginx,
}

impl Service {
    /// Urutan start; stop memakai urutan sebaliknya.
    pub const ALL: [Service; 3] = [Service::Mariadb, Service::Php, Service::Nginx];

    pub fn name(self) -> &'static str {
        match self {
            Service::Mariadb => "mariadb",
            Service::Php => "php",
            Service::Nginx => "nginx",
        }
    }
    pub fn kind(self) -> Kind {
        match self {
            Service::Mariadb => Kind::Mariadb,
            Service::Php => Kind::Php,
            Service::Nginx => Kind::Nginx,
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|x| x.name() == s)
    }
}

#[derive(Debug)]
pub struct SyncReport {
    pub projects: Vec<projects::Project>,
    /// `None` bila hosts tidak disentuh; `Some(true)` bila berubah.
    pub hosts_changed: Option<bool>,
}

#[derive(Debug)]
pub struct Status {
    pub service: Service,
    pub component: Option<String>,
    pub port: u16,
    pub pid: Option<u32>,
}

pub struct Env {
    pub home: Home,
    pub settings: Settings,
}

impl Env {
    pub fn load() -> Result<Self> {
        Self::from_home(Home::discover()?)
    }

    pub fn from_home(home: Home) -> Result<Self> {
        home.ensure()?;
        let settings = Settings::load(&home)?;
        fs::create_dir_all(settings.www_dir(&home))?;
        Ok(Self { home, settings })
    }

    pub fn port(&self, svc: Service) -> u16 {
        match svc {
            Service::Mariadb => self.settings.db_port,
            Service::Php => self.settings.php_port,
            Service::Nginx => self.settings.http_port,
        }
    }

    fn pid_file(&self, svc: Service) -> PathBuf {
        self.home.run().join(format!("{}.pid", svc.name()))
    }

    pub fn running_pid(&self, svc: Service) -> Option<u32> {
        let pid: u32 = fs::read_to_string(self.pid_file(svc))
            .ok()?
            .trim()
            .parse()
            .ok()?;
        if process::is_alive(pid) {
            Some(pid)
        } else {
            let _ = fs::remove_file(self.pid_file(svc));
            None
        }
    }

    pub fn status(&self) -> Vec<Status> {
        Service::ALL
            .into_iter()
            .map(|svc| Status {
                service: svc,
                component: components::resolve(&self.home, &self.settings, svc.kind()),
                port: self.port(svc),
                pid: self.running_pid(svc),
            })
            .collect()
    }

    pub fn projects(&self) -> Vec<projects::Project> {
        projects::scan(&self.settings.www_dir(&self.home), &self.settings.tld)
    }

    /// Tulis ulang konfigurasi dari daftar proyek. `update_hosts` butuh hak admin.
    pub fn sync(&self, update_hosts: bool) -> Result<SyncReport> {
        let ps = self.projects();
        let conf = nginx::render_nginx(&self.home, &self.settings, &ps);
        fs::write(self.home.config().join("nginx.conf"), conf)?;
        let hosts_changed = if update_hosts {
            let domains: Vec<String> = ps.iter().map(|p| p.domain.clone()).collect();
            Some(hosts::apply(&self.home, &domains)?)
        } else {
            None
        };
        Ok(SyncReport {
            projects: ps,
            hosts_changed,
        })
    }

    pub fn hosts_up_to_date(&self) -> bool {
        let domains: Vec<String> = self.projects().iter().map(|p| p.domain.clone()).collect();
        hosts::is_up_to_date(&domains).unwrap_or(false)
    }

    fn comp(&self, svc: Service) -> Result<(String, PathBuf)> {
        let id =
            components::resolve(&self.home, &self.settings, svc.kind()).with_context(|| {
                format!(
                "komponen {} belum terpasang. Jalankan `devlokal fetch` atau `devlokal import`.",
                svc.name()
            )
            })?;
        let dir = component_dir(&self.home, &id);
        if !dir.exists() {
            bail!(
                "komponen {id} dipilih tetapi tidak ada di {}",
                dir.display()
            );
        }
        Ok((id, dir))
    }

    fn nginx_cmd(&self, exe: &PathBuf) -> Command {
        let prefix = self.home.run().join("nginx");
        let mut c = Command::new(exe);
        c.arg("-p")
            .arg(&prefix)
            .arg("-e")
            .arg(self.home.logs().join("nginx-error.log"))
            .arg("-c")
            .arg(self.home.config().join("nginx.conf"));
        c
    }

    /// Jalankan satu layanan. Mengembalikan PID; `Ok(None)` bila sudah berjalan.
    pub fn start(&self, svc: Service) -> Result<Option<u32>> {
        if self.running_pid(svc).is_some() {
            return Ok(None);
        }
        let port = self.port(svc);
        if !process::port_free(port) {
            bail!(
                "port {port} untuk {} sedang dipakai program lain (mis. IIS, Skype, XAMPP, atau MySQL lain). Hentikan program itu atau ubah port di devlokal.toml.",
                svc.name()
            );
        }
        let (_id, dir) = self.comp(svc)?;
        let log = self.home.logs().join(format!("{}.log", svc.name()));
        let pid = match svc {
            Service::Php => {
                let exe = find_exe(&dir, &["php-cgi"])
                    .context("php-cgi tidak ditemukan di komponen PHP")?;
                let ini = self.home.config().join("php.ini");
                fs::write(&ini, nginx::render_php_ini(&dir))?;
                let mut c = Command::new(exe);
                c.arg("-c")
                    .arg(&ini)
                    .arg("-b")
                    .arg(format!("127.0.0.1:{port}"))
                    .env("PHP_FCGI_MAX_REQUESTS", "0")
                    .current_dir(&dir);
                process::spawn_detached(&mut c, &log)?
            }
            Service::Nginx => {
                let exe = find_exe(&dir, &["nginx"])
                    .context("nginx tidak ditemukan di komponen nginx")?;
                let prefix = self.home.run().join("nginx");
                fs::create_dir_all(prefix.join("logs"))?;
                fs::create_dir_all(prefix.join("temp"))?;
                self.sync(false)?;
                // Uji konfigurasi dulu agar pesan galat jelas.
                let out = self.nginx_cmd(&exe).arg("-t").output()?;
                if !out.status.success() {
                    bail!(
                        "konfigurasi nginx tidak valid:\n{}",
                        String::from_utf8_lossy(&out.stderr)
                    );
                }
                let mut c = self.nginx_cmd(&exe);
                c.current_dir(&dir);
                process::spawn_detached(&mut c, &log)?
            }
            Service::Mariadb => {
                let mysqld = find_exe(&dir, &["mariadbd", "mysqld"])
                    .context("mariadbd/mysqld tidak ditemukan")?;
                let data = self.home.data().join("mariadb");
                let ini = self.home.config().join("my.ini");
                fs::write(&ini, nginx::render_my_ini(&self.home, &self.settings, &dir))?;
                if !data.join("mysql").exists() {
                    let init = find_exe(&dir, &["mariadb-install-db", "mysql_install_db"])
                        .context("mariadb-install-db tidak ditemukan")?;
                    fs::create_dir_all(&data)?;
                    let mut c = Command::new(init);
                    c.arg(format!("--datadir={}", data.display()));
                    if !cfg!(windows) {
                        c.arg(format!("--basedir={}", dir.display()));
                    }
                    let out = c.output()?;
                    if !out.status.success() {
                        bail!(
                            "inisialisasi database gagal:\n{}{}",
                            String::from_utf8_lossy(&out.stdout),
                            String::from_utf8_lossy(&out.stderr)
                        );
                    }
                }
                let mut c = Command::new(mysqld);
                c.arg(format!("--defaults-file={}", ini.display()))
                    .arg("--console");
                process::spawn_detached(&mut c, &log)?
            }
        };
        fs::write(self.pid_file(svc), pid.to_string())?;
        Ok(Some(pid))
    }

    /// Hentikan satu layanan. `true` bila ada yang dihentikan.
    pub fn stop(&self, svc: Service) -> Result<bool> {
        let Some(pid) = self.running_pid(svc) else {
            return Ok(false);
        };
        process::terminate(pid)?;
        let _ = fs::remove_file(self.pid_file(svc));
        Ok(true)
    }
}
