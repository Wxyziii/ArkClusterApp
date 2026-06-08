//! ARK Cluster Windows travel node agent.
//! Connects to the manager over Tailscale. Reports heartbeats. Polls tasks.

#![allow(dead_code)]

mod ark_server;
mod checks;
mod config;
mod heartbeat;
mod rcon;
mod tasks;

use std::sync::Arc;
use anyhow::{Context, Result};
use tokio::sync::watch;
use tracing_subscriber::EnvFilter;

const SERVICE_NAME: &str = "ArkClusterNodeAgent";

// ── SMB auto-mount ────────────────────────────────────────────────────────────

#[cfg(windows)]
fn mount_smb_share(cfg: &config::NodeConfig) {
    if cfg.smb_unc_path.is_empty() || cfg.smb_user.is_empty() { return; }
    // Extract drive letter from cluster_share_path (e.g. "Z:\")
    let drive = cfg.cluster_share_path.chars().next().unwrap_or('Z');
    let drive_arg = format!("{}:", drive);
    tracing::info!("Mounting SMB share {} -> {}", cfg.smb_unc_path, drive_arg);
    let out = std::process::Command::new("net")
        .args(["use", &drive_arg, &cfg.smb_unc_path,
               &format!("/user:{}", cfg.smb_user), &cfg.smb_password,
               "/persistent:no"])
        .output();
    match out {
        Ok(o) if o.status.success() => tracing::info!("SMB share mounted"),
        Ok(o) => tracing::warn!("net use output: {}", String::from_utf8_lossy(&o.stdout).trim()),
        Err(e) => tracing::warn!("net use failed: {}", e),
    }
}

#[cfg(not(windows))]
fn mount_smb_share(_cfg: &config::NodeConfig) {}

// ── Core agent logic ──────────────────────────────────────────────────────────

async fn run_agent(cfg: Arc<config::NodeConfig>, mut shutdown: watch::Receiver<bool>) -> Result<()> {
    cfg.validate_minimal()?;

    tracing::info!(
        node_id = %cfg.node_id,
        node_name = %cfg.node_name,
        manager = %cfg.manager_url,
        "ARK Node Agent starting"
    );

    mount_smb_share(&cfg);
    let checks = checks::run_checks(&cfg).await;
    if !checks.tailscale_online {
        tracing::warn!("Manager not reachable at {}. Will retry via heartbeat loop.", cfg.manager_url);
    }
    tracing::info!(
        share_mounted = checks.cluster_share_mounted,
        ark_installed = checks.ark_server_installed,
        mods_valid = checks.mods_valid,
        ram_mb = checks.available_ram_mb,
        "system checks complete"
    );
    if let Some(ref err) = checks.last_error {
        tracing::warn!("system check warnings: {}", err);
    }

    let server_state = ark_server::new_shared_state();

    {
        let cfg = cfg.clone();
        let state = server_state.clone();
        tokio::spawn(async move { heartbeat::run_loop(cfg, state).await });
    }
    {
        let cfg = cfg.clone();
        let state = server_state.clone();
        tokio::spawn(async move { tasks::run_loop(cfg, state).await });
    }

    tracing::info!("Node agent running.");

    // Wait for shutdown signal
    shutdown.changed().await.ok();

    tracing::info!("Shutdown signal received. Stopping...");
    if ark_server::is_running(&server_state).await {
        tracing::info!("Saving and stopping travel server before exit...");
        let _ = ark_server::stop(&cfg, true, server_state).await;
    }

    Ok(())
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn load_config() -> Result<Arc<config::NodeConfig>> {
    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(config::default_config_path);
    tracing::info!("Loading config from {}", config_path);
    let cfg = config::NodeConfig::load(&config_path)
        .with_context(|| format!("Failed to load config: {}", config_path))?;
    Ok(Arc::new(cfg))
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,ark_node_agent=debug"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}

// ── Windows service implementation ────────────────────────────────────────────

#[cfg(windows)]
mod windows_service_impl {
    use super::*;
    use std::ffi::OsString;
    use std::time::Duration;
    use windows_service::{
        define_windows_service,
        service::{
            ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
            ServiceType,
        },
        service_control_handler::{self, ServiceControlHandlerResult},
        service_dispatcher,
    };

    define_windows_service!(ffi_service_main, service_main);

    pub fn run_as_service() -> Result<()> {
        service_dispatcher::start(super::SERVICE_NAME, ffi_service_main)
            .map_err(|e| anyhow::anyhow!("service_dispatcher failed: {}", e))
    }

    fn service_main(_args: Vec<OsString>) {
        if let Err(e) = run_service_inner() {
            tracing::error!("Service error: {}", e);
        }
    }

    fn run_service_inner() -> Result<()> {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let status_handle = service_control_handler::register(
            super::SERVICE_NAME,
            move |ctrl| match ctrl {
                ServiceControl::Stop | ServiceControl::Shutdown => {
                    let _ = shutdown_tx.send(true);
                    ServiceControlHandlerResult::NoError
                }
                ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
                _ => ServiceControlHandlerResult::NotImplemented,
            },
        )
        .map_err(|e| anyhow::anyhow!("register control handler: {}", e))?;

        status_handle
            .set_service_status(ServiceStatus {
                service_type: ServiceType::OWN_PROCESS,
                current_state: ServiceState::Running,
                controls_accepted: ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
                exit_code: ServiceExitCode::Win32(0),
                checkpoint: 0,
                wait_hint: Duration::default(),
                process_id: None,
            })
            .map_err(|e| anyhow::anyhow!("set running status: {}", e))?;

        let cfg = load_config()?;
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(run_agent(cfg, shutdown_rx))?;

        status_handle
            .set_service_status(ServiceStatus {
                service_type: ServiceType::OWN_PROCESS,
                current_state: ServiceState::Stopped,
                controls_accepted: ServiceControlAccept::empty(),
                exit_code: ServiceExitCode::Win32(0),
                checkpoint: 0,
                wait_hint: Duration::default(),
                process_id: None,
            })
            .map_err(|e| anyhow::anyhow!("set stopped status: {}", e))?;

        Ok(())
    }
}

// ── main() ────────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    init_tracing();

    #[cfg(windows)]
    {
        // Try to run as a Windows service first.
        // service_dispatcher::start fails immediately if not invoked by SCM,
        // so we fall through to direct mode in that case.
        match windows_service_impl::run_as_service() {
            Ok(()) => return Ok(()),
            Err(_) => {
                // Not running under SCM — continue as direct process below.
            }
        }
    }

    // Direct / non-Windows mode
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let cfg = load_config()?;

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async move {
        let cfg_clone = cfg.clone();
        let mut rx = shutdown_rx.clone();

        // Spawn agent
        let agent = tokio::spawn(async move {
            run_agent(cfg_clone, rx.clone()).await
        });

        // Wait for Ctrl+C
        tokio::signal::ctrl_c().await.ok();
        tracing::info!("Ctrl+C received.");
        let _ = shutdown_tx.send(true);

        let _ = agent.await;
        Ok::<(), anyhow::Error>(())
    })?;

    Ok(())
}
