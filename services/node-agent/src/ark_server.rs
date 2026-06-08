//! ARK dedicated server process lifecycle.
use anyhow::{bail, Result};
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::RwLock;

use crate::config::NodeConfig;
use crate::rcon;

#[derive(Debug, Clone)]
pub struct ServerState {
    pub running: bool,
    pub map_id: String,
    pub session_id: String,
    pub pid: Option<u32>,
}

pub type SharedServerState = Arc<RwLock<ServerState>>;

pub fn new_shared_state() -> SharedServerState {
    Arc::new(RwLock::new(ServerState {
        running: false,
        map_id: String::new(),
        session_id: String::new(),
        pid: None,
    }))
}

pub async fn start(
    cfg: &NodeConfig,
    map_id: &str,
    ark_map_name: &str,
    session_name: &str,
    session_id: &str,
    cluster_share_override: Option<&str>,
    state: SharedServerState,
) -> Result<()> {
    {
        let s = state.read().await;
        if s.running {
            bail!("travel server already running (map={})", s.map_id);
        }
    }

    let exe = cfg.ark_server_exe();
    if !exe.exists() {
        bail!("ARK server exe not found: {}", exe.display());
    }

    let cluster_dir = cluster_share_override.unwrap_or(&cfg.cluster_share_path);
    if !std::path::Path::new(cluster_dir).exists() {
        bail!("cluster share not mounted: {}", cluster_dir);
    }

    // Build mod list string
    let mod_list = cfg.mod_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");

    let map_arg = format!(
        "{ark_map_name}?SessionName={session_name}?Port={game}?QueryPort={query}\
         ?RCONEnabled=True?RCONPort={rcon}?ServerAdminPassword={pwd}\
         ?AltSaveDirectoryName=external-{node}?listen\
         {mods_arg}",
        game = cfg.game_port,
        query = cfg.query_port,
        rcon = cfg.rcon_port,
        pwd = cfg.server_admin_password,
        node = cfg.node_id,
        mods_arg = if mod_list.is_empty() { String::new() } else { format!("?GameModIds={}", mod_list) }
    );

    let cluster_id = extract_cluster_id_from_share(cluster_dir);

    tracing::info!("Starting ARK: {} map={}", ark_map_name, map_id);

    let mut child = Command::new(&exe)
        .arg(&map_arg)
        .arg(format!("-clusterid={}", cluster_id))
        .arg(format!("-ClusterDirOverride={}", cluster_dir))
        .arg("-NoBattlEye")
        .arg("-server")
        .arg("-log")
        .arg("-servergamelog")
        .spawn()?;

    let pid = child.id();

    {
        let mut s = state.write().await;
        s.running = true;
        s.map_id = map_id.to_string();
        s.session_id = session_id.to_string();
        s.pid = pid;
    }

    let state_clone = state.clone();
    tokio::spawn(async move {
        let _ = child.wait().await;
        let mut s = state_clone.write().await;
        s.running = false;
        s.pid = None;
        tracing::info!("ARK server process exited");
    });

    Ok(())
}

pub async fn stop(cfg: &NodeConfig, save_first: bool, state: SharedServerState) -> Result<()> {
    let pid = {
        let s = state.read().await;
        if !s.running { return Ok(()); }
        s.pid
    };

    if save_first {
        match rcon::save_world("127.0.0.1", cfg.rcon_port, &cfg.server_admin_password).await {
            Ok(_) => tracing::info!("World saved via RCON"),
            Err(e) => tracing::warn!("SaveWorld failed (continuing stop): {}", e),
        }
        // Give the server a moment to flush save
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }

    if let Some(pid) = pid {
        #[cfg(target_os = "windows")]
        {
            let _ = Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/F"])
                .output()
                .await;
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = Command::new("kill").arg(pid.to_string()).output().await;
        }
    }

    {
        let mut s = state.write().await;
        s.running = false;
        s.pid = None;
        s.map_id.clear();
        s.session_id.clear();
    }
    Ok(())
}

pub async fn is_running(state: &SharedServerState) -> bool {
    state.read().await.running
}

pub async fn check_rcon_ready(cfg: &NodeConfig) -> bool {
    rcon::save_world("127.0.0.1", cfg.rcon_port, &cfg.server_admin_password)
        .await
        .is_ok()
}

fn extract_cluster_id_from_share(share_path: &str) -> String {
    // e.g. "Z:\ark-cluster-main" → "ark-prime-7f3a" from config ideally
    // Fall back to a hash of the path
    let mut h = sha2::Sha256::new();
    use sha2::Digest;
    h.update(share_path.as_bytes());
    let hash = hex::encode(h.finalize());
    format!("ark-{}", &hash[..8])
}
