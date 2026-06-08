//! External node registry — pairing, tokens, heartbeat, status.

use hex;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;

// ── token helpers ─────────────────────────────────────────────────────────────

pub fn generate_token() -> String {
    let bytes: [u8; 32] = rand::thread_rng().gen();
    hex::encode(bytes)
}

pub fn generate_pairing_code() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::thread_rng();
    let mut part = |n: usize| -> String {
        (0..n)
            .map(|_| char::from(CHARSET[rng.gen_range(0..CHARSET.len())]))
            .collect()
    };
    format!("{}-{}", part(4), part(4))
}

pub fn hash_token(token: &str) -> String {
    let mut h = Sha256::new();
    h.update(token.as_bytes());
    hex::encode(h.finalize())
}

pub fn gen_id() -> String {
    let bytes: [u8; 8] = rand::thread_rng().gen();
    hex::encode(bytes)
}

// ── models ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Node {
    pub id: String,
    pub display_name: String,
    pub owner_discord_id: String,
    pub node_type: String,
    pub status: String,
    pub max_travel_servers: i64,
    pub tailscale_ip: String,
    pub version: String,
    pub active_travel_servers: i64,
    pub current_map: Option<String>,
    pub available_ram_mb: Option<i64>,
    pub total_ram_mb: Option<i64>,
    pub cluster_share_mounted: i64,
    pub ark_server_installed: i64,
    pub mods_valid: i64,
    pub config_valid: i64,
    pub ports_free: i64,
    pub rcon_ready: i64,
    pub last_heartbeat: Option<String>,
    pub last_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct NodeHeartbeat {
    #[serde(rename = "nodeId")]
    pub node_id: String,
    #[serde(rename = "nodeName")]
    pub node_name: Option<String>,
    #[serde(rename = "ownerDiscordUserId")]
    pub owner_discord_user_id: Option<String>,
    pub version: Option<String>,
    #[serde(rename = "tailscaleOnline")]
    pub tailscale_online: Option<bool>,
    #[serde(rename = "tailscaleIp")]
    pub tailscale_ip: Option<String>,
    #[serde(rename = "activeTravelServers")]
    pub active_travel_servers: Option<i64>,
    #[serde(rename = "currentMap")]
    pub current_map: Option<String>,
    #[serde(rename = "availableRamMb")]
    pub available_ram_mb: Option<i64>,
    #[serde(rename = "totalRamMb")]
    pub total_ram_mb: Option<i64>,
    #[serde(rename = "clusterShareMounted")]
    pub cluster_share_mounted: Option<bool>,
    #[serde(rename = "arkServerInstalled")]
    pub ark_server_installed: Option<bool>,
    #[serde(rename = "modsValid")]
    pub mods_valid: Option<bool>,
    #[serde(rename = "configValid")]
    pub config_valid: Option<bool>,
    #[serde(rename = "portsFree")]
    pub ports_free: Option<bool>,
    #[serde(rename = "rconReady")]
    pub rcon_ready: Option<bool>,
    #[serde(rename = "lastError")]
    pub last_error: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PairingInvite {
    pub code: String,
    pub suggested_name: String,
    pub created_by: String,
    pub expires_at: String,
    pub used: i64,
    pub node_id: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct PairCompleteRequest {
    pub code: String,
    #[serde(rename = "nodeId")]
    pub node_id: String,
    #[serde(rename = "nodeName")]
    pub node_name: String,
    #[serde(rename = "ownerDiscordUserId")]
    pub owner_discord_user_id: Option<String>,
    #[serde(rename = "nodeType")]
    pub node_type: Option<String>,
}

// ── DB ops ────────────────────────────────────────────────────────────────────

pub async fn list(pool: &SqlitePool) -> Vec<Node> {
    sqlx::query_as::<_, Node>("SELECT * FROM nodes ORDER BY created_at")
        .fetch_all(pool)
        .await
        .unwrap_or_default()
}

pub async fn get(pool: &SqlitePool, id: &str) -> Option<Node> {
    sqlx::query_as::<_, Node>("SELECT * FROM nodes WHERE id = ?1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .unwrap_or(None)
}

pub async fn get_by_owner(pool: &SqlitePool, discord_id: &str) -> Option<Node> {
    sqlx::query_as::<_, Node>("SELECT * FROM nodes WHERE owner_discord_id = ?1 LIMIT 1")
        .bind(discord_id)
        .fetch_optional(pool)
        .await
        .unwrap_or(None)
}

pub async fn create_from_pairing(pool: &SqlitePool, req: &PairCompleteRequest) -> Result<String, sqlx::Error> {
    let node_type = req.node_type.as_deref().unwrap_or("external-windows");
    let owner = req.owner_discord_user_id.as_deref().unwrap_or("");
    sqlx::query(
        "INSERT INTO nodes (id, display_name, owner_discord_id, node_type, status)
         VALUES (?1, ?2, ?3, ?4, 'offline')
         ON CONFLICT(id) DO UPDATE SET
           display_name = excluded.display_name,
           owner_discord_id = CASE WHEN excluded.owner_discord_id != '' THEN excluded.owner_discord_id ELSE owner_discord_id END,
           updated_at = datetime('now')",
    )
    .bind(&req.node_id)
    .bind(&req.node_name)
    .bind(owner)
    .bind(node_type)
    .execute(pool)
    .await?;
    Ok(req.node_id.clone())
}

pub async fn apply_heartbeat(pool: &SqlitePool, hb: &NodeHeartbeat) -> Result<(), sqlx::Error> {
    let status = compute_status(hb);
    sqlx::query(
        "UPDATE nodes SET
            display_name           = COALESCE(?2, display_name),
            owner_discord_id       = CASE WHEN ?3 != '' THEN ?3 ELSE owner_discord_id END,
            status                 = ?4,
            tailscale_ip           = COALESCE(?5, tailscale_ip),
            version                = COALESCE(?6, version),
            active_travel_servers  = COALESCE(?7, active_travel_servers),
            current_map            = ?8,
            available_ram_mb       = ?9,
            total_ram_mb           = ?10,
            cluster_share_mounted  = COALESCE(?11, cluster_share_mounted),
            ark_server_installed   = COALESCE(?12, ark_server_installed),
            mods_valid             = COALESCE(?13, mods_valid),
            config_valid           = COALESCE(?14, config_valid),
            ports_free             = COALESCE(?15, ports_free),
            rcon_ready             = COALESCE(?16, rcon_ready),
            last_heartbeat         = datetime('now'),
            last_error             = ?17,
            updated_at             = datetime('now')
         WHERE id = ?1",
    )
    .bind(&hb.node_id)
    .bind(hb.node_name.as_deref())
    .bind(hb.owner_discord_user_id.as_deref().unwrap_or(""))
    .bind(&status)
    .bind(hb.tailscale_ip.as_deref())
    .bind(hb.version.as_deref())
    .bind(hb.active_travel_servers)
    .bind(hb.current_map.as_deref())
    .bind(hb.available_ram_mb)
    .bind(hb.total_ram_mb)
    .bind(hb.cluster_share_mounted.map(|b| b as i64))
    .bind(hb.ark_server_installed.map(|b| b as i64))
    .bind(hb.mods_valid.map(|b| b as i64))
    .bind(hb.config_valid.map(|b| b as i64))
    .bind(hb.ports_free.map(|b| b as i64))
    .bind(hb.rcon_ready.map(|b| b as i64))
    .bind(hb.last_error.as_deref())
    .execute(pool)
    .await?;
    Ok(())
}

fn compute_status(hb: &NodeHeartbeat) -> &'static str {
    if hb.active_travel_servers.unwrap_or(0) > 0 {
        return "busy";
    }
    let ready = hb.cluster_share_mounted.unwrap_or(false)
        && hb.ark_server_installed.unwrap_or(false)
        && hb.mods_valid.unwrap_or(false)
        && hb.config_valid.unwrap_or(false)
        && hb.ports_free.unwrap_or(false)
        && hb.tailscale_online.unwrap_or(false);
    if ready { "online" } else { "not_ready" }
}

pub async fn mark_offline_stale(pool: &SqlitePool, timeout_secs: i64) -> Result<u64, sqlx::Error> {
    let res = sqlx::query(
        "UPDATE nodes SET status = 'offline', updated_at = datetime('now')
         WHERE status != 'offline'
           AND (last_heartbeat IS NULL
                OR (strftime('%s','now') - strftime('%s', last_heartbeat)) > ?1)",
    )
    .bind(timeout_secs)
    .execute(pool)
    .await?;
    Ok(res.rows_affected())
}

// ── token ops ─────────────────────────────────────────────────────────────────

pub async fn store_token(pool: &SqlitePool, node_id: &str, token: &str) -> Result<(), sqlx::Error> {
    let hash = hash_token(token);
    sqlx::query(
        "INSERT INTO node_tokens (node_id, token_hash, revoked)
         VALUES (?1, ?2, 0)
         ON CONFLICT(node_id) DO UPDATE SET token_hash = excluded.token_hash, revoked = 0, created_at = datetime('now')",
    )
    .bind(node_id)
    .bind(&hash)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn validate_token(pool: &SqlitePool, token: &str) -> Option<String> {
    let hash = hash_token(token);
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT node_id FROM node_tokens WHERE token_hash = ?1 AND revoked = 0",
    )
    .bind(&hash)
    .fetch_optional(pool)
    .await
    .unwrap_or(None);
    row.map(|(id,)| id)
}

pub async fn revoke_token(pool: &SqlitePool, node_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE node_tokens SET revoked = 1 WHERE node_id = ?1")
        .bind(node_id)
        .execute(pool)
        .await?;
    sqlx::query("UPDATE nodes SET status = 'offline', updated_at = datetime('now') WHERE id = ?1")
        .bind(node_id)
        .execute(pool)
        .await?;
    Ok(())
}

// ── pairing ops ───────────────────────────────────────────────────────────────

pub async fn create_pairing_invite(
    pool: &SqlitePool,
    suggested_name: &str,
    created_by: &str,
    ttl_mins: i64,
) -> Result<PairingInvite, sqlx::Error> {
    let code = generate_pairing_code();
    sqlx::query(
        "INSERT INTO node_pairing_invites (code, suggested_name, created_by, expires_at)
         VALUES (?1, ?2, ?3, datetime('now', ?4))",
    )
    .bind(&code)
    .bind(suggested_name)
    .bind(created_by)
    .bind(format!("+{} minutes", ttl_mins))
    .execute(pool)
    .await?;
    get_pairing_invite(pool, &code).await.ok_or(sqlx::Error::RowNotFound)
}

pub async fn get_pairing_invite(pool: &SqlitePool, code: &str) -> Option<PairingInvite> {
    sqlx::query_as::<_, PairingInvite>("SELECT * FROM node_pairing_invites WHERE code = ?1")
        .bind(code)
        .fetch_optional(pool)
        .await
        .unwrap_or(None)
}

pub async fn consume_pairing_invite(
    pool: &SqlitePool,
    code: &str,
    node_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE node_pairing_invites SET used = 1, node_id = ?2 WHERE code = ?1",
    )
    .bind(code)
    .bind(node_id)
    .execute(pool)
    .await?;
    Ok(())
}
