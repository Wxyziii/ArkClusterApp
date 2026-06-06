//! RCON connection model — DATA MODEL ONLY for Phase 1.
//!
//! No socket is opened in this phase. These structs describe a per-map RCON
//! endpoint and connection state, plus the concept of an all-map chat listener
//! that a later phase will use to read `!travel <map>` commands across every
//! running server.

use serde::Serialize;

use crate::config::Config;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum RconConnState {
    Connected,
    Connecting,
    Disconnected,
}

impl RconConnState {
    pub fn as_str(&self) -> &'static str {
        match self {
            RconConnState::Connected => "Connected",
            RconConnState::Connecting => "Connecting",
            RconConnState::Disconnected => "Disconnected",
        }
    }
}

/// One RCON endpoint, bound to a running server's host/port.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RconEndpoint {
    pub map_id: String,
    pub host: String,
    pub port: u16,
    pub state: String,
    /// ISO-ish timestamp of the last chat poll, or null if never polled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_chat_poll: Option<String>,
    pub player_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    pub configured: bool,
}

impl RconEndpoint {
    pub fn test_endpoint(map_id: &str, host: &str, port: u16, connected: bool, players: u32) -> Self {
        Self {
            map_id: map_id.to_string(),
            host: host.to_string(),
            port,
            state: if connected {
                RconConnState::Connected.as_str().into()
            } else {
                RconConnState::Disconnected.as_str().into()
            },
            last_chat_poll: connected.then(|| "2026-06-03 20:55:01".to_string()),
            player_count: players,
            last_error: None,
            configured: true,
        }
    }
}

/// The all-map chat listener concept (future). Aggregates chat across every
/// running server so `!travel <map>` can be issued from any map.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RconListener {
    pub enabled: bool,
    pub poll_interval_secs: u32,
    pub endpoints: Vec<RconEndpoint>,
    /// Marker so the UI knows this is not yet live.
    pub implemented: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub id: String,
    pub slot_id: String,
    pub player: String,
    pub message: String,
    pub detected_command: Option<DetectedCommand>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedCommand {
    pub command: String,
    pub argument: String,
    pub mode: String,
}

pub fn status_from_config(config: &Config) -> RconListener {
    let enabled = config.operations.rcon_enabled && config.rcon.enabled;
    let mut endpoints = Vec::new();
    if let Some(slots) = &config.slots {
        for (slot, _, _) in slots.iter() {
            let mut endpoint = RconEndpoint {
                map_id: slot.map_key.clone(),
                host: slot
                    .rcon
                    .as_ref()
                    .map(|r| r.host.clone())
                    .unwrap_or_else(|| "127.0.0.1".into()),
                port: slot.rcon.as_ref().map(|r| r.port).unwrap_or(slot.rcon_port),
                state: RconConnState::Disconnected.as_str().into(),
                last_chat_poll: None,
                player_count: 0,
                last_error: None,
                configured: slot.rcon.is_some(),
            };
            if !enabled {
                endpoint.last_error = Some("RCON disabled in manager config".into());
            } else if let Some(rcon) = &slot.rcon {
                if std::env::var(&rcon.password_env)
                    .unwrap_or_default()
                    .is_empty()
                {
                    endpoint.last_error =
                        Some(format!("missing password env {}", rcon.password_env));
                } else {
                    endpoint.state = RconConnState::Connecting.as_str().into();
                    endpoint.last_error =
                        Some("read-only RCON poller is not connected in this phase".into());
                }
            } else {
                endpoint.last_error = Some("RCON endpoint not configured for slot".into());
            }
            endpoints.push(endpoint);
        }
    }
    RconListener {
        enabled,
        poll_interval_secs: config.rcon.poll_interval_seconds,
        endpoints,
        implemented: true,
    }
}

pub fn parse_detected_command(message: &str) -> Option<DetectedCommand> {
    let trimmed = message.trim();
    let rest = trimmed.strip_prefix("!travel ")?;
    let argument = rest.split_whitespace().next()?.trim();
    if argument.is_empty() {
        return None;
    }
    Some(DetectedCommand {
        command: "travel".into(),
        argument: argument.into(),
        mode: "detected_only".into(),
    })
}

pub fn parse_players(input: &str) -> Vec<String> {
    input
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("no players connected") {
                return None;
            }
            let without_index = trimmed
                .split_once(". ")
                .map(|(_, name)| name)
                .unwrap_or(trimmed);
            Some(without_index.trim().to_string())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_travel_command_detected_only() {
        let parsed = parse_detected_command("  !travel ragnarok please").unwrap();
        assert_eq!(parsed.command, "travel");
        assert_eq!(parsed.argument, "ragnarok");
        assert_eq!(parsed.mode, "detected_only");
    }

    #[test]
    fn parses_player_list_lines() {
        assert_eq!(
            parse_players("1. Marcel\n2. Nyx\n"),
            vec!["Marcel".to_string(), "Nyx".to_string()]
        );
    }

    #[test]
    fn disabled_config_reports_disabled() {
        let cfg = crate::config::tests_support::base_config();
        let status = status_from_config(&cfg);
        assert!(!status.enabled);
        assert!(status.endpoints[0]
            .last_error
            .as_ref()
            .unwrap()
            .contains("disabled"));
    }
}
