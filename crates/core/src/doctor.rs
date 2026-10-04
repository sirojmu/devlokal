//! Pemeriksaan lingkungan: komponen, port, dan izin hosts.

use crate::components::{self, Kind};
use crate::env::{Env, Service};
use crate::{hosts, i18n::tr, process};
use std::fs::OpenOptions;

#[derive(Debug)]
pub struct Check {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

pub fn run(env: &Env) -> Vec<Check> {
    let mut out = vec![];
    for k in [Kind::Nginx, Kind::Php, Kind::Mariadb] {
        let id = components::resolve(&env.home, &env.settings, k);
        out.push(Check {
            name: format!("{:?}", k).to_lowercase(),
            ok: id.is_some(),
            detail: id.unwrap_or_else(|| tr("belum terpasang", "not installed").into()),
        });
    }
    for svc in Service::ALL {
        let port = env.port(svc);
        let running = env.running_pid(svc).is_some();
        let free = process::port_free(port);
        out.push(Check {
            name: format!("port {port} ({})", svc.name()),
            ok: free || running,
            detail: if running {
                tr(
                    "dipakai layanan ini (berjalan)",
                    "used by this service (running)",
                )
                .into()
            } else if free {
                tr("bebas", "free").into()
            } else {
                tr(
                    "dipakai program lain (IIS/Skype/XAMPP/MySQL lain?)",
                    "used by another program (IIS/Skype/XAMPP/other MySQL?)",
                )
                .into()
            },
        });
    }
    let hp = hosts::hosts_path();
    let writable = OpenOptions::new().append(true).open(&hp).is_ok();
    out.push(Check {
        name: "hosts".into(),
        ok: writable,
        detail: if writable {
            tr("bisa ditulis", "writable").into()
        } else {
            tr(
                "tidak bisa ditulis; `devlokal sync` perlu Administrator/sudo",
                "not writable; `devlokal sync` needs Administrator/sudo",
            )
            .into()
        },
    });
    out
}
