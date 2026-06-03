//! Configuration loading and validation.
//!
//! Config is read from a TOML file (default `manager.toml`). The API token may
//! be overridden by the `ARK_MANAGER_API_TOKEN` environment variable, which is
//! the preferred way to supply secrets — they should never live in committed
//! files. The whole config is validated on startup; invalid configs abort the
//! process before any listener binds.

use std::collections::HashSet;
use std::net::IpAddr;
use std::path::Path;

use serde::Deserialize;

pub const API_TOKEN_ENV: &str = "ARK_MANAGER_API_TOKEN";

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub cluster: ClusterConfig,
    pub server: ServerConfig,
    pub resource_policy: ResourcePolicy,
    pub backup_policy: BackupPolicy,
    pub discord: DiscordConfig,
    #[serde(default)]
    pub travel_slots: Vec<TravelSlot>,
    #[serde(default)]
    pub maps: Vec<MapConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ClusterConfig {
    pub name: String,
    pub id: String,
    pub directory: String,
    #[serde(default)]
    pub manager_version: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub bind_address: String,
    pub port: u16,
    #[serde(default)]
    pub api_token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResourcePolicy {
    pub ram_warn_pct: u8,
    pub ram_pressure_pct: u8,
    pub ram_emergency_pct: u8,
    pub max_travel_servers: u32,
    pub empty_shutdown_mins: u32,
    pub home_standby_enabled: bool,
    pub never_stop_with_players: bool,
    pub home_stops_only_when_empty: bool,
    pub prefer_active_player_maps: bool,
    pub auto_restart_home: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BackupPolicy {
    pub before_shutdown: bool,
    pub before_config_save: bool,
    pub before_mod_change: bool,
    pub retention: String,
    pub directory: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DiscordConfig {
    pub enabled: bool,
    pub guild: String,
    pub status_channel: String,
    #[serde(default)]
    pub bot_token_env: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TravelSlot {
    pub name: String,
    pub role: String, // "home" | "travel"
}

#[derive(Debug, Clone, Deserialize)]
pub struct MapConfig {
    pub id: String,
    pub name: String,
    pub alias: String,
    pub ark_map_name: String,
    pub systemd_unit: String,
    pub query_port: u16,
    pub rcon_port: u16,
    pub game_port: u16,
    #[serde(default)]
    pub slot_priority: i32,
    #[serde(default)]
    pub can_be_home: bool,
    #[serde(default = "default_true")]
    pub can_auto_stop_when_empty: bool,
    #[serde(default)]
    pub can_enter_standby: bool,
    #[serde(default = "default_unassigned")]
    pub assignment: String,
    #[serde(default)]
    pub mods: Vec<String>,
}

fn default_true() -> bool {
    true
}
fn default_unassigned() -> String {
    "Unassigned".to_string()
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("config file not found: {0}")]
    NotFound(String),
    #[error("failed to read config file {0}: {1}")]
    Read(String, std::io::Error),
    #[error("failed to parse TOML config: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("config validation failed: {0}")]
    Invalid(String),
}

impl Config {
    /// Load config from `path`, apply the env-var token override, then validate.
    pub fn load(path: &str) -> Result<Self, ConfigError> {
        if !Path::new(path).exists() {
            return Err(ConfigError::NotFound(path.to_string()));
        }
        let raw =
            std::fs::read_to_string(path).map_err(|e| ConfigError::Read(path.to_string(), e))?;
        let mut cfg: Config = toml::from_str(&raw)?;

        if let Ok(tok) = std::env::var(API_TOKEN_ENV) {
            if !tok.trim().is_empty() {
                cfg.server.api_token = tok;
            }
        }

        cfg.validate()?;
        Ok(cfg)
    }

    /// Validate the config. Rejects duplicate ports, duplicate map keys, a
    /// missing Home map, too many travel slots, and unsafe paths.
    pub fn validate(&self) -> Result<(), ConfigError> {
        let inv = |m: String| ConfigError::Invalid(m);

        // --- token must exist (from file or env) ---
        if self.server.api_token.trim().is_empty() {
            return Err(inv(format!(
                "api_token is empty; set it in [server] or via {API_TOKEN_ENV}"
            )));
        }

        // --- bind address must parse as an IP ---
        if self.server.bind_address.parse::<IpAddr>().is_err() {
            return Err(inv(format!(
                "server.bind_address '{}' is not a valid IP address",
                self.server.bind_address
            )));
        }

        // --- thresholds sane and ordered ---
        let p = &self.resource_policy;
        if !(p.ram_warn_pct <= p.ram_pressure_pct && p.ram_pressure_pct <= p.ram_emergency_pct) {
            return Err(inv(
                "resource_policy thresholds must satisfy ram_warn_pct <= ram_pressure_pct <= ram_emergency_pct".into(),
            ));
        }
        if p.ram_emergency_pct > 100 {
            return Err(inv(
                "resource_policy.ram_emergency_pct must be <= 100".into()
            ));
        }

        // --- safe paths ---
        validate_safe_path("cluster.directory", &self.cluster.directory)?;
        validate_safe_path("backup_policy.directory", &self.backup_policy.directory)?;

        // --- travel slots: exactly one home, travel count within policy ---
        let home_slots = self
            .travel_slots
            .iter()
            .filter(|s| s.role.eq_ignore_ascii_case("home"))
            .count();
        let travel_slots = self
            .travel_slots
            .iter()
            .filter(|s| s.role.eq_ignore_ascii_case("travel"))
            .count();
        for s in &self.travel_slots {
            if !s.role.eq_ignore_ascii_case("home") && !s.role.eq_ignore_ascii_case("travel") {
                return Err(inv(format!(
                    "travel slot '{}' has invalid role '{}' (expected 'home' or 'travel')",
                    s.name, s.role
                )));
            }
        }
        if home_slots != 1 {
            return Err(inv(format!(
                "exactly one travel slot must have role 'home' (found {home_slots})"
            )));
        }
        if travel_slots as u32 > p.max_travel_servers {
            return Err(inv(format!(
                "travel slot count ({travel_slots}) exceeds resource_policy.max_travel_servers ({})",
                p.max_travel_servers
            )));
        }

        // --- maps present ---
        if self.maps.is_empty() {
            return Err(inv("at least one map must be configured".into()));
        }

        // --- duplicate map keys / ports ---
        let mut ids = HashSet::new();
        let mut ports = HashSet::new();
        let mut home_capable = false;
        let mut home_assigned = 0u32;
        for m in &self.maps {
            if !ids.insert(m.id.as_str()) {
                return Err(inv(format!("duplicate map id '{}'", m.id)));
            }
            for (label, port) in [
                ("query_port", m.query_port),
                ("rcon_port", m.rcon_port),
                ("game_port", m.game_port),
            ] {
                if !ports.insert(port) {
                    return Err(inv(format!(
                        "duplicate port {port} ({label}) on map '{}' — every query/rcon/game port must be unique",
                        m.id
                    )));
                }
            }
            if m.can_be_home {
                home_capable = true;
            }
            if m.assignment.eq_ignore_ascii_case("home") {
                home_assigned += 1;
                if !m.can_be_home {
                    return Err(inv(format!(
                        "map '{}' is assigned Home but can_be_home = false",
                        m.id
                    )));
                }
            }
        }
        if !home_capable {
            return Err(inv(
                "no Home map: at least one map must set can_be_home = true".into(),
            ));
        }
        if home_assigned > 1 {
            return Err(inv(format!(
                "{home_assigned} maps are assigned Home; only one map may hold the Home slot"
            )));
        }

        Ok(())
    }

    pub fn socket_addr(&self) -> std::net::SocketAddr {
        let ip: IpAddr = self
            .server
            .bind_address
            .parse()
            .expect("bind_address validated during Config::validate");
        std::net::SocketAddr::new(ip, self.server.port)
    }

    /// True when the bind address is a private/loopback address — i.e. safe for
    /// the intended Tailscale/LAN-only deployment.
    pub fn bind_is_private(&self) -> bool {
        match self.server.bind_address.parse::<IpAddr>() {
            Ok(IpAddr::V4(v4)) => {
                v4.is_loopback() || v4.is_private() || v4.octets()[0] == 100 // Tailscale CGNAT 100.64.0.0/10
            }
            Ok(IpAddr::V6(v6)) => v6.is_loopback() || v6.is_unique_local(),
            Err(_) => false,
        }
    }
}

/// Reject relative paths, empty paths, and any path containing a `..` segment.
fn validate_safe_path(label: &str, path: &str) -> Result<(), ConfigError> {
    if path.trim().is_empty() {
        return Err(ConfigError::Invalid(format!("{label} must not be empty")));
    }
    let p = Path::new(path);
    if p.components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(ConfigError::Invalid(format!(
            "{label} '{path}' contains a '..' segment which is not allowed"
        )));
    }
    // Must be absolute (Unix '/...' or Windows 'C:\...').
    let absolute = path.starts_with('/') || p.is_absolute();
    if !absolute {
        return Err(ConfigError::Invalid(format!(
            "{label} '{path}' must be an absolute path"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Config {
        Config {
            cluster: ClusterConfig {
                name: "c".into(),
                id: "id".into(),
                directory: "/srv/ark".into(),
                manager_version: "v".into(),
            },
            server: ServerConfig {
                bind_address: "127.0.0.1".into(),
                port: 8787,
                api_token: "tok".into(),
            },
            resource_policy: ResourcePolicy {
                ram_warn_pct: 70,
                ram_pressure_pct: 82,
                ram_emergency_pct: 92,
                max_travel_servers: 2,
                empty_shutdown_mins: 10,
                home_standby_enabled: true,
                never_stop_with_players: true,
                home_stops_only_when_empty: true,
                prefer_active_player_maps: true,
                auto_restart_home: true,
            },
            backup_policy: BackupPolicy {
                before_shutdown: true,
                before_config_save: true,
                before_mod_change: true,
                retention: "r".into(),
                directory: "/srv/ark/backups".into(),
            },
            discord: DiscordConfig {
                enabled: false,
                guild: "g".into(),
                status_channel: "#c".into(),
                bot_token_env: "X".into(),
            },
            travel_slots: vec![
                TravelSlot {
                    name: "Home".into(),
                    role: "home".into(),
                },
                TravelSlot {
                    name: "Travel A".into(),
                    role: "travel".into(),
                },
                TravelSlot {
                    name: "Travel B".into(),
                    role: "travel".into(),
                },
            ],
            maps: vec![
                MapConfig {
                    id: "home".into(),
                    name: "Home".into(),
                    alias: "home".into(),
                    ark_map_name: "TheIsland".into(),
                    systemd_unit: "ark@home.service".into(),
                    query_port: 27015,
                    rcon_port: 27020,
                    game_port: 7777,
                    slot_priority: 0,
                    can_be_home: true,
                    can_auto_stop_when_empty: false,
                    can_enter_standby: true,
                    assignment: "Home".into(),
                    mods: vec![],
                },
                MapConfig {
                    id: "rag".into(),
                    name: "Ragnarok".into(),
                    alias: "rag".into(),
                    ark_map_name: "Ragnarok".into(),
                    systemd_unit: "ark@a.service".into(),
                    query_port: 27017,
                    rcon_port: 27022,
                    game_port: 7779,
                    slot_priority: 1,
                    can_be_home: false,
                    can_auto_stop_when_empty: true,
                    can_enter_standby: false,
                    assignment: "Travel A".into(),
                    mods: vec![],
                },
            ],
        }
    }

    #[test]
    fn valid_config_passes() {
        assert!(base().validate().is_ok());
    }

    #[test]
    fn duplicate_map_id_rejected() {
        let mut c = base();
        c.maps[1].id = "home".into();
        assert!(c.validate().is_err());
    }

    #[test]
    fn duplicate_port_rejected() {
        let mut c = base();
        c.maps[1].query_port = 27015;
        assert!(c.validate().is_err());
    }

    #[test]
    fn missing_home_rejected() {
        let mut c = base();
        c.maps[0].can_be_home = false;
        c.maps[0].assignment = "Unassigned".into();
        assert!(c.validate().is_err());
    }

    #[test]
    fn too_many_travel_slots_rejected() {
        let mut c = base();
        c.resource_policy.max_travel_servers = 1;
        assert!(c.validate().is_err());
    }

    #[test]
    fn unsafe_path_rejected() {
        let mut c = base();
        c.backup_policy.directory = "/srv/../etc".into();
        assert!(c.validate().is_err());
    }

    #[test]
    fn empty_token_rejected() {
        let mut c = base();
        c.server.api_token = "".into();
        assert!(c.validate().is_err());
    }

    #[test]
    fn private_bind_detection() {
        let mut c = base();
        assert!(c.bind_is_private());
        c.server.bind_address = "100.84.1.2".into();
        assert!(c.bind_is_private());
        c.server.bind_address = "8.8.8.8".into();
        assert!(!c.bind_is_private());
    }
}
