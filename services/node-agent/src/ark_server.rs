//! ARK dedicated server process lifecycle.
use anyhow::{bail, Result};
use std::sync::Arc;
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

    let mod_list = cfg.mod_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");

    let map_arg = format!(
        "{ark_map_name}?SessionName={session_name}?Port={game}?QueryPort={query}\
         ?RCONEnabled=True?RCONPort={rcon}?ServerAdminPassword={pwd}\
         ?AltSaveDirectoryName=external-{node}?listen\
         {mods_arg}",
        game  = cfg.game_port,
        query = cfg.query_port,
        rcon  = cfg.rcon_port,
        pwd   = cfg.server_admin_password,
        node  = cfg.node_id,
        mods_arg = if mod_list.is_empty() { String::new() } else { format!("?GameModIds={}", mod_list) }
    );

    let cluster_id = if cfg.cluster_id.is_empty() {
        extract_cluster_id_from_share(cluster_dir)
    } else {
        cfg.cluster_id.clone()
    };

    tracing::info!("Starting ARK: {} map={} cluster_id={}", ark_map_name, map_id, cluster_id);

    let pid = spawn_ark_server(
        &exe,
        &map_arg,
        &cluster_id,
        cluster_dir,
        &cfg.additional_args,
    ).await?;

    {
        let mut s = state.write().await;
        s.running = true;
        s.map_id = map_id.to_string();
        s.session_id = session_id.to_string();
        s.pid = Some(pid);
    }

    // Monitor: poll until PID disappears
    let state_clone = state.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            if !pid_alive(pid).await {
                let mut s = state_clone.write().await;
                s.running = false;
                s.pid = None;
                tracing::info!("ARK server process (pid={}) exited", pid);
                break;
            }
        }
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
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }

    if let Some(pid) = pid {
        kill_pid(pid).await;
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

/// Returns true if ShooterGameServer.exe is running, regardless of server_state.
pub async fn detect_shooter_process() -> bool {
    find_process_pid("ShooterGameServer.exe").await.is_some()
}

/// Returns PID of running ShooterGameServer.exe, or None.
pub async fn find_shooter_pid() -> Option<u32> {
    find_process_pid("ShooterGameServer.exe").await
}

// ── internals ─────────────────────────────────────────────────────────────────

const LAUNCH_BAT: &str = r"C:\ProgramData\ArkClusterNode\launch_ark.bat";

/// On Windows: write a .bat and launch via schtasks /ru INTERACTIVE so the
/// process spawns in the user's desktop session (needed for UE4 GPU init).
/// On other platforms: spawn directly.
async fn spawn_ark_server(
    exe: &std::path::Path,
    map_arg: &str,
    cluster_id: &str,
    cluster_dir: &str,
    additional_args: &str,
) -> Result<u32> {
    #[cfg(windows)]
    {
        let exe_str = exe.to_string_lossy();
        // Strip trailing backslash — cmd.exe \"...\" with trailing \ escapes the closing quote
        let cluster_dir_clean = cluster_dir.trim_end_matches('\\').trim_end_matches('/');
        let extra = if additional_args.is_empty() { String::new() } else { format!(" {}", additional_args) };
        let bat = format!(
            "@echo off\r\nstart \"ARK Travel\" \"{}\" \"{}\" -clusterid={} -ClusterDirOverride=\"{}\" -NoBattlEye -server -log -servergamelog{}\r\n",
            exe_str, map_arg, cluster_id, cluster_dir_clean, extra
        );
        std::fs::write(LAUNCH_BAT, bat)?;

        // Run bat directly — `start` inside the bat detaches ShooterGameServer
        let out = tokio::process::Command::new("cmd")
            .args(["/c", LAUNCH_BAT])
            .output().await?;
        if !out.status.success() {
            let msg = String::from_utf8_lossy(&out.stderr);
            bail!("launch bat failed: {}", msg.trim());
        }

        // Wait for process to appear (batch spawns child process)
        for _ in 0..12 {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            if let Some(pid) = find_process_pid("ShooterGameServer.exe").await {
                tracing::info!("ARK server launched via schtask, pid={}", pid);
                return Ok(pid);
            }
        }
        bail!("ShooterGameServer.exe did not appear within 36s after schtask launch");
    }

    #[cfg(not(windows))]
    {
        let child = tokio::process::Command::new(exe)
            .arg(map_arg)
            .arg(format!("-clusterid={}", cluster_id))
            .arg(format!("-ClusterDirOverride={}", cluster_dir))
            .arg("-NoBattlEye")
            .arg("-server")
            .arg("-log")
            .spawn()?;
        child.id().ok_or_else(|| anyhow::anyhow!("failed to get child PID"))
    }
}

async fn find_process_pid(image: &str) -> Option<u32> {
    let out = tokio::process::Command::new("tasklist")
        .args(["/fi", &format!("imagename eq {}", image), "/fo", "csv", "/nh"])
        .output().await.ok()?;
    let s = String::from_utf8_lossy(&out.stdout);
    for line in s.lines() {
        if line.to_lowercase().contains(&image.to_lowercase()) {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 2 {
                return parts[1].trim().trim_matches('"').parse().ok();
            }
        }
    }
    None
}

async fn pid_alive(pid: u32) -> bool {
    #[cfg(windows)]
    {
        let out = tokio::process::Command::new("tasklist")
            .args(["/fi", &format!("pid eq {}", pid), "/fo", "csv", "/nh"])
            .output().await;
        match out {
            Ok(o) => String::from_utf8_lossy(&o.stdout).contains(&pid.to_string()),
            Err(_) => false,
        }
    }
    #[cfg(not(windows))]
    {
        tokio::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .output().await
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

async fn kill_pid(pid: u32) {
    #[cfg(windows)]
    {
        let _ = tokio::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .output().await;
    }
    #[cfg(not(windows))]
    {
        let _ = tokio::process::Command::new("kill")
            .args(["-9", &pid.to_string()])
            .output().await;
    }
}

fn extract_cluster_id_from_share(share_path: &str) -> String {
    let mut h = sha2::Sha256::new();
    use sha2::Digest;
    h.update(share_path.as_bytes());
    let hash = hex::encode(h.finalize());
    format!("ark-{}", &hash[..8])
}
