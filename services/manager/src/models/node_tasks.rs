//! Node task queue — manager enqueues, node polls and reports results.

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use super::nodes::gen_id;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct NodeTask {
    pub id: String,
    pub node_id: String,
    pub session_id: Option<String>,
    pub task_type: String,
    pub payload: String,
    pub status: String,
    pub created_at: String,
    pub sent_at: Option<String>,
    pub completed_at: Option<String>,
    pub result: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    StartTravel,
    StopTravel,
    SaveWorld,
    BackupTravel,
    ValidateMods,
    SyncConfig,
    UpdateArkServer,
    Ping,
    StatusRefresh,
}

impl TaskType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskType::StartTravel => "start_travel",
            TaskType::StopTravel => "stop_travel",
            TaskType::SaveWorld => "save_world",
            TaskType::BackupTravel => "backup_travel",
            TaskType::ValidateMods => "validate_mods",
            TaskType::SyncConfig => "sync_config",
            TaskType::UpdateArkServer => "update_ark_server",
            TaskType::Ping => "ping",
            TaskType::StatusRefresh => "status_refresh",
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct TaskResultRequest {
    pub success: bool,
    pub result: Option<String>,
    pub error: Option<String>,
}

pub async fn enqueue(
    pool: &SqlitePool,
    node_id: &str,
    session_id: Option<&str>,
    task_type: TaskType,
    payload: serde_json::Value,
) -> Result<String, sqlx::Error> {
    let id = gen_id();
    sqlx::query(
        "INSERT INTO node_tasks (id, node_id, session_id, task_type, payload, status)
         VALUES (?1, ?2, ?3, ?4, ?5, 'pending')",
    )
    .bind(&id)
    .bind(node_id)
    .bind(session_id)
    .bind(task_type.as_str())
    .bind(payload.to_string())
    .execute(pool)
    .await?;
    Ok(id)
}

pub async fn poll_pending(pool: &SqlitePool, node_id: &str, limit: i64) -> Vec<NodeTask> {
    let tasks = sqlx::query_as::<_, NodeTask>(
        "SELECT * FROM node_tasks
         WHERE node_id = ?1 AND status = 'pending'
         ORDER BY created_at
         LIMIT ?2",
    )
    .bind(node_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    if !tasks.is_empty() {
        let ids: Vec<String> = tasks.iter().map(|t| format!("'{}'", t.id)).collect();
        let placeholders = ids.join(",");
        let _ = sqlx::query(&format!(
            "UPDATE node_tasks SET status = 'sent', sent_at = datetime('now') WHERE id IN ({})",
            placeholders
        ))
        .execute(pool)
        .await;
    }

    tasks
}

pub async fn complete_task(
    pool: &SqlitePool,
    task_id: &str,
    node_id: &str,
    req: &TaskResultRequest,
) -> Result<(), sqlx::Error> {
    let status = if req.success { "completed" } else { "failed" };
    sqlx::query(
        "UPDATE node_tasks SET
            status = ?3,
            completed_at = datetime('now'),
            result = ?4,
            error = ?5
         WHERE id = ?1 AND node_id = ?2",
    )
    .bind(task_id)
    .bind(node_id)
    .bind(status)
    .bind(req.result.as_deref())
    .bind(req.error.as_deref())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_by_id(pool: &SqlitePool, task_id: &str) -> Option<NodeTask> {
    sqlx::query_as::<_, NodeTask>("SELECT * FROM node_tasks WHERE id = ?1")
        .bind(task_id)
        .fetch_optional(pool)
        .await
        .unwrap_or(None)
}

pub async fn list_for_session(pool: &SqlitePool, session_id: &str) -> Vec<NodeTask> {
    sqlx::query_as::<_, NodeTask>(
        "SELECT * FROM node_tasks WHERE session_id = ?1 ORDER BY created_at DESC LIMIT 20",
    )
    .bind(session_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}
