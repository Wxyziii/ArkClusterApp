//! RCON connection model — DATA MODEL ONLY for Phase 1.
//!
//! No socket is opened in this phase. These structs describe a per-map RCON
//! endpoint and connection state, plus the concept of an all-map chat listener
//! that a later phase will use to read `!travel <map>` commands across every
//! running server.

use serde::Serialize;

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
}

impl RconEndpoint {
    pub fn mock(map_id: &str, host: &str, port: u16, connected: bool, players: u32) -> Self {
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
