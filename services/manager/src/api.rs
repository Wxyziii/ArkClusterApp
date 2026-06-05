//! HTTP API. Versioned routes under `/api`, all behind Bearer auth (mounted by
//! the caller). Every handler returns realistic mock data matching the UI.
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

use crate::config::{MapConfig, ServerSlot};
use crate::mock;
use crate::models::audit::{self, AuditEvent, Severity};
use crate::models::backup;
use crate::models::domain::{ArkMap, MapConfigSummary};
use crate::models::governor;
use crate::models::operations::{self, ActionRequest, GuardError, ServerAction, SystemdGuardInput};
use crate::models::rcon::{RconEndpoint, RconListener};
use crate::models::resources;
use crate::models::systemd::UnitStatus;
use crate::models::{config_edit, maintenance, mods as mod_ops, runtime, travel as travel_model};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
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

async fn status(State(s): State<AppState>) -> impl IntoResponse {
    let res = resources::sample(&s.config.cluster.directory, s.manager_started_at).await;
    let rp = ram_pct(&res);
    let (label, tone) = pressure_label(rp, &s.config.resource_policy);
    let maps = maps_with_status(&s).await;
    let systemd = systemd_summary(&maps);
    let running = maps
        .iter()
        .filter(|m| matches!(m.state.as_str(), "Online" | "Ready" | "Starting"))
        .count();

    Json(json!({
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
            // Mock — real Tailscale status is not queried in this phase.
            "status": "Connected",
            "tone": "cyan",
            "bindPrivate": s.config.bind_is_private(),
            "bindAddress": s.config.server.bind_address
        },
        "discord": {
            "status": if s.config.discord.enabled { "Online" } else { "Disabled (placeholder)" },
            "tone": if s.config.discord.enabled { "green" } else { "gray" }
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
        "players": mock::players().len(),
        "runningMaps": running
    }))
}

async fn capabilities(State(s): State<AppState>) -> impl IntoResponse {
    let source = if cfg!(target_os = "linux") {
        "host"
    } else {
        "fallback"
    };
    Json(operations::capabilities(&s.config, source))
}

async fn servers(State(s): State<AppState>) -> impl IntoResponse {
    Json(maps_with_status(&s).await)
}

async fn server_detail(State(s): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    match maps_with_status(&s).await.into_iter().find(|m| m.id == id) {
        Some(map) => {
            let players: Vec<_> = mock::players()
                .into_iter()
                .filter(|p| p.map == map.name)
                .collect();
            Json(json!({ "server": map, "players": players })).into_response()
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
    let slot = |name: &str| maps.iter().find(|m| m.assignment == name).cloned();
    let active = mock::active_travel();
    let history = travel_model::history(&s.pool).await.unwrap_or_default();
    Json(json!({
        "slots": {
            "home": maps.iter().find(|m| m.is_home).cloned(),
            "travelA": slot("Travel A"),
            "travelB": slot("Travel B")
        },
        "maxTravelServers": s.config.resource_policy.max_travel_servers,
        "activeRequest": active,
        "stepper": { "current": 2, "blocked": true, "blockedAt": 2 },
        "blockReason": "Both travel slots have active players. Request queued until a slot frees or Home leaves Resource Standby.",
        "queue": [],
        "enabled": s.config.operations.travel_scheduler_enabled,
        "idleShutdownSecs": s.config.operations.travel_idle_shutdown_secs,
        "idleShutdownProduction": s.config.operations.travel_idle_shutdown_secs == 10800,
        "homeResourceStandby": s.config.resource_policy.home_standby_enabled,
        "recent": history
    }))
}

async fn travel_request(
    State(s): State<AppState>,
    Json(req): Json<travel_model::TravelRequestBody>,
) -> impl IntoResponse {
    let statuses = slot_statuses(&s).await;
    match travel_model::decide(&s.pool, &s.config, req, statuses).await {
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
        Ok(decision) => {
            audit::record(
                &s.pool,
                &AuditEvent::new(Severity::Warn, "Travel", "travel request blocked")
                    .target(decision.resolved_map.as_deref().unwrap_or(""))
                    .detail(format!("id={} reason={}", decision.id, decision.reason)),
            )
            .await;
            (StatusCode::CONFLICT, Json(decision)).into_response()
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

    // Player split for governor preview (does NOT actuate anything).
    let maps = mock::maps();
    let home_players: u32 = maps.iter().filter(|m| m.is_home).map(|m| m.players).sum();
    let travel_players: u32 = maps.iter().filter(|m| !m.is_home).map(|m| m.players).sum();
    let decision = governor::evaluate(p, &res, home_players, travel_players);

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
    let backups = if records.is_empty() {
        mock::backups()
            .into_iter()
            .map(|b| {
                json!({
                    "id": b.id,
                    "map": b.map,
                    "type": b.kind,
                    "sizeMb": b.size_mb,
                    "sizeBytes": b.size_mb as u64 * 1024 * 1024,
                    "created": b.created,
                    "createdAt": b.created,
                    "completedAt": null,
                    "reason": b.reason,
                    "status": b.status,
                    "path": "",
                    "source": "mock",
                    "error": b.error
                })
            })
            .collect::<Vec<_>>()
    } else {
        records
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
            .collect::<Vec<_>>()
    };
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
    Json(json!({
        "activity": mock::activity_log(),
        "recent": mock::recent_activity(),
        "source": "mock"
    }))
}

async fn rcon_status(State(s): State<AppState>) -> impl IntoResponse {
    Json(crate::models::rcon::status_from_config(&s.config))
}

async fn players() -> impl IntoResponse {
    Json(json!({ "players": mock::players(), "source": "mock", "rconEnabled": false }))
}

async fn chat_recent(State(s): State<AppState>) -> impl IntoResponse {
    Json(json!({
        "messages": [],
        "detectedCommands": [],
        "source": if s.config.operations.rcon_enabled && s.config.rcon.enabled { "rcon_unavailable" } else { "disabled" }
    }))
}

async fn config(State(s): State<AppState>) -> impl IntoResponse {
    let shared = config_edit::read_shared(&s.config);
    Json(json!({
        "fields": mock::config_fields(),
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
        "name": format!("Steam Workshop {}", id),
        "url": format!("https://steamcommunity.com/sharedfiles/filedetails/?id={}", id),
        "game": "ARK: Survival Evolved",
        "installAvailable": s.config.operations.mod_management_enabled,
        "mutable": s.config.operations.mod_management_enabled,
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
            "permissionsOk": true,
            "implemented": true,
            "dashboard": {
                "category": "ARK Cluster",
                "channels": ["ark-status", "ark-travel", "ark-players", "ark-logs", "ark-admin"],
                "stateFile": "/var/lib/ark-cluster-discord-bot/state.json"
            }
        },
        "commands": mock::discord_commands(),
        "events": mock::discord_events(),
        "alertSettings": mock::alert_settings()
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
        "rcon": rcon_overview(),
        "systemdUnits": maps.iter().map(|m| json!({
            "map": m.name,
            "unit": m.config.systemd_unit,
            "state": m.systemd,
            "detail": m.systemd_detail,
            "controlImplemented": s.config.operations.systemd_control_enabled
        })).collect::<Vec<_>>()
    }))
}

/// RCON model preview (no sockets opened in this phase).
fn rcon_overview() -> RconListener {
    let endpoints = mock::maps()
        .iter()
        .map(|m| {
            let mut endpoint = RconEndpoint::mock(
                &m.id,
                "127.0.0.1",
                m.config.rcon_port,
                m.rcon == "Connected",
                m.players,
            );
            endpoint.last_error = Some("RCON disabled or mock-only".into());
            endpoint
        })
        .collect();
    RconListener {
        enabled: false,
        poll_interval_secs: 5,
        endpoints,
        implemented: false,
    }
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
    let player_count = current_map.map(|m| m.players).unwrap_or(0);
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
        ServerAction::Stop => s.systemd.stop_unit(&slot.systemd_unit).await,
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
    slots
        .iter()
        .into_iter()
        .find(|(slot, key, _)| {
            id == slot.id || id == slot.map_key || normalized == *key || id == key.replace('_', "-")
        })
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

async fn slot_statuses(s: &AppState) -> Vec<(ServerSlot, &'static str, UnitStatus, u32)> {
    let mut out = Vec::new();
    if let Some(slots) = &s.config.slots {
        for (slot, key, _) in slots.iter() {
            let status = s
                .systemd
                .get_status(&slot.systemd_unit)
                .await
                .unwrap_or_else(|e| {
                    UnitStatus::unavailable(&slot.systemd_unit, "fallback", e.to_string())
                });
            let players = mock::maps()
                .iter()
                .find(|m| m.id == slot.map_key)
                .map(|m| m.players)
                .unwrap_or(0);
            out.push((slot.clone(), key, status, players));
        }
    }
    out
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
    let mock_maps = mock::maps();
    let mut maps = Vec::new();

    for cfg in &s.config.maps {
        let mut map = mock_maps
            .iter()
            .find(|m| m.id == cfg.id)
            .cloned()
            .unwrap_or_else(|| map_from_config(cfg));
        apply_config_to_map(&mut map, cfg);
        if let Some((slot, slot_key)) = configured_slot_for_map(&s.config, &cfg.id) {
            apply_slot_to_map(&mut map, slot, slot_key);
        }

        let detail = match s.systemd.get_status(&map.config.systemd_unit).await {
            Ok(status) => status,
            Err(err) => {
                UnitStatus::unavailable(&map.config.systemd_unit, "fallback", err.to_string())
            }
        };
        map.systemd = detail.state.clone();
        map.state = map_state_from_systemd(&map, &detail);
        map.systemd_detail = Some(detail);
        maps.push(map);
    }

    maps
}

fn map_from_config(cfg: &MapConfig) -> ArkMap {
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
        max_players: 20,
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
        next_action: "Read-only status; control disabled in this phase".into(),
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
        .find(|(slot, _, _)| slot.map_key == map_id)
        .map(|(slot, key, _)| (slot, key))
}

fn apply_slot_to_map(map: &mut ArkMap, slot: &ServerSlot, slot_key: &str) {
    map.assignment = match slot_key {
        "home" => "Home",
        "travel_a" => "Travel A",
        "travel_b" => "Travel B",
        _ => &slot.label,
    }
    .into();
    map.is_home = slot_key == "home";
    map.protected = slot.protected;
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
        return json!({ "status": "Unavailable", "tone": "gray", "available": false, "source": "fallback" });
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
