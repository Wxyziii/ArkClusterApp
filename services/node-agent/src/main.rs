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
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(config::default_config_path);

    tracing::info!("Loading config from {}", config_path);
    let cfg = config::NodeConfig::load(&config_path)
        .with_context(|| format!("Failed to load config: {}", config_path))?;

    cfg.validate_minimal()?;

    tracing::info!(
        node_id = %cfg.node_id,
        node_name = %cfg.node_name,
        manager = %cfg.manager_url,
        "ARK Node Agent starting"
    );

    // Pre-flight: verify manager reachable
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

    let cfg = Arc::new(cfg);
    let server_state = ark_server::new_shared_state();

    // Spawn heartbeat loop
    {
        let cfg = cfg.clone();
        let state = server_state.clone();
        tokio::spawn(async move {
            heartbeat::run_loop(cfg, state).await;
        });
    }

    // Spawn task polling loop
    {
        let cfg = cfg.clone();
        let state = server_state.clone();
        tokio::spawn(async move {
            tasks::run_loop(cfg, state).await;
        });
    }

    tracing::info!("Node agent running. Press Ctrl+C to stop.");

    // Block until SIGTERM/Ctrl+C
    tokio::signal::ctrl_c().await?;
    tracing::info!("Shutdown signal received. Stopping...");

    // Attempt graceful stop of any running travel server
    if ark_server::is_running(&server_state).await {
        tracing::info!("Saving and stopping travel server before exit...");
        let _ = ark_server::stop(&cfg, true, server_state).await;
    }

    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,ark_node_agent=debug"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}
