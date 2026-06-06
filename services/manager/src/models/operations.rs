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
        guard_travel_start(config, sample, active_travel_slots)?;
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
) -> Result<(), GuardError> {
    if active_travel_slots >= config.resource_policy.max_travel_servers as usize {
        return Err(GuardError::ResourcePolicyBlocked(format!(
            "active travel slot count {active_travel_slots} is already at max {}",
            config.resource_policy.max_travel_servers
        )));
    }
    if sample.ram_available_gb < 2.0 {
        return Err(GuardError::ResourcePolicyBlocked(format!(
            "available RAM is too low ({:.1} GB)",
            sample.ram_available_gb
        )));
    }
    if sample.swap_total_gb > 0.0 && sample.swap_used_gb / sample.swap_total_gb > 0.70 {
        return Err(GuardError::ResourcePolicyBlocked(
            "swap pressure is too high to start a travel slot".into(),
        ));
    }
    if sample.disk_free_gb < 10.0 {
        return Err(GuardError::ResourcePolicyBlocked(format!(
            "disk free space is too low ({:.1} GB)",
            sample.disk_free_gb
        )));
    }
    Ok(())
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
            }),
            Err(GuardError::ResourcePolicyBlocked(_))
        ));
    }
}
