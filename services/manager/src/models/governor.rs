//! Resource governor — DATA MODEL ONLY for Phase 1.
//!
//! This phase does NOT make any real decisions and never starts or stops a
//! server. The structs below model the inputs (resource snapshot) and the
//! policy that a future governor loop will evaluate. The `evaluate` helper is a
//! pure, side-effect-free preview that returns a human-readable decision string
//! so the UI can show what the governor *would* consider — it actuates nothing.

use serde::Serialize;

use crate::config::ResourcePolicy;
use crate::models::domain::ResourceSample;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GovernorPolicy {
    pub never_stop_with_players: bool,
    pub home_standby_enabled: bool,
    pub home_stops_only_when_empty: bool,
    pub prefer_active_player_maps: bool,
    pub auto_restart_home: bool,
    pub max_travel_servers: u32,
    pub empty_shutdown_mins: u32,
}

impl From<&ResourcePolicy> for GovernorPolicy {
    fn from(p: &ResourcePolicy) -> Self {
        Self {
            never_stop_with_players: p.never_stop_with_players,
            home_standby_enabled: p.home_standby_enabled,
            home_stops_only_when_empty: p.home_stops_only_when_empty,
            prefer_active_player_maps: p.prefer_active_player_maps,
            auto_restart_home: p.auto_restart_home,
            max_travel_servers: p.max_travel_servers,
            empty_shutdown_mins: p.empty_shutdown_mins,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GovernorThresholds {
    pub ram_warn_pct: u8,
    pub ram_pressure_pct: u8,
    pub ram_emergency_pct: u8,
    pub max_travel: u32,
    pub empty_shutdown_mins: u32,
}

/// A non-actuating preview of the governor's reasoning.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GovernorDecision {
    pub decision: String,
    pub why: String,
    pub examples: Vec<String>,
    pub policy: GovernorPolicy,
}

/// Pure preview. Given a snapshot + counts, describe what the governor would
/// consider. Returns text only — NO server is ever touched here.
pub fn evaluate(
    policy: &ResourcePolicy,
    sample: &ResourceSample,
    home_players: u32,
    travel_players: u32,
) -> GovernorDecision {
    let ram_pct = ((sample.ram_used_gb / sample.ram_total_gb) * 100.0).round() as u8;
    let under_pressure = ram_pct >= policy.ram_pressure_pct;

    let (decision, why) = if home_players == 0 && travel_players > 0 && under_pressure {
        (
            "Home eligible for Resource Standby".to_string(),
            format!(
                "Home has 0 players and RAM pressure is high ({ram_pct}% >= {}%). Travel maps have active players. \
                 Home could be saved, backed up, and stopped so travel players keep playing. (Preview only — no action taken.)",
                policy.ram_pressure_pct
            ),
        )
    } else if travel_players > 0 && !under_pressure {
        (
            "No action needed".to_string(),
            format!(
                "Resources healthy ({ram_pct}%). Travel maps have players; nothing to reclaim."
            ),
        )
    } else {
        (
            "Monitoring".to_string(),
            format!("RAM at {ram_pct}%. No standby/shutdown conditions met. (Preview only.)"),
        )
    };

    GovernorDecision {
        decision,
        why,
        examples: vec![
            "All on-demand slots have active players. New travel requests are blocked.".into(),
            "Home has 0 players and RAM pressure is high. Home is eligible for Resource Standby."
                .into(),
            format!(
                "A travel map empty for {} minutes is eligible for save, backup, and shutdown.",
                policy.empty_shutdown_mins
            ),
            "No action needed. Resources are healthy.".into(),
        ],
        policy: GovernorPolicy::from(policy),
    }
}
