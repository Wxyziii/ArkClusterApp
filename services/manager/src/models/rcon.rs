//! Source RCON runtime support for ARK.
//!
//! The manager only exposes structured status/player/chat data. It does not
//! expose a public "run arbitrary RCON command" API.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio::time;

use crate::config::{Config, ServerSlot};
use crate::models::audit::{self, AuditEvent, Severity};
use crate::models::backup;
use crate::models::domain::Player;
use crate::models::resources;
use crate::models::systemd::UnitStatus;
use crate::models::travel::{self, TravelRequestBody};
use crate::state::AppState;

pub type SharedRconRuntime = Arc<RwLock<RconRuntimeState>>;

const RCON_AUTH: i32 = 3;
const RCON_AUTH_RESPONSE: i32 = 2;
const RCON_EXECCOMMAND: i32 = 2;
const RCON_RESPONSE_VALUE: i32 = 0;
const MAX_PACKET_SIZE: i32 = 1024 * 1024;
const MAX_CHAT_LINES: usize = 100;
const MAX_SEEN_LINES: usize = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum RconConnState {
    Connected,
    Connecting,
    Disconnected,
    Disabled,
    Unavailable,
}

impl RconConnState {
    pub fn as_str(&self) -> &'static str {
        match self {
            RconConnState::Connected => "Connected",
            RconConnState::Connecting => "Connecting",
            RconConnState::Disconnected => "Disconnected",
            RconConnState::Disabled => "Disabled",
            RconConnState::Unavailable => "Unavailable",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RconEndpoint {
    pub slot_id: String,
    pub map_id: String,
    pub host: String,
    pub port: u16,
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_player_poll: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_chat_poll: Option<String>,
    pub player_count: Option<u32>,
    pub player_count_source: String,
    pub players: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    pub configured: bool,
    pub active: bool,
    pub chat_source: String,
}

impl RconEndpoint {
    pub fn test_endpoint(
        map_id: &str,
        host: &str,
        port: u16,
        connected: bool,
        players: u32,
    ) -> Self {
        Self {
            slot_id: map_id.to_string(),
            map_id: map_id.to_string(),
            host: host.to_string(),
            port,
            state: if connected {
                RconConnState::Connected.as_str().into()
            } else {
                RconConnState::Disconnected.as_str().into()
            },
            last_player_poll: connected.then(now_string),
            last_chat_poll: connected.then(now_string),
            player_count: Some(players),
            player_count_source: if connected {
                "rcon"
            } else {
                "rcon_unavailable"
            }
            .into(),
            players: Vec::new(),
            last_error: None,
            configured: true,
            active: connected,
            chat_source: if connected {
                "rcon_gamelog"
            } else {
                "unavailable"
            }
            .into(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RconListener {
    pub enabled: bool,
    pub poll_interval_secs: u32,
    pub endpoints: Vec<RconEndpoint>,
    pub implemented: bool,
    pub chat_command_source: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub id: String,
    pub ts: String,
    pub slot_id: String,
    pub player: String,
    pub message: String,
    pub source: String,
    pub detected_command: Option<DetectedCommand>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedCommand {
    pub command: String,
    pub argument: String,
    pub mode: String,
}

#[derive(Debug, Clone)]
pub struct SlotRuntime {
    pub slot_id: String,
    pub slot_key: String,
    pub map_id: String,
    pub host: String,
    pub port: u16,
    pub configured: bool,
    pub active: bool,
    pub state: RconConnState,
    pub player_count: Option<u32>,
    pub players: Vec<String>,
    pub last_player_poll: Option<String>,
    pub last_chat_poll: Option<String>,
    pub last_error: Option<String>,
    pub chat_source: String,
    pub idle_zero_since: Option<u64>,
}

#[derive(Debug, Default)]
pub struct RconRuntimeState {
    pub slots: HashMap<String, SlotRuntime>,
    pub messages: VecDeque<ChatMessage>,
    pub detected_commands: VecDeque<ChatMessage>,
    seen_lines: HashSet<String>,
    seen_order: VecDeque<String>,
    log_offsets: HashMap<PathBuf, u64>,
    /// Tracks when the home slot last had 0 players, for home idle shutdown.
    pub home_idle_since: Option<u64>,
}

impl RconRuntimeState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn endpoint_for(&self, slot_id: &str) -> Option<&SlotRuntime> {
        self.slots.get(slot_id)
    }

    pub fn player_count(&self, slot_id: &str) -> Option<u32> {
        self.slots.get(slot_id).and_then(|slot| {
            (slot.active && slot.player_count.is_some())
                .then_some(slot.player_count)
                .flatten()
        })
    }

    pub fn status_from_config(&self, config: &Config) -> RconListener {
        let enabled = config.operations.rcon_enabled && config.rcon.enabled;
        let mut endpoints = Vec::new();
        if let Some(slots) = &config.slots {
            for (slot, key, _) in slots.iter() {
                let map_id = travel::effective_slot_map_id(config, slot);
                let runtime = self.slots.get(&slot.id);
                let rcon = slot.rcon.as_ref();
                let configured = rcon.is_some();
                let missing_password = rcon
                    .map(|r| {
                        std::env::var(&r.password_env)
                            .unwrap_or_default()
                            .is_empty()
                    })
                    .unwrap_or(false);
                let state = if !enabled {
                    RconConnState::Disabled
                } else if !configured {
                    RconConnState::Unavailable
                } else {
                    runtime
                        .map(|r| r.state)
                        .unwrap_or(RconConnState::Connecting)
                };
                let last_error = if !enabled {
                    Some("RCON disabled in manager config".into())
                } else if !configured {
                    Some("RCON endpoint not configured for slot".into())
                } else if missing_password {
                    rcon.map(|r| format!("missing password env {}", r.password_env))
                } else {
                    runtime.and_then(|r| r.last_error.clone())
                };
                endpoints.push(RconEndpoint {
                    slot_id: slot.id.clone(),
                    map_id,
                    host: rcon
                        .map(|r| r.host.clone())
                        .unwrap_or_else(|| "127.0.0.1".into()),
                    port: rcon.map(|r| r.port).unwrap_or(slot.rcon_port),
                    state: state.as_str().into(),
                    last_player_poll: runtime.and_then(|r| r.last_player_poll.clone()),
                    last_chat_poll: runtime.and_then(|r| r.last_chat_poll.clone()),
                    player_count: runtime.and_then(|r| r.player_count),
                    player_count_source: runtime
                        .map(|r| {
                            if r.player_count.is_some() {
                                "rcon"
                            } else if r.active {
                                "rcon_unavailable"
                            } else {
                                "stopped"
                            }
                        })
                        .unwrap_or(if enabled { "pending" } else { "disabled" })
                        .into(),
                    players: runtime.map(|r| r.players.clone()).unwrap_or_default(),
                    last_error,
                    configured,
                    active: runtime.map(|r| r.active).unwrap_or(false),
                    chat_source: runtime
                        .map(|r| r.chat_source.clone())
                        .unwrap_or_else(|| if enabled { "pending" } else { "disabled" }.into()),
                });
                if runtime.is_none() && enabled && configured {
                    tracing::debug!(slot = %slot.id, key, "RCON slot pending first poll");
                }
            }
        }
        let chat_command_source = if !enabled {
            "disabled"
        } else if endpoints.iter().any(|e| e.chat_source == "rcon_gamelog") {
            "rcon_gamelog"
        } else if endpoints.iter().any(|e| e.chat_source == "log_tail") {
            "log_tail"
        } else {
            "unavailable"
        };
        RconListener {
            enabled,
            poll_interval_secs: config.rcon.poll_interval_seconds,
            endpoints,
            implemented: true,
            chat_command_source: chat_command_source.into(),
        }
    }

    pub fn players_response(&self, config: &Config) -> PlayersResponse {
        let mut rows = Vec::new();
        let mut known = false;
        let mut active_unavailable = false;
        let mut errors = Vec::new();
        for slot in self.slots.values() {
            if !slot.active {
                continue;
            }
            if slot.player_count.is_some() {
                known = true;
                let map_name = map_name(config, &slot.map_id);
                rows.extend(slot.players.iter().map(|name| Player {
                    name: name.clone(),
                    level: 0,
                    tribe: "".into(),
                    connected_mins: 0,
                    map: map_name.clone(),
                }));
            } else {
                active_unavailable = true;
                if let Some(error) = &slot.last_error {
                    errors.push(format!("{}: {}", slot.slot_id, error));
                }
            }
        }
        let enabled = config.operations.rcon_enabled && config.rcon.enabled;
        let available = enabled && known && !active_unavailable;
        let source = if known {
            "rcon"
        } else if enabled {
            "rcon_unavailable"
        } else {
            "disabled"
        };
        let reason = if available {
            "RCON player polling live".into()
        } else if !enabled {
            "RCON disabled in manager config".into()
        } else if errors.is_empty() {
            "RCON player polling has not succeeded yet".into()
        } else {
            errors.join("; ")
        };
        PlayersResponse {
            players: rows,
            source: source.into(),
            rcon_enabled: enabled,
            available,
            reason,
        }
    }

    pub fn chat_response(&self, config: &Config) -> ChatResponse {
        let enabled = config.operations.rcon_enabled && config.rcon.enabled;
        let source = if !enabled {
            "disabled"
        } else if self.messages.iter().any(|m| m.source == "rcon_gamelog") {
            "rcon_gamelog"
        } else if self.messages.iter().any(|m| m.source == "log_tail") {
            "log_tail"
        } else if self
            .slots
            .values()
            .any(|slot| slot.chat_source == "rcon_gamelog")
        {
            "rcon_gamelog"
        } else if self
            .slots
            .values()
            .any(|slot| slot.chat_source == "log_tail")
        {
            "log_tail"
        } else {
            "unavailable"
        };
        ChatResponse {
            messages: self.messages.iter().rev().take(50).cloned().collect(),
            detected_commands: self
                .detected_commands
                .iter()
                .rev()
                .take(50)
                .cloned()
                .collect(),
            source: source.into(),
            available: source != "disabled" && source != "unavailable",
            reason: if source == "unavailable" {
                "No RCON game log or ARK log chat line has been observed yet".into()
            } else {
                "chat command source active".into()
            },
        }
    }

    fn update_slot(
        &mut self,
        slot: &ServerSlot,
        slot_key: &str,
        map_id: String,
        active: bool,
        endpoint: Option<(&str, u16)>,
        result: PollResult,
    ) {
        let (host, port) = endpoint
            .map(|(host, port)| (host.to_string(), port))
            .unwrap_or_else(|| ("127.0.0.1".into(), slot.rcon_port));
        let mut entry = self.slots.remove(&slot.id).unwrap_or(SlotRuntime {
            slot_id: slot.id.clone(),
            slot_key: slot_key.into(),
            map_id: map_id.clone(),
            host: host.clone(),
            port,
            configured: endpoint.is_some(),
            active,
            state: RconConnState::Connecting,
            player_count: None,
            players: Vec::new(),
            last_player_poll: None,
            last_chat_poll: None,
            last_error: None,
            chat_source: "pending".into(),
            idle_zero_since: None,
        });
        entry.slot_key = slot_key.into();
        entry.map_id = map_id;
        entry.host = host;
        entry.port = port;
        entry.configured = endpoint.is_some();
        entry.active = active;
        match result {
            PollResult::Disabled(reason) => {
                entry.state = RconConnState::Disabled;
                entry.player_count = None;
                entry.players.clear();
                entry.last_error = Some(reason);
                entry.chat_source = "disabled".into();
            }
            PollResult::Stopped => {
                entry.state = RconConnState::Disconnected;
                entry.player_count = None;
                entry.players.clear();
                entry.last_error = None;
                entry.chat_source = "stopped".into();
                entry.idle_zero_since = None;
            }
            PollResult::Unavailable(reason) => {
                entry.state = RconConnState::Unavailable;
                entry.player_count = None;
                entry.players.clear();
                entry.last_error = Some(reason);
                entry.chat_source = "unavailable".into();
            }
            PollResult::Players {
                players,
                chat_source,
            } => {
                entry.state = RconConnState::Connected;
                entry.player_count = Some(players.len() as u32);
                entry.players = players;
                entry.last_player_poll = Some(now_string());
                entry.last_error = None;
                entry.chat_source = chat_source;
            }
        }
        self.slots.insert(slot.id.clone(), entry);
    }

    fn push_chat_line(&mut self, slot_id: &str, source: &str, raw: &str) -> Option<ChatMessage> {
        let normalized = raw.trim();
        if normalized.is_empty() {
            return None;
        }
        let seen_key = format!("{slot_id}:{source}:{normalized}");
        if !self.seen_lines.insert(seen_key.clone()) {
            return None;
        }
        self.seen_order.push_back(seen_key);
        while self.seen_order.len() > MAX_SEEN_LINES {
            if let Some(old) = self.seen_order.pop_front() {
                self.seen_lines.remove(&old);
            }
        }
        let parsed = parse_chat_line(normalized);
        if parsed.message.is_empty() && parsed.detected_command.is_none() {
            return None;
        }
        let message = ChatMessage {
            id: format!("chat-{}-{}", epoch_secs(), self.messages.len()),
            ts: now_string(),
            slot_id: slot_id.into(),
            player: parsed.player,
            message: parsed.message,
            source: source.into(),
            detected_command: parsed.detected_command,
        };
        self.messages.push_back(message.clone());
        while self.messages.len() > MAX_CHAT_LINES {
            self.messages.pop_front();
        }
        if message.detected_command.is_some() {
            self.detected_commands.push_back(message.clone());
            while self.detected_commands.len() > MAX_CHAT_LINES {
                self.detected_commands.pop_front();
            }
            return Some(message);
        }
        None
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayersResponse {
    pub players: Vec<Player>,
    pub source: String,
    pub rcon_enabled: bool,
    pub available: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatResponse {
    pub messages: Vec<ChatMessage>,
    pub detected_commands: Vec<ChatMessage>,
    pub source: String,
    pub available: bool,
    pub reason: String,
}

enum PollResult {
    Disabled(String),
    Stopped,
    Unavailable(String),
    Players {
        players: Vec<String>,
        chat_source: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RconPacket {
    pub id: i32,
    pub packet_type: i32,
    pub body: String,
}

#[derive(Debug, thiserror::Error)]
pub enum RconError {
    #[error("connect timed out")]
    ConnectTimeout,
    #[error("command timed out")]
    CommandTimeout,
    #[error("authentication failed")]
    AuthFailed,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("malformed RCON packet: {0}")]
    Malformed(String),
}

pub fn encode_packet(packet: &RconPacket) -> Vec<u8> {
    let body = packet.body.as_bytes();
    let size = (4 + 4 + body.len() + 2) as i32;
    let mut out = Vec::with_capacity(size as usize + 4);
    out.extend_from_slice(&size.to_le_bytes());
    out.extend_from_slice(&packet.id.to_le_bytes());
    out.extend_from_slice(&packet.packet_type.to_le_bytes());
    out.extend_from_slice(body);
    out.extend_from_slice(&[0, 0]);
    out
}

pub fn decode_packet(mut bytes: &[u8]) -> Result<RconPacket, RconError> {
    if bytes.len() < 14 {
        return Err(RconError::Malformed("packet too short".into()));
    }
    let size = read_i32(&mut bytes)?;
    if !(10..=MAX_PACKET_SIZE).contains(&size) {
        return Err(RconError::Malformed(format!("invalid size {size}")));
    }
    if bytes.len() != size as usize {
        return Err(RconError::Malformed("size does not match body".into()));
    }
    let id = read_i32(&mut bytes)?;
    let packet_type = read_i32(&mut bytes)?;
    if bytes.len() < 2 || bytes[bytes.len() - 2..] != [0, 0] {
        return Err(RconError::Malformed("missing null terminators".into()));
    }
    let body = String::from_utf8_lossy(&bytes[..bytes.len() - 2]).to_string();
    Ok(RconPacket {
        id,
        packet_type,
        body,
    })
}

fn read_i32(bytes: &mut &[u8]) -> Result<i32, RconError> {
    if bytes.len() < 4 {
        return Err(RconError::Malformed("missing i32".into()));
    }
    let (value, rest) = bytes.split_at(4);
    *bytes = rest;
    Ok(i32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}

async fn read_packet(stream: &mut TcpStream) -> Result<RconPacket, RconError> {
    let mut size_bytes = [0u8; 4];
    stream.read_exact(&mut size_bytes).await?;
    let size = i32::from_le_bytes(size_bytes);
    if !(10..=MAX_PACKET_SIZE).contains(&size) {
        return Err(RconError::Malformed(format!("invalid size {size}")));
    }
    let mut rest = vec![0u8; size as usize];
    stream.read_exact(&mut rest).await?;
    let mut framed = Vec::with_capacity(size as usize + 4);
    framed.extend_from_slice(&size_bytes);
    framed.extend_from_slice(&rest);
    decode_packet(&framed)
}

async fn write_packet(stream: &mut TcpStream, packet: &RconPacket) -> Result<(), RconError> {
    stream.write_all(&encode_packet(packet)).await?;
    Ok(())
}

pub async fn run_rcon_command(
    host: &str,
    port: u16,
    password: &str,
    command: &str,
    timeout: Duration,
) -> Result<String, RconError> {
    let address = format!("{host}:{port}");
    let stream = time::timeout(timeout, TcpStream::connect(address))
        .await
        .map_err(|_| RconError::ConnectTimeout)??;
    let mut stream = stream;

    write_packet(
        &mut stream,
        &RconPacket {
            id: 1,
            packet_type: RCON_AUTH,
            body: password.into(),
        },
    )
    .await?;

    let mut authed = false;
    for _ in 0..4 {
        let packet = time::timeout(timeout, read_packet(&mut stream))
            .await
            .map_err(|_| RconError::CommandTimeout)??;
        if packet.id == -1 {
            return Err(RconError::AuthFailed);
        }
        if packet.id == 1 && packet.packet_type == RCON_AUTH_RESPONSE {
            authed = true;
            break;
        }
    }
    if !authed {
        return Err(RconError::AuthFailed);
    }

    write_packet(
        &mut stream,
        &RconPacket {
            id: 2,
            packet_type: RCON_EXECCOMMAND,
            body: command.into(),
        },
    )
    .await?;

    let mut response = String::new();
    let mut got_any = false;
    loop {
        let read_timeout = if got_any {
            Duration::from_millis(250)
        } else {
            timeout
        };
        match time::timeout(read_timeout, read_packet(&mut stream)).await {
            Ok(Ok(packet)) => {
                if packet.id == 2 && packet.packet_type == RCON_RESPONSE_VALUE {
                    got_any = true;
                    response.push_str(&packet.body);
                }
            }
            Ok(Err(err)) => return Err(err),
            Err(_) if got_any => break,
            Err(_) => return Err(RconError::CommandTimeout),
        }
    }
    Ok(response)
}

pub fn status_from_config(config: &Config) -> RconListener {
    RconRuntimeState::new().status_from_config(config)
}

pub fn parse_detected_command(message: &str) -> Option<DetectedCommand> {
    let lower = message.to_ascii_lowercase();
    let idx = lower.find("!travel")?;
    let after = &message[idx + "!travel".len()..];
    if !after.is_empty() && !after.chars().next().is_some_and(char::is_whitespace) {
        return None;
    }
    let argument = after
        .trim()
        .trim_matches(|c: char| c == '"' || c == '\'' || c == '.' || c == '!');
    if argument.is_empty() {
        return None;
    }
    Some(DetectedCommand {
        command: "travel".into(),
        argument: argument.into(),
        mode: "live_chat".into(),
    })
}

struct ParsedChatLine {
    player: String,
    message: String,
    detected_command: Option<DetectedCommand>,
}

fn parse_chat_line(raw: &str) -> ParsedChatLine {
    let detected = parse_detected_command(raw);
    let Some(cmd) = &detected else {
        return ParsedChatLine {
            player: String::new(),
            message: raw.trim().into(),
            detected_command: None,
        };
    };
    let idx = raw.to_ascii_lowercase().find("!travel").unwrap_or_default();
    let before = raw[..idx].trim();
    let mut player = before
        .rsplit_once(':')
        .map(|(actor, _)| actor)
        .unwrap_or(before)
        .trim()
        .to_string();
    while player.starts_with('[') {
        let Some(end) = player.find(']') else {
            break;
        };
        player = player[end + 1..].trim().to_string();
    }
    ParsedChatLine {
        player,
        message: format!("!travel {}", cmd.argument),
        detected_command: detected,
    }
}

pub fn parse_players(input: &str) -> Vec<String> {
    input
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty()
                || trimmed.eq_ignore_ascii_case("no players connected")
                || trimmed.to_ascii_lowercase().contains("no players")
            {
                return None;
            }
            let without_index = trimmed
                .split_once(". ")
                .map(|(_, name)| name)
                .unwrap_or(trimmed);
            let name = without_index
                .split(',')
                .next()
                .unwrap_or(without_index)
                .trim();
            if name.is_empty() {
                None
            } else {
                Some(name.to_string())
            }
        })
        .collect()
}

pub fn spawn_runtime_tasks(state: AppState) {
    tokio::spawn(async move {
        loop {
            poll_once(&state).await;
            let secs = state.config.rcon.poll_interval_seconds.max(1) as u64;
            time::sleep(Duration::from_secs(secs)).await;
        }
    });
}

async fn poll_once(state: &AppState) {
    let enabled = state.config.operations.rcon_enabled && state.config.rcon.enabled;
    let Some(slots) = &state.config.slots else {
        return;
    };
    let mut detected = Vec::new();
    for (slot, key, _) in slots.iter() {
        let map_id = travel::effective_slot_map_id(&state.config, slot);
        let status = state
            .systemd
            .get_status(&slot.systemd_unit)
            .await
            .unwrap_or_else(|e| {
                UnitStatus::unavailable(&slot.systemd_unit, "systemd", e.to_string())
            });
        let endpoint = slot
            .rcon
            .as_ref()
            .map(|r| (r.host.as_str(), r.port, r.password_env.as_str()));
        let result = if !enabled {
            PollResult::Disabled("RCON disabled in manager config".into())
        } else if !status.active {
            PollResult::Stopped
        } else if let Some((host, port, password_env)) = endpoint {
            let password = std::env::var(password_env).unwrap_or_default();
            if password.trim().is_empty() {
                PollResult::Unavailable(format!("missing password env {password_env}"))
            } else {
                match run_rcon_command(host, port, &password, "ListPlayers", Duration::from_secs(3))
                    .await
                {
                    Ok(raw) => {
                        let players = parse_players(&raw);
                        let mut chat_source = "rcon".to_string();
                        if let Ok(log) = run_rcon_command(
                            host,
                            port,
                            &password,
                            "GetGameLog",
                            Duration::from_secs(3),
                        )
                        .await
                        {
                            let lines = log
                                .lines()
                                .filter(|line| !is_ignorable_game_log_line(line))
                                .collect::<Vec<_>>();
                            if !lines.is_empty() {
                                chat_source = "rcon_gamelog".into();
                                let mut runtime = state.rcon_runtime.write().await;
                                for line in lines {
                                    if let Some(message) =
                                        runtime.push_chat_line(&slot.id, "rcon_gamelog", line)
                                    {
                                        detected.push(message);
                                    }
                                }
                                if let Some(entry) = runtime.slots.get_mut(&slot.id) {
                                    entry.last_chat_poll = Some(now_string());
                                }
                            }
                        }
                        PollResult::Players {
                            players,
                            chat_source,
                        }
                    }
                    Err(err) => PollResult::Unavailable(sanitize_error(&err.to_string())),
                }
            }
        } else {
            PollResult::Unavailable("RCON endpoint not configured for slot".into())
        };
        let endpoint_for_update = slot.rcon.as_ref().map(|r| (r.host.as_str(), r.port));
        {
            let mut runtime = state.rcon_runtime.write().await;
            runtime.update_slot(
                slot,
                key,
                map_id,
                status.active,
                endpoint_for_update,
                result,
            );
        }
    }

    detected.extend(tail_log_chat(state).await);
    for message in detected {
        process_detected_command(state, message).await;
    }
    apply_player_count_policies(state).await;
    apply_home_idle_shutdown(state).await;
}

async fn tail_log_chat(state: &AppState) -> Vec<ChatMessage> {
    let mut out = Vec::new();
    for path in log_candidates(&state.config) {
        if !path.exists() {
            continue;
        }
        let Ok(meta) = std::fs::metadata(&path) else {
            continue;
        };
        let len = meta.len();
        let start = {
            let mut runtime = state.rcon_runtime.write().await;
            let offset = runtime.log_offsets.entry(path.clone()).or_insert(len);
            if len < *offset {
                *offset = 0;
            }
            if len == *offset {
                None
            } else {
                let start = *offset;
                *offset = len;
                Some(start)
            }
        };
        let Some(start) = start else {
            continue;
        };
        let Ok(mut file) = std::fs::File::open(&path) else {
            continue;
        };
        use std::io::{Read, Seek};
        if file.seek(std::io::SeekFrom::Start(start)).is_err() {
            continue;
        }
        let mut raw = String::new();
        if file.read_to_string(&mut raw).is_err() {
            continue;
        }
        let mut runtime = state.rcon_runtime.write().await;
        let slot_id = default_chat_slot_id(&state.config, &runtime);
        for line in raw.lines() {
            if let Some(message) = runtime.push_chat_line(&slot_id, "log_tail", line) {
                out.push(message);
            }
        }
    }
    out
}

fn log_candidates(config: &Config) -> Vec<PathBuf> {
    let ark_root = if config.paths.ark_root.trim().is_empty() {
        "/srv/ark"
    } else {
        &config.paths.ark_root
    };
    let log_dir = Path::new(ark_root)
        .join("server")
        .join("ShooterGame")
        .join("Saved")
        .join("Logs");
    let mut out = vec![log_dir.join("ShooterGame.log")];
    if let Ok(entries) = std::fs::read_dir(&log_dir) {
        let mut discovered = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| {
                        name == "ShooterGame.log"
                            || (name.starts_with("ShooterGame_") && name.ends_with(".log"))
                    })
            })
            .collect::<Vec<_>>();
        discovered.sort();
        for path in discovered {
            if !out.iter().any(|existing| existing == &path) {
                out.push(path);
            }
        }
    }
    out
}

fn default_chat_slot_id(config: &Config, runtime: &RconRuntimeState) -> String {
    if let Some((slot_id, _)) = runtime.slots.iter().find(|(_, slot)| slot.active) {
        return slot_id.clone();
    }
    config
        .slots
        .as_ref()
        .map(|slots| slots.home.id.clone())
        .unwrap_or_else(|| "unknown".into())
}

async fn process_detected_command(state: &AppState, message: ChatMessage) {
    let Some(cmd) = message.detected_command.clone() else {
        return;
    };
    if cmd.command != "travel" {
        return;
    }
    let requested_map = cmd.argument.clone();
    let received_map_name = travel::resolve_map(&state.config, &requested_map)
        .map(|map| map.name)
        .unwrap_or_else(|| requested_map.clone());
    let received_feedback = format!("[ARK Cluster] Travel request received: {received_map_name}.");
    let received_feedback_result =
        send_chat_feedback(state, &message.slot_id, &received_feedback).await;
    if let Err(err) = &received_feedback_result {
        record_feedback_unavailable(state, &message, err).await;
    }
    let req = TravelRequestBody {
        map: requested_map.clone(),
        source: "in_game_chat".into(),
        actor: if message.player.trim().is_empty() {
            "unknown-player".into()
        } else {
            message.player.clone()
        },
    };
    let statuses = travel_slot_statuses(state).await;
    match travel::request_with_start(
        &state.pool,
        &state.config,
        state.systemd.clone(),
        state.manager_started_at,
        req,
        statuses,
    )
    .await
    {
        Ok(decision) => {
            let decision_feedback = ark_feedback_for_decision(&decision);
            let decision_feedback_result =
                send_chat_feedback(state, &message.slot_id, &decision_feedback).await;
            if let Err(err) = &decision_feedback_result {
                record_feedback_unavailable(state, &message, err).await;
            }
            let severity = if decision.accepted {
                Severity::Info
            } else {
                Severity::Warn
            };
            audit::record(
                &state.pool,
                &AuditEvent::new(severity, "Travel", "in-game travel command processed")
                    .actor(&message.player)
                    .target(decision.resolved_map.as_deref().unwrap_or(""))
                    .detail(format!(
                        "id={} status={} reason={} feedbackReceived={} feedbackResult={}",
                        decision.id,
                        decision.status,
                        decision.reason,
                        feedback_result_label(&received_feedback_result),
                        feedback_result_label(&decision_feedback_result)
                    )),
            )
            .await;
            let detail =
                feedback_history_detail(&received_feedback_result, &decision_feedback_result);
            if let Err(err) =
                travel::append_history_detail(&state.pool, &decision.id, &detail).await
            {
                audit::record(
                    &state.pool,
                    &AuditEvent::new(Severity::Warn, "Travel", "feedback history update failed")
                        .actor(&message.player)
                        .target(decision.resolved_map.as_deref().unwrap_or(""))
                        .detail(sanitize_error(&err.to_string())),
                )
                .await;
            }
        }
        Err(err) => {
            audit::record(
                &state.pool,
                &AuditEvent::new(Severity::Error, "Travel", "in-game travel command failed")
                    .actor(&message.player)
                    .detail(sanitize_error(&err.to_string())),
            )
            .await;
        }
    }
}

async fn send_chat_feedback(state: &AppState, slot_id: &str, message: &str) -> Result<(), String> {
    let Some(slots) = &state.config.slots else {
        return Err("slot config unavailable".into());
    };
    let Some((slot, _, _)) = slots
        .iter()
        .into_iter()
        .find(|(slot, _, _)| slot.id == slot_id)
    else {
        return Err(format!("source slot {slot_id} is not configured"));
    };
    let Some(rcon) = &slot.rcon else {
        return Err(format!("source slot {slot_id} has no RCON endpoint"));
    };
    let password = std::env::var(&rcon.password_env).unwrap_or_default();
    if password.trim().is_empty() {
        return Err(format!("missing password env {}", rcon.password_env));
    }
    let safe_message = sanitize_chat_message(message);
    run_rcon_command(
        &rcon.host,
        rcon.port,
        &password,
        &format!("ServerChat {safe_message}"),
        Duration::from_secs(5),
    )
    .await
    .map(|_| ())
    .map_err(|err| sanitize_error(&err.to_string()))
}

async fn record_feedback_unavailable(state: &AppState, message: &ChatMessage, error: &str) {
    audit::record(
        &state.pool,
        &AuditEvent::new(Severity::Warn, "Travel", "feedback_unavailable")
            .actor(&message.player)
            .target(&message.slot_id)
            .detail(error),
    )
    .await;
}

fn feedback_result_label(result: &Result<(), String>) -> String {
    match result {
        Ok(()) => "sent".into(),
        Err(err) => format!("feedback_unavailable:{err}"),
    }
}

fn feedback_history_detail(received: &Result<(), String>, decision: &Result<(), String>) -> String {
    let sent = received.is_ok() && decision.is_ok();
    let error = received
        .as_ref()
        .err()
        .or_else(|| decision.as_ref().err())
        .map(|err| token_value(err))
        .unwrap_or_else(|| "none".into());
    format!(
        "feedbackAttempted=true feedbackSent={} feedbackResult={} feedbackError={}",
        sent,
        if sent { "sent" } else { "failed" },
        error
    )
}

fn token_value(value: &str) -> String {
    let token = sanitize_error(value)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("_");
    if token.trim().is_empty() {
        "unknown".into()
    } else {
        token.chars().take(160).collect()
    }
}

fn ark_feedback_for_decision(decision: &travel::TravelDecision) -> String {
    let message =
        if decision.status == "blocked" && decision.reason.contains("travel scheduler disabled") {
            format!("Travel is currently unavailable: {}.", decision.reason)
        } else {
            decision.user_message.clone()
        };
    format!("[ARK Cluster] {message}")
}

fn is_ignorable_game_log_line(line: &str) -> bool {
    let normalized = line.trim();
    normalized.is_empty()
        || normalized.eq_ignore_ascii_case("Server received, But no response!!")
        || normalized.eq_ignore_ascii_case("Server received, but no response!!")
}

fn sanitize_chat_message(message: &str) -> String {
    message
        .chars()
        .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(220)
        .collect()
}

pub async fn travel_slot_statuses(state: &AppState) -> Vec<travel::SlotStatusSnapshot> {
    let mut out = Vec::new();
    if let Some(slots) = &state.config.slots {
        for (slot, key, _) in slots.iter() {
            let status = state
                .systemd
                .get_status(&slot.systemd_unit)
                .await
                .unwrap_or_else(|e| {
                    UnitStatus::unavailable(&slot.systemd_unit, "systemd", e.to_string())
                });
            let player_count = if status.active {
                state.rcon_runtime.read().await.player_count(&slot.id)
            } else {
                Some(0)
            };
            out.push(travel::SlotStatusSnapshot {
                slot: slot.clone(),
                key,
                status,
                player_count,
            });
        }
    }
    out
}

async fn apply_player_count_policies(state: &AppState) {
    if !(state.config.operations.systemd_control_enabled
        && state.config.operations.travel_scheduler_enabled)
    {
        return;
    }
    let now = epoch_secs();
    let threshold = state.config.operations.travel_idle_shutdown_secs as u64;
    let Some(slots) = &state.config.slots else {
        return;
    };
    for (slot, key, is_home) in slots.iter() {
        let mut shutdown_due = false;
        {
            let mut runtime = state.rcon_runtime.write().await;
            let Some(entry) = runtime.slots.get_mut(&slot.id) else {
                continue;
            };
            let decision = idle_policy_update(
                is_home,
                entry.active,
                entry.player_count,
                entry.idle_zero_since,
                now,
                threshold,
            );
            match decision {
                IdlePolicyDecision::SetIdleSince(ts) => entry.idle_zero_since = Some(ts),
                IdlePolicyDecision::ClearIdle => entry.idle_zero_since = None,
                IdlePolicyDecision::ShutdownDue => shutdown_due = true,
                IdlePolicyDecision::Noop => {}
            }
        }
        if shutdown_due && key != "home" {
            let _ = save_world_if_available(state, slot).await;
            let _ = backup::run_slot_backup(
                &state.pool,
                &state.config,
                key,
                slot,
                "travel_idle_shutdown",
            )
            .await;
            match state.systemd.stop_unit(&slot.systemd_unit).await {
                Ok(()) => {
                    audit::record(
                        &state.pool,
                        &AuditEvent::new(
                            Severity::Success,
                            "Policy",
                            "travel slot stopped after confirmed idle",
                        )
                        .target(&slot.map_key)
                        .detail(format!("slot={} threshold_secs={threshold}", slot.id)),
                    )
                    .await;
                    // ARK may exit via SIGABRT after a controlled stop; reset-failed clears
                    // any "failed" state left behind so the unit can start cleanly next time.
                    let _ = state.systemd.reset_failed_unit(&slot.systemd_unit).await;
                }
                Err(err) => {
                    audit::record(
                        &state.pool,
                        &AuditEvent::new(Severity::Error, "Policy", "travel idle stop failed")
                            .target(&slot.map_key)
                            .detail(sanitize_error(&err.to_string())),
                    )
                    .await;
                }
            }
        }
    }
    apply_home_standby(state).await;
}

async fn apply_home_standby(state: &AppState) {
    let policy = &state.config.resource_policy;
    if !policy.home_standby_enabled {
        return;
    }
    let Some(slots) = &state.config.slots else {
        return;
    };
    let runtime = state.rcon_runtime.read().await;
    let home_count = runtime.player_count(&slots.home.id);
    let travel_has_players = [&slots.travel_a, &slots.travel_b]
        .into_iter()
        .flatten()
        .any(|slot| {
            runtime
                .player_count(&slot.id)
                .is_some_and(|count| count > 0)
        });
    drop(runtime);
    let sample = resources::sample(&state.config.cluster.directory, state.manager_started_at).await;
    let ram_pct = if sample.ram_total_gb <= 0.0 {
        0
    } else {
        ((sample.ram_used_gb / sample.ram_total_gb) * 100.0).round() as u32
    };
    if !home_standby_allowed(
        home_count,
        travel_has_players,
        ram_pct >= policy.ram_pressure_pct as u32,
    ) {
        return;
    }
    let _ = save_world_if_available(state, &slots.home).await;
    let _ = backup::run_slot_backup(
        &state.pool,
        &state.config,
        "home",
        &slots.home,
        "home_resource_standby",
    )
    .await;
    match state.systemd.stop_unit(&slots.home.systemd_unit).await {
        Ok(()) => {
            audit::record(
                &state.pool,
                &AuditEvent::new(Severity::Warn, "Policy", "Home entered Resource Standby")
                    .target(&slots.home.map_key)
                    .detail(
                        "confirmed empty, travel players active, RAM pressure threshold reached",
                    ),
            )
            .await;
            let _ = state.systemd.reset_failed_unit(&slots.home.systemd_unit).await;
        }
        Err(err) => {
            audit::record(
                &state.pool,
                &AuditEvent::new(Severity::Error, "Policy", "Home Resource Standby failed")
                    .target(&slots.home.map_key)
                    .detail(sanitize_error(&err.to_string())),
            )
            .await;
        }
    }
}

async fn apply_home_idle_shutdown(state: &AppState) {
    if !state.config.operations.systemd_control_enabled
        || !state.config.operations.home_idle_shutdown_enabled
    {
        return;
    }
    let threshold = state.config.operations.home_idle_shutdown_secs as u64;
    let now = epoch_secs();
    let Some(slots) = &state.config.slots else {
        return;
    };

    let mut shutdown_due = false;
    {
        let mut runtime = state.rcon_runtime.write().await;
        let Some(entry) = runtime.slots.get(&slots.home.id).cloned() else {
            return;
        };
        if !entry.active {
            runtime.home_idle_since = None;
            return;
        }
        match entry.player_count {
            Some(0) => match runtime.home_idle_since {
                None => runtime.home_idle_since = Some(now),
                Some(since) if now.saturating_sub(since) >= threshold => shutdown_due = true,
                Some(_) => {}
            },
            Some(_) => runtime.home_idle_since = None,
            None => {} // unknown count, don't start timer
        }
    }

    if !shutdown_due {
        return;
    }

    {
        let mut runtime = state.rcon_runtime.write().await;
        runtime.home_idle_since = None;
    }

    let _ = save_world_if_available(state, &slots.home).await;
    let _ = backup::run_slot_backup(
        &state.pool,
        &state.config,
        "home",
        &slots.home,
        "home_idle_shutdown",
    )
    .await;
    match state.systemd.stop_unit(&slots.home.systemd_unit).await {
        Ok(()) => {
            audit::record(
                &state.pool,
                &AuditEvent::new(Severity::Warn, "Policy", "Home stopped after idle timeout")
                    .target(&slots.home.map_key)
                    .detail(format!("confirmed empty for threshold_secs={threshold}")),
            )
            .await;
            let _ = state.systemd.reset_failed_unit(&slots.home.systemd_unit).await;
        }
        Err(err) => {
            audit::record(
                &state.pool,
                &AuditEvent::new(Severity::Error, "Policy", "Home idle shutdown failed")
                    .target(&slots.home.map_key)
                    .detail(sanitize_error(&err.to_string())),
            )
            .await;
        }
    }
}

async fn save_world_if_available(_state: &AppState, slot: &ServerSlot) -> Result<(), RconError> {
    let Some(rcon) = &slot.rcon else {
        return Ok(());
    };
    let password = std::env::var(&rcon.password_env).unwrap_or_default();
    if password.trim().is_empty() {
        return Ok(());
    }
    let _ = run_rcon_command(
        &rcon.host,
        rcon.port,
        &password,
        "SaveWorld",
        Duration::from_secs(5),
    )
    .await?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdlePolicyDecision {
    Noop,
    SetIdleSince(u64),
    ClearIdle,
    ShutdownDue,
}

pub fn idle_policy_update(
    is_home: bool,
    active: bool,
    player_count: Option<u32>,
    idle_zero_since: Option<u64>,
    now: u64,
    threshold_secs: u64,
) -> IdlePolicyDecision {
    if is_home || !active {
        return IdlePolicyDecision::ClearIdle;
    }
    match player_count {
        Some(0) => match idle_zero_since {
            Some(since) if now.saturating_sub(since) >= threshold_secs => {
                IdlePolicyDecision::ShutdownDue
            }
            Some(_) => IdlePolicyDecision::Noop,
            None => IdlePolicyDecision::SetIdleSince(now),
        },
        Some(_) => IdlePolicyDecision::ClearIdle,
        None => IdlePolicyDecision::ClearIdle,
    }
}

pub fn home_standby_allowed(
    home_player_count: Option<u32>,
    travel_has_players: bool,
    under_pressure: bool,
) -> bool {
    matches!(home_player_count, Some(0)) && travel_has_players && under_pressure
}

fn map_name(config: &Config, map_id: &str) -> String {
    config
        .maps
        .iter()
        .find(|m| m.id == map_id)
        .map(|m| m.name.clone())
        .unwrap_or_else(|| map_id.into())
}

fn sanitize_error(input: &str) -> String {
    input
        .split_whitespace()
        .map(|part| {
            if part.len() > 48 && part.chars().any(|c| c.is_ascii_digit()) {
                "[redacted]"
            } else {
                part
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn now_string() -> String {
    epoch_secs().to_string()
}

fn epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packet_round_trips() {
        let packet = RconPacket {
            id: 42,
            packet_type: RCON_EXECCOMMAND,
            body: "ListPlayers".into(),
        };
        let encoded = encode_packet(&packet);
        let decoded = decode_packet(&encoded).unwrap();
        assert_eq!(decoded, packet);
    }

    #[test]
    fn malformed_packet_rejected() {
        assert!(decode_packet(&[1, 2, 3]).is_err());
    }

    #[test]
    fn parses_travel_command_with_multi_word_map() {
        let parsed = parse_detected_command("  [Global] Marcel: !travel the island ").unwrap();
        assert_eq!(parsed.command, "travel");
        assert_eq!(parsed.argument, "the island");
        assert_eq!(parsed.mode, "live_chat");
    }

    #[test]
    fn parses_chat_actor_examples() {
        let parsed = parse_chat_line("[Global] PlayerName: !travel fjordur");
        assert_eq!(parsed.player, "PlayerName");
        assert_eq!(parsed.detected_command.unwrap().argument, "fjordur");

        let parsed = parse_chat_line("TribeName(Tribe): !travel genesis 2");
        assert_eq!(parsed.player, "TribeName(Tribe)");
        assert_eq!(parsed.detected_command.unwrap().argument, "genesis 2");

        let parsed = parse_chat_line("Człowiek: !travel gen1");
        assert_eq!(parsed.player, "Człowiek");
        assert_eq!(parsed.detected_command.unwrap().argument, "gen1");

        let parsed = parse_chat_line("Człowiek(PLEMIE):  !travel   gen1");
        assert_eq!(parsed.player, "Człowiek(PLEMIE)");
        assert_eq!(parsed.detected_command.unwrap().argument, "gen1");

        let parsed = parse_chat_line("[Lokalny] Człowiek(PLEMIE):  !TrAvEl   fjord");
        assert_eq!(parsed.player, "Człowiek(PLEMIE)");
        assert_eq!(parsed.detected_command.unwrap().argument, "fjord");
    }

    #[test]
    fn parses_player_list_lines() {
        assert_eq!(
            parse_players("1. Marcel, 76561198000000000\n2. Nyx\n"),
            vec!["Marcel".to_string(), "Nyx".to_string()]
        );
        assert!(parse_players("No Players Connected").is_empty());
    }

    #[test]
    fn duplicate_chat_line_is_ignored() {
        let mut runtime = RconRuntimeState::new();
        assert!(runtime
            .push_chat_line("home", "log_tail", "Player: !travel fjordur")
            .is_some());
        assert!(runtime
            .push_chat_line("home", "log_tail", "Player: !travel fjordur")
            .is_none());
    }

    #[test]
    fn idle_policy_requires_confirmed_zero() {
        assert_eq!(
            idle_policy_update(false, true, None, None, 100, 10),
            IdlePolicyDecision::ClearIdle
        );
        assert_eq!(
            idle_policy_update(false, true, Some(0), None, 100, 10),
            IdlePolicyDecision::SetIdleSince(100)
        );
        assert_eq!(
            idle_policy_update(false, true, Some(1), Some(90), 100, 10),
            IdlePolicyDecision::ClearIdle
        );
        assert_eq!(
            idle_policy_update(false, true, Some(0), Some(89), 100, 10),
            IdlePolicyDecision::ShutdownDue
        );
        assert_eq!(
            idle_policy_update(true, true, Some(0), Some(1), 100, 10),
            IdlePolicyDecision::ClearIdle
        );
    }

    #[test]
    fn home_standby_blocks_unknown_or_players() {
        assert!(!home_standby_allowed(None, true, true));
        assert!(!home_standby_allowed(Some(1), true, true));
        assert!(!home_standby_allowed(Some(0), false, true));
        assert!(!home_standby_allowed(Some(0), true, false));
        assert!(home_standby_allowed(Some(0), true, true));
    }

    #[test]
    fn ark_feedback_uses_player_safe_message() {
        // Ports not yet open when systemd reports "starting" — connection info must be absent.
        let decision = travel::TravelDecision {
            id: "travel-1".into(),
            accepted: true,
            requested_map: "gen1".into(),
            resolved_map: Some("genesis-1".into()),
            resolved_map_name: Some("Genesis: Part 1".into()),
            chosen_slot: Some("travel-a".into()),
            status: "starting".into(),
            reason: "starting Genesis: Part 1".into(),
            user_message: "Starting Genesis: Part 1. This may take a few minutes. Connection info unavailable: server starting: waiting for game ports to open.".into(),
            connect_host: Some("100.68.7.42".into()),
            game_port: Some(7781),
            query_port: Some(27016),
            connection_address: None,
            query_address: None,
            connection_source: Some("slot_config".into()),
            connection_available: false,
            connection_unavailable_reason: Some("server starting: waiting for game ports to open".into()),
            resource_warning: None,
        };
        assert_eq!(
            ark_feedback_for_decision(&decision),
            "[ARK Cluster] Starting Genesis: Part 1. This may take a few minutes. Connection info unavailable: server starting: waiting for game ports to open."
        );
        assert_eq!(sanitize_chat_message("a\n b\r c"), "a b c");
    }
}
