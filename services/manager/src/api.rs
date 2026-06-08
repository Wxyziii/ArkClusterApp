//! HTTP API. Versioned routes under `/api`, all behind Bearer auth (mounted by
//! the caller). Live handlers return host/config/database data only; unknown
//! data is reported as unavailable instead of guessed.
//!
//! Mutating routes are guarded by operation flags, confirmation checks, and
//! audit entries so the API cannot touch the host by accident.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashSet;

use crate::config::{MapConfig, ServerSlot};
use crate::models::audit::{self, AuditEvent, Severity};
use crate::models::backup;
use crate::models::domain::{ArkMap, ConfigField, MapConfigSummary};
use crate::models::governor;
use crate::models::operations::{self, ActionRequest, GuardError, ServerAction, SystemdGuardInput};
use crate::models::resources;
use crate::models::systemd::UnitStatus;
use crate::models::{config_edit, maintenance, mods as mod_ops, nodes as nodes_model, runtime, travel as travel_model, travel_sessions};
use crate::state::AppState;

pub fn admin_router() -> Router<AppState> {
    Router::new()
        .route("/status", get(status))
        .route("/capabilities", get(capabilities))
        .route("/servers", get(servers))
        .route("/servers/{id}", get(server_detail))
        .route("/servers/{id}/actions/start", post(server_start))
        .route("/servers/{id}/actions/stop", post(server_stop))
        .route("/servers/{id}/actions/restart", post(server_restart))
        .route("/servers/{id}/actions/backup", post(server_backup))
        .route("/travel", get(travel))
        .route("/travel/request", post(travel_request))
        .route("/travel/history", get(travel_history))
        .route("/resources", get(resources))
        .route("/runtime", get(runtime_status))
        .route("/backups", get(backups))
        .route("/activity", get(activity))
        .route("/rcon/status", get(rcon_status))
        .route("/players", get(players))
        .route("/chat/recent", get(chat_recent))
        .route("/config", get(config))
        .route("/config/raw", get(config_raw))
        .route("/config/preview", post(config_preview))
        .route("/config/apply", post(config_set))
        .route("/config/rollback", post(config_rollback))
        .route("/config/versions", get(config_versions))
        .route("/config/set", post(config_set))
        .route("/mods", get(mods))
        .route("/mods/lookup", post(mod_lookup))
        .route("/mods/add", post(mod_add))
        .route("/mods/update", post(mod_update))
        .route("/mods/enable", post(mod_enable))
        .route("/mods/disable", post(mod_disable))
        .route("/mods/remove", post(mod_remove))
        .route("/mods/reorder", post(mod_reorder))
        .route("/maintenance/status", get(maintenance_status))
        .route("/maintenance/update/ark", post(maintenance_ark_update))
        .route("/maintenance/ark/update", post(maintenance_ark_update))
        .route("/discord/status", get(discord_status))
        .route("/settings", get(settings))
        .route("/home/set-map", post(home_set_map))
}

fn ram_pct(s: &crate::models::domain::ResourceSample) -> u32 {
    if s.ram_total_gb <= 0.0 {
        0
    } else {
        ((s.ram_used_gb / s.ram_total_gb) * 100.0).round() as u32
    }
}

fn pct(used: f64, total: f64) -> u32 {
    if total <= 0.0 {
        0
    } else {
        ((used / total) * 100.0).round() as u32
    }
}

fn pressure_label(
    ram_pct: u32,
    cfg: &crate::config::ResourcePolicy,
) -> (&'static str, &'static str) {
    if ram_pct >= cfg.ram_emergency_pct as u32 {
        ("Critical", "red")
    } else if ram_pct >= cfg.ram_pressure_pct as u32 {
        ("Resource Pressure", "amber")
    } else if ram_pct >= cfg.ram_warn_pct as u32 {
        ("Warning", "amber")
    } else {
        ("Healthy", "green")
    }
}

fn slot_role_label(key: &str) -> &'static str {
    if key == "home" {
        "Home"
    } else {
        "On-demand"
    }
}

async fn status(State(s): State<AppState>) -> impl IntoResponse {
    let res = resources::sample(&s.config.cluster.directory, s.manager_started_at).await;
    let rp = ram_pct(&res);
    let (label, tone) = pressure_label(rp, &s.config.resource_policy);
    let maps = maps_with_status(&s).await;
    let systemd = systemd_summary(&maps);
    let active_travel_slots = active_travel_slot_count_from_maps(&maps);
    let resource_guard =
        operations::travel_resource_guard_status(&s.config, &res, active_travel_slots, None);
    let running = maps
        .iter()
        .filter(|m| matches!(m.state.as_str(), "Online" | "Ready" | "Starting"))
        .count();
    let player_count_known = maps
        .iter()
        .any(|m| m.configured && m.player_count_source == "rcon");
    let players = player_count_known.then(|| maps.iter().map(|m| m.players).sum::<u32>());

    Json(json!({
        "dataMode": "live",
        "cluster": {
            "name": s.config.cluster.name,
            "id": s.config.cluster.id,
            "directory": s.config.cluster.directory,
            "managerVersion": s.config.cluster.manager_version,
            "maxTravelServers": s.config.resource_policy.max_travel_servers,
            "emptyShutdownMins": s.config.resource_policy.empty_shutdown_mins
        },
        "manager": { "status": "Online", "tone": "green" },
        "tailscale": {
            "status": if s.config.bind_is_private() { "Private bind configured" } else { "Public bind risk" },
            "tone": if s.config.bind_is_private() { "cyan" } else { "red" },
            "bindPrivate": s.config.bind_is_private(),
            "bindAddress": s.config.server.bind_address,
            "source": "manager_config",
            "connected": null
        },
        "discord": {
            "status": if s.config.discord.enabled { "Configured" } else { "Disabled" },
            "tone": if s.config.discord.enabled { "cyan" } else { "gray" },
            "source": "manager_config"
        },
        "systemd": systemd,
        "resourcePressure": {
            "ramPct": rp,
            "label": label,
            "tone": tone,
            "source": res.source,
            "load1": res.load1,
            "load5": res.load5,
            "load15": res.load15
        },
        "resourceGuard": resource_guard,
        "players": players,
        "playerCountSource": if player_count_known { "rcon" } else { "unavailable" },
        "runningMaps": running
    }))
}

async fn capabilities(State(s): State<AppState>) -> impl IntoResponse {
    let source = if cfg!(target_os = "linux") {
        "host"
    } else {
        "unavailable"
    };
    Json(operations::capabilities(&s.config, source))
}

async fn servers(State(s): State<AppState>) -> impl IntoResponse {
    Json(maps_with_status(&s).await)
}

async fn server_detail(State(s): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    match maps_with_status(&s).await.into_iter().find(|m| m.id == id) {
        Some(map) => {
            let player_count_source = map.player_count_source.clone();
            let map_name = map.name.clone();
            let players = s
                .rcon_runtime
                .read()
                .await
                .players_response(&s.config)
                .players;
            let map_players = players
                .into_iter()
                .filter(|p| p.map == map_name)
                .collect::<Vec<_>>();
            let available = player_count_source == "rcon";
            Json(json!({
                "server": map,
                "players": map_players,
                "playerCountSource": player_count_source,
                "available": available,
                "reason": if available { "RCON player polling live" } else { "RCON player polling unavailable" }
            }))
            .into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(api_error_value(
                "NOT_FOUND",
                format!("no server with id '{id}'"),
                json!({}),
            )),
        )
            .into_response(),
    }
}

async fn server_start(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<ActionRequest>,
) -> impl IntoResponse {
    server_systemd_action(s, id, ServerAction::Start, req).await
}

async fn server_stop(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<ActionRequest>,
) -> impl IntoResponse {
    server_systemd_action(s, id, ServerAction::Stop, req).await
}

async fn server_restart(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<ActionRequest>,
) -> impl IntoResponse {
    server_systemd_action(s, id, ServerAction::Restart, req).await
}

async fn server_backup(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<ActionRequest>,
) -> impl IntoResponse {
    let Some((slot, slot_key)) = configured_slot_for_request(&s.config, &id) else {
        return api_error(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "server/slot not found",
            json!({}),
        );
    };
    if let Err(err) = operations::guard_backup(&s.config, &req) {
        return guard_error_response(err);
    }

    let audit_id = audit::record_with_id(
        &s.pool,
        &AuditEvent::new(Severity::Info, "Backup", "manual backup requested")
            .target(&slot.map_key)
            .detail(format!("slot={} reason={}", slot.id, req.reason)),
    )
    .await;
    match backup::run_slot_backup(&s.pool, &s.config, slot_key, slot, &req.reason).await {
        Ok(record) => {
            audit::record(
                &s.pool,
                &AuditEvent::new(Severity::Success, "Backup", "backup completed")
                    .target(&slot.map_key)
                    .detail(format!(
                        "backup_id={} size_bytes={}",
                        record.id, record.size_bytes
                    )),
            )
            .await;
            Json(json!({
                "accepted": true,
                "actionId": record.id,
                "serverId": id,
                "operation": "backup",
                "result": "success",
                "message": "Backup completed.",
                "auditEventId": audit_id,
                "backup": record
            }))
            .into_response()
        }
        Err(err) => {
            audit::record(
                &s.pool,
                &AuditEvent::new(Severity::Error, "Backup", "backup failed")
                    .target(&slot.map_key)
                    .detail(format!("slot={} error={}", slot.id, err)),
            )
            .await;
            api_error(
                status_for_error_code(err.code()),
                err.code(),
                err.to_string(),
                json!({ "auditEventId": audit_id }),
            )
        }
    }
}

async fn travel(State(s): State<AppState>) -> impl IntoResponse {
    let maps = maps_with_status(&s).await;
    let destinations = maps
        .iter()
        .filter(|map| map.launch_ready && !map.is_home)
        .cloned()
        .collect::<Vec<_>>();
    let statuses = slot_statuses(&s).await;
    let res = resources::sample(&s.config.cluster.directory, s.manager_started_at).await;
    let active_travel_slots = active_travel_slot_count_from_statuses(&statuses);
    let resource_guard =
        operations::travel_resource_guard_status(&s.config, &res, active_travel_slots, None);
    let slots = statuses
        .iter()
        .map(|snapshot| {
            let current_map_id = travel_model::effective_slot_map_id(&s.config, &snapshot.slot);
            let current_map =
                maps.iter()
                    .find(|m| m.id == current_map_id)
                    .cloned()
                    .map(|mut map| {
                        if !snapshot.status.active {
                            map.state = "Offline".into();
                            map.players = 0;
                            map.player_count_source = "stopped".into();
                            map.rcon = "Disconnected".into();
                            map.systemd = snapshot.status.state.clone();
                            map.connection_available = false;
                            map.connection_address.clear();
                            map.query_address.clear();
                            map.connection_source = "systemd".into();
                            map.connection_unavailable_reason = "map not running".into();
                        }
                        map
                    });
            let player_count_source = if snapshot.status.active {
                current_map
                    .as_ref()
                    .map(|m| m.player_count_source.as_str())
                    .unwrap_or("unavailable")
            } else {
                "stopped"
            };
            json!({
                "slotId": snapshot.slot.id,
                "role": slot_role_label(snapshot.key),
                "mapKey": current_map_id,
                "map": current_map,
                "unit": snapshot.slot.systemd_unit,
                "systemd": snapshot.status.state,
                "active": snapshot.status.active,
                "playerCount": snapshot.player_count,
                "playerCountSource": player_count_source,
                "idleShutdownSecs": s.config.operations.travel_idle_shutdown_secs,
                "policy": if snapshot.slot.protected { "protected" } else { "on_demand" }
            })
        })
        .collect::<Vec<_>>();
    let history = travel_model::history(&s.pool).await.unwrap_or_default();
    Json(json!({
        "slots": slots,
        "maxTravelServers": s.config.resource_policy.max_travel_servers,
        "activeRequest": null,
        "stepper": null,
        "blockReason": null,
        "queue": [],
        "enabled": s.config.operations.travel_scheduler_enabled,
        "idleShutdownSecs": s.config.operations.travel_idle_shutdown_secs,
        "idleShutdownProduction": s.config.operations.travel_idle_shutdown_secs == 10800,
        "homeResourceStandby": s.config.resource_policy.home_standby_enabled,
        "resourceGuard": resource_guard,
        "destinations": destinations,
        "recent": history
    }))
}

async fn travel_request(
    State(s): State<AppState>,
    Json(req): Json<travel_model::TravelRequestBody>,
) -> impl IntoResponse {
    let actor = req.actor.clone();
    let discord_id = req.actor_discord_id.clone().unwrap_or_default();
    let map_input = req.map.clone();
    let statuses = slot_statuses(&s).await;
    match travel_model::request_with_start(
        &s.pool,
        &s.config,
        s.systemd.clone(),
        s.manager_started_at,
        req,
        statuses,
    )
    .await
    {
        Ok(decision) if decision.accepted => {
            audit::record(
                &s.pool,
                &AuditEvent::new(Severity::Info, "Travel", "travel request accepted")
                    .target(decision.resolved_map.as_deref().unwrap_or(""))
                    .detail(format!(
                        "id={} slot={:?} reason={}",
                        decision.id, decision.chosen_slot, decision.reason
                    )),
            )
            .await;
            Json(decision).into_response()
        }
        // Local travel unavailable — try external node
        Ok(_decision) => {
            if let Some(resp) = crate::api_nodes::try_external_travel(&s, &map_input, &actor, &discord_id).await {
                return resp;
            }
            audit::record(
                &s.pool,
                &AuditEvent::new(Severity::Warn, "Travel", "travel request blocked (no node available)")
                    .detail(format!("map={}", map_input)),
            )
            .await;
            (StatusCode::CONFLICT, Json(serde_json::json!({
                "accepted": false,
                "status": "blocked",
                "reason": "No local slot available and no external node is online/free for this map.",
                "requestedMap": map_input
            }))).into_response()
        }
        Err(err) => api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "OPERATION_FAILED",
            err.to_string(),
            json!({}),
        ),
    }
}

async fn travel_history(State(s): State<AppState>) -> impl IntoResponse {
    Json(json!({ "history": travel_model::history(&s.pool).await.unwrap_or_default() }))
}

async fn resources(State(s): State<AppState>) -> impl IntoResponse {
    let res = resources::sample(&s.config.cluster.directory, s.manager_started_at).await;
    let rp = ram_pct(&res);
    let (label, tone) = pressure_label(rp, &s.config.resource_policy);
    let p = &s.config.resource_policy;

    let maps = maps_with_status(&s).await;
    let active_travel_slots = active_travel_slot_count_from_maps(&maps);
    let resource_guard =
        operations::travel_resource_guard_status(&s.config, &res, active_travel_slots, None);
    let player_counts_available = maps
        .iter()
        .filter(|m| m.configured && matches!(m.state.as_str(), "Online" | "Ready" | "Starting"))
        .all(|m| m.player_count_source == "rcon");
    let decision = if player_counts_available {
        let home_players: u32 = maps.iter().filter(|m| m.is_home).map(|m| m.players).sum();
        let travel_players: u32 = maps.iter().filter(|m| !m.is_home).map(|m| m.players).sum();
        json!(governor::evaluate(p, &res, home_players, travel_players))
    } else {
        json!({
            "decision": "unavailable",
            "why": "player counts are unavailable because RCON polling is not connected",
            "examples": [],
            "policy": {
                "neverStopWithPlayers": p.never_stop_with_players,
                "homeStandbyEnabled": p.home_standby_enabled,
                "homeStopsOnlyWhenEmpty": p.home_stops_only_when_empty,
                "preferActivePlayerMaps": p.prefer_active_player_maps,
                "autoRestartHome": p.auto_restart_home,
                "maxTravelServers": p.max_travel_servers,
                "emptyShutdownMins": p.empty_shutdown_mins
            }
        })
    };

    Json(json!({
        "sample": res,
        "derived": {
            "ramPct": rp,
            "cpuPct": res.cpu_pct,
            "swapPct": pct(res.swap_used_gb, res.swap_total_gb),
            "diskPct": pct(res.disk_used_gb, res.disk_total_gb),
            "pressure": { "label": label, "tone": tone }
        },
        "thresholds": {
            "ramWarnPct": p.ram_warn_pct,
            "ramPressurePct": p.ram_pressure_pct,
            "ramEmergencyPct": p.ram_emergency_pct,
            "maxTravel": p.max_travel_servers,
            "emptyShutdownMins": p.empty_shutdown_mins
        },
        "governor": decision,
        "resourceGuard": resource_guard,
        "source": res.source,
        "uptime": {
            "managerSecs": res.manager_uptime_secs,
            "systemSecs": res.system_uptime_secs
        },
        "loadAverage": {
            "one": res.load1,
            "five": res.load5,
            "fifteen": res.load15
        },
        "perProcess": maps.iter().filter(|m| m.ram_mb > 0).map(|m| json!({
            "map": m.name, "ramMb": m.ram_mb, "cpuPct": m.cpu_pct
        })).collect::<Vec<_>>()
    }))
}

async fn runtime_status(State(s): State<AppState>) -> impl IntoResponse {
    Json(runtime::status(&s.config).await)
}

async fn backups(State(s): State<AppState>) -> impl IntoResponse {
    let p = &s.config.backup_policy;
    let records = backup::list(&s.pool).await.unwrap_or_default();
    let backups = records
        .into_iter()
        .map(|b| {
            json!({
                "id": b.id,
                "map": b.map_key,
                "slotId": b.slot_id,
                "serverId": b.server_id,
                "type": b.kind,
                "sizeMb": ((b.size_bytes as f64 / 1024.0 / 1024.0).round() as u64),
                "sizeBytes": b.size_bytes,
                "created": b.created_at,
                "createdAt": b.created_at,
                "completedAt": b.completed_at,
                "reason": b.reason,
                "status": b.status,
                "path": b.path,
                "source": "sqlite",
                "error": b.error
            })
        })
        .collect::<Vec<_>>();
    Json(json!({
        "backups": backups,
        "policy": {
            "beforeShutdown": p.before_shutdown,
            "beforeConfigSave": p.before_config_save,
            "beforeModChange": p.before_mod_change,
            "retention": p.retention,
            "enabled": s.config.operations.backup_enabled
        }
    }))
}

async fn activity(State(s): State<AppState>) -> impl IntoResponse {
    let real = real_activity(&s.pool).await.unwrap_or_default();
    if !real.is_empty() {
        let recent = real.iter().take(5).cloned().collect::<Vec<_>>();
        return Json(json!({ "activity": real, "recent": recent, "source": "sqlite" }));
    }
    Json(json!({ "activity": [], "recent": [], "source": "sqlite", "empty": true }))
}

async fn rcon_status(State(s): State<AppState>) -> impl IntoResponse {
    Json(s.rcon_runtime.read().await.status_from_config(&s.config))
}

async fn players(State(s): State<AppState>) -> impl IntoResponse {
    Json(s.rcon_runtime.read().await.players_response(&s.config))
}

async fn chat_recent(State(s): State<AppState>) -> impl IntoResponse {
    Json(s.rcon_runtime.read().await.chat_response(&s.config))
}

async fn config(State(s): State<AppState>) -> impl IntoResponse {
    let shared = config_edit::read_shared(&s.config);
    let fields = config_fields_from_shared(&shared);
    Json(json!({
        "fields": fields,
        "gameIni": shared.game_ini,
        "gameUserSettingsIni": shared.game_user_settings_ini,
        "shared": shared,
        "restartRequired": true,
        "writable": s.config.operations.config_writes_enabled
    }))
}

async fn config_raw(State(s): State<AppState>) -> impl IntoResponse {
    let shared = config_edit::read_shared(&s.config);
    Json(json!({
        "gameIni": shared.game_ini,
        "gameUserSettingsIni": shared.game_user_settings_ini,
        "shared": shared,
        "masked": true
    }))
}

fn config_fields_from_shared(shared: &config_edit::SharedConfig) -> Vec<ConfigField> {
    let mut fields = Vec::new();
    fields.extend(parse_config_fields("Game.ini", &shared.game_ini));
    fields.extend(parse_config_fields(
        "GameUserSettings.ini",
        &shared.game_user_settings_ini,
    ));
    fields
}

fn parse_config_fields(file: &str, raw: &str) -> Vec<ConfigField> {
    let mut section = "General".to_string();
    let mut fields = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            section = trimmed.trim_matches(&['[', ']'][..]).to_string();
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() || key.to_ascii_lowercase().contains("password") {
            continue;
        }
        let value = value.trim();
        let (kind, parsed) = parse_config_value(value);
        fields.push(ConfigField {
            key: key.to_string(),
            label: key.to_string(),
            value: parsed,
            kind,
            group: format!("{file} / {section}"),
            min: None,
            max: None,
            step: None,
            options: None,
            hint: format!("Read from {file}; restart may be required after changes."),
            restart_required: true,
        });
    }
    fields
}

fn parse_config_value(value: &str) -> (String, serde_json::Value) {
    if value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("false") {
        ("bool".into(), json!(value.eq_ignore_ascii_case("true")))
    } else if let Ok(v) = value.parse::<f64>() {
        ("number".into(), json!(v))
    } else {
        ("string".into(), json!(value))
    }
}

async fn config_preview(
    State(s): State<AppState>,
    Json(req): Json<config_edit::ConfigSetRequest>,
) -> impl IntoResponse {
    if !s.config.operations.config_writes_enabled {
        return api_error(
            StatusCode::FORBIDDEN,
            "CONFIG_WRITES_DISABLED",
            "config writes disabled in manager config",
            json!({}),
        );
    }
    Json(json!({
        "accepted": true,
        "preview": {
            "file": req.file,
            "key": req.key,
            "value": req.value,
            "restartRequired": true,
            "backupFirst": true,
            "masked": req.key.to_ascii_lowercase().contains("password")
        }
    }))
    .into_response()
}

async fn config_set(
    State(s): State<AppState>,
    Json(req): Json<config_edit::ConfigSetRequest>,
) -> impl IntoResponse {
    match config_edit::set_value(&s.pool, &s.config, req).await {
        Ok(shared) => Json(json!({ "accepted": true, "shared": shared })).into_response(),
        Err(err) => api_error(
            status_for_error_code(err.code()),
            err.code(),
            err.to_string(),
            json!({}),
        ),
    }
}

async fn config_rollback(State(s): State<AppState>) -> impl IntoResponse {
    Json(json!({
        "accepted": false,
        "reason": "rollback requires selecting a config snapshot; direct rollback endpoint is not enabled",
        "writable": s.config.operations.config_writes_enabled
    }))
}

async fn config_versions(State(s): State<AppState>) -> impl IntoResponse {
    let rows = sqlx::query_as::<_, ConfigVersionRow>(
        "SELECT id, ts, actor, file, reason, backup_path, status FROM config_snapshots ORDER BY ts DESC LIMIT 50",
    )
    .fetch_all(&s.pool)
    .await
    .unwrap_or_default();
    Json(json!({ "versions": rows }))
}

async fn mods(State(s): State<AppState>) -> impl IntoResponse {
    match mod_ops::list(&s.pool, &s.config).await {
        Ok(value) => Json(value).into_response(),
        Err(err) => api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "OPERATION_FAILED",
            err.to_string(),
            json!({}),
        ),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModLookupRequest {
    workshop_id: Option<String>,
    url: Option<String>,
}

async fn mod_lookup(
    State(s): State<AppState>,
    Json(req): Json<ModLookupRequest>,
) -> impl IntoResponse {
    let raw = req.workshop_id.or(req.url).unwrap_or_default();
    let id = raw
        .split("id=")
        .nth(1)
        .unwrap_or(&raw)
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>();
    if id.is_empty() {
        return api_error(
            StatusCode::BAD_REQUEST,
            "INVALID_WORKSHOP_ID",
            "provide a Steam Workshop URL or numeric id",
            json!({}),
        );
    }
    Json(json!({
        "workshopId": id,
        "name": null,
        "url": format!("https://steamcommunity.com/sharedfiles/filedetails/?id={}", id),
        "game": "ARK: Survival Evolved",
        "installAvailable": s.config.operations.mod_management_enabled,
        "mutable": s.config.operations.mod_management_enabled,
        "metadataSource": "unavailable",
        "metadataAvailable": false,
        "reason": "Steam Workshop metadata lookup is not configured in the manager",
        "disabledReason": if s.config.operations.mod_management_enabled { "" } else { "mod management disabled in manager config" }
    }))
    .into_response()
}

async fn mod_add(
    State(s): State<AppState>,
    Json(req): Json<mod_ops::ModMutation>,
) -> impl IntoResponse {
    mod_mutation(s, req, "add").await
}

async fn mod_update(
    State(s): State<AppState>,
    Json(req): Json<mod_ops::ModMutation>,
) -> impl IntoResponse {
    mod_mutation(s, req, "update").await
}

async fn mod_enable(
    State(s): State<AppState>,
    Json(req): Json<mod_ops::ModMutation>,
) -> impl IntoResponse {
    mod_mutation(s, req, "enable").await
}

async fn mod_disable(
    State(s): State<AppState>,
    Json(req): Json<mod_ops::ModMutation>,
) -> impl IntoResponse {
    mod_mutation(s, req, "disable").await
}

async fn mod_remove(
    State(s): State<AppState>,
    Json(req): Json<mod_ops::ModMutation>,
) -> impl IntoResponse {
    mod_mutation(s, req, "remove").await
}

async fn mod_reorder(State(s): State<AppState>) -> impl IntoResponse {
    api_error(
        StatusCode::FORBIDDEN,
        "MOD_REORDER_DISABLED",
        if s.config.operations.mod_management_enabled {
            "mod reorder is not implemented yet"
        } else {
            "mod management disabled in manager config"
        },
        json!({}),
    )
}

async fn mod_mutation(
    s: AppState,
    req: mod_ops::ModMutation,
    action: &'static str,
) -> axum::response::Response {
    match mod_ops::record_known(&s.pool, &s.config, req, action).await {
        Ok(value) => {
            Json(json!({ "accepted": true, "action": action, "state": value })).into_response()
        }
        Err(err) => api_error(
            status_for_error_code(err.code()),
            err.code(),
            err.to_string(),
            json!({}),
        ),
    }
}

async fn maintenance_status(State(s): State<AppState>) -> impl IntoResponse {
    match maintenance::status(&s.pool, &s.config).await {
        Ok(value) => Json(value).into_response(),
        Err(err) => api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "OPERATION_FAILED",
            err.to_string(),
            json!({}),
        ),
    }
}

async fn maintenance_ark_update(
    State(s): State<AppState>,
    Json(req): Json<maintenance::MaintenanceRequest>,
) -> impl IntoResponse {
    match maintenance::ark_update(&s.pool, &s.config, req).await {
        Ok(value) => Json(value).into_response(),
        Err(err) => api_error(
            status_for_error_code(err.code()),
            err.code(),
            err.to_string(),
            json!({}),
        ),
    }
}

async fn discord_status(State(s): State<AppState>) -> impl IntoResponse {
    let service = read_unit_summary("ark-cluster-discord-bot.service").await;
    Json(json!({
        "status": {
            "online": service.active,
            "guild": s.config.discord.guild,
            "statusChannel": s.config.discord.status_channel,
            "service": service,
            "lastHeartbeat": if service.active { "service active" } else { "unknown" },
            "permissionsOk": null,
            "implemented": service.active,
            "source": "systemd",
            "dashboard": {
                "category": "ARK Cluster",
                "channels": ["ark-status", "ark-travel", "ark-players", "ark-logs", "ark-admin"],
                "stateFile": "/var/lib/ark-cluster-discord-bot/state.json"
            }
        },
        "commands": [],
        "events": [],
        "alertSettings": [],
        "commandsSource": "unavailable",
        "eventsSource": "unavailable"
    }))
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct UnitSummary {
    active: bool,
    enabled: bool,
    active_state: String,
    sub_state: String,
}

async fn read_unit_summary(_unit: &str) -> UnitSummary {
    #[cfg(target_os = "linux")]
    {
        let output = tokio::process::Command::new("systemctl")
            .args([
                "show",
                _unit,
                "-p",
                "ActiveState",
                "-p",
                "SubState",
                "-p",
                "UnitFileState",
                "--no-pager",
            ])
            .output()
            .await;
        if let Ok(out) = output {
            let text = String::from_utf8_lossy(&out.stdout);
            let mut active_state = "unknown".to_string();
            let mut sub_state = "unknown".to_string();
            let mut unit_file_state = "unknown".to_string();
            for line in text.lines() {
                if let Some(v) = line.strip_prefix("ActiveState=") {
                    active_state = v.into();
                } else if let Some(v) = line.strip_prefix("SubState=") {
                    sub_state = v.into();
                } else if let Some(v) = line.strip_prefix("UnitFileState=") {
                    unit_file_state = v.into();
                }
            }
            return UnitSummary {
                active: active_state == "active",
                enabled: unit_file_state == "enabled",
                active_state,
                sub_state,
            };
        }
    }
    UnitSummary {
        active: false,
        enabled: false,
        active_state: "unavailable".into(),
        sub_state: "unknown".into(),
    }
}

async fn settings(State(s): State<AppState>) -> impl IntoResponse {
    let p = &s.config.resource_policy;
    let g = &s.config.resource_guard;
    let b = &s.config.backup_policy;
    let maps = maps_with_status(&s).await;
    Json(json!({
        "cluster": {
            "name": s.config.cluster.name,
            "id": s.config.cluster.id,
            "directory": s.config.cluster.directory,
            "managerVersion": s.config.cluster.manager_version
        },
        "privateAccess": {
            "bindAddress": s.config.server.bind_address,
            "port": s.config.server.port,
            "bindPrivate": s.config.bind_is_private(),
            "note": "Private/Tailscale/LAN access only. Do not expose this dashboard publicly."
        },
        "travelPolicy": {
            "maxTravelServers": p.max_travel_servers,
            "slots": s.config.travel_slots.iter().map(|sl| json!({ "name": sl.name, "role": sl.role })).collect::<Vec<_>>()
        },
        "resourcePolicy": {
            "ramWarnPct": p.ram_warn_pct,
            "ramPressurePct": p.ram_pressure_pct,
            "ramEmergencyPct": p.ram_emergency_pct,
            "emptyShutdownMins": p.empty_shutdown_mins,
            "homeStandbyEnabled": p.home_standby_enabled,
            "neverStopWithPlayers": p.never_stop_with_players,
            "homeStopsOnlyWhenEmpty": p.home_stops_only_when_empty,
            "preferActivePlayerMaps": p.prefer_active_player_maps,
            "autoRestartHome": p.auto_restart_home
        },
        "resourceGuard": {
            "enabled": g.enabled,
            "blockOnUnknownResources": g.block_on_unknown_resources,
            "minAvailableRamMbForFirstTravel": g.min_available_ram_mb_for_first_travel,
            "minAvailableRamMbForSecondTravel": g.min_available_ram_mb_for_second_travel,
            "maxRamUsedPercentBeforeTravel": g.max_ram_used_percent_before_travel,
            "swapUsedPercentWarn": g.swap_used_percent_warn,
            "swapUsedPercentHard": g.swap_used_percent_hard,
            "minFreeSwapMb": g.min_free_swap_mb,
            "activeSwapIoBlockThresholdPages": g.active_swap_io_block_threshold_pages,
            "minDiskFreeGb": g.min_disk_free_gb
        },
        "backupPolicy": {
            "beforeShutdown": b.before_shutdown,
            "beforeConfigSave": b.before_config_save,
            "beforeModChange": b.before_mod_change,
            "retention": b.retention
        },
        "configModPolicy": {
            "configWritable": s.config.operations.config_writes_enabled,
            "modsMutable": s.config.operations.mod_management_enabled,
            "maintenanceEnabled": s.config.operations.maintenance_enabled,
            "note": "Secrets are masked. Mutations follow manager operation flags and confirmation checks."
        },
        "security": {
            "authScheme": "Bearer token",
            "tokenSource": "config or ARK_MANAGER_API_TOKEN env",
            "tokenMasked": "••••••••",
            "note": "Token is never logged or returned by the API."
        },
        "rcon": s.rcon_runtime.read().await.status_from_config(&s.config),
        "systemdUnits": maps.iter().map(|m| json!({
            "map": m.name,
            "unit": m.config.systemd_unit,
            "state": m.systemd,
            "detail": m.systemd_detail,
            "controlImplemented": s.config.operations.systemd_control_enabled
        })).collect::<Vec<_>>()
    }))
}

async fn server_systemd_action(
    s: AppState,
    id: String,
    action: ServerAction,
    req: ActionRequest,
) -> axum::response::Response {
    let Some((slot, slot_key)) = configured_slot_for_request(&s.config, &id) else {
        return api_error(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "server/slot not found",
            json!({}),
        );
    };
    let maps = maps_with_status(&s).await;
    let current_map = maps
        .iter()
        .find(|m| m.id == slot.map_key || m.config.systemd_unit == slot.systemd_unit);
    let player_count = current_map
        .map(|m| {
            if m.player_count_source == "rcon" {
                m.players
            } else if matches!(m.state.as_str(), "Online" | "Ready" | "Starting") {
                1
            } else {
                0
            }
        })
        .unwrap_or(0);
    let active_travel_slots = maps
        .iter()
        .filter(|m| !m.is_home && matches!(m.state.as_str(), "Online" | "Ready" | "Starting"))
        .count();
    let sample = resources::sample(&s.config.cluster.directory, s.manager_started_at).await;

    if let Err(err) = operations::guard_systemd_action(SystemdGuardInput {
        config: &s.config,
        slot_key,
        slot,
        action,
        req: &req,
        sample: &sample,
        active_travel_slots,
        player_count,
        map_ark_name: None,
    }) {
        audit::record(
            &s.pool,
            &AuditEvent::new(Severity::Warn, "Systemd", "systemd action blocked")
                .target(&slot.map_key)
                .detail(format!(
                    "slot={} action={} code={} reason={}",
                    slot.id,
                    action.as_str(),
                    err.code(),
                    req.reason
                )),
        )
        .await;
        return guard_error_response(err);
    }

    let action_id = format!("op-{}-{}-{}", slot.id, action.as_str(), epoch_secs());
    let severity = if slot_key == "home" && action == ServerAction::Stop {
        Severity::Warn
    } else {
        Severity::Info
    };
    let audit_id = audit::record_with_id(
        &s.pool,
        &AuditEvent::new(severity, "Systemd", "systemd action requested")
            .target(&slot.map_key)
            .detail(format!(
                "action_id={} slot={} action={} unit={} reason={}",
                action_id,
                slot.id,
                action.as_str(),
                slot.systemd_unit,
                req.reason
            )),
    )
    .await;

    let result = match action {
        ServerAction::Start => s.systemd.start_unit(&slot.systemd_unit).await,
        ServerAction::Stop => {
            let r = s.systemd.stop_unit(&slot.systemd_unit).await;
            if r.is_ok() {
                // ARK may core-dump on controlled stop; reset-failed so the unit
                // can start cleanly without manual intervention.
                let _ = s.systemd.reset_failed_unit(&slot.systemd_unit).await;
            }
            r
        }
        ServerAction::Restart => s.systemd.restart_unit(&slot.systemd_unit).await,
        ServerAction::Backup => unreachable!("backup handled separately"),
    };

    match result {
        Ok(()) => {
            let status = s
                .systemd
                .get_status(&slot.systemd_unit)
                .await
                .unwrap_or_else(|e| {
                    UnitStatus::unavailable(&slot.systemd_unit, "systemd", e.to_string())
                });
            let _ = write_systemd_action(
                &s.pool,
                SystemdActionInsert {
                    id: &action_id,
                    server_id: &id,
                    slot_id: &slot.id,
                    unit: &slot.systemd_unit,
                    operation: action.as_str(),
                    reason: &req.reason,
                    result: "success",
                    message: "systemd action completed",
                },
            )
            .await;
            audit::record(
                &s.pool,
                &AuditEvent::new(Severity::Success, "Systemd", "systemd action completed")
                    .target(&slot.map_key)
                    .detail(format!("action_id={action_id}")),
            )
            .await;
            Json(json!({
                "accepted": true,
                "actionId": action_id,
                "serverId": id,
                "operation": action.as_str(),
                "result": "success",
                "message": "Systemd action completed.",
                "auditEventId": audit_id,
                "updatedStatus": status
            }))
            .into_response()
        }
        Err(err) => {
            let message = err.to_string();
            let _ = write_systemd_action(
                &s.pool,
                SystemdActionInsert {
                    id: &action_id,
                    server_id: &id,
                    slot_id: &slot.id,
                    unit: &slot.systemd_unit,
                    operation: action.as_str(),
                    reason: &req.reason,
                    result: "failed",
                    message: &message,
                },
            )
            .await;
            audit::record(
                &s.pool,
                &AuditEvent::new(Severity::Error, "Systemd", "systemd action failed")
                    .target(&slot.map_key)
                    .detail(format!("action_id={action_id} error={message}")),
            )
            .await;
            api_error(
                StatusCode::BAD_GATEWAY,
                "OPERATION_FAILED",
                message,
                json!({ "actionId": action_id, "auditEventId": audit_id }),
            )
        }
    }
}

fn configured_slot_for_request<'a>(
    config: &'a crate::config::Config,
    id: &str,
) -> Option<(&'a ServerSlot, &'static str)> {
    let slots = config.slots.as_ref()?;
    let normalized = id.replace('-', "_");
    // First: match by slot identity (id, key) — most specific.
    let by_identity = slots.iter().into_iter().find(|(slot, key, _)| {
        id == slot.id || normalized == *key || id == key.replace('_', "-")
    });
    if let Some((slot, key, _)) = by_identity {
        return Some((slot, key));
    }
    // Second: match by effective running map id (follows runtime overrides).
    // This lets users type the map name that is *currently* running in a slot,
    // even when the slot's static map_key differs (e.g. travel-a running Aberration
    // while its static map_key is "ragnarok").
    let by_effective = slots.iter().into_iter().find(|(slot, _, _)| {
        let effective = travel_model::effective_slot_map_id(config, slot);
        effective == id || effective.replace('-', "_") == normalized
    });
    if let Some((slot, key, _)) = by_effective {
        return Some((slot, key));
    }
    // Third: fall back to static map_key (lowest priority — avoids hitting
    // an inactive slot whose map_key matches a map running elsewhere).
    slots
        .iter()
        .into_iter()
        .find(|(slot, _, _)| id == slot.map_key)
        .map(|(slot, key, _)| (slot, key))
}

fn guard_error_response(err: GuardError) -> axum::response::Response {
    api_error(
        status_for_error_code(err.code()),
        err.code(),
        err.message(),
        json!({}),
    )
}

fn status_for_error_code(code: &str) -> StatusCode {
    match code {
        "NOT_FOUND" => StatusCode::NOT_FOUND,
        "INVALID_CONFIRMATION" | "HOME_PROTECTED" | "PLAYERS_ONLINE" | "PATH_NOT_ALLOWED" => {
            StatusCode::BAD_REQUEST
        }
        "CONTROL_DISABLED"
        | "BACKUP_DISABLED"
        | "RCON_DISABLED"
        | "CONFIG_WRITES_DISABLED"
        | "MOD_MANAGEMENT_DISABLED"
        | "MAINTENANCE_DISABLED" => StatusCode::FORBIDDEN,
        "INVALID_CONFIG_FILE" | "INVALID_CONFIG_KEY" | "INVALID_WORKSHOP_ID" => {
            StatusCode::BAD_REQUEST
        }
        "RESOURCE_POLICY_BLOCKED" => StatusCode::CONFLICT,
        "BACKUP_PATH_MISSING" | "UNIT_NOT_CONFIGURED" => StatusCode::SERVICE_UNAVAILABLE,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn api_error(
    status: StatusCode,
    code: &str,
    message: impl Into<String>,
    details: serde_json::Value,
) -> axum::response::Response {
    (status, Json(api_error_value(code, message, details))).into_response()
}

fn api_error_value(
    code: &str,
    message: impl Into<String>,
    details: serde_json::Value,
) -> serde_json::Value {
    json!({ "error": { "code": code, "message": message.into(), "details": details } })
}

struct SystemdActionInsert<'a> {
    id: &'a str,
    server_id: &'a str,
    slot_id: &'a str,
    unit: &'a str,
    operation: &'a str,
    reason: &'a str,
    result: &'a str,
    message: &'a str,
}

async fn write_systemd_action(
    pool: &sqlx::SqlitePool,
    row: SystemdActionInsert<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO systemd_actions (id, server_id, slot_id, unit, operation, reason, result, message) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
    )
    .bind(row.id)
    .bind(row.server_id)
    .bind(row.slot_id)
    .bind(row.unit)
    .bind(row.operation)
    .bind(row.reason)
    .bind(row.result)
    .bind(row.message)
    .execute(pool)
    .await?;
    Ok(())
}

async fn real_activity(pool: &sqlx::SqlitePool) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    let rows = sqlx::query_as::<_, ActivityRow>(
        "SELECT id, ts, severity, source, actor, target_map, message, detail \
         FROM activity_log ORDER BY ts DESC, id DESC LIMIT 100",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            json!({
                "id": format!("db-{}", r.id),
                "ts": r.ts,
                "severity": r.severity,
                "source": r.source,
                "actor": r.actor,
                "targetMap": r.target_map,
                "message": r.message,
                "detail": r.detail,
                "dataSource": "sqlite"
            })
        })
        .collect())
}

async fn slot_statuses(s: &AppState) -> Vec<travel_model::SlotStatusSnapshot> {
    let mut out = Vec::new();
    if let Some(slots) = &s.config.slots {
        for (slot, key, _) in slots.iter() {
            let status = s
                .systemd
                .get_status(&slot.systemd_unit)
                .await
                .unwrap_or_else(|e| {
                    UnitStatus::unavailable(&slot.systemd_unit, "systemd", e.to_string())
                });
            let player_count = if status.active {
                s.rcon_runtime.read().await.player_count(&slot.id)
            } else {
                Some(0)
            };
            out.push(travel_model::SlotStatusSnapshot {
                slot: slot.clone(),
                key,
                status,
                player_count,
            });
        }
    }
    out
}

fn active_travel_slot_count_from_statuses(statuses: &[travel_model::SlotStatusSnapshot]) -> usize {
    statuses
        .iter()
        .filter(|snapshot| snapshot.key != "home" && snapshot.status.active)
        .count()
}

fn active_travel_slot_count_from_maps(maps: &[ArkMap]) -> usize {
    maps.iter()
        .filter(|map| {
            !map.is_home
                && map
                    .systemd_detail
                    .as_ref()
                    .map(|status| status.active)
                    .unwrap_or(false)
        })
        .count()
}

#[derive(sqlx::FromRow)]
struct ActivityRow {
    id: i64,
    ts: String,
    severity: String,
    source: String,
    actor: String,
    target_map: String,
    message: String,
    detail: String,
}

#[derive(serde::Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct ConfigVersionRow {
    id: String,
    ts: String,
    actor: String,
    file: String,
    reason: String,
    backup_path: String,
    status: String,
}

fn epoch_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

async fn maps_with_status(s: &AppState) -> Vec<ArkMap> {
    let mut maps = Vec::new();
    let mut used_config_ids = HashSet::new();
    let shared = config_edit::read_shared(&s.config);
    let (max_players, max_players_source) = max_players_from_shared(&shared);

    for official in travel_model::official_maps() {
        let cfg = s.config.maps.iter().find(|cfg| {
            !used_config_ids.contains(&cfg.id) && config_matches_official(cfg, official)
        });
        let mut map = if let Some(cfg) = cfg {
            used_config_ids.insert(cfg.id.clone());
            map_from_config(cfg, max_players, &max_players_source)
        } else {
            map_from_official(*official, max_players, &max_players_source)
        };
        enrich_map_runtime(&mut map, s).await;
        maps.push(map);
    }

    for cfg in &s.config.maps {
        if used_config_ids.contains(&cfg.id) {
            continue;
        }
        let mut map = map_from_config(cfg, max_players, &max_players_source);
        enrich_map_runtime(&mut map, s).await;
        maps.push(map);
    }

    // Overlay active external node travel sessions
    let active_sessions = travel_sessions::list_active(&s.pool).await;
    let relay_ip = s.config.server.relay_public_ip.clone();
    for session in &active_sessions {
        let node = nodes_model::get(&s.pool, &session.node_id).await;
        let connect_host = node.as_ref().map(|n| n.tailscale_ip.clone()).unwrap_or_default();
        let node_name = node.as_ref().map(|n| n.display_name.clone()).unwrap_or_else(|| session.node_id.clone());
        let game_port = session.game_port.unwrap_or(0) as u16;
        let query_port = session.query_port.unwrap_or(0) as u16;
        let is_ready = session.status == "ready";
        let connection_available = is_ready && !connect_host.is_empty();
        let conn_addr = if connection_available { format!("{}:{}", connect_host, game_port) } else { String::new() };
        let query_addr = if connection_available { format!("{}:{}", connect_host, query_port) } else { String::new() };
        let pub_conn = if connection_available && !relay_ip.is_empty() {
            Some(format!("{}:{}", relay_ip, game_port))
        } else {
            None
        };
        let pub_query = if connection_available && !relay_ip.is_empty() {
            Some(format!("{}:{}", relay_ip, query_port))
        } else {
            None
        };

        if let Some(existing) = maps.iter_mut().find(|m| m.id == session.map_id) {
            existing.state = if is_ready { "Online".into() } else { "Starting".into() };
            existing.assignment = format!("External node ({})", node_name);
            existing.rcon = if is_ready { "Connected".into() } else { "Connecting".into() };
            existing.systemd = "external-node".into();
            existing.connect_host = connect_host.clone();
            existing.game_port = game_port;
            existing.query_port = query_port;
            existing.connection_address = conn_addr.clone();
            existing.query_address = query_addr.clone();
            existing.connection_source = "external-node".into();
            existing.connection_available = connection_available;
            existing.connection_unavailable_reason = if connection_available { String::new() } else { "node not yet ready".into() };
            existing.next_action = format!("Running on external node {}", node_name);
            existing.systemd_detail = None;
            existing.public_connection_address = pub_conn;
            existing.public_query_address = pub_query;
        }
    }

    maps
}

async fn enrich_map_runtime(map: &mut ArkMap, s: &AppState) {
    let active_slot = configured_slot_for_map(&s.config, &map.id);
    if let Some((slot, slot_key)) = active_slot {
        apply_slot_to_map(map, slot, slot_key);
    }

    if map.configured || active_slot.is_some() {
        if active_slot.is_none() {
            if let Some((slot, _)) = configured_slot_for_unit(&s.config, &map.config.systemd_unit) {
                let effective = travel_model::effective_slot_map_id(&s.config, slot);
                if effective != map.id {
                    map.assignment = "Available destination".into();
                    map.state = "Not running".into();
                    map.systemd = "on-demand slot assigned to another map".into();
                    map.rcon = "Disconnected".into();
                    map.player_count_source = "not_running".into();
                    map.slot_role = "On-demand pool".into();
                    map.next_action =
                        format!("Launch-ready destination; current slot is running {effective}");
                    map.systemd_detail = None;
                    return;
                }
            }
        }
        if map.config.systemd_unit.trim().is_empty() {
            map.systemd = "unavailable".into();
            map.systemd_detail = Some(UnitStatus::unavailable(
                "",
                "config",
                "systemd unit not configured",
            ));
        } else {
            let detail = match s.systemd.get_status(&map.config.systemd_unit).await {
                Ok(status) => status,
                Err(err) => {
                    UnitStatus::unavailable(&map.config.systemd_unit, "systemd", err.to_string())
                }
            };
            map.systemd = detail.state.clone();
            map.state = map_state_from_systemd(map, &detail);
            if let Some(memory) = detail.memory_current_bytes {
                map.ram_mb = (memory / 1024 / 1024) as u32;
            }
            map.systemd_detail = Some(detail);
        }
        if let Some(slot_id) = &map.slot_id {
            if let Some(runtime) = s.rcon_runtime.read().await.endpoint_for(slot_id).cloned() {
                map.rcon = runtime.state.as_str().into();
                if runtime.active {
                    if let Some(count) = runtime.player_count {
                        map.players = count;
                        map.player_count_source = "rcon".into();
                    } else {
                        map.players = 0;
                        map.player_count_source = "rcon_unavailable".into();
                        map.unavailable_reason = runtime.last_error.clone().or_else(|| {
                            Some("RCON connected but player count is not available yet".into())
                        });
                    }
                    map.next_action = runtime
                        .last_error
                        .clone()
                        .unwrap_or_else(|| "RCON player polling live".into());
                }
            } else {
                map.rcon = if s.config.operations.rcon_enabled && s.config.rcon.enabled {
                    "Connecting".into()
                } else {
                    "Disabled".into()
                };
            }
        }
    } else {
        map.systemd_detail = None;
    }
    if let Some((slot, _)) = active_slot {
        apply_connection_to_map(map, &s.config, slot);
    }
}

fn map_from_config(cfg: &MapConfig, max_players: u32, max_players_source: &str) -> ArkMap {
    ArkMap {
        id: cfg.id.clone(),
        name: cfg.name.clone(),
        alias: cfg.alias.clone(),
        role: if cfg.can_be_home {
            "Home-capable".into()
        } else {
            "Travel-capable".into()
        },
        assignment: cfg.assignment.clone(),
        state: "Unknown".into(),
        players: 0,
        player_count_source: "unavailable".into(),
        max_players,
        max_players_source: max_players_source.into(),
        ram_mb: 0,
        ram_estimate_mb: 0,
        uptime_mins: 0,
        idle_mins: 0,
        last_backup: "unknown".into(),
        rcon: "Disconnected".into(),
        systemd: "unknown".into(),
        restart_required: false,
        cpu_pct: 0,
        save_size_mb: 0,
        is_home: cfg.assignment.eq_ignore_ascii_case("home"),
        protected: cfg.can_be_home && cfg.assignment.eq_ignore_ascii_case("home"),
        configured: true,
        launch_ready: !cfg.ark_map_name.trim().is_empty(),
        unavailable_reason: None,
        slot_id: None,
        slot_role: if cfg.assignment.eq_ignore_ascii_case("home") {
            "Home".into()
        } else {
            "On-demand".into()
        },
        next_action: "Read-only status; control disabled in this phase".into(),
        connect_host: String::new(),
        game_port: 0,
        query_port: 0,
        connection_address: String::new(),
        query_address: String::new(),
        connection_source: "unavailable".into(),
        connection_available: false,
        connection_unavailable_reason: "map not running".into(),
        public_connection_address: None,
        public_query_address: None,
        config: MapConfigSummary {
            systemd_unit: cfg.systemd_unit.clone(),
            ark_map_name: cfg.ark_map_name.clone(),
            query_port: cfg.query_port,
            rcon_port: cfg.rcon_port,
            game_port: cfg.game_port,
            slot_priority: cfg.slot_priority,
            auto_shutdown_enabled: cfg.can_auto_stop_when_empty,
            can_be_home: cfg.can_be_home,
            can_auto_stop_when_empty: cfg.can_auto_stop_when_empty,
            can_enter_standby: cfg.can_enter_standby,
            mod_list: cfg.mods.clone(),
        },
        systemd_detail: None,
    }
}

fn map_from_official(
    official: travel_model::OfficialMapProfile,
    max_players: u32,
    max_players_source: &str,
) -> ArkMap {
    ArkMap {
        id: official.id.into(),
        name: official.name.into(),
        alias: official.alias.into(),
        role: "Travel-capable".into(),
        assignment: "Available destination".into(),
        state: "Not running".into(),
        players: 0,
        player_count_source: "not_running".into(),
        max_players,
        max_players_source: if max_players > 0 {
            max_players_source.into()
        } else {
            "unknown".into()
        },
        ram_mb: 0,
        ram_estimate_mb: 0,
        uptime_mins: 0,
        idle_mins: 0,
        last_backup: "not started".into(),
        rcon: "Disconnected".into(),
        systemd: "not running".into(),
        restart_required: false,
        cpu_pct: 0,
        save_size_mb: 0,
        is_home: false,
        protected: false,
        configured: false,
        launch_ready: true,
        unavailable_reason: None,
        slot_id: None,
        slot_role: "On-demand pool".into(),
        next_action: "Launch-ready on-demand destination".into(),
        connect_host: String::new(),
        game_port: 0,
        query_port: 0,
        connection_address: String::new(),
        query_address: String::new(),
        connection_source: "unavailable".into(),
        connection_available: false,
        connection_unavailable_reason: "map not running".into(),
        public_connection_address: None,
        public_query_address: None,
        config: MapConfigSummary {
            systemd_unit: "".into(),
            ark_map_name: official.ark_map_name.into(),
            query_port: 0,
            rcon_port: 0,
            game_port: 0,
            slot_priority: 0,
            auto_shutdown_enabled: true,
            can_be_home: false,
            can_auto_stop_when_empty: true,
            can_enter_standby: false,
            mod_list: Vec::new(),
        },
        systemd_detail: None,
    }
}

fn config_matches_official(cfg: &MapConfig, official: &travel_model::OfficialMapProfile) -> bool {
    let official_keys = [
        official.id,
        official.name,
        official.alias,
        official.ark_map_name,
    ]
    .into_iter()
    .map(normalize_key)
    .collect::<Vec<_>>();
    [
        cfg.id.as_str(),
        cfg.name.as_str(),
        cfg.alias.as_str(),
        cfg.ark_map_name.as_str(),
    ]
    .into_iter()
    .map(normalize_key)
    .any(|key| official_keys.iter().any(|official| official == &key))
}

fn max_players_from_shared(shared: &config_edit::SharedConfig) -> (u32, String) {
    for (file, raw) in [
        (
            "GameUserSettings.ini",
            shared.game_user_settings_ini.as_str(),
        ),
        ("Game.ini", shared.game_ini.as_str()),
    ] {
        if let Some(value) = parse_max_players(raw) {
            return (value, format!("{file}: MaxPlayers"));
        }
    }
    (0, "unknown".into())
}

fn parse_max_players(raw: &str) -> Option<u32> {
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(';') || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        if matches!(
            key.as_str(),
            "maxplayers" | "maxplayersoverride" | "maxplayersallowed"
        ) {
            if let Ok(parsed) = value.trim().parse::<u32>() {
                if parsed > 0 {
                    return Some(parsed);
                }
            }
        }
    }
    None
}

fn normalize_key(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

fn apply_config_to_map(map: &mut ArkMap, cfg: &MapConfig) {
    map.name = cfg.name.clone();
    map.alias = cfg.alias.clone();
    map.assignment = cfg.assignment.clone();
    map.role = if cfg.can_be_home {
        "Home-capable".into()
    } else {
        "Travel-capable".into()
    };
    map.is_home = cfg.assignment.eq_ignore_ascii_case("home");
    map.protected = map.is_home && cfg.can_be_home;
    map.launch_ready = !cfg.ark_map_name.trim().is_empty();
    map.unavailable_reason = None;
    map.config = MapConfigSummary {
        systemd_unit: cfg.systemd_unit.clone(),
        ark_map_name: cfg.ark_map_name.clone(),
        query_port: cfg.query_port,
        rcon_port: cfg.rcon_port,
        game_port: cfg.game_port,
        slot_priority: cfg.slot_priority,
        auto_shutdown_enabled: cfg.can_auto_stop_when_empty,
        can_be_home: cfg.can_be_home,
        can_auto_stop_when_empty: cfg.can_auto_stop_when_empty,
        can_enter_standby: cfg.can_enter_standby,
        mod_list: cfg.mods.clone(),
    };
}

fn configured_slot_for_map<'a>(
    config: &'a crate::config::Config,
    map_id: &str,
) -> Option<(&'a ServerSlot, &'static str)> {
    let slots = config.slots.as_ref()?;
    slots
        .iter()
        .into_iter()
        .find(|(slot, _, _)| travel_model::effective_slot_map_id(config, slot) == map_id)
        .map(|(slot, key, _)| (slot, key))
}

fn configured_slot_for_unit<'a>(
    config: &'a crate::config::Config,
    unit: &str,
) -> Option<(&'a ServerSlot, &'static str)> {
    let slots = config.slots.as_ref()?;
    slots
        .iter()
        .into_iter()
        .find(|(slot, _, _)| slot.systemd_unit == unit)
        .map(|(slot, key, _)| (slot, key))
}

fn apply_slot_to_map(map: &mut ArkMap, slot: &ServerSlot, slot_key: &str) {
    map.assignment = slot_role_label(slot_key).into();
    map.is_home = slot_key == "home";
    map.protected = slot.protected;
    map.slot_id = Some(slot.id.clone());
    map.slot_role = slot_role_label(slot_key).into();
    map.config.systemd_unit = slot.systemd_unit.clone();
    map.config.game_port = slot.game_port;
    map.config.query_port = slot.query_port;
    map.config.rcon_port = slot.rcon_port;
    if !slot.enabled {
        map.role = "Disabled".into();
        map.state = "Offline".into();
        map.next_action = "Slot disabled in config".into();
    }
}

fn apply_connection_to_map(map: &mut ArkMap, config: &crate::config::Config, slot: &ServerSlot) {
    map.connect_host = config.player_connect_host();
    map.game_port = slot.game_port;
    map.query_port = slot.query_port;
    let running = matches!(map.state.as_str(), "Online" | "Ready" | "Starting");
    if !running {
        map.connection_available = false;
        map.connection_source = "systemd".into();
        map.connection_address.clear();
        map.query_address.clear();
        map.connection_unavailable_reason = "map not running".into();
        return;
    }
    if map.connect_host.trim().is_empty() {
        map.connection_available = false;
        map.connection_source = "manager_config".into();
        map.connection_address.clear();
        map.query_address.clear();
        map.connection_unavailable_reason = "player connect host is not configured".into();
        return;
    }
    if slot.game_port == 0 || slot.query_port == 0 {
        map.connection_available = false;
        map.connection_source = "slot_config".into();
        map.connection_address.clear();
        map.query_address.clear();
        map.connection_unavailable_reason = "slot game/query port is not configured".into();
        return;
    }
    // When RCON is enabled, require an active RCON connection before advertising the server as
    // connectable. systemd "active" fires before game ports are open; RCON connection is a
    // reliable proxy for the game being ready to accept players.
    if config.rcon.enabled && map.rcon != "Connected" {
        map.connection_available = false;
        map.connection_source = "rcon".into();
        map.connection_address.clear();
        map.query_address.clear();
        map.connection_unavailable_reason = "server loading: waiting for RCON connection".into();
        return;
    }
    map.connection_available = true;
    map.connection_source = "slot_config".into();
    map.connection_address = format!("{}:{}", map.connect_host, slot.game_port);
    map.query_address = format!("{}:{}", map.connect_host, slot.query_port);
    map.connection_unavailable_reason.clear();
    let relay_ip = &config.server.relay_public_ip;
    if !relay_ip.is_empty() {
        map.public_connection_address = Some(format!("{}:{}", relay_ip, slot.game_port));
        map.public_query_address = Some(format!("{}:{}", relay_ip, slot.query_port));
    }
}

fn map_state_from_systemd(map: &ArkMap, status: &UnitStatus) -> String {
    match status.active_state.as_str() {
        "active" => "Online".into(),
        "activating" => "Starting".into(),
        "failed" => "Error".into(),
        "inactive" if map.is_home && map.config.can_enter_standby => "Resource Standby".into(),
        "inactive" => "Offline".into(),
        "unavailable" => map.state.clone(),
        _ => "Offline".into(),
    }
}

fn systemd_summary(maps: &[ArkMap]) -> serde_json::Value {
    let statuses: Vec<_> = maps
        .iter()
        .filter_map(|m| m.systemd_detail.as_ref())
        .collect();
    if statuses.is_empty() {
        return json!({ "status": "Unavailable", "tone": "gray", "available": false, "source": "unavailable" });
    }
    let unavailable = statuses.iter().filter(|st| st.error.is_some()).count();
    let active = statuses.iter().filter(|st| st.active).count();
    let failed = statuses
        .iter()
        .filter(|st| st.active_state == "failed" || st.state == "failed")
        .count();
    let source = statuses
        .iter()
        .map(|st| st.source.as_str())
        .find(|source| *source == "systemd")
        .unwrap_or_else(|| statuses[0].source.as_str());

    if unavailable == statuses.len() {
        json!({
            "status": "Unavailable",
            "tone": "gray",
            "available": false,
            "source": source,
            "activeUnits": active,
            "failedUnits": failed,
            "checkedUnits": statuses.len()
        })
    } else if failed > 0 {
        json!({
            "status": "Failures detected",
            "tone": "red",
            "available": true,
            "source": source,
            "activeUnits": active,
            "failedUnits": failed,
            "checkedUnits": statuses.len()
        })
    } else {
        json!({
            "status": "Read-only available",
            "tone": "green",
            "available": true,
            "source": source,
            "activeUnits": active,
            "failedUnits": failed,
            "checkedUnits": statuses.len()
        })
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetMapBody {
    map_id: String,
    #[serde(default)]
    confirm: bool,
    #[serde(default)]
    reason: String,
}

async fn home_set_map(
    State(s): State<AppState>,
    Json(body): Json<SetMapBody>,
) -> axum::response::Response {
    if !s.config.operations.systemd_control_enabled {
        return api_error(
            StatusCode::FORBIDDEN,
            "CONTROL_DISABLED",
            "systemd control is disabled",
            json!({}),
        );
    }
    if !body.confirm {
        return api_error(
            StatusCode::BAD_REQUEST,
            "INVALID_CONFIRMATION",
            "confirm must be true",
            json!({}),
        );
    }

    // Resolve map by id, alias, or ark_map_name.
    let map_cfg = s.config.maps.iter().find(|m| {
        m.id == body.map_id
            || m.alias == body.map_id
            || m.ark_map_name.eq_ignore_ascii_case(&body.map_id)
    });
    let Some(map_cfg) = map_cfg else {
        return api_error(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            format!("map '{}' not found", body.map_id),
            json!({}),
        );
    };
    if !map_cfg.can_be_home {
        return api_error(
            StatusCode::BAD_REQUEST,
            "HOME_PROTECTED",
            format!("map '{}' has can_be_home = false", map_cfg.id),
            json!({}),
        );
    }

    let ark_map_name = map_cfg.ark_map_name.clone();
    let map_id = map_cfg.id.clone();

    // Write runtime override so the home slot picks up the new map on next start.
    let override_path = "/var/lib/ark-cluster-manager/runtime-slots/home.env";
    let content = format!("ARK_MAP={ark_map_name}\n");
    match std::fs::write(override_path, &content) {
        Ok(()) => {}
        Err(err) => {
            return api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "WRITE_FAILED",
                format!("failed to write runtime override: {err}"),
                json!({}),
            );
        }
    }

    let reason = if body.reason.trim().is_empty() {
        "discord_home_set_map".to_string()
    } else {
        body.reason.clone()
    };
    audit::record(
        &s.pool,
        &AuditEvent::new(Severity::Warn, "Config", "Home map override set")
            .target(&map_id)
            .detail(format!(
                "ark_map_name={ark_map_name} override_path={override_path} reason={reason}"
            )),
    )
    .await;

    Json(json!({
        "accepted": true,
        "mapId": map_id,
        "arkMapName": ark_map_name,
        "overridePath": override_path,
        "message": format!(
            "Home map override set to {ark_map_name}. Restart the Home server for the change to take effect."
        ),
        "restartRequired": true
    }))
    .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn official_destination_is_launch_ready_not_unavailable() {
        let profile = travel_model::official_maps()
            .iter()
            .copied()
            .find(|map| map.id == "scorched-earth")
            .unwrap();
        let map = map_from_official(profile, 70, "GameUserSettings.ini: MaxPlayers");
        assert_eq!(map.assignment, "Available destination");
        assert_eq!(map.state, "Not running");
        assert_eq!(map.player_count_source, "not_running");
        assert_eq!(map.max_players, 70);
        assert!(map.launch_ready);
        assert!(!map.configured);
        assert_ne!(map.assignment, "Unassigned");
        assert_ne!(map.state, "Unavailable");
    }
}
