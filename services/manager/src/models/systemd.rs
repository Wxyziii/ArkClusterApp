//! systemd abstraction — SAFE MODEL ONLY for Phase 1.
//!
//! Defines the trait a future controller will implement to manage ARK server
//! instances via systemd template units (`ark-server@<slot>.service`). In this
//! phase the only implementation is [`MockSystemd`], whose start/stop/restart
//! methods are explicitly NOT IMPLEMENTED — they return `NotImplemented` and
//! never shell out. No API route is wired to start/stop in Phase 1.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UnitState {
    ActiveRunning,
    Activating,
    InactiveDead,
    Failed,
}

impl UnitState {
    /// Render in the systemd-style string the UI expects.
    pub fn as_str(&self) -> &'static str {
        match self {
            UnitState::ActiveRunning => "active (running)",
            UnitState::Activating => "activating",
            UnitState::InactiveDead => "inactive (dead)",
            UnitState::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnitStatus {
    pub unit: String,
    pub state: String,
    pub active: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum SystemdError {
    #[error("systemd control is not implemented in this phase (unit: {0})")]
    NotImplemented(String),
}

/// Future controller surface. Implementors will later wrap `systemctl`.
pub trait SystemdController: Send + Sync {
    fn get_status(&self, unit: &str) -> Result<UnitStatus, SystemdError>;
    fn start_unit(&self, unit: &str) -> Result<(), SystemdError>;
    fn stop_unit(&self, unit: &str) -> Result<(), SystemdError>;
    fn restart_unit(&self, unit: &str) -> Result<(), SystemdError>;
}

/// Read-only mock. Reports a fixed state; all mutating calls are refused.
#[derive(Debug, Default, Clone)]
pub struct MockSystemd;

impl SystemdController for MockSystemd {
    fn get_status(&self, unit: &str) -> Result<UnitStatus, SystemdError> {
        // Mock: anything containing "travel-a"/"travel-b" looks running.
        let active = unit.contains("travel-a") || unit.contains("travel-b");
        Ok(UnitStatus {
            unit: unit.to_string(),
            state: if active {
                UnitState::ActiveRunning.as_str().into()
            } else {
                UnitState::InactiveDead.as_str().into()
            },
            active,
        })
    }

    fn start_unit(&self, unit: &str) -> Result<(), SystemdError> {
        Err(SystemdError::NotImplemented(unit.to_string()))
    }
    fn stop_unit(&self, unit: &str) -> Result<(), SystemdError> {
        Err(SystemdError::NotImplemented(unit.to_string()))
    }
    fn restart_unit(&self, unit: &str) -> Result<(), SystemdError> {
        Err(SystemdError::NotImplemented(unit.to_string()))
    }
}
