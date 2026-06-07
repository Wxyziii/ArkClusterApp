use serde::{Deserialize, Serialize};

use crate::config::{Config, ServerSlot};
use crate::models::domain::ResourceSample;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ServerAction {
    Start,
    Stop,
    Restart,
    Backup,
}

impl ServerAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Stop => "stop",
            Self::Restart => "restart",
            Self::Backup => "backup",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionRequest {
    #[serde(default)]
    pub confirm: bool,
    #[serde(default)]
    pub strong_confirm: bool,
    #[serde(default)]
    pub admin_override: bool,
    #[serde(default = "default_reason")]
    pub reason: String,
}

fn default_reason() -> String {
    "manual".into()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityItem {
    pub enabled: bool,
    pub available: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub systemd_control: CapabilityItem,
    pub backup: CapabilityItem,
    pub rcon: CapabilityItem,
    pub discord: CapabilityItem,
    pub config_writes: CapabilityItem,
    pub mod_management: CapabilityItem,
    pub restore: CapabilityItem,
    pub travel_scheduler: CapabilityItem,
    pub maintenance: CapabilityItem,
    pub mode: String,
    pub backend_source: String,
}

pub fn capabilities(config: &Config, backend_source: &str) -> Capabilities {
    let ops = &config.operations;
    Capabilities {
        systemd_control: item(
            ops.systemd_control_enabled,
            ops.systemd_control_enabled,
            "systemd control disabled in manager config",
        ),
        backup: item(
            ops.backup_enabled,
            ops.backup_enabled,
            "backup disabled in manager config",
        ),
        rcon: item(
            ops.rcon_enabled && config.rcon.enabled,
            ops.rcon_enabled && config.rcon.enabled,
            "RCON disabled or unconfigured",
        ),
        discord: item(false, false, "Discord actions are not implemented"),
        config_writes: item(
            ops.config_writes_enabled,
            ops.config_writes_enabled,
            "config writes disabled in manager config",
        ),
        mod_management: item(
            ops.mod_management_enabled,
            ops.mod_management_enabled,
            "mod management disabled in manager config",
        ),
        restore: item(false, false, "backup restore is not implemented"),
        travel_scheduler: item(
            ops.travel_scheduler_enabled,
            ops.travel_scheduler_enabled,
            "travel scheduler disabled in manager config",
        ),
        maintenance: item(
            ops.maintenance_enabled,
            ops.maintenance_enabled,
            "ARK maintenance disabled in manager config",
        ),
        mode: "guarded_operations_foundation".into(),
        backend_source: backend_source.into(),
    }
}

fn item(enabled: bool, available: bool, disabled_reason: &str) -> CapabilityItem {
    CapabilityItem {
        enabled,
        available,
        reason: if enabled && available {
            "available".into()
        } else {
            disabled_reason.into()
        },
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardError {
    ControlDisabled,
    BackupDisabled,
    InvalidConfirmation,
    HomeProtected,
    ResourcePolicyBlocked(String),
    PlayersOnline,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TravelResourceGuardStatus {
    pub enabled: bool,
    pub allowed: bool,
    pub reason: String,
    pub active_travel_slots: usize,
    pub max_travel_servers: u32,
    pub sample_source: String,
    pub min_available_ram_mb: u32,
    pub available_ram_mb: u32,
    pub ram_used_percent: u32,
    pub max_ram_used_percent: u8,
    pub swap_used_percent: u32,
    pub max_swap_used_percent: u8,
    pub free_swap_mb: u32,
    pub min_free_swap_mb: u32,
    pub disk_free_gb: f64,
    pub min_disk_free_gb: u32,
    /// Map-specific memory estimate used for the RAM floor calculation (0 when not map-aware).
    pub map_memory_estimate_mb: u32,
}

impl GuardError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::ControlDisabled => "CONTROL_DISABLED",
            Self::BackupDisabled => "BACKUP_DISABLED",
            Self::InvalidConfirmation => "INVALID_CONFIRMATION",
            Self::HomeProtected => "HOME_PROTECTED",
            Self::ResourcePolicyBlocked(_) => "RESOURCE_POLICY_BLOCKED",
            Self::PlayersOnline => "PLAYERS_ONLINE",
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::ControlDisabled => "Systemd control is disabled in manager config.".into(),
            Self::BackupDisabled => "Backups are disabled in manager config.".into(),
            Self::InvalidConfirmation => "This action requires explicit confirmation.".into(),
            Self::HomeProtected => "Home stop is protected by manager policy.".into(),
            Self::ResourcePolicyBlocked(msg) => msg.clone(),
            Self::PlayersOnline => {
                "Players are online; explicit admin override is required.".into()
            }
        }
    }
}

pub struct SystemdGuardInput<'a> {
    pub config: &'a Config,
    pub slot_key: &'a str,
    pub slot: &'a ServerSlot,
    pub action: ServerAction,
    pub req: &'a ActionRequest,
    pub sample: &'a ResourceSample,
    pub active_travel_slots: usize,
    pub player_count: u32,
    /// ARK map name for the destination slot (e.g. "Genesis"). None → use legacy generic thresholds.
    pub map_ark_name: Option<String>,
}

pub fn guard_systemd_action(input: SystemdGuardInput<'_>) -> Result<(), GuardError> {
    let config = input.config;
    let slot_key = input.slot_key;
    let slot = input.slot;
    let action = input.action;
    let req = input.req;
    let sample = input.sample;
    let active_travel_slots = input.active_travel_slots;
    let player_count = input.player_count;
    if !config.operations.systemd_control_enabled {
        return Err(GuardError::ControlDisabled);
    }
    if !slot.enabled {
        return Err(GuardError::ResourcePolicyBlocked(
            "slot is disabled in config".into(),
        ));
    }
    let needs_confirm = matches!(action, ServerAction::Stop | ServerAction::Restart);
    if needs_confirm && config.operations.require_confirmation_token && !req.confirm {
        return Err(GuardError::InvalidConfirmation);
    }

    if slot_key == "home" {
        if action == ServerAction::Stop {
            let allowed_reason = matches!(
                req.reason.as_str(),
                "manual_admin_override" | "resource_standby_preparation"
            );
            if !config.operations.allow_home_manual_stop || !req.strong_confirm || !allowed_reason {
                return Err(GuardError::HomeProtected);
            }
        }
        return Ok(());
    }

    if matches!(action, ServerAction::Stop | ServerAction::Restart)
        && player_count > 0
        && !req.admin_override
    {
        return Err(GuardError::PlayersOnline);
    }
    if action == ServerAction::Stop && !config.operations.allow_travel_manual_stop {
        return Err(GuardError::ControlDisabled);
    }
    if action == ServerAction::Start {
        guard_travel_start(config, sample, active_travel_slots, input.map_ark_name.as_deref())?;
    }
    Ok(())
}

pub fn guard_backup(config: &Config, req: &ActionRequest) -> Result<(), GuardError> {
    if !config.operations.backup_enabled {
        return Err(GuardError::BackupDisabled);
    }
    if config.operations.require_confirmation_token && !req.confirm {
        return Err(GuardError::InvalidConfirmation);
    }
    Ok(())
}

fn guard_travel_start(
    config: &Config,
    sample: &ResourceSample,
    active_travel_slots: usize,
    map_ark_name: Option<&str>,
) -> Result<(), GuardError> {
    let status = travel_resource_guard_status(config, sample, active_travel_slots, map_ark_name);
    if status.allowed {
        Ok(())
    } else {
        Err(GuardError::ResourcePolicyBlocked(status.reason))
    }
}

pub fn travel_resource_guard_status(
    config: &Config,
    sample: &ResourceSample,
    active_travel_slots: usize,
    map_ark_name: Option<&str>,
) -> TravelResourceGuardStatus {
    let guard = &config.resource_guard;
    let max_travel_servers = config.resource_policy.max_travel_servers;
    let (min_available_ram_mb, map_memory_estimate_mb) = match map_ark_name {
        Some(name) => {
            let estimate = guard.map_estimate_mb(name);
            let extra = if active_travel_slots == 0 {
                0
            } else {
                guard.second_slot_extra_mb
            };
            (estimate + guard.map_memory_reserve_mb + extra, estimate)
        }
        None => {
            let threshold = if active_travel_slots == 0 {
                guard.min_available_ram_mb_for_first_travel
            } else {
                guard.min_available_ram_mb_for_second_travel
            };
            (threshold, 0)
        }
    };
    let available_ram_mb = gb_to_mb(sample.ram_available_gb);
    let ram_used_percent = pct_u32(sample.ram_used_gb, sample.ram_total_gb);
    let swap_used_percent = pct_u32(sample.swap_used_gb, sample.swap_total_gb);
    let free_swap_mb = gb_to_mb((sample.swap_total_gb - sample.swap_used_gb).max(0.0));
    let resources_unknown = sample.source != "host" || sample.ram_total_gb <= 0.0;

    let (allowed, reason) = if !guard.enabled {
        (true, "Resource guard disabled by config.".into())
    } else if active_travel_slots >= max_travel_servers as usize {
        (
            false,
            format!(
                "Travel rejected: active travel slot count {active_travel_slots} is already at max {max_travel_servers}."
            ),
        )
    } else if resources_unknown && guard.block_on_unknown_resources {
        (
            false,
            "Travel rejected: resource sample unavailable; refusing to start an on-demand map."
                .into(),
        )
    } else if resources_unknown {
        (
            true,
            "Resource sample unavailable; guard skipped by config.".into(),
        )
    } else if available_ram_mb < min_available_ram_mb {
        (
            false,
            format!(
                "Travel rejected: not enough available RAM ({available_ram_mb} MB available, need {min_available_ram_mb} MB)."
            ),
        )
    } else if ram_used_percent > guard.max_ram_used_percent_before_travel as u32 {
        (
            false,
            format!(
                "Travel rejected: RAM usage is too high ({ram_used_percent}% used, max {}%).",
                guard.max_ram_used_percent_before_travel
            ),
        )
    } else if swap_used_percent > guard.max_swap_used_percent as u32 {
        (
            false,
            format!(
                "Travel rejected: swap usage is too high ({swap_used_percent}% used, max {}%).",
                guard.max_swap_used_percent
            ),
        )
    } else if free_swap_mb < guard.min_free_swap_mb {
        (
            false,
            format!(
                "Travel rejected: free swap is too low ({free_swap_mb} MB free, need {} MB).",
                guard.min_free_swap_mb
            ),
        )
    } else if sample.disk_free_gb < guard.min_disk_free_gb as f64 {
        (
            false,
            format!(
                "Travel rejected: disk free space is too low ({:.1} GB free, need {} GB).",
                sample.disk_free_gb, guard.min_disk_free_gb
            ),
        )
    } else {
        (true, "Resource guard passed.".into())
    };

    TravelResourceGuardStatus {
        enabled: guard.enabled,
        allowed,
        reason,
        active_travel_slots,
        max_travel_servers,
        sample_source: sample.source.clone(),
        min_available_ram_mb,
        available_ram_mb,
        ram_used_percent,
        max_ram_used_percent: guard.max_ram_used_percent_before_travel,
        swap_used_percent,
        max_swap_used_percent: guard.max_swap_used_percent,
        free_swap_mb,
        min_free_swap_mb: guard.min_free_swap_mb,
        disk_free_gb: sample.disk_free_gb,
        min_disk_free_gb: guard.min_disk_free_gb,
        map_memory_estimate_mb,
    }
}

fn pct_u32(used: f64, total: f64) -> u32 {
    if total <= 0.0 || !used.is_finite() || !total.is_finite() {
        0
    } else {
        ((used / total) * 100.0).round().clamp(0.0, u32::MAX as f64) as u32
    }
}

fn gb_to_mb(value: f64) -> u32 {
    if value <= 0.0 || !value.is_finite() {
        0
    } else {
        (value * 1024.0).round().clamp(0.0, u32::MAX as f64) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_data;

    fn cfg() -> Config {
        let mut c = crate::config::tests_support::base_config();
        c.operations.systemd_control_enabled = true;
        c
    }

    fn safe_sample() -> ResourceSample {
        let mut sample = test_data::resources();
        sample.source = "host".into();
        sample.ram_used_gb = 5.0;
        sample.ram_total_gb = 16.0;
        sample.ram_available_gb = 9.0;
        sample.swap_used_gb = 0.2;
        sample.swap_total_gb = 4.0;
        sample.disk_free_gb = 100.0;
        sample
    }

    #[test]
    fn home_stop_blocked_by_default() {
        let mut c = cfg();
        c.operations.allow_home_manual_stop = false;
        let slot = &c.slots.as_ref().unwrap().home;
        let req = ActionRequest {
            confirm: true,
            strong_confirm: true,
            admin_override: true,
            reason: "manual_admin_override".into(),
        };
        assert_eq!(
            guard_systemd_action(SystemdGuardInput {
                config: &c,
                slot_key: "home",
                slot,
                action: ServerAction::Stop,
                req: &req,
                sample: &test_data::resources(),
                active_travel_slots: 0,
                player_count: 0,
                map_ark_name: None,
            }),
            Err(GuardError::HomeProtected)
        );
    }

    #[test]
    fn missing_confirmation_rejected() {
        let c = cfg();
        let slot = c.slots.as_ref().unwrap().travel_a.as_ref().unwrap();
        let req = ActionRequest {
            confirm: false,
            strong_confirm: false,
            admin_override: false,
            reason: "manual".into(),
        };
        assert_eq!(
            guard_systemd_action(SystemdGuardInput {
                config: &c,
                slot_key: "travel_a",
                slot,
                action: ServerAction::Restart,
                req: &req,
                sample: &test_data::resources(),
                active_travel_slots: 0,
                player_count: 0,
                map_ark_name: None,
            }),
            Err(GuardError::InvalidConfirmation)
        );
    }

    #[test]
    fn travel_start_resource_blocked() {
        let c = cfg();
        let slot = c.slots.as_ref().unwrap().travel_a.as_ref().unwrap();
        let mut sample = test_data::resources();
        sample.ram_available_gb = 0.5;
        let req = ActionRequest {
            confirm: false,
            strong_confirm: false,
            admin_override: false,
            reason: "manual".into(),
        };
        assert!(matches!(
            guard_systemd_action(SystemdGuardInput {
                config: &c,
                slot_key: "travel_a",
                slot,
                action: ServerAction::Start,
                req: &req,
                sample: &sample,
                active_travel_slots: 0,
                player_count: 0,
                map_ark_name: None,
            }),
            Err(GuardError::ResourcePolicyBlocked(_))
        ));
    }

    #[test]
    fn travel_resource_guard_allows_first_travel_when_safe() {
        let c = cfg();
        let status = travel_resource_guard_status(&c, &safe_sample(), 0, None);
        assert!(status.allowed);
        assert_eq!(status.reason, "Resource guard passed.");
        assert_eq!(status.min_available_ram_mb, 6144);
    }

    #[test]
    fn travel_resource_guard_rejects_unknown_resource_sample() {
        let c = cfg();
        let status = travel_resource_guard_status(&c, &test_data::resources(), 0, None);
        assert!(!status.allowed);
        assert!(status.reason.contains("resource sample unavailable"));
    }

    #[test]
    fn travel_resource_guard_can_skip_unknown_resource_sample_by_config() {
        let mut c = cfg();
        c.resource_guard.block_on_unknown_resources = false;
        let status = travel_resource_guard_status(&c, &test_data::resources(), 0, None);
        assert!(status.allowed);
        assert!(status.reason.contains("skipped by config"));
    }

    #[test]
    fn travel_resource_guard_rejects_low_available_ram_for_first_slot() {
        let c = cfg();
        let mut sample = safe_sample();
        sample.ram_available_gb = 5.0;
        let status = travel_resource_guard_status(&c, &sample, 0, None);
        assert!(!status.allowed);
        assert!(status.reason.contains("not enough available RAM"));
    }

    #[test]
    fn travel_resource_guard_rejects_second_slot_with_stricter_ram_floor() {
        let c = cfg();
        let sample = safe_sample();
        assert!(travel_resource_guard_status(&c, &sample, 0, None).allowed);
        let status = travel_resource_guard_status(&c, &sample, 1, None);
        assert!(!status.allowed);
        assert_eq!(status.min_available_ram_mb, 10240);
        assert!(status.reason.contains("not enough available RAM"));
    }

    #[test]
    fn travel_resource_guard_rejects_high_ram_used_percent() {
        let c = cfg();
        let mut sample = safe_sample();
        sample.ram_used_gb = 13.0;
        sample.ram_available_gb = 10.0;
        let status = travel_resource_guard_status(&c, &sample, 0, None);
        assert!(!status.allowed);
        assert!(status.reason.contains("RAM usage is too high"));
    }

    #[test]
    fn travel_resource_guard_rejects_high_swap_used_percent() {
        let c = cfg();
        let mut sample = safe_sample();
        sample.swap_used_gb = 2.0;
        sample.swap_total_gb = 4.0;
        let status = travel_resource_guard_status(&c, &sample, 0, None);
        assert!(!status.allowed);
        assert!(status.reason.contains("swap usage is too high"));
    }

    #[test]
    fn travel_resource_guard_rejects_low_free_swap() {
        let mut c = cfg();
        c.resource_guard.max_swap_used_percent = 90;
        let mut sample = safe_sample();
        sample.swap_used_gb = 2.2;
        sample.swap_total_gb = 4.0;
        let status = travel_resource_guard_status(&c, &sample, 0, None);
        assert!(!status.allowed);
        assert!(status.reason.contains("free swap is too low"));
    }

    #[test]
    fn travel_resource_guard_rejects_low_disk_free_space() {
        let c = cfg();
        let mut sample = safe_sample();
        sample.disk_free_gb = 9.0;
        let status = travel_resource_guard_status(&c, &sample, 0, None);
        assert!(!status.allowed);
        assert!(status.reason.contains("disk free space is too low"));
    }

    #[test]
    fn travel_resource_guard_rejects_max_active_travel_slots() {
        let c = cfg();
        let status = travel_resource_guard_status(&c, &safe_sample(), 2, None);
        assert!(!status.allowed);
        assert!(status.reason.contains("already at max"));
    }

    #[test]
    fn map_aware_guard_rejects_genesis_with_10gb_available() {
        let c = cfg();
        let mut sample = safe_sample();
        // 10 GB available — passes legacy 6 GB threshold but fails Genesis (11 GB + 1.5 GB reserve = 12.5 GB)
        sample.ram_available_gb = 10.0;
        let status = travel_resource_guard_status(&c, &sample, 0, Some("Genesis"));
        assert!(!status.allowed, "Genesis must be rejected when only 10 GB free");
        assert!(status.reason.contains("not enough available RAM"));
        assert_eq!(status.map_memory_estimate_mb, 11_000);
        assert_eq!(status.min_available_ram_mb, 11_000 + 1_500); // estimate + reserve
    }

    #[test]
    fn map_aware_guard_allows_genesis_with_13gb_available() {
        let c = cfg();
        let mut sample = safe_sample();
        sample.ram_available_gb = 13.0;
        let status = travel_resource_guard_status(&c, &sample, 0, Some("Genesis"));
        assert!(status.allowed, "Genesis must pass with 13 GB free");
        assert_eq!(status.map_memory_estimate_mb, 11_000);
    }

    #[test]
    fn map_aware_guard_uses_config_override_over_builtin() {
        let mut c = cfg();
        c.resource_guard
            .map_memory_mb
            .insert("Ragnarok".into(), 10_000);
        let mut sample = safe_sample();
        sample.ram_available_gb = 12.0;
        // config override 10000 + 1500 reserve = 11500 → should pass
        let status = travel_resource_guard_status(&c, &sample, 0, Some("Ragnarok"));
        assert!(status.allowed);
        assert_eq!(status.map_memory_estimate_mb, 10_000);
    }

    #[test]
    fn map_aware_guard_uses_unknown_map_memory_for_unrecognized_map() {
        let c = cfg();
        let mut sample = safe_sample();
        sample.ram_available_gb = 10.0;
        // Unknown map → unknown_map_memory_mb default 11000 + 1500 reserve = 12500 MB → reject
        let status = travel_resource_guard_status(&c, &sample, 0, Some("SomeNewMap"));
        assert!(!status.allowed);
        assert_eq!(status.map_memory_estimate_mb, 11_000);
    }
}
