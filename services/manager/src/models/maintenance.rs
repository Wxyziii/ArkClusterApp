use serde::Deserialize;
use sqlx::SqlitePool;

use crate::config::Config;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaintenanceRequest {
    #[serde(default)]
    pub dry_run: bool,
    #[serde(default)]
    pub confirm: bool,
    #[serde(default = "default_reason")]
    pub reason: String,
}

fn default_reason() -> String {
    "manual_maintenance".into()
}

pub async fn status(pool: &SqlitePool, config: &Config) -> Result<serde_json::Value, sqlx::Error> {
    let jobs = sqlx::query_as::<_, JobRow>(
        "SELECT id, ts, kind, status, reason, detail FROM maintenance_jobs ORDER BY ts DESC LIMIT 20",
    )
    .fetch_all(pool)
    .await?;
    Ok(serde_json::json!({
        "enabled": config.operations.maintenance_enabled,
        "steamAppId": "376030",
        "installPath": "/srv/ark/server",
        "safeCommand": "steamcmd +force_install_dir /srv/ark/server +login anonymous +app_update 376030 validate +quit",
        "jobs": jobs
    }))
}

pub async fn ark_update(
    pool: &SqlitePool,
    config: &Config,
    req: MaintenanceRequest,
) -> Result<serde_json::Value, MaintenanceError> {
    if !config.operations.maintenance_enabled {
        return Err(MaintenanceError::Disabled);
    }
    if !req.dry_run && !req.confirm {
        return Err(MaintenanceError::InvalidConfirmation);
    }
    let id = format!("maint-{}", epoch_secs());
    let status = if req.dry_run {
        "dry_run_ready"
    } else {
        "queued"
    };
    let detail = if req.dry_run {
        "dry run only; no SteamCMD process started"
    } else {
        "real update should be run by deploy script after backups"
    };
    sqlx::query(
        "INSERT INTO maintenance_jobs (id, kind, status, reason, detail) VALUES (?1, 'ark_update', ?2, ?3, ?4)",
    )
    .bind(&id)
    .bind(status)
    .bind(&req.reason)
    .bind(detail)
    .execute(pool)
    .await?;
    Ok(serde_json::json!({ "accepted": true, "jobId": id, "status": status, "detail": detail }))
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
struct JobRow {
    id: String,
    ts: String,
    kind: String,
    status: String,
    reason: String,
    detail: String,
}

#[derive(Debug, thiserror::Error)]
pub enum MaintenanceError {
    #[error("ARK maintenance disabled in manager config")]
    Disabled,
    #[error("explicit confirmation required")]
    InvalidConfirmation,
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
}

impl MaintenanceError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Disabled => "MAINTENANCE_DISABLED",
            Self::InvalidConfirmation => "INVALID_CONFIRMATION",
            Self::Db(_) => "OPERATION_FAILED",
        }
    }
}

fn epoch_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
