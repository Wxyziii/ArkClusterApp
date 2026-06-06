use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use sqlx::SqlitePool;

use crate::config::{Config, ServerSlot};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRecord {
    pub id: String,
    pub slot_id: String,
    pub server_id: String,
    pub map_key: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub reason: String,
    pub status: String,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub size_bytes: u64,
    pub path: String,
    pub error: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum BackupError {
    #[error("backup path missing: {0}")]
    PathMissing(String),
    #[error("path is not allowed: {0}")]
    PathNotAllowed(String),
    #[error("backup failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
}

impl BackupError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::PathMissing(_) => "BACKUP_PATH_MISSING",
            Self::PathNotAllowed(_) => "PATH_NOT_ALLOWED",
            Self::Io(_) | Self::Db(_) => "OPERATION_FAILED",
        }
    }
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<BackupRecord>, sqlx::Error> {
    sqlx::query_as::<_, BackupRow>(
        "SELECT id, slot_id, server_id, map_key, type, reason, status, created_at, completed_at, \
         size_bytes, path, error FROM backup_records ORDER BY created_at DESC LIMIT 100",
    )
    .fetch_all(pool)
    .await
    .map(|rows| rows.into_iter().map(BackupRecord::from).collect())
}

pub async fn run_slot_backup(
    pool: &SqlitePool,
    config: &Config,
    slot_key: &str,
    slot: &ServerSlot,
    reason: &str,
) -> Result<BackupRecord, BackupError> {
    let id = format!("bk-{}-{}", slot.id, epoch_secs());
    let created_at = now_sql();
    let backup_root = backup_root(config);
    validate_clean_abs("backup root", backup_root)?;
    fs::create_dir_all(backup_root)
        .map_err(|e| io_context("create backup root", backup_root, e))?;

    let dest = backup_root.join(&slot.id).join(&id);
    validate_under_root("backup destination", &dest, backup_root)?;

    let sources = configured_sources(slot);
    if sources.is_empty() {
        let mut record = new_record(&id, slot_key, slot, reason, "failed", &created_at, &dest);
        record.error = Some("no save_dir/config_dir configured for slot".into());
        insert_record(pool, &record).await?;
        return Err(BackupError::PathMissing(record.error.clone().unwrap()));
    }

    let ark_root = ark_root(config);
    validate_clean_abs("ark root", ark_root)?;
    let mut copied_any = false;
    let mut size = 0u64;
    fs::create_dir_all(&dest).map_err(|e| io_context("create backup destination", &dest, e))?;

    for (name, src) in sources {
        validate_clean_abs(name, &src)?;
        validate_under_root(name, &src, ark_root)?;
        if !src.exists() {
            continue;
        }
        let canonical_src =
            fs::canonicalize(&src).map_err(|e| io_context("canonicalize source", &src, e))?;
        let canonical_root = fs::canonicalize(ark_root)
            .map_err(|e| io_context("canonicalize ARK root", ark_root, e))?;
        if !canonical_src.starts_with(&canonical_root) {
            return Err(BackupError::PathNotAllowed(format!(
                "{} escapes allowed root",
                src.display()
            )));
        }
        let target = dest.join(name);
        size = size.saturating_add(copy_tree(&canonical_src, &target)?);
        copied_any = true;
    }

    let mut record = new_record(
        &id,
        slot_key,
        slot,
        reason,
        if copied_any { "success" } else { "failed" },
        &created_at,
        &dest,
    );
    record.size_bytes = size;
    record.completed_at = Some(now_sql());
    if !copied_any {
        record.error = Some("configured backup source paths do not exist".into());
    }
    insert_record(pool, &record).await?;

    if copied_any {
        Ok(record)
    } else {
        Err(BackupError::PathMissing(
            "configured backup source paths do not exist".into(),
        ))
    }
}

fn configured_sources(slot: &ServerSlot) -> Vec<(&'static str, PathBuf)> {
    let mut out = Vec::new();
    if let Some(path) = slot.save_dir() {
        out.push(("save", PathBuf::from(path)));
    }
    if let Some(path) = slot.config_dir() {
        out.push(("config", PathBuf::from(path)));
    }
    out
}

fn new_record(
    id: &str,
    slot_key: &str,
    slot: &ServerSlot,
    reason: &str,
    status: &str,
    created_at: &str,
    dest: &Path,
) -> BackupRecord {
    BackupRecord {
        id: id.into(),
        slot_id: slot.id.clone(),
        server_id: slot_key.replace('_', "-"),
        map_key: slot.map_key.clone(),
        kind: "combined".into(),
        reason: reason.into(),
        status: status.into(),
        created_at: created_at.into(),
        completed_at: None,
        size_bytes: 0,
        path: dest.display().to_string(),
        error: None,
    }
}

async fn insert_record(pool: &SqlitePool, r: &BackupRecord) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO backup_records (id, slot_id, server_id, map_key, type, reason, status, \
         created_at, completed_at, size_bytes, path, error) VALUES (?1, ?2, ?3, ?4, ?5, ?6, \
         ?7, ?8, ?9, ?10, ?11, ?12)",
    )
    .bind(&r.id)
    .bind(&r.slot_id)
    .bind(&r.server_id)
    .bind(&r.map_key)
    .bind(&r.kind)
    .bind(&r.reason)
    .bind(&r.status)
    .bind(&r.created_at)
    .bind(&r.completed_at)
    .bind(r.size_bytes as i64)
    .bind(&r.path)
    .bind(&r.error)
    .execute(pool)
    .await?;
    Ok(())
}

fn backup_root(config: &Config) -> &Path {
    if config.paths.backup_root.trim().is_empty() {
        Path::new(&config.backup_policy.directory)
    } else {
        Path::new(&config.paths.backup_root)
    }
}

fn ark_root(config: &Config) -> &Path {
    if config.paths.ark_root.trim().is_empty() {
        Path::new(&config.cluster.directory)
    } else {
        Path::new(&config.paths.ark_root)
    }
}

pub fn validate_clean_abs(label: &str, path: &Path) -> Result<(), BackupError> {
    if path.as_os_str().is_empty() || !path.is_absolute() {
        return Err(BackupError::PathNotAllowed(format!(
            "{label} must be an absolute path"
        )));
    }
    if path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(BackupError::PathNotAllowed(format!(
            "{label} contains a parent-directory segment"
        )));
    }
    Ok(())
}

pub fn validate_under_root(label: &str, path: &Path, root: &Path) -> Result<(), BackupError> {
    validate_clean_abs(label, path)?;
    validate_clean_abs("root", root)?;
    let p = normalize_lexical(path);
    let r = normalize_lexical(root);
    if p == r || p.starts_with(&r) {
        Ok(())
    } else {
        Err(BackupError::PathNotAllowed(format!(
            "{} '{}' must stay under '{}'",
            label,
            path.display(),
            root.display()
        )))
    }
}

fn normalize_lexical(path: &Path) -> PathBuf {
    path.components().collect()
}

fn io_context(action: &str, path: &Path, err: io::Error) -> BackupError {
    BackupError::Io(io::Error::new(
        err.kind(),
        format!("{action} '{}': {err}", path.display()),
    ))
}

fn copy_tree(src: &Path, dest: &Path) -> Result<u64, BackupError> {
    let meta = fs::symlink_metadata(src).map_err(|e| io_context("inspect source", src, e))?;
    if meta.file_type().is_symlink() {
        return Err(BackupError::PathNotAllowed(format!(
            "symlink source rejected: {}",
            src.display()
        )));
    }
    if meta.is_file() {
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| io_context("create backup parent", parent, e))?;
        }
        fs::copy(src, dest).map_err(|e| {
            BackupError::Io(io::Error::new(
                e.kind(),
                format!("copy '{}' to '{}': {e}", src.display(), dest.display()),
            ))
        })?;
        return Ok(meta.len());
    }
    fs::create_dir_all(dest).map_err(|e| io_context("create backup directory", dest, e))?;
    let mut size = 0u64;
    for entry in fs::read_dir(src).map_err(|e| io_context("read source directory", src, e))? {
        let entry = entry.map_err(|e| io_context("read source directory entry", src, e))?;
        let child_src = entry.path();
        let child_dest = dest.join(entry.file_name());
        size = size.saturating_add(copy_tree(&child_src, &child_dest)?);
    }
    Ok(size)
}

fn now_sql() -> String {
    format!("{}", epoch_secs())
}

fn epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[derive(sqlx::FromRow)]
struct BackupRow {
    id: String,
    slot_id: String,
    server_id: String,
    map_key: String,
    #[sqlx(rename = "type")]
    kind: String,
    reason: String,
    status: String,
    created_at: String,
    completed_at: Option<String>,
    size_bytes: i64,
    path: String,
    error: Option<String>,
}

impl From<BackupRow> for BackupRecord {
    fn from(row: BackupRow) -> Self {
        Self {
            id: row.id,
            slot_id: row.slot_id,
            server_id: row.server_id,
            map_key: row.map_key,
            kind: row.kind,
            reason: row.reason,
            status: row.status,
            created_at: row.created_at,
            completed_at: row.completed_at,
            size_bytes: row.size_bytes.max(0) as u64,
            path: row.path,
            error: row.error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_traversal() {
        assert!(validate_clean_abs("bad", Path::new("/srv/ark/../etc")).is_err());
    }

    #[test]
    fn rejects_outside_root() {
        assert!(validate_under_root("src", Path::new("/etc/ark"), Path::new("/srv/ark")).is_err());
    }

    #[tokio::test]
    async fn missing_source_records_failed_backup() {
        let pool = crate::db::init(":memory:").await.unwrap();
        let mut cfg = crate::config::tests_support::base_config();
        let root = std::env::temp_dir().join(format!("ark-manager-test-{}", epoch_secs()));
        cfg.paths.ark_root = root.display().to_string();
        cfg.paths.backup_root = root.join("backups").display().to_string();
        cfg.backup_policy.directory = cfg.paths.backup_root.clone();
        cfg.slots.as_mut().unwrap().home.save_path =
            Some(root.join("missing-save").display().to_string());
        cfg.slots.as_mut().unwrap().home.config_path = None;
        let slot = &cfg.slots.as_ref().unwrap().home;
        let err = run_slot_backup(&pool, &cfg, "home", slot, "manual")
            .await
            .unwrap_err();
        assert!(matches!(err, BackupError::PathMissing(_)));
        assert_eq!(list(&pool).await.unwrap().len(), 1);
    }
}
