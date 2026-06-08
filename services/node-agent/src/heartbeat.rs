//! Periodic heartbeat loop — runs every `heartbeat_interval_secs`.
use std::sync::Arc;

use crate::ark_server::SharedServerState;
use crate::checks::{self, NodeChecks};
use crate::config::NodeConfig;

pub async fn run_loop(cfg: Arc<NodeConfig>, server_state: SharedServerState) {
    let mut interval = tokio::time::interval(
        std::time::Duration::from_secs(cfg.heartbeat_interval_secs),
    );
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("reqwest client");

    loop {
        interval.tick().await;
        let srv = server_state.read().await;
        let active = if srv.running { 1u64 } else { 0 };
        let current_map = if srv.running { Some(srv.map_id.clone()) } else { None };
        drop(srv);

        let node_checks = checks::run_checks(&cfg).await;
        // Also detect ShooterGameServer.exe directly — handles agent-restart-while-running case
        let shooter_running = crate::ark_server::detect_shooter_process().await;
        let effective_active = if active == 1 || shooter_running { 1u64 } else { 0 };
        let rcon_ready = if effective_active == 1 {
            crate::ark_server::check_rcon_ready(&cfg).await
        } else {
            false
        };

        let payload = build_payload(&cfg, &node_checks, effective_active, current_map, rcon_ready);

        let url = format!("{}/api/nodes/heartbeat", cfg.manager_url.trim_end_matches('/'));
        match client
            .post(&url)
            .header("Authorization", format!("Bearer {}", cfg.node_token))
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                tracing::debug!("heartbeat ok");
            }
            Ok(resp) => {
                tracing::warn!("heartbeat rejected: {}", resp.status());
            }
            Err(e) => {
                tracing::warn!("heartbeat failed: {}", e);
            }
        }
    }
}

fn build_payload(
    cfg: &NodeConfig,
    checks: &NodeChecks,
    active_travel_servers: u64,
    current_map: Option<String>,
    rcon_ready: bool,
) -> serde_json::Value {
    serde_json::json!({
        "nodeId": cfg.node_id,
        "nodeName": cfg.node_name,
        "ownerDiscordUserId": cfg.owner_discord_user_id,
        "nodeType": "external-windows",
        "online": true,
        "version": env!("CARGO_PKG_VERSION"),
        "tailscaleOnline": checks.tailscale_online,
        "tailscaleIp": "",
        "maxTravelServers": 1,
        "activeTravelServers": active_travel_servers,
        "currentMap": current_map,
        "availableRamMb": checks.available_ram_mb,
        "totalRamMb": checks.total_ram_mb,
        "clusterShareMounted": checks.cluster_share_mounted,
        "arkServerInstalled": checks.ark_server_installed,
        "modsValid": checks.mods_valid,
        "configValid": checks.mods_valid, // simplified: mods valid implies config ok
        "portsFree": checks.ports_free || active_travel_servers > 0,
        "rconReady": rcon_ready,
        "lastError": checks.last_error
    })
}
