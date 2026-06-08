use std::net::{TcpListener, UdpSocket};
use std::path::Path;
use sysinfo::System;

use crate::config::NodeConfig;

#[derive(Debug, Clone)]
pub struct NodeChecks {
    pub tailscale_online: bool,
    pub cluster_share_mounted: bool,
    pub ark_server_installed: bool,
    pub mods_valid: bool,
    pub ports_free: bool,
    pub available_ram_mb: u64,
    pub total_ram_mb: u64,
    pub last_error: Option<String>,
}

pub async fn run_checks(cfg: &NodeConfig) -> NodeChecks {
    let mut errors: Vec<String> = Vec::new();

    // RAM
    let mut sys = System::new();
    sys.refresh_memory();
    let total_ram_mb = sys.total_memory() / 1024 / 1024;
    let available_ram_mb = sys.available_memory() / 1024 / 1024;

    // Tailscale: check if manager is reachable on Tailscale IP
    let tailscale_online = check_tailscale_reachable(&cfg.manager_url).await;
    if !tailscale_online {
        errors.push("Tailscale or manager unreachable".into());
    }

    // Cluster share
    let cluster_share_mounted = Path::new(&cfg.cluster_share_path).exists();
    if !cluster_share_mounted {
        errors.push(format!("Cluster share not mounted: {}", cfg.cluster_share_path));
    }

    // ARK server installed
    let exe = cfg.ark_server_exe();
    let ark_server_installed = exe.exists();
    if !ark_server_installed {
        errors.push(format!("ARK server exe not found: {}", exe.display()));
    }

    // Mods
    let mods_valid = validate_mods(cfg);
    if !mods_valid {
        errors.push("One or more required mods missing".into());
    }

    // Ports
    let ports_free = check_ports_free(cfg);
    if !ports_free {
        errors.push(format!("Ports in use: {}/{}/{}/{}", cfg.game_port, cfg.raw_port, cfg.query_port, cfg.rcon_port));
    }

    NodeChecks {
        tailscale_online,
        cluster_share_mounted,
        ark_server_installed,
        mods_valid,
        ports_free,
        available_ram_mb,
        total_ram_mb,
        last_error: if errors.is_empty() { None } else { Some(errors.join("; ")) },
    }
}

async fn check_tailscale_reachable(manager_url: &str) -> bool {
    // Parse host from URL and attempt TCP connect with 3s timeout
    let url = manager_url.trim_end_matches('/');
    let host = url.strip_prefix("http://").or_else(|| url.strip_prefix("https://")).unwrap_or(url);
    let (host, port) = if let Some(colon) = host.rfind(':') {
        let p: u16 = host[colon + 1..].parse().unwrap_or(80);
        (&host[..colon], p)
    } else {
        (host, 80)
    };
    let addr = format!("{}:{}", host, port);
    tokio::time::timeout(
        std::time::Duration::from_secs(3),
        tokio::net::TcpStream::connect(&addr),
    )
    .await
    .map(|r| r.is_ok())
    .unwrap_or(false)
}

fn validate_mods(cfg: &NodeConfig) -> bool {
    let mods_dir = cfg.mods_dir();
    for mod_id in &cfg.mod_ids {
        let mod_folder = mods_dir.join(mod_id.to_string());
        let mod_file = mods_dir.join(format!("{}.mod", mod_id));
        if !mod_folder.exists() || !mod_file.exists() {
            tracing::warn!("mod {} missing (folder={}, .mod={})", mod_id, mod_folder.exists(), mod_file.exists());
            return false;
        }
    }
    true
}

fn check_ports_free(cfg: &NodeConfig) -> bool {
    // For TCP (RCON) check TcpListener bind; for UDP check UdpSocket bind
    let rcon_free = TcpListener::bind(format!("0.0.0.0:{}", cfg.rcon_port)).is_ok();
    let game_free = UdpSocket::bind(format!("0.0.0.0:{}", cfg.game_port)).is_ok();
    let raw_free = UdpSocket::bind(format!("0.0.0.0:{}", cfg.raw_port)).is_ok();
    let query_free = UdpSocket::bind(format!("0.0.0.0:{}", cfg.query_port)).is_ok();
    rcon_free && game_free && raw_free && query_free
}

pub fn check_enough_ram(checks: &NodeChecks, min_mb: u64) -> bool {
    checks.available_ram_mb >= min_mb
}
