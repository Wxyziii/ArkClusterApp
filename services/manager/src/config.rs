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

use crate::models::systemd::is_safe_unit_name;

pub const API_TOKEN_ENV: &str = "ARK_MANAGER_API_TOKEN";

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub cluster: ClusterConfig,
    pub server: ServerConfig,
    #[serde(default)]
    pub operations: OperationsConfig,
    #[serde(default)]
    pub paths: PathsConfig,
    pub resource_policy: ResourcePolicy,
    pub backup_policy: BackupPolicy,
    #[serde(default)]
    pub rcon: RconConfig,
    pub discord: DiscordConfig,
    #[serde(default)]
    pub slots: Option<SlotsConfig>,
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
pub struct OperationsConfig {
    #[serde(default)]
    pub systemd_control_enabled: bool,
    #[serde(default = "default_true")]
    pub backup_enabled: bool,
    #[serde(default)]
    pub rcon_enabled: bool,
    #[serde(default)]
    pub allow_home_manual_stop: bool,
    #[serde(default = "default_true")]
    pub allow_travel_manual_stop: bool,
    #[serde(default = "default_true")]
    pub require_confirmation_token: bool,
}

impl Default for OperationsConfig {
    fn default() -> Self {
        Self {
            systemd_control_enabled: false,
            backup_enabled: true,
            rcon_enabled: false,
            allow_home_manual_stop: false,
            allow_travel_manual_stop: true,
            require_confirmation_token: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PathsConfig {
    #[serde(default)]
    pub ark_root: String,
    #[serde(default)]
    pub backup_root: String,
    #[serde(default)]
    pub cluster_dir: String,
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
pub struct RconConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_rcon_poll")]
    pub poll_interval_seconds: u32,
}

impl Default for RconConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            poll_interval_seconds: default_rcon_poll(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct TravelSlot {
    pub name: String,
    pub role: String, // "home" | "travel"
}

#[derive(Debug, Clone, Deserialize)]
pub struct SlotsConfig {
    pub home: ServerSlot,
    #[serde(default)]
    pub travel_a: Option<ServerSlot>,
    #[serde(default)]
    pub travel_b: Option<ServerSlot>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerSlot {
    pub id: String,
    pub label: String,
    pub map_key: String,
    pub systemd_unit: String,
    pub game_port: u16,
    pub query_port: u16,
    pub rcon_port: u16,
    #[serde(default)]
    pub save_path: Option<String>,
    #[serde(default)]
    pub config_path: Option<String>,
    #[serde(default)]
    pub paths: SlotPaths,
    #[serde(default)]
    pub rcon: Option<SlotRconConfig>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub protected: bool,
    #[serde(default)]
    pub home_resource_standby_enabled: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SlotPaths {
    #[serde(default)]
    pub save_dir: Option<String>,
    #[serde(default)]
    pub config_dir: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SlotRconConfig {
    pub host: String,
    pub port: u16,
    pub password_env: String,
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
fn default_rcon_poll() -> u32 {
    5
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
        if !self.paths.ark_root.trim().is_empty() {
            validate_safe_path("paths.ark_root", &self.paths.ark_root)?;
        }
        if !self.paths.backup_root.trim().is_empty() {
            validate_safe_path("paths.backup_root", &self.paths.backup_root)?;
        }
        if !self.paths.cluster_dir.trim().is_empty() {
            validate_safe_path("paths.cluster_dir", &self.paths.cluster_dir)?;
        }
        if self.rcon.poll_interval_seconds == 0 {
            return Err(inv("rcon.poll_interval_seconds must be > 0".into()));
        }

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
        let mut travel_slot_names = HashSet::new();
        for s in &self.travel_slots {
            if !travel_slot_names.insert(s.name.to_ascii_lowercase()) {
                return Err(inv(format!("duplicate travel slot name '{}'", s.name)));
            }
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

        if let Some(slots) = &self.slots {
            self.validate_server_slots(slots)?;
        }

        // --- duplicate map keys / units / ports ---
        let mut ids = HashSet::new();
        let mut units = HashSet::new();
        let mut ports = HashSet::new();
        let mut home_capable = false;
        let mut home_assigned = 0u32;
        for m in &self.maps {
            if !ids.insert(m.id.as_str()) {
                return Err(inv(format!("duplicate map id '{}'", m.id)));
            }
            if !is_safe_unit_name(&m.systemd_unit) {
                return Err(inv(format!(
                    "map '{}' has unsafe systemd unit name '{}'",
                    m.id, m.systemd_unit
                )));
            }
            if !units.insert(m.systemd_unit.as_str()) {
                return Err(inv(format!("duplicate systemd unit '{}'", m.systemd_unit)));
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

    fn validate_server_slots(&self, slots: &SlotsConfig) -> Result<(), ConfigError> {
        let inv = |m: String| ConfigError::Invalid(m);
        let mut ids = HashSet::new();
        let mut units = HashSet::new();
        let mut ports = HashSet::new();
        let all_slots = slots.iter();
        let travel_count = all_slots
            .iter()
            .filter(|(_, key, _)| *key != "home")
            .count();

        if travel_count > 2 {
            return Err(inv(format!(
                "slots config has {travel_count} travel slots; max travel slots is 2 for T1.1"
            )));
        }

        for (slot, key, is_home) in all_slots {
            if slot.id.trim().is_empty() {
                return Err(inv(format!("slots.{key}.id must not be empty")));
            }
            if slot.label.trim().is_empty() {
                return Err(inv(format!("slots.{key}.label must not be empty")));
            }
            if slot.map_key.trim().is_empty() {
                return Err(inv(format!("slots.{key}.map_key must not be empty")));
            }
            if !ids.insert(slot.id.as_str()) {
                return Err(inv(format!("duplicate slot id '{}'", slot.id)));
            }
            if !is_safe_unit_name(&slot.systemd_unit) {
                return Err(inv(format!(
                    "slots.{key}.systemd_unit '{}' is not a safe unit name",
                    slot.systemd_unit
                )));
            }
            if !units.insert(slot.systemd_unit.as_str()) {
                return Err(inv(format!(
                    "duplicate slot systemd unit '{}'",
                    slot.systemd_unit
                )));
            }
            for (label, port) in [
                ("game_port", slot.game_port),
                ("query_port", slot.query_port),
                ("rcon_port", slot.rcon_port),
            ] {
                if !ports.insert(port) {
                    return Err(inv(format!(
                        "duplicate slot port {port} ({label}) on slot '{}'",
                        slot.id
                    )));
                }
            }
            if is_home && !slot.protected {
                return Err(inv("slots.home.protected must be true".into()));
            }
            for (label, maybe_path) in [
                ("save_path", slot.save_path.as_deref()),
                ("config_path", slot.config_path.as_deref()),
                ("paths.save_dir", slot.paths.save_dir.as_deref()),
                ("paths.config_dir", slot.paths.config_dir.as_deref()),
            ] {
                if let Some(path) = maybe_path {
                    validate_safe_path(&format!("slots.{key}.{label}"), path)?;
                    let allowed_root = if self.paths.ark_root.trim().is_empty() {
                        &self.cluster.directory
                    } else {
                        &self.paths.ark_root
                    };
                    validate_path_under_root(&format!("slots.{key}.{label}"), path, allowed_root)?;
                }
            }
            if let Some(rcon) = &slot.rcon {
                if rcon.host.trim().is_empty() {
                    return Err(inv(format!("slots.{key}.rcon.host must not be empty")));
                }
                if rcon.password_env.trim().is_empty()
                    || !rcon
                        .password_env
                        .chars()
                        .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
                {
                    return Err(inv(format!(
                        "slots.{key}.rcon.password_env must be an uppercase env var name"
                    )));
                }
            }
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

impl SlotsConfig {
    pub fn iter(&self) -> Vec<(&ServerSlot, &'static str, bool)> {
        let mut slots = vec![(&self.home, "home", true)];
        if let Some(slot) = &self.travel_a {
            slots.push((slot, "travel_a", false));
        }
        if let Some(slot) = &self.travel_b {
            slots.push((slot, "travel_b", false));
        }
        slots
    }
}

impl ServerSlot {
    pub fn save_dir(&self) -> Option<&str> {
        self.paths.save_dir.as_deref().or(self.save_path.as_deref())
    }

    pub fn config_dir(&self) -> Option<&str> {
        self.paths
            .config_dir
            .as_deref()
            .or(self.config_path.as_deref())
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

fn validate_path_under_root(label: &str, path: &str, root: &str) -> Result<(), ConfigError> {
    let p = path.trim_end_matches('/');
    let r = root.trim_end_matches('/');
    if p == r || p.starts_with(&format!("{r}/")) {
        Ok(())
    } else {
        Err(ConfigError::Invalid(format!(
            "{label} '{path}' must stay under cluster.directory '{root}'"
        )))
    }
}

#[cfg(test)]
pub(crate) mod tests_support {
    use super::*;

    pub fn base_config() -> Config {
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
            operations: OperationsConfig::default(),
            paths: PathsConfig {
                ark_root: "/srv/ark".into(),
                backup_root: "/srv/ark/backups".into(),
                cluster_dir: "/srv/ark/clusters/main".into(),
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
            rcon: RconConfig::default(),
            discord: DiscordConfig {
                enabled: false,
                guild: "g".into(),
                status_channel: "#c".into(),
                bot_token_env: "X".into(),
            },
            slots: Some(SlotsConfig {
                home: ServerSlot {
                    id: "home".into(),
                    label: "Home".into(),
                    map_key: "home".into(),
                    systemd_unit: "ark-server@home.service".into(),
                    game_port: 7777,
                    query_port: 27015,
                    rcon_port: 27020,
                    save_path: Some("/srv/ark/home/Saved".into()),
                    config_path: Some("/srv/ark/home/Saved/Config/LinuxServer".into()),
                    paths: SlotPaths::default(),
                    rcon: Some(SlotRconConfig {
                        host: "127.0.0.1".into(),
                        port: 27020,
                        password_env: "ARK_HOME_RCON_PASSWORD".into(),
                    }),
                    enabled: true,
                    protected: true,
                    home_resource_standby_enabled: true,
                },
                travel_a: Some(ServerSlot {
                    id: "travel-a".into(),
                    label: "Travel A".into(),
                    map_key: "rag".into(),
                    systemd_unit: "ark-server@travel-a.service".into(),
                    game_port: 7779,
                    query_port: 27017,
                    rcon_port: 27022,
                    save_path: None,
                    config_path: None,
                    paths: SlotPaths::default(),
                    rcon: None,
                    enabled: true,
                    protected: false,
                    home_resource_standby_enabled: false,
                }),
                travel_b: Some(ServerSlot {
                    id: "travel-b".into(),
                    label: "Travel B".into(),
                    map_key: "ab".into(),
                    systemd_unit: "ark-server@travel-b.service".into(),
                    game_port: 7781,
                    query_port: 27019,
                    rcon_port: 27024,
                    save_path: None,
                    config_path: None,
                    paths: SlotPaths::default(),
                    rcon: None,
                    enabled: true,
                    protected: false,
                    home_resource_standby_enabled: false,
                }),
            }),
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Config {
        tests_support::base_config()
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
    fn duplicate_systemd_unit_rejected() {
        let mut c = base();
        c.maps[1].systemd_unit = c.maps[0].systemd_unit.clone();
        assert!(c.validate().is_err());
    }

    #[test]
    fn unsafe_systemd_unit_rejected() {
        let mut c = base();
        c.maps[0].systemd_unit = "ark@home.service;shutdown".into();
        assert!(c.validate().is_err());
    }

    #[test]
    fn slot_paths_must_stay_under_cluster_dir() {
        let mut c = base();
        c.slots.as_mut().unwrap().home.save_path = Some("/etc/ark".into());
        assert!(c.validate().is_err());
    }

    #[test]
    fn nested_slot_paths_must_stay_under_ark_root() {
        let mut c = base();
        c.slots.as_mut().unwrap().home.save_path = None;
        c.slots.as_mut().unwrap().home.paths.save_dir = Some("/tmp/ark".into());
        assert!(c.validate().is_err());
    }

    #[test]
    fn bad_rcon_password_env_rejected() {
        let mut c = base();
        c.slots
            .as_mut()
            .unwrap()
            .home
            .rcon
            .as_mut()
            .unwrap()
            .password_env = "bad-name".into();
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
