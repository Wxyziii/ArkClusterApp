//! HTTP API. Versioned routes under `/api`, all behind Bearer auth (mounted by
//! the caller). Every handler returns realistic mock data matching the UI.
//!
//! Phase 1 exposes READ-ONLY endpoints. No start/stop/restart, config write,
//! mod mutation, or backup/restore routes exist — those remain unimplemented by
//! design so the API cannot affect the host.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::json;

use crate::mock;
use crate::models::governor;
use crate::models::rcon::{RconEndpoint, RconListener};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/status", get(status))
        .route("/servers", get(servers))
        .route("/servers/{id}", get(server_detail))
        .route("/travel", get(travel))
        .route("/resources", get(resources))
        .route("/backups", get(backups))
        .route("/activity", get(activity))
        .route("/config", get(config))
        .route("/mods", get(mods))
        .route("/discord/status", get(discord_status))
        .route("/settings", get(settings))
}

fn ram_pct(s: &crate::models::domain::ResourceSample) -> u32 {
    ((s.ram_used_gb / s.ram_total_gb) * 100.0).round() as u32
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
    let res = mock::resources();
    let rp = ram_pct(&res);
    let (label, tone) = pressure_label(rp, &s.config.resource_policy);
    let maps = mock::maps();
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
        "systemd": { "status": "Available (mock)", "tone": "green" },
        "resourcePressure": { "ramPct": rp, "label": label, "tone": tone },
        "players": mock::players().len(),
        "runningMaps": running
    }))
}

async fn servers() -> impl IntoResponse {
    Json(mock::maps())
}

async fn server_detail(Path(id): Path<String>) -> impl IntoResponse {
    match mock::maps().into_iter().find(|m| m.id == id) {
        Some(map) => {
            let players: Vec<_> = mock::players()
                .into_iter()
                .filter(|p| p.map == map.name)
                .collect();
            Json(json!({ "server": map, "players": players })).into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "not_found", "message": format!("no server with id '{id}'") })),
        )
            .into_response(),
    }
}

async fn travel(State(s): State<AppState>) -> impl IntoResponse {
    let maps = mock::maps();
    let slot = |name: &str| maps.iter().find(|m| m.assignment == name).cloned();
    let active = mock::active_travel();
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
        "queue": []
    }))
}

async fn resources(State(s): State<AppState>) -> impl IntoResponse {
    let res = mock::resources();
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
            "swapPct": ((res.swap_used_gb / res.swap_total_gb) * 100.0).round() as u32,
            "diskPct": ((res.disk_used_gb / res.disk_total_gb) * 100.0).round() as u32,
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
        "perProcess": maps.iter().filter(|m| m.ram_mb > 0).map(|m| json!({
            "map": m.name, "ramMb": m.ram_mb, "cpuPct": m.cpu_pct
        })).collect::<Vec<_>>()
    }))
}

async fn backups(State(s): State<AppState>) -> impl IntoResponse {
    let p = &s.config.backup_policy;
    Json(json!({
        "backups": mock::backups(),
        "policy": {
            "beforeShutdown": p.before_shutdown,
            "beforeConfigSave": p.before_config_save,
            "beforeModChange": p.before_mod_change,
            "retention": p.retention
        }
    }))
}

async fn activity() -> impl IntoResponse {
    Json(json!({
        "activity": mock::activity_log(),
        "recent": mock::recent_activity()
    }))
}

async fn config() -> impl IntoResponse {
    Json(json!({
        "fields": mock::config_fields(),
        "gameIni": mock::RAW_GAME_INI,
        "gameUserSettingsIni": mock::RAW_GUS_INI,
        "restartRequired": true,
        // Phase 1: config writing is not implemented; UI editors are preview-only.
        "writable": false
    }))
}

async fn mods() -> impl IntoResponse {
    let list = mock::mods();
    let restart_required = list.iter().any(|m| m.state == "disabled" || !m.installed);
    Json(json!({
        "mods": list,
        "restartRequired": restart_required,
        // Phase 1: no download/enable/disable/remove is implemented.
        "mutable": false
    }))
}

async fn discord_status(State(s): State<AppState>) -> impl IntoResponse {
    Json(json!({
        "status": {
            "online": s.config.discord.enabled,
            "guild": s.config.discord.guild,
            "statusChannel": s.config.discord.status_channel,
            "lastHeartbeat": "2026-06-03 20:55:12",
            "permissionsOk": true,
            "implemented": false
        },
        "commands": mock::discord_commands(),
        "events": mock::discord_events(),
        "alertSettings": mock::alert_settings()
    }))
}

async fn settings(State(s): State<AppState>) -> impl IntoResponse {
    let p = &s.config.resource_policy;
    let b = &s.config.backup_policy;
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
            "configWritable": false,
            "modsMutable": false,
            "note": "Config writes and mod changes are not implemented in this phase."
        },
        "security": {
            "authScheme": "Bearer token",
            "tokenSource": "config or ARK_MANAGER_API_TOKEN env",
            "tokenMasked": "••••••••",
            "note": "Token is never logged or returned by the API."
        },
        "rcon": rcon_overview(),
        "systemdUnits": mock::maps().iter().map(|m| json!({
            "map": m.name,
            "unit": m.config.systemd_unit,
            "state": m.systemd,
            "controlImplemented": false
        })).collect::<Vec<_>>()
    }))
}

/// RCON model preview (no sockets opened in this phase).
fn rcon_overview() -> RconListener {
    let endpoints = mock::maps()
        .iter()
        .map(|m| {
            RconEndpoint::mock(
                &m.id,
                "127.0.0.1",
                m.config.rcon_port,
                m.rcon == "Connected",
                m.players,
            )
        })
        .collect();
    RconListener {
        enabled: false,
        poll_interval_secs: 5,
        endpoints,
        implemented: false,
    }
}
