use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::config::Config;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigSetRequest {
    pub file: String,
    pub key: String,
    pub value: String,
    #[serde(default)]
    pub confirm: bool,
    #[serde(default = "default_reason")]
    pub reason: String,
}

fn default_reason() -> String {
    "manual_config_change".into()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedConfig {
    pub writable: bool,
    pub shared_config_dir: String,
    pub game_ini_path: String,
    pub game_user_settings_ini_path: String,
    pub game_ini: String,
    pub game_user_settings_ini: String,
    pub masked: bool,
}

pub fn read_shared(config: &Config) -> SharedConfig {
    let dir = shared_config_dir(config);
    let game_ini_path = dir.join("Game.ini");
    let gus_path = dir.join("GameUserSettings.ini");
    SharedConfig {
        writable: config.operations.config_writes_enabled,
        shared_config_dir: dir.display().to_string(),
        game_ini_path: game_ini_path.display().to_string(),
        game_user_settings_ini_path: gus_path.display().to_string(),
        game_ini: mask(&fs::read_to_string(&game_ini_path).unwrap_or_default()),
        game_user_settings_ini: mask(&fs::read_to_string(&gus_path).unwrap_or_default()),
        masked: true,
    }
}

pub async fn set_value(
    pool: &SqlitePool,
    config: &Config,
    req: ConfigSetRequest,
) -> Result<SharedConfig, ConfigEditError> {
    if !config.operations.config_writes_enabled {
        return Err(ConfigEditError::Disabled);
    }
    if !req.confirm {
        return Err(ConfigEditError::InvalidConfirmation);
    }
    let path = match req.file.as_str() {
        "Game.ini" => shared_config_dir(config).join("Game.ini"),
        "GameUserSettings.ini" => shared_config_dir(config).join("GameUserSettings.ini"),
        _ => return Err(ConfigEditError::InvalidFile),
    };
    let mut raw = fs::read_to_string(&path)?;
    let backup_path = format!("{}.bak-{}", path.display(), epoch_secs());
    fs::copy(&path, &backup_path)?;
    replace_ini_key(&mut raw, &req.key, &req.value)?;
    fs::write(&path, raw)?;
    sqlx::query(
        "INSERT INTO config_snapshots (id, actor, file, reason, backup_path, status) \
         VALUES (?1, 'manager', ?2, ?3, ?4, 'applied')",
    )
    .bind(format!("cfg-{}", epoch_secs()))
    .bind(&req.file)
    .bind(&req.reason)
    .bind(&backup_path)
    .execute(pool)
    .await?;
    Ok(read_shared(config))
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigEditError {
    #[error("config writes disabled in manager config")]
    Disabled,
    #[error("explicit confirmation required")]
    InvalidConfirmation,
    #[error("invalid config file")]
    InvalidFile,
    #[error("invalid config key")]
    InvalidKey,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
}

impl ConfigEditError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Disabled => "CONFIG_WRITES_DISABLED",
            Self::InvalidConfirmation => "INVALID_CONFIRMATION",
            Self::InvalidFile => "INVALID_CONFIG_FILE",
            Self::InvalidKey => "INVALID_CONFIG_KEY",
            Self::Io(_) | Self::Db(_) => "OPERATION_FAILED",
        }
    }
}

fn shared_config_dir(config: &Config) -> PathBuf {
    let root = if config.paths.ark_root.trim().is_empty() {
        "/srv/ark"
    } else {
        &config.paths.ark_root
    };
    Path::new(root).join("server/ShooterGame/Saved/Config/LinuxServer")
}

fn replace_ini_key(raw: &mut String, key: &str, value: &str) -> Result<(), ConfigEditError> {
    if key.is_empty()
        || !key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
        || key.to_ascii_lowercase().contains("password")
    {
        return Err(ConfigEditError::InvalidKey);
    }
    let line = format!("{key}={value}");
    let mut found = false;
    let mut lines = raw
        .lines()
        .map(|existing| {
            if existing
                .split_once('=')
                .map(|(k, _)| k.trim() == key)
                .unwrap_or(false)
            {
                found = true;
                line.clone()
            } else {
                existing.to_string()
            }
        })
        .collect::<Vec<_>>();
    if !found {
        lines.push(line);
    }
    *raw = lines.join("\n");
    raw.push('\n');
    Ok(())
}

fn mask(raw: &str) -> String {
    raw.lines()
        .map(|line| {
            if line
                .split_once('=')
                .map(|(k, _)| k.to_ascii_lowercase().contains("password"))
                .unwrap_or(false)
            {
                format!("{}=********", line.split_once('=').unwrap().0)
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn epoch_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
