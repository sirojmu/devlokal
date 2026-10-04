// Mencegah jendela konsol tambahan di Windows (rilis).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use devlokal_core::{Env, Service};
use serde::Serialize;

#[derive(Serialize)]
struct ServiceInfo {
    name: &'static str,
    component: Option<String>,
    port: u16,
    running: bool,
}

#[derive(Serialize)]
struct ProjectInfo {
    domain: String,
    url: String,
    docroot: String,
}

fn svc(name: &str) -> Result<Service, String> {
    Service::parse(name).ok_or_else(|| format!("layanan tidak dikenal: {name}"))
}

#[tauri::command]
fn status() -> Result<Vec<ServiceInfo>, String> {
    let env = Env::load().map_err(|e| format!("{e:#}"))?;
    Ok(env
        .status()
        .into_iter()
        .map(|s| ServiceInfo {
            name: s.service.name(),
            component: s.component,
            port: s.port,
            running: s.pid.is_some(),
        })
        .collect())
}

#[tauri::command]
fn projects() -> Result<Vec<ProjectInfo>, String> {
    let env = Env::load().map_err(|e| format!("{e:#}"))?;
    let port = env.settings.http_port;
    Ok(env
        .projects()
        .into_iter()
        .map(|p| ProjectInfo {
            url: if port == 80 {
                format!("http://{}", p.domain)
            } else {
                format!("http://{}:{port}", p.domain)
            },
            domain: p.domain,
            docroot: p.docroot.display().to_string(),
        })
        .collect())
}

#[tauri::command]
fn start(name: String) -> Result<(), String> {
    let env = Env::load().map_err(|e| format!("{e:#}"))?;
    env.start(svc(&name)?).map(|_| ()).map_err(|e| format!("{e:#}"))
}

#[tauri::command]
fn stop(name: String) -> Result<(), String> {
    let env = Env::load().map_err(|e| format!("{e:#}"))?;
    env.stop(svc(&name)?).map(|_| ()).map_err(|e| format!("{e:#}"))
}

#[tauri::command]
fn sync() -> Result<usize, String> {
    let env = Env::load().map_err(|e| format!("{e:#}"))?;
    env.sync(false).map(|r| r.projects.len()).map_err(|e| format!("{e:#}"))
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![status, projects, start, stop, sync])
        .run(tauri::generate_context!())
        .expect("gagal menjalankan aplikasi devlokal");
}
