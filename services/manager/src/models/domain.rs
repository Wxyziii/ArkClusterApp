//! Domain types mirroring the frontend contract in `src/lib/types.ts`.
//!
//! All structs serialize with `camelCase` field names so the SvelteKit UI can
//! consume them without a translation layer.

use serde::{Serialize, Serializer};

use crate::models::systemd::UnitStatus;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Player {
    pub name: String,
    pub level: u32,
    pub tribe: String,
    pub connected_mins: u32,
    pub map: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapConfigSummary {
    pub systemd_unit: String,
    pub ark_map_name: String,
    pub query_port: u16,
    pub rcon_port: u16,
    pub game_port: u16,
    pub slot_priority: i32,
    pub auto_shutdown_enabled: bool,
    pub can_be_home: bool,
    pub can_auto_stop_when_empty: bool,
    pub can_enter_standby: bool,
    pub mod_list: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArkMap {
    pub id: String,
    pub name: String,
    pub alias: String,
    pub role: String,
    pub assignment: String,
    pub state: String,
    pub players: u32,
    pub player_count_source: String,
    #[serde(serialize_with = "zero_as_null")]
    pub max_players: u32,
    pub max_players_source: String,
    pub ram_mb: u32,
    pub ram_estimate_mb: u32,
    pub uptime_mins: u32,
    pub idle_mins: u32,
    pub last_backup: String,
    pub rcon: String,
    pub systemd: String,
    pub restart_required: bool,
    pub cpu_pct: u32,
    pub save_size_mb: u32,
    pub is_home: bool,
    pub protected: bool,
    pub configured: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slot_id: Option<String>,
    pub slot_role: String,
    pub next_action: String,
    pub config: MapConfigSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub systemd_detail: Option<UnitStatus>,
}

fn zero_as_null<S>(value: &u32, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if *value == 0 {
        serializer.serialize_none()
    } else {
        serializer.serialize_some(value)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TravelRequest {
    pub id: String,
    pub map: String,
    pub requested_by: String,
    pub source: String,
    pub source_raw: String,
    pub source_map: String,
    pub step: u32,
    pub result: String,
    pub reason: String,
    pub at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSample {
    pub source: String,
    pub ram_used_gb: f64,
    pub ram_total_gb: f64,
    pub ram_available_gb: f64,
    pub cpu_pct: u32,
    pub swap_used_gb: f64,
    pub swap_total_gb: f64,
    pub disk_used_gb: f64,
    pub disk_total_gb: f64,
    pub disk_free_gb: f64,
    pub ark_proc_mem_gb: f64,
    pub load1: f64,
    pub load5: f64,
    pub load15: f64,
    pub manager_uptime_secs: u64,
    pub system_uptime_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEvent {
    pub id: String,
    pub ts: String,
    pub severity: String,
    pub source: String,
    pub actor: String,
    pub target_map: String,
    pub message: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Backup {
    pub id: String,
    pub map: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub size_mb: u32,
    pub created: String,
    pub reason: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mod {
    pub id: String,
    pub name: String,
    pub workshop_id: String,
    pub enabled: bool,
    pub installed: bool,
    pub load_order: u32,
    pub last_updated: String,
    pub size_mb: u32,
    pub used_by: Vec<String>,
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigField {
    pub key: String,
    pub label: String,
    pub value: serde_json::Value, // number | bool | string
    #[serde(rename = "type")]
    pub kind: String,
    pub group: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
    pub hint: String,
    pub restart_required: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscordEvent {
    pub id: String,
    pub ts: String,
    pub kind: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertSetting {
    pub key: String,
    pub label: String,
    pub enabled: bool,
}
