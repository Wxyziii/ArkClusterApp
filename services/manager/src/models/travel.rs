use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::io::Write;
use std::sync::Arc;
use std::time::Instant;

use crate::config::{Config, MapConfig, ServerSlot};
use crate::models::operations::{self, ActionRequest, ServerAction, SystemdGuardInput};
use crate::models::resources;
use crate::models::systemd::SystemdController;
use crate::models::systemd::UnitStatus;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TravelRequestBody {
    pub map: String,
    #[serde(default = "default_source")]
    pub source: String,
    #[serde(default)]
    pub actor: String,
}

fn default_source() -> String {
    "api".into()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TravelDecision {
    pub id: String,
    pub accepted: bool,
    pub requested_map: String,
    pub resolved_map: Option<String>,
    pub chosen_slot: Option<String>,
    pub status: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TravelState {
    pub enabled: bool,
    pub idle_shutdown_secs: u32,
    pub max_travel_servers: u32,
    pub home_standby_enabled: bool,
    pub slots: Vec<TravelSlotState>,
    pub recent: Vec<TravelHistoryRow>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TravelSlotState {
    pub slot_id: String,
    pub role: String,
    pub map_key: String,
    pub unit: String,
    pub systemd: String,
    pub active: bool,
    pub player_count: Option<u32>,
    pub idle_shutdown_secs: u32,
    pub policy: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct TravelHistoryRow {
    pub id: String,
    pub ts: String,
    pub source: String,
    pub actor: String,
    pub requested_map: String,
    pub resolved_map: String,
    pub chosen_slot: String,
    pub status: String,
    pub reason: String,
    pub detail: String,
}

#[derive(Debug, Clone)]
pub struct SlotStatusSnapshot {
    pub slot: ServerSlot,
    pub key: &'static str,
    pub status: UnitStatus,
    pub player_count: Option<u32>,
}

pub fn resolve_map<'a>(config: &'a Config, raw: &str) -> Option<&'a MapConfig> {
    let key = normalize(raw);
    config.maps.iter().find(|m| {
        let aliases = aliases_for(&m.id, &m.alias, &m.ark_map_name, &m.name);
        aliases.iter().any(|a| normalize(a) == key)
    })
}

pub async fn decide(
    pool: &SqlitePool,
    config: &Config,
    req: TravelRequestBody,
    slot_statuses: Vec<SlotStatusSnapshot>,
) -> Result<TravelDecision, sqlx::Error> {
    let id = format!("travel-{}", epoch_millis());
    let Some(map) = resolve_map(config, &req.map) else {
        let reason = if known_official_map(&req.map) {
            "official map is not configured in this cluster"
        } else {
            "unknown map"
        };
        let decision = TravelDecision {
            id,
            accepted: false,
            requested_map: req.map.clone(),
            resolved_map: None,
            chosen_slot: None,
            status: "rejected".into(),
            reason: reason.into(),
        };
        insert_history(pool, &decision, &req, "").await?;
        return Ok(decision);
    };
    if !config.operations.travel_scheduler_enabled {
        let decision = TravelDecision {
            id,
            accepted: false,
            requested_map: req.map.clone(),
            resolved_map: Some(map.id.clone()),
            chosen_slot: None,
            status: "blocked".into(),
            reason: "travel scheduler disabled in manager config".into(),
        };
        insert_history(pool, &decision, &req, "").await?;
        return Ok(decision);
    }
    if let Some(snapshot) = slot_statuses.iter().find(|snapshot| {
        effective_slot_map_id(config, &snapshot.slot) == map.id && snapshot.status.active
    }) {
        let decision = TravelDecision {
            id,
            accepted: true,
            requested_map: req.map.clone(),
            resolved_map: Some(map.id.clone()),
            chosen_slot: Some(snapshot.slot.id.clone()),
            status: "already_online".into(),
            reason: "map already online".into(),
        };
        insert_history(pool, &decision, &req, snapshot.key).await?;
        return Ok(decision);
    }
    let free = slot_statuses
        .iter()
        .find(|snapshot| snapshot.key != "home" && !snapshot.status.active);
    let Some(snapshot) = free else {
        let empty = slot_statuses.iter().find(|snapshot| {
            snapshot.key != "home" && snapshot.status.active && snapshot.player_count == Some(0)
        });
        if let Some(snapshot) = empty {
            let decision = TravelDecision {
                id,
                accepted: true,
                requested_map: req.map.clone(),
                resolved_map: Some(map.id.clone()),
                chosen_slot: Some(snapshot.slot.id.clone()),
                status: "accepted_reuse_empty_slot".into(),
                reason: "empty active travel slot can be backed up and reused".into(),
            };
            insert_history(pool, &decision, &req, snapshot.key).await?;
            return Ok(decision);
        }
        let decision = TravelDecision {
            id,
            accepted: false,
            requested_map: req.map.clone(),
            resolved_map: Some(map.id.clone()),
            chosen_slot: None,
            status: "queued".into(),
            reason: "both travel slots have active players".into(),
        };
        insert_history(pool, &decision, &req, "").await?;
        return Ok(decision);
    };
    let decision = TravelDecision {
        id,
        accepted: true,
        requested_map: req.map.clone(),
        resolved_map: Some(map.id.clone()),
        chosen_slot: Some(snapshot.slot.id.clone()),
        status: "accepted".into(),
        reason: "free on-demand slot selected".into(),
    };
    insert_history(pool, &decision, &req, snapshot.key).await?;
    Ok(decision)
}

#[derive(Debug, thiserror::Error)]
pub enum TravelServiceError {
    #[error(transparent)]
    Sql(#[from] sqlx::Error),
    #[error("travel start failed: {0}")]
    Start(String),
}

pub async fn request_with_start(
    pool: &SqlitePool,
    config: &Config,
    systemd: Arc<dyn SystemdController>,
    manager_started_at: Instant,
    req: TravelRequestBody,
    slot_statuses: Vec<SlotStatusSnapshot>,
) -> Result<TravelDecision, TravelServiceError> {
    let mut decision = decide(pool, config, req, slot_statuses.clone()).await?;
    if !decision.accepted || decision.status == "already_online" {
        return Ok(decision);
    }
    if decision.status == "accepted_reuse_empty_slot" {
        decision.accepted = false;
        decision.status = "blocked".into();
        decision.reason = "reuse of active empty slots requires a stop/backup handoff; no free stopped slot is available".into();
        update_history(pool, &decision, "reuse_active_slot_blocked").await?;
        return Ok(decision);
    }
    let Some(map_id) = decision.resolved_map.clone() else {
        return Ok(decision);
    };
    let Some(map) = config.maps.iter().find(|m| m.id == map_id) else {
        decision.accepted = false;
        decision.status = "failed".into();
        decision.reason = "resolved map missing from config".into();
        update_history(pool, &decision, "").await?;
        return Ok(decision);
    };
    let Some(slot_id) = decision.chosen_slot.clone() else {
        return Ok(decision);
    };
    let Some(snapshot) = slot_statuses
        .iter()
        .find(|snapshot| snapshot.slot.id == slot_id)
    else {
        decision.accepted = false;
        decision.status = "failed".into();
        decision.reason = "chosen travel slot missing from config".into();
        update_history(pool, &decision, "").await?;
        return Ok(decision);
    };
    if snapshot.key == "home" {
        decision.accepted = false;
        decision.status = "failed".into();
        decision.reason = "Home cannot be used as an on-demand travel slot".into();
        update_history(pool, &decision, "").await?;
        return Ok(decision);
    }

    let sample = resources::sample(&config.cluster.directory, manager_started_at).await;
    let active_travel_slots = slot_statuses
        .iter()
        .filter(|snapshot| snapshot.key != "home" && snapshot.status.active)
        .count();
    let req_guard = ActionRequest {
        confirm: true,
        strong_confirm: false,
        admin_override: false,
        reason: "travel_scheduler_start".into(),
    };
    if let Err(err) = operations::guard_systemd_action(SystemdGuardInput {
        config,
        slot_key: snapshot.key,
        slot: &snapshot.slot,
        action: ServerAction::Start,
        req: &req_guard,
        sample: &sample,
        active_travel_slots,
        player_count: 0,
    }) {
        decision.accepted = false;
        decision.status = "blocked".into();
        decision.reason = err.message();
        update_history(pool, &decision, "guard_blocked").await?;
        return Ok(decision);
    }

    if let Err(err) = write_runtime_slot_override(&snapshot.slot, map) {
        decision.accepted = false;
        decision.status = "failed_start".into();
        decision.reason = format!("failed to write travel slot runtime override: {err}");
        update_history(pool, &decision, "override_write_failed").await?;
        return Ok(decision);
    }
    match systemd.start_unit(&snapshot.slot.systemd_unit).await {
        Ok(()) => {
            decision.status = "starting".into();
            decision.reason = format!("starting {}", map.name);
            update_history(pool, &decision, snapshot.key).await?;
        }
        Err(err) => {
            decision.accepted = false;
            decision.status = "failed_start".into();
            decision.reason = format!("{err}");
            update_history(pool, &decision, snapshot.key).await?;
        }
    }
    Ok(decision)
}

pub async fn history(pool: &SqlitePool) -> Result<Vec<TravelHistoryRow>, sqlx::Error> {
    sqlx::query_as::<_, TravelHistoryRow>(
        "SELECT id, ts, source, actor, requested_map, resolved_map, chosen_slot, status, reason, detail \
         FROM travel_requests ORDER BY ts DESC LIMIT 50",
    )
    .fetch_all(pool)
    .await
}

async fn insert_history(
    pool: &SqlitePool,
    decision: &TravelDecision,
    req: &TravelRequestBody,
    slot_key: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO travel_requests \
         (id, map, requested_by, source, source_raw, source_map, step, result, reason, created_at, \
          ts, actor, requested_map, resolved_map, chosen_slot, status, detail) \
         VALUES (?1, ?4, ?3, ?2, ?4, ?9, 0, ?7, ?8, datetime('now'), datetime('now'), ?3, ?4, ?5, ?6, ?7, ?9)",
    )
    .bind(&decision.id)
    .bind(&req.source)
    .bind(&req.actor)
    .bind(&decision.requested_map)
    .bind(decision.resolved_map.as_deref().unwrap_or(""))
    .bind(decision.chosen_slot.as_deref().unwrap_or(""))
    .bind(&decision.status)
    .bind(&decision.reason)
    .bind(slot_key)
    .execute(pool)
    .await?;
    Ok(())
}

async fn update_history(
    pool: &SqlitePool,
    decision: &TravelDecision,
    detail: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE travel_requests SET chosen_slot = ?2, status = ?3, result = ?3, reason = ?4, detail = ?5 WHERE id = ?1",
    )
    .bind(&decision.id)
    .bind(decision.chosen_slot.as_deref().unwrap_or(""))
    .bind(&decision.status)
    .bind(&decision.reason)
    .bind(detail)
    .execute(pool)
    .await?;
    Ok(())
}

pub fn effective_slot_map_id(config: &Config, slot: &ServerSlot) -> String {
    let Ok(content) = std::fs::read_to_string(runtime_slot_override_path(slot)) else {
        return slot.map_key.clone();
    };
    let Some(ark_map) = env_value(&content, "ARK_MAP") else {
        return slot.map_key.clone();
    };
    config
        .maps
        .iter()
        .find(|map| normalize(&map.ark_map_name) == normalize(&ark_map))
        .map(|map| map.id.clone())
        .unwrap_or_else(|| slot.map_key.clone())
}

pub fn runtime_slot_override_path(slot: &ServerSlot) -> std::path::PathBuf {
    runtime_slot_override_root().join(format!("{}.env", slot.id))
}

fn runtime_slot_override_root() -> std::path::PathBuf {
    if let Ok(path) = std::env::var("ARK_MANAGER_RUNTIME_SLOT_DIR") {
        return path.into();
    }
    if cfg!(test) {
        return std::path::Path::new("/tmp/ark-manager-test-runtime-slots").into();
    }
    std::path::Path::new("/var/lib/ark-cluster-manager/runtime-slots").into()
}

fn write_runtime_slot_override(slot: &ServerSlot, map: &MapConfig) -> Result<(), std::io::Error> {
    let path = runtime_slot_override_path(slot);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("env.tmp");
    let mut file = std::fs::File::create(&tmp)?;
    writeln!(
        file,
        "# Managed by ark-cluster-manager. Do not put secrets here."
    )?;
    writeln!(file, "ARK_MAP={}", shell_quote(&map.ark_map_name))?;
    writeln!(
        file,
        "ARK_SESSION_NAME={}",
        shell_quote(&format!("ARK {}", map.name))
    )?;
    writeln!(file, "ARK_EFFECTIVE_MAP_ID={}", shell_quote(&map.id))?;
    file.sync_all()?;
    std::fs::rename(tmp, path)?;
    Ok(())
}

fn env_value(content: &str, key: &str) -> Option<String> {
    content.lines().find_map(|line| {
        let line = line.trim();
        let (k, value) = line.split_once('=')?;
        if k.trim() != key {
            return None;
        }
        Some(
            value
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .replace("'\\''", "'"),
        )
    })
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn aliases_for(id: &str, alias: &str, ark: &str, name: &str) -> Vec<String> {
    let mut out = vec![id.into(), alias.into(), ark.into(), name.into()];
    match id {
        "the-island" | "home-island" => out.extend(["island", "theisland"].map(str::to_string)),
        "the-center" => out.extend(["center"].map(str::to_string)),
        "scorched-earth" => out.extend(["scorched", "scorched earth"].map(str::to_string)),
        "extinction" => out.extend(["ext"].map(str::to_string)),
        "ragnarok" | "travel-rag" => out.extend(["rag"].map(str::to_string)),
        "aberration" | "travel-ab" => out.extend(["abb"].map(str::to_string)),
        "valguero" => out.extend(["val", "valg"].map(str::to_string)),
        "genesis-1" => {
            out.extend(["gen1", "genesis", "genesis 1", "genesis part 1"].map(str::to_string))
        }
        "crystal-isles" => out.extend(["crystal", "crystal isles"].map(str::to_string)),
        "genesis-2" => out.extend(["gen2", "genesis 2", "genesis part 2"].map(str::to_string)),
        "lost-island" => out.extend(["lost", "lost island"].map(str::to_string)),
        "fjordur" | "map-fjordur" => out.extend(["fjord"].map(str::to_string)),
        _ => {}
    }
    out
}

fn known_official_map(raw: &str) -> bool {
    let key = normalize(raw);
    official_aliases()
        .iter()
        .any(|alias| normalize(alias) == key)
}

fn official_aliases() -> &'static [&'static str] {
    &[
        "the-island",
        "The Island",
        "TheIsland",
        "island",
        "scorched-earth",
        "Scorched Earth",
        "ScorchedEarth_P",
        "scorched",
        "aberration",
        "Aberration_P",
        "abb",
        "extinction",
        "ext",
        "genesis-1",
        "Genesis: Part 1",
        "genesis 1",
        "Genesis",
        "gen1",
        "genesis part 1",
        "genesis-2",
        "Genesis: Part 2",
        "genesis 2",
        "Gen2",
        "gen2",
        "genesis part 2",
        "the-center",
        "The Center",
        "TheCenter",
        "center",
        "ragnarok",
        "rag",
        "valguero",
        "Valguero_P",
        "val",
        "valg",
        "crystal-isles",
        "Crystal Isles",
        "CrystalIsles",
        "crystal",
        "crystal isles",
        "lost-island",
        "Lost Island",
        "LostIsland",
        "lost",
        "lost island",
        "fjordur",
        "fjord",
    ]
}

fn normalize(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

fn epoch_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_alias() {
        let cfg = crate::config::tests_support::base_config();
        assert_eq!(resolve_map(&cfg, "rag").unwrap().id, "rag");
    }

    #[test]
    fn recognizes_unconfigured_official_alias() {
        for alias in [
            "island",
            "the island",
            "scorched earth",
            "abb",
            "ext",
            "genesis 1",
            "genesis part 1",
            "genesis 2",
            "genesis part 2",
            "the center",
            "rag",
            "valg",
            "crystal isles",
            "lost island",
            "fjord",
            "fjordur",
        ] {
            assert!(
                known_official_map(alias),
                "alias {alias} should be official"
            );
        }
        assert!(!known_official_map("not-a-map"));
    }

    #[tokio::test]
    async fn unknown_active_counts_do_not_reuse_slot() {
        let mut cfg = crate::config::tests_support::base_config();
        cfg.operations.travel_scheduler_enabled = true;
        cfg.maps.push(MapConfig {
            id: "fjordur".into(),
            name: "Fjordur".into(),
            alias: "fjordur".into(),
            ark_map_name: "Fjordur".into(),
            systemd_unit: "ark-map@fjordur.service".into(),
            query_port: 27030,
            rcon_port: 27031,
            game_port: 7799,
            slot_priority: 5,
            can_be_home: false,
            can_auto_stop_when_empty: true,
            can_enter_standby: false,
            assignment: "Unassigned".into(),
            mods: vec![],
        });
        let pool = crate::db::init(":memory:").await.unwrap();
        let slots = cfg.slots.as_ref().unwrap();
        let statuses = vec![
            SlotStatusSnapshot {
                slot: slots.home.clone(),
                key: "home",
                status: active_status(&slots.home.systemd_unit),
                player_count: Some(0),
            },
            SlotStatusSnapshot {
                slot: slots.travel_a.as_ref().unwrap().clone(),
                key: "travel_a",
                status: active_status(&slots.travel_a.as_ref().unwrap().systemd_unit),
                player_count: None,
            },
            SlotStatusSnapshot {
                slot: slots.travel_b.as_ref().unwrap().clone(),
                key: "travel_b",
                status: active_status(&slots.travel_b.as_ref().unwrap().systemd_unit),
                player_count: None,
            },
        ];

        let decision = decide(
            &pool,
            &cfg,
            TravelRequestBody {
                map: "fjordur".into(),
                source: "test".into(),
                actor: "tester".into(),
            },
            statuses,
        )
        .await
        .unwrap();

        assert!(!decision.accepted);
        assert_eq!(decision.status, "queued");
    }

    fn active_status(unit: &str) -> UnitStatus {
        UnitStatus {
            unit: unit.into(),
            source: "test".into(),
            exists: true,
            loaded: true,
            state: "active (running)".into(),
            active: true,
            active_state: "active".into(),
            sub_state: "running".into(),
            description: None,
            since: None,
            main_pid: Some(1),
            memory_current_bytes: None,
            tasks_current: None,
            error: None,
        }
    }
}
