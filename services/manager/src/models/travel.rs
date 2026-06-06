use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::config::{Config, MapConfig, ServerSlot};
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
    slot_statuses: Vec<(ServerSlot, &'static str, UnitStatus, u32)>,
) -> Result<TravelDecision, sqlx::Error> {
    let id = format!("travel-{}", epoch_secs());
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
    if let Some((slot, key, _, _)) = slot_statuses
        .iter()
        .find(|(slot, _, status, _)| slot.map_key == map.id && status.active)
    {
        let decision = TravelDecision {
            id,
            accepted: true,
            requested_map: req.map.clone(),
            resolved_map: Some(map.id.clone()),
            chosen_slot: Some(slot.id.clone()),
            status: "already_online".into(),
            reason: format!("map already online in {key}"),
        };
        insert_history(pool, &decision, &req, key).await?;
        return Ok(decision);
    }
    let free = slot_statuses
        .iter()
        .find(|(_, key, status, _)| *key != "home" && !status.active);
    let Some((slot, key, _, _)) = free else {
        let empty = slot_statuses
            .iter()
            .find(|(_, key, status, players)| *key != "home" && status.active && *players == 0);
        if let Some((slot, key, _, _)) = empty {
            let decision = TravelDecision {
                id,
                accepted: true,
                requested_map: req.map.clone(),
                resolved_map: Some(map.id.clone()),
                chosen_slot: Some(slot.id.clone()),
                status: "accepted_reuse_empty_slot".into(),
                reason: "empty active travel slot can be backed up and reused".into(),
            };
            insert_history(pool, &decision, &req, key).await?;
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
        chosen_slot: Some(slot.id.clone()),
        status: "accepted".into(),
        reason: format!("free travel slot {key} selected"),
    };
    insert_history(pool, &decision, &req, key).await?;
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

fn aliases_for(id: &str, alias: &str, ark: &str, name: &str) -> Vec<String> {
    let mut out = vec![id.into(), alias.into(), ark.into(), name.into()];
    match id {
        "the-island" | "home-island" => out.extend(["island", "theisland"].map(str::to_string)),
        "the-center" => out.extend(["center"].map(str::to_string)),
        "scorched-earth" => out.extend(["scorched"].map(str::to_string)),
        "ragnarok" | "travel-rag" => out.extend(["rag"].map(str::to_string)),
        "aberration" | "travel-ab" => out.extend(["abb"].map(str::to_string)),
        "valguero" => out.extend(["val"].map(str::to_string)),
        "genesis-1" => out.extend(["gen1", "genesis"].map(str::to_string)),
        "crystal-isles" => out.extend(["crystal"].map(str::to_string)),
        "genesis-2" => out.extend(["gen2"].map(str::to_string)),
        "lost-island" => out.extend(["lost"].map(str::to_string)),
        "fjordur" | "map-fjordur" => out.extend(["fjord"].map(str::to_string)),
        _ => {}
    }
    out
}

fn known_official_map(raw: &str) -> bool {
    let key = normalize(raw);
    official_aliases().iter().any(|alias| normalize(alias) == key)
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
        "genesis-1",
        "Genesis: Part 1",
        "Genesis",
        "gen1",
        "genesis-2",
        "Genesis: Part 2",
        "Gen2",
        "gen2",
        "the-center",
        "The Center",
        "TheCenter",
        "center",
        "ragnarok",
        "rag",
        "valguero",
        "Valguero_P",
        "val",
        "crystal-isles",
        "Crystal Isles",
        "CrystalIsles",
        "crystal",
        "lost-island",
        "Lost Island",
        "LostIsland",
        "lost",
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

fn epoch_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
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
        assert!(known_official_map("crystal"));
        assert!(known_official_map("Genesis Part 2"));
        assert!(!known_official_map("not-a-map"));
    }
}
