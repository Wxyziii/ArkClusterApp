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

use crate::config::{MapConfig, ServerSlot};
use crate::mock;
use crate::models::domain::{ArkMap, MapConfigSummary};
use crate::models::governor;
use crate::models::rcon::{RconEndpoint, RconListener};
use crate::models::resources;
use crate::models::systemd::UnitStatus;
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
            Json(json!({ "error": "not_found", "message": format!("no server with id '{id}'") })),
        )
            .into_response(),
    }
}

async fn travel(State(s): State<AppState>) -> impl IntoResponse {
    let maps = maps_with_status(&s).await;
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
        "systemdUnits": maps.iter().map(|m| json!({
            "map": m.name,
            "unit": m.config.systemd_unit,
            "state": m.systemd,
            "detail": m.systemd_detail,
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
