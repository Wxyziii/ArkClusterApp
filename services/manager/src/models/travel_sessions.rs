//! Travel session lifecycle for external node travel maps.

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use super::nodes::gen_id;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct TravelSession {
    pub id: String,
    pub node_id: String,
    pub map_id: String,
    pub map_name: String,
    pub session_name: String,
    pub status: String,
    pub requester_discord_id: String,
    pub game_port: Option<i64>,
    pub raw_port: Option<i64>,
    pub query_port: Option<i64>,
    pub rcon_port: Option<i64>,
    pub started_at: String,
    pub ready_at: Option<String>,
    pub closed_at: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TravelCloseRequest {
    #[serde(rename = "sessionId")]
    pub session_id: Option<String>,
    pub force: Option<bool>,
}

pub async fn create(
    pool: &SqlitePool,
    node_id: &str,
    map_id: &str,
    map_name: &str,
    requester_discord_id: &str,
    game_port: Option<i64>,
    raw_port: Option<i64>,
    query_port: Option<i64>,
    rcon_port: Option<i64>,
) -> Result<TravelSession, sqlx::Error> {
    let id = gen_id();
    let session_name = format!("ARK Travel - {}", map_name);
    sqlx::query(
        "INSERT INTO travel_sessions
            (id, node_id, map_id, map_name, session_name, status, requester_discord_id,
             game_port, raw_port, query_port, rcon_port)
         VALUES (?1, ?2, ?3, ?4, ?5, 'starting', ?6, ?7, ?8, ?9, ?10)",
    )
    .bind(&id)
    .bind(node_id)
    .bind(map_id)
    .bind(map_name)
    .bind(&session_name)
    .bind(requester_discord_id)
    .bind(game_port)
    .bind(raw_port)
    .bind(query_port)
    .bind(rcon_port)
    .execute(pool)
    .await?;
    get(pool, &id).await.ok_or(sqlx::Error::RowNotFound)
}

pub async fn get(pool: &SqlitePool, id: &str) -> Option<TravelSession> {
    sqlx::query_as::<_, TravelSession>("SELECT * FROM travel_sessions WHERE id = ?1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .unwrap_or(None)
}

pub async fn active_for_node(pool: &SqlitePool, node_id: &str) -> Option<TravelSession> {
    sqlx::query_as::<_, TravelSession>(
        "SELECT * FROM travel_sessions
         WHERE node_id = ?1 AND status NOT IN ('closed','error')
         ORDER BY started_at DESC LIMIT 1",
    )
    .bind(node_id)
    .fetch_optional(pool)
    .await
    .unwrap_or(None)
}

pub async fn list_active(pool: &SqlitePool) -> Vec<TravelSession> {
    sqlx::query_as::<_, TravelSession>(
        "SELECT * FROM travel_sessions WHERE status NOT IN ('closed','error') ORDER BY started_at DESC",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

pub async fn list_all(pool: &SqlitePool, limit: i64) -> Vec<TravelSession> {
    sqlx::query_as::<_, TravelSession>(
        "SELECT * FROM travel_sessions ORDER BY started_at DESC LIMIT ?1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

pub async fn set_status(pool: &SqlitePool, id: &str, status: &str) -> Result<(), sqlx::Error> {
    let now_col = if status == "closed" || status == "error" { "closed_at" } else if status == "ready" { "ready_at" } else { "started_at" };
    let sql = if status == "closed" || status == "error" {
        "UPDATE travel_sessions SET status = ?2, closed_at = datetime('now') WHERE id = ?1".to_string()
    } else if status == "ready" {
        "UPDATE travel_sessions SET status = ?2, ready_at = datetime('now') WHERE id = ?1".to_string()
    } else {
        let _ = now_col;
        "UPDATE travel_sessions SET status = ?2 WHERE id = ?1".to_string()
    };
    sqlx::query(&sql)
        .bind(id)
        .bind(status)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_error(pool: &SqlitePool, id: &str, error: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE travel_sessions SET status = 'error', last_error = ?2, closed_at = datetime('now') WHERE id = ?1",
    )
    .bind(id)
    .bind(error)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn close_session(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    set_status(pool, id, "closed").await
}
