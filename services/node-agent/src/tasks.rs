//! Task polling loop — polls manager, executes tasks, reports results.
use std::sync::Arc;

use crate::ark_server::{self, SharedServerState};
use crate::checks;
use crate::config::NodeConfig;

pub async fn run_loop(cfg: Arc<NodeConfig>, server_state: SharedServerState) {
    let mut interval = tokio::time::interval(
        std::time::Duration::from_secs(cfg.task_poll_interval_secs),
    );
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .expect("reqwest client");

    loop {
        interval.tick().await;
        let url = format!("{}/api/nodes/{}/tasks/poll", cfg.manager_url.trim_end_matches('/'), cfg.node_id);
        let resp = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", cfg.node_token))
            .send()
            .await;

        match resp {
            Ok(r) if r.status().is_success() => {
                if let Ok(body) = r.json::<serde_json::Value>().await {
                    let tasks = body["tasks"].as_array().cloned().unwrap_or_default();
                    for task in tasks {
                        let task_id = task["id"].as_str().unwrap_or("").to_string();
                        let task_type = task["task_type"].as_str().unwrap_or("").to_string();
                        let payload: serde_json::Value = task["payload"]
                            .as_str()
                            .and_then(|s| serde_json::from_str(s).ok())
                            .unwrap_or(serde_json::Value::Object(Default::default()));

                        tracing::info!("executing task {} type={}", task_id, task_type);
                        let (success, result, error) = execute_task(
                            &cfg,
                            &task_type,
                            &payload,
                            server_state.clone(),
                        )
                        .await;

                        report_result(&client, &cfg, &task_id, success, result, error).await;
                    }
                }
            }
            Ok(r) => tracing::debug!("task poll {}", r.status()),
            Err(e) => tracing::debug!("task poll err: {}", e),
        }
    }
}

async fn execute_task(
    cfg: &NodeConfig,
    task_type: &str,
    payload: &serde_json::Value,
    server_state: SharedServerState,
) -> (bool, Option<String>, Option<String>) {
    match task_type {
        "start_travel" => {
            let map_id = payload["mapId"].as_str().unwrap_or("").to_string();
            let ark_map = payload["arkMapName"].as_str().unwrap_or(&map_id).to_string();
            let session_name = payload["sessionName"].as_str().unwrap_or("ARK Travel").to_string();
            let session_id = payload["sessionId"].as_str().unwrap_or("").to_string();
            let cluster_share = payload["clusterSharePath"].as_str().filter(|s| !s.is_empty());

            // Pre-flight checks
            let checks = checks::run_checks(cfg).await;
            if !checks::check_enough_ram(&checks, cfg.max_ram_mb_before_block) {
                return (false, None, Some(format!("Not enough RAM ({} MB available, need {} MB)", checks.available_ram_mb, cfg.max_ram_mb_before_block)));
            }
            if !checks.cluster_share_mounted {
                return (false, None, Some(format!("Cluster share not mounted: {}", cfg.cluster_share_path)));
            }
            if !checks.mods_valid {
                return (false, None, Some("Mods not validated. Run setup.ps1 to install mods.".into()));
            }

            match ark_server::start(cfg, &map_id, &ark_map, &session_name, &session_id, cluster_share, server_state).await {
                Ok(_) => (true, Some(format!("Started {} (map={})", session_name, map_id)), None),
                Err(e) => (false, None, Some(e.to_string())),
            }
        }
        "stop_travel" => {
            let save_first = payload["saveFirst"].as_bool().unwrap_or(true);
            let force = payload["force"].as_bool().unwrap_or(false);
            match ark_server::stop(cfg, save_first && !force, server_state).await {
                Ok(_) => (true, Some("Travel server stopped.".into()), None),
                Err(e) => (false, None, Some(e.to_string())),
            }
        }
        "save_world" => {
            match crate::rcon::save_world("127.0.0.1", cfg.rcon_port, &cfg.server_admin_password).await {
                Ok(_) => (true, Some("World saved.".into()), None),
                Err(e) => (false, None, Some(e.to_string())),
            }
        }
        "validate_mods" => {
            let checks = checks::run_checks(cfg).await;
            if checks.mods_valid {
                (true, Some(format!("{} mods validated.", cfg.mod_ids.len())), None)
            } else {
                (false, None, Some(checks.last_error.unwrap_or("Mods invalid".into())))
            }
        }
        "ping" => (true, Some("pong".into()), None),
        "status_refresh" => {
            let running = ark_server::is_running(&server_state).await;
            (true, Some(format!("running={}", running)), None)
        }
        other => (false, None, Some(format!("unknown task type: {}", other))),
    }
}

async fn report_result(
    client: &reqwest::Client,
    cfg: &NodeConfig,
    task_id: &str,
    success: bool,
    result: Option<String>,
    error: Option<String>,
) {
    let url = format!(
        "{}/api/nodes/{}/tasks/{}/result",
        cfg.manager_url.trim_end_matches('/'),
        cfg.node_id,
        task_id
    );
    let body = serde_json::json!({
        "success": success,
        "result": result,
        "error": error
    });
    match client
        .post(&url)
        .header("Authorization", format!("Bearer {}", cfg.node_token))
        .json(&body)
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => tracing::debug!("task {} result reported", task_id),
        Ok(r) => tracing::warn!("task result rejected {}: {}", task_id, r.status()),
        Err(e) => tracing::warn!("task result send failed {}: {}", task_id, e),
    }
}
