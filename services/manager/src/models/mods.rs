use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::config::Config;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModMutation {
    pub workshop_id: String,
    #[serde(default)]
    pub confirm: bool,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ModRecord {
    pub workshop_id: String,
    pub name: String,
    pub enabled: i64,
    pub installed: i64,
    pub load_order: i64,
    pub last_updated: Option<String>,
    pub status: String,
    pub error: Option<String>,
}

pub async fn list(pool: &SqlitePool, config: &Config) -> Result<serde_json::Value, sqlx::Error> {
    let records = sqlx::query_as::<_, ModRecord>(
        "SELECT workshop_id, name, enabled, installed, load_order, last_updated, status, error \
         FROM mod_records ORDER BY load_order ASC, workshop_id ASC",
    )
    .fetch_all(pool)
    .await?;
    Ok(serde_json::json!({
        "mutable": config.operations.mod_management_enabled,
        "steamcmdRequired": true,
        "activeModsConfig": "/srv/ark/server/ShooterGame/Saved/Config/LinuxServer/GameUserSettings.ini",
        "mods": records,
        "testModId": "1428596566"
    }))
}

pub async fn record_known(
    pool: &SqlitePool,
    config: &Config,
    req: ModMutation,
    action: &str,
) -> Result<serde_json::Value, ModError> {
    if !config.operations.mod_management_enabled {
        return Err(ModError::Disabled);
    }
    if !req.confirm {
        return Err(ModError::InvalidConfirmation);
    }
    if !req.workshop_id.chars().all(|c| c.is_ascii_digit()) {
        return Err(ModError::InvalidWorkshopId);
    }
    sqlx::query(
        "INSERT INTO mod_records (workshop_id, name, enabled, installed, load_order, status) \
         VALUES (?1, ?2, ?3, ?4, 0, ?5) \
         ON CONFLICT(workshop_id) DO UPDATE SET enabled=excluded.enabled, status=excluded.status",
    )
    .bind(&req.workshop_id)
    .bind(format!("Workshop {}", req.workshop_id))
    .bind(if action == "enable" || action == "add" {
        1
    } else {
        0
    })
    .bind(if action == "remove" { 0 } else { 1 })
    .bind(format!("{action}_recorded"))
    .execute(pool)
    .await?;
    Ok(list(pool, config).await?)
}

#[derive(Debug, thiserror::Error)]
pub enum ModError {
    #[error("mod management disabled in manager config")]
    Disabled,
    #[error("explicit confirmation required")]
    InvalidConfirmation,
    #[error("invalid workshop id")]
    InvalidWorkshopId,
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
}

impl ModError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Disabled => "MOD_MANAGEMENT_DISABLED",
            Self::InvalidConfirmation => "INVALID_CONFIRMATION",
            Self::InvalidWorkshopId => "INVALID_WORKSHOP_ID",
            Self::Db(_) => "OPERATION_FAILED",
        }
    }
}
