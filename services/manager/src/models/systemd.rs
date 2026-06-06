//! systemd abstraction.
//!
//! T1.1 adds read-only unit inspection. Mutating operations remain explicitly
//! not implemented and no API route exposes them.

use std::collections::HashMap;
#[cfg(target_os = "linux")]
use std::time::Duration;

use async_trait::async_trait;
use serde::Serialize;
#[cfg(target_os = "linux")]
use tokio::process::Command;
#[cfg(target_os = "linux")]
use tokio::time;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UnitState {
    ActiveRunning,
    Activating,
    InactiveDead,
    Failed,
    Unknown,
}

impl UnitState {
    pub fn as_str(&self) -> &'static str {
        match self {
            UnitState::ActiveRunning => "active (running)",
            UnitState::Activating => "activating",
            UnitState::InactiveDead => "inactive (dead)",
            UnitState::Failed => "failed",
            UnitState::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnitStatus {
    pub unit: String,
    pub source: String,
    pub exists: bool,
    pub loaded: bool,
    pub state: String,
    pub active: bool,
    pub active_state: String,
    pub sub_state: String,
    pub description: Option<String>,
    pub since: Option<String>,
    pub main_pid: Option<u32>,
    pub memory_current_bytes: Option<u64>,
    pub tasks_current: Option<u64>,
    pub error: Option<String>,
}

impl UnitStatus {
    pub fn unavailable(unit: &str, source: &str, error: impl Into<String>) -> Self {
        Self {
            unit: unit.to_string(),
            source: source.to_string(),
            exists: false,
            loaded: false,
            state: "systemd unavailable".into(),
            active: false,
            active_state: "unavailable".into(),
            sub_state: "unknown".into(),
            description: None,
            since: None,
            main_pid: None,
            memory_current_bytes: None,
            tasks_current: None,
            error: Some(error.into()),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SystemdError {
    #[error("systemd control is not implemented in this phase (unit: {0})")]
    NotImplemented(String),
    #[error("unsafe systemd unit name rejected: {0}")]
    UnsafeUnitName(String),
    #[error("systemctl {action} failed for {unit}: {message}")]
    CommandFailed {
        action: String,
        unit: String,
        message: String,
    },
    #[error("systemctl {action} timed out for {unit}")]
    Timeout { action: String, unit: String },
}

#[async_trait]
pub trait SystemdController: Send + Sync {
    async fn get_status(&self, unit: &str) -> Result<UnitStatus, SystemdError>;
    async fn start_unit(&self, unit: &str) -> Result<(), SystemdError>;
    async fn stop_unit(&self, unit: &str) -> Result<(), SystemdError>;
    async fn restart_unit(&self, unit: &str) -> Result<(), SystemdError>;
}

#[derive(Debug, Default, Clone)]
pub struct RealSystemd;

#[async_trait]
impl SystemdController for RealSystemd {
    async fn get_status(&self, unit: &str) -> Result<UnitStatus, SystemdError> {
        if !is_safe_unit_name(unit) {
            return Err(SystemdError::UnsafeUnitName(unit.to_string()));
        }

        #[cfg(not(target_os = "linux"))]
        {
            return Ok(UnitStatus::unavailable(
                unit,
                "fallback",
                "systemd is only queried on Linux hosts",
            ));
        }

        #[cfg(target_os = "linux")]
        {
            let output = time::timeout(
                Duration::from_secs(2),
                Command::new("systemctl")
                    .args([
                        "show",
                        unit,
                        "--no-page",
                        "--property=Id,LoadState,ActiveState,SubState,Description,ActiveEnterTimestamp,InactiveEnterTimestamp,MainPID,MemoryCurrent,TasksCurrent",
                    ])
                    .output(),
            )
            .await;

            match output {
                Ok(Ok(out)) if out.status.success() => {
                    let text = String::from_utf8_lossy(&out.stdout);
                    Ok(parse_systemctl_show(unit, &text, "systemd"))
                }
                Ok(Ok(out)) => {
                    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                    Ok(UnitStatus::unavailable(
                        unit,
                        "systemd",
                        if stderr.is_empty() {
                            format!("systemctl show exited with {}", out.status)
                        } else {
                            stderr
                        },
                    ))
                }
                Ok(Err(e)) => Ok(UnitStatus::unavailable(
                    unit,
                    "systemd",
                    format!("failed to execute systemctl: {e}"),
                )),
                Err(_) => Ok(UnitStatus::unavailable(
                    unit,
                    "systemd",
                    "systemctl show timed out",
                )),
            }
        }
    }

    async fn start_unit(&self, unit: &str) -> Result<(), SystemdError> {
        run_systemctl_action("start", unit).await
    }
    async fn stop_unit(&self, unit: &str) -> Result<(), SystemdError> {
        run_systemctl_action("stop", unit).await
    }
    async fn restart_unit(&self, unit: &str) -> Result<(), SystemdError> {
        run_systemctl_action("restart", unit).await
    }
}

#[derive(Debug, Default, Clone)]
pub struct TestSystemd;

#[async_trait]
impl SystemdController for TestSystemd {
    async fn get_status(&self, unit: &str) -> Result<UnitStatus, SystemdError> {
        let active = unit.contains("travel-a") || unit.contains("travel-b");
        Ok(UnitStatus {
            unit: unit.to_string(),
            source: "test".into(),
            exists: true,
            loaded: true,
            state: if active {
                UnitState::ActiveRunning.as_str().into()
            } else {
                UnitState::InactiveDead.as_str().into()
            },
            active,
            active_state: if active { "active" } else { "inactive" }.into(),
            sub_state: if active { "running" } else { "dead" }.into(),
            description: Some(format!("Test ARK unit {unit}")),
            since: None,
            main_pid: None,
            memory_current_bytes: None,
            tasks_current: None,
            error: None,
        })
    }

    async fn start_unit(&self, unit: &str) -> Result<(), SystemdError> {
        Err(SystemdError::NotImplemented(unit.to_string()))
    }
    async fn stop_unit(&self, unit: &str) -> Result<(), SystemdError> {
        Err(SystemdError::NotImplemented(unit.to_string()))
    }
    async fn restart_unit(&self, unit: &str) -> Result<(), SystemdError> {
        Err(SystemdError::NotImplemented(unit.to_string()))
    }
}

async fn run_systemctl_action(action: &str, unit: &str) -> Result<(), SystemdError> {
    if !is_safe_unit_name(unit) {
        return Err(SystemdError::UnsafeUnitName(unit.to_string()));
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = action;
        Err(SystemdError::NotImplemented(unit.to_string()))
    }

    #[cfg(target_os = "linux")]
    {
        let output = time::timeout(
            Duration::from_secs(240),
            Command::new("sudo")
                .args(["-n", "systemctl", action, unit])
                .output(),
        )
        .await;

        match output {
            Ok(Ok(out)) if out.status.success() => Ok(()),
            Ok(Ok(out)) => {
                let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                Err(SystemdError::CommandFailed {
                    action: action.into(),
                    unit: unit.into(),
                    message: if stderr.is_empty() {
                        format!("systemctl exited with {}", out.status)
                    } else {
                        stderr
                    },
                })
            }
            Ok(Err(e)) => Err(SystemdError::CommandFailed {
                action: action.into(),
                unit: unit.into(),
                message: format!("failed to execute systemctl: {e}"),
            }),
            Err(_) => Err(SystemdError::Timeout {
                action: action.into(),
                unit: unit.into(),
            }),
        }
    }
}

pub fn is_safe_unit_name(unit: &str) -> bool {
    !unit.is_empty()
        && unit.len() <= 128
        && unit.ends_with(".service")
        && !unit.contains("..")
        && unit
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '@' | '.' | '_' | '-'))
}

pub fn parse_systemctl_show(unit: &str, input: &str, source: &str) -> UnitStatus {
    let values: HashMap<&str, &str> = input
        .lines()
        .filter_map(|line| line.split_once('='))
        .collect();
    let load_state = values.get("LoadState").copied().unwrap_or("unknown");
    let active_state = values.get("ActiveState").copied().unwrap_or("unknown");
    let sub_state = values.get("SubState").copied().unwrap_or("unknown");
    let state = state_label(active_state, sub_state);
    let since = values
        .get(if active_state == "active" {
            "ActiveEnterTimestamp"
        } else {
            "InactiveEnterTimestamp"
        })
        .map(|v| v.trim())
        .filter(|v| !v.is_empty() && *v != "n/a")
        .map(str::to_string);

    UnitStatus {
        unit: values.get("Id").copied().unwrap_or(unit).to_string(),
        source: source.to_string(),
        exists: load_state != "not-found" && load_state != "not-found-bad-setting",
        loaded: load_state == "loaded",
        state: state.into(),
        active: active_state == "active",
        active_state: active_state.to_string(),
        sub_state: sub_state.to_string(),
        description: values
            .get("Description")
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .map(str::to_string),
        since,
        main_pid: parse_nonzero(values.get("MainPID").copied()),
        memory_current_bytes: parse_nonzero(values.get("MemoryCurrent").copied()),
        tasks_current: parse_nonzero(values.get("TasksCurrent").copied()),
        error: None,
    }
}

fn state_label(active_state: &str, sub_state: &str) -> &'static str {
    match (active_state, sub_state) {
        ("active", "running") => UnitState::ActiveRunning.as_str(),
        ("activating", _) => UnitState::Activating.as_str(),
        ("inactive", "dead") => UnitState::InactiveDead.as_str(),
        ("failed", _) => UnitState::Failed.as_str(),
        _ => UnitState::Unknown.as_str(),
    }
}

fn parse_nonzero<T>(value: Option<&str>) -> Option<T>
where
    T: TryFrom<u64>,
{
    let n = value?.trim().parse::<u64>().ok()?;
    if n == 0 {
        return None;
    }
    T::try_from(n).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_unit_names() {
        assert!(is_safe_unit_name("ark-server@home.service"));
        assert!(is_safe_unit_name("ark_server@travel-a.service"));
        assert!(!is_safe_unit_name(""));
        assert!(!is_safe_unit_name("ark-server@home"));
        assert!(!is_safe_unit_name("../ark-server@home.service"));
        assert!(!is_safe_unit_name("ark-server@home.service;shutdown"));
    }

    #[test]
    fn parses_active_systemctl_show_output() {
        let parsed = parse_systemctl_show(
            "ark-server@travel-a.service",
            "Id=ark-server@travel-a.service\nLoadState=loaded\nActiveState=active\nSubState=running\nDescription=ARK On-demand\nActiveEnterTimestamp=Wed 2026-06-03 17:00:00 CEST\nMainPID=1234\nMemoryCurrent=987654321\nTasksCurrent=42\n",
            "systemd",
        );

        assert_eq!(parsed.unit, "ark-server@travel-a.service");
        assert!(parsed.exists);
        assert!(parsed.loaded);
        assert!(parsed.active);
        assert_eq!(parsed.state, "active (running)");
        assert_eq!(parsed.description.as_deref(), Some("ARK On-demand"));
        assert_eq!(parsed.main_pid, Some(1234));
        assert_eq!(parsed.memory_current_bytes, Some(987654321));
        assert_eq!(parsed.tasks_current, Some(42));
    }

    #[test]
    fn parses_not_found_unit() {
        let parsed = parse_systemctl_show(
            "ark-server@missing.service",
            "Id=ark-server@missing.service\nLoadState=not-found\nActiveState=inactive\nSubState=dead\nMainPID=0\n",
            "systemd",
        );

        assert!(!parsed.exists);
        assert!(!parsed.loaded);
        assert_eq!(parsed.state, "inactive (dead)");
        assert_eq!(parsed.main_pid, None);
    }
}
