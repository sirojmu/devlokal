//! Lokasi data devlokal dan pengaturan pengguna.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Folder kerja devlokal. Bisa diganti lewat variabel `DEVLOKAL_HOME`.
#[derive(Debug, Clone)]
pub struct Home {
    pub root: PathBuf,
}

impl Home {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn discover() -> Result<Self> {
        if let Some(p) = std::env::var_os("DEVLOKAL_HOME") {
            return Ok(Self::new(p));
        }
        let base: PathBuf = if cfg!(windows) {
            std::env::var_os("LOCALAPPDATA")
                .map(PathBuf::from)
                .context("LOCALAPPDATA tidak ditemukan")?
        } else if cfg!(target_os = "macos") {
            PathBuf::from(std::env::var_os("HOME").context("HOME tidak ditemukan")?)
                .join("Library/Application Support")
        } else if let Some(x) = std::env::var_os("XDG_DATA_HOME") {
            PathBuf::from(x)
        } else {
            PathBuf::from(std::env::var_os("HOME").context("HOME tidak ditemukan")?)
                .join(".local/share")
        };
        Ok(Self::new(base.join("devlokal")))
    }

    pub fn cache(&self) -> PathBuf {
        self.root.join("cache")
    }
    pub fn components(&self) -> PathBuf {
        self.root.join("components")
    }
    pub fn config(&self) -> PathBuf {
        self.root.join("config")
    }
    pub fn logs(&self) -> PathBuf {
        self.root.join("logs")
    }
    pub fn run(&self) -> PathBuf {
        self.root.join("run")
    }
    pub fn data(&self) -> PathBuf {
        self.root.join("data")
    }
    pub fn backup(&self) -> PathBuf {
        self.root.join("backup")
    }
    pub fn settings_file(&self) -> PathBuf {
        self.root.join("devlokal.toml")
    }

    pub fn ensure(&self) -> Result<()> {
        for d in [
            self.cache(),
            self.components(),
            self.config(),
            self.logs(),
            self.run(),
            self.data(),
            self.backup(),
        ] {
            std::fs::create_dir_all(&d)
                .with_context(|| format!("gagal membuat folder {}", d.display()))?;
        }
        Ok(())
    }
}

/// Pengaturan pengguna, disimpan di `devlokal.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Folder proyek. Bawaan: `<home>/www`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub www: Option<PathBuf>,
    /// Akhiran domain lokal. `.test` dipilih karena dicadangkan untuk pengujian (RFC 6761).
    pub tld: String,
    pub http_port: u16,
    pub php_port: u16,
    pub db_port: u16,
    /// Komponen aktif per layanan, mis. `php = "php-8.3.12"`.
    pub active: BTreeMap<String, String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            www: None,
            tld: "test".into(),
            // Port di bawah 1024 butuh hak istimewa di Linux/macOS.
            http_port: if cfg!(windows) { 80 } else { 8080 },
            php_port: 9000,
            db_port: 3306,
            active: BTreeMap::new(),
        }
    }
}

impl Settings {
    pub fn load(home: &Home) -> Result<Self> {
        let p = home.settings_file();
        if !p.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(&p)?;
        toml::from_str(&text).with_context(|| format!("{} tidak valid", p.display()))
    }

    pub fn save(&self, home: &Home) -> Result<()> {
        std::fs::create_dir_all(&home.root)?;
        std::fs::write(home.settings_file(), toml::to_string_pretty(self)?)?;
        Ok(())
    }

    pub fn www_dir(&self, home: &Home) -> PathBuf {
        self.www.clone().unwrap_or_else(|| home.root.join("www"))
    }
}
