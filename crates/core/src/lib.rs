//! devlokal-core: inti lingkungan pengembangan web lokal.
//!
//! Modul ini tidak bergantung pada UI sehingga bisa dipakai CLI maupun aplikasi desktop (Tauri).

pub mod components;
pub mod doctor;
pub mod download;
pub mod env;
pub mod extract;
pub mod home;
pub mod hosts;
pub mod i18n;
pub mod install;
pub mod nginx;
pub mod process;
pub mod projects;

pub use env::{Env, Service};
pub use home::{Home, Settings};
