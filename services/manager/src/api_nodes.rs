//! Node/pairing/task/travel-session API routes.
//! Three router groups: admin (admin token), node (node token), open (pair complete).

use hex;
use sha2::{Digest, Sha256};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use serde::Deserialize;
use serde_json::json;

use crate::auth::NodeClaims;
use crate::models::audit::{self, AuditEvent, Severity};
use crate::models::node_tasks::{self, TaskResultRequest, TaskType};
use crate::models::nodes::{self, generate_token, NodeHeartbeat, PairCompleteRequest};
use crate::models::travel::{OfficialMapProfile, OFFICIAL_MAPS};
use crate::models::travel_sessions;
use crate::state::AppState;

// ── routers ───────────────────────────────────────────────────────────────────

pub fn admin_node_router() -> Router<AppState> {
    Router::new()
        .route("/nodes", get(list_nodes))
        .route("/nodes/{id}", get(node_detail))
        .route("/nodes/{id}/revoke", post(revoke_node))
        .route("/nodes/pair/start", post(pair_start))
        .route("/travel/status", get(travel_status))
        .route("/travel/close", post(travel_close))
        .route("/travel/force-close", post(travel_force_close))
        .route("/config/version-hash", get(config_version_hash))
}

pub fn node_router() -> Router<AppState> {
    Router::new()
        .route("/nodes/heartbeat", post(node_heartbeat))
        .route("/nodes/{id}/tasks/poll", post(task_poll))
        .route("/nodes/{id}/tasks/{task_id}/result", post(task_result))
}

pub fn open_node_router() -> Router<AppState> {
    Router::new()
        .route("/nodes/pair/complete", post(pair_complete))
}

// ── admin: node management ────────────────────────────────────────────────────

async fn list_nodes(State(s): State<AppState>) -> impl IntoResponse {
    let nodes = nodes::list(&s.pool).await;
    Json(json!({ "nodes": nodes, "count": nodes.len() }))
}

async fn node_detail(State(s): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    match nodes::get(&s.pool, &id).await {
        Some(n) => {
            let active_session = travel_sessions::active_for_node(&s.pool, &id).await;
            Json(json!({ "node": n, "activeSession": active_session })).into_response()
        }
        None => not_found(&format!("node '{id}' not found")),
    }
}

#[derive(Deserialize)]
struct RevokeReq {
    confirm: Option<bool>,
}

async fn revoke_node(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<RevokeReq>,
) -> impl IntoResponse {
    if req.confirm != Some(true) {
        return api_err(StatusCode::BAD_REQUEST, "CONFIRM_REQUIRED", "set confirm:true to revoke");
    }
    if nodes::get(&s.pool, &id).await.is_none() {
        return not_found(&format!("node '{id}' not found"));
    }
    if let Err(e) = nodes::revoke_token(&s.pool, &id).await {
        return api_err(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", &e.to_string());
    }
    audit::record(
        &s.pool,
        &AuditEvent::new(Severity::Warn, "NodeAuth", "node token revoked")
            .detail(format!("node_id={id}")),
    )
    .await;
    Json(json!({ "accepted": true, "nodeId": id, "message": "Node token revoked. Node will go offline." })).into_response()
}

#[derive(Deserialize)]
struct PairStartReq {
    name: String,
    #[serde(rename = "createdBy")]
    created_by: Option<String>,
    #[serde(rename = "ttlMins")]
    ttl_mins: Option<i64>,
}

async fn pair_start(State(s): State<AppState>, Json(req): Json<PairStartReq>) -> impl IntoResponse {
    let ttl = req.ttl_mins.unwrap_or(15).clamp(5, 60);
    let by = req.created_by.as_deref().unwrap_or("admin");
    match nodes::create_pairing_invite(&s.pool, &req.name, by, ttl).await {
        Ok(invite) => {
            audit::record(
                &s.pool,
                &AuditEvent::new(Severity::Info, "NodePairing", "pairing invite created")
                    .detail(format!("name={} ttl_mins={} by={}", req.name, ttl, by)),
            )
            .await;
            Json(json!({
                "code": invite.code,
                "suggestedName": invite.suggested_name,
                "expiresAt": invite.expires_at,
                "ttlMins": ttl,
                "message": format!("Pairing code: {} (expires in {} min)", invite.code, ttl)
            }))
            .into_response()
        }
        Err(e) => api_err(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", &e.to_string()),
    }
}

// ── open: pair complete ───────────────────────────────────────────────────────

async fn pair_complete(State(s): State<AppState>, Json(req): Json<PairCompleteRequest>) -> impl IntoResponse {
    let Some(invite) = nodes::get_pairing_invite(&s.pool, &req.code).await else {
        return api_err(StatusCode::BAD_REQUEST, "INVALID_CODE", "pairing code not found");
    };
    if invite.used != 0 {
        return api_err(StatusCode::BAD_REQUEST, "CODE_USED", "pairing code already used");
    }
    // expiry check via DB comparison
    let expired: Option<(i64,)> = sqlx::query_as(
        "SELECT 1 FROM node_pairing_invites WHERE code = ?1 AND expires_at > datetime('now')",
    )
    .bind(&req.code)
    .fetch_optional(&s.pool)
    .await
    .unwrap_or(None);
    if expired.is_none() {
        return api_err(StatusCode::BAD_REQUEST, "CODE_EXPIRED", "pairing code has expired");
    }

    if let Err(e) = nodes::create_from_pairing(&s.pool, &req).await {
        return api_err(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", &e.to_string());
    }
    let token = generate_token();
    if let Err(e) = nodes::store_token(&s.pool, &req.node_id, &token).await {
        return api_err(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", &e.to_string());
    }
    let _ = nodes::consume_pairing_invite(&s.pool, &req.code, &req.node_id).await;

    audit::record(
        &s.pool,
        &AuditEvent::new(Severity::Success, "NodePairing", "node paired successfully")
            .detail(format!("node_id={} name={}", req.node_id, req.node_name)),
    )
    .await;

    // Token returned ONCE here. Not logged. Node must store it.
    Json(json!({
        "accepted": true,
        "nodeId": req.node_id,
        "nodeName": req.node_name,
        "nodeToken": token,
        "message": "Node paired. Store nodeToken securely — it will not be shown again."
    }))
    .into_response()
}

// ── node: heartbeat ───────────────────────────────────────────────────────────

async fn node_heartbeat(
    State(s): State<AppState>,
    Extension(claims): Extension<NodeClaims>,
    Json(hb): Json<NodeHeartbeat>,
) -> impl IntoResponse {
    if hb.node_id != claims.node_id {
        return api_err(StatusCode::FORBIDDEN, "NODE_ID_MISMATCH", "token node_id does not match payload nodeId");
    }
    if let Err(e) = nodes::apply_heartbeat(&s.pool, &hb).await {
        tracing::warn!("heartbeat DB error for {}: {}", claims.node_id, e);
        return api_err(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", &e.to_string());
    }
    // If node reports RCON ready, promote any "starting" session to "ready"
    if hb.rcon_ready == Some(true) {
        if let Some(session) = travel_sessions::active_for_node(&s.pool, &claims.node_id).await {
            if session.status == "starting" {
                let _ = travel_sessions::set_status(&s.pool, &session.id, "ready").await;
                tracing::info!("session {} marked ready via heartbeat rcon_ready", session.id);
            }
        }
    }
    Json(json!({ "accepted": true, "nodeId": claims.node_id })).into_response()
}

// ── node: task queue ──────────────────────────────────────────────────────────

async fn task_poll(
    State(s): State<AppState>,
    Extension(claims): Extension<NodeClaims>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if id != claims.node_id {
        return api_err(StatusCode::FORBIDDEN, "NODE_ID_MISMATCH", "token does not match node id");
    }
    let tasks = node_tasks::poll_pending(&s.pool, &id, 5).await;
    Json(json!({ "nodeId": id, "tasks": tasks, "count": tasks.len() })).into_response()
}

async fn task_result(
    State(s): State<AppState>,
    Extension(claims): Extension<NodeClaims>,
    Path((id, task_id)): Path<(String, String)>,
    Json(req): Json<TaskResultRequest>,
) -> impl IntoResponse {
    if id != claims.node_id {
        return api_err(StatusCode::FORBIDDEN, "NODE_ID_MISMATCH", "token does not match node id");
    }
    if let Err(e) = node_tasks::complete_task(&s.pool, &task_id, &id, &req).await {
        return api_err(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", &e.to_string());
    }

    // On stop_travel/start_travel task success → update session status
    if req.success {
        if let Some(task) = node_tasks::get_by_id(&s.pool, &task_id).await {
            if let Some(sid) = &task.session_id {
                if task.task_type == "stop_travel" {
                    let _ = travel_sessions::close_session(&s.pool, sid).await;
                } else if task.task_type == "start_travel" {
                    let _ = travel_sessions::set_status(&s.pool, sid, "ready").await;
                }
            }
        }
    }

    Json(json!({ "accepted": true, "taskId": task_id })).into_response()
}

// ── admin: travel session management ─────────────────────────────────────────

async fn travel_status(State(s): State<AppState>) -> impl IntoResponse {
    let sessions = travel_sessions::list_active(&s.pool).await;
    let nodes = nodes::list(&s.pool).await;
    Json(json!({ "sessions": sessions, "nodes": nodes }))
}

#[derive(Deserialize)]
struct TravelCloseReq {
    #[serde(rename = "sessionId")]
    session_id: Option<String>,
    #[serde(rename = "nodeId")]
    node_id: Option<String>,
    force: Option<bool>,
}

async fn travel_close(State(s): State<AppState>, Json(req): Json<TravelCloseReq>) -> impl IntoResponse {
    let session = resolve_session(&s, &req.session_id, &req.node_id).await;
    let Some(session) = session else {
        return api_err(StatusCode::NOT_FOUND, "NOT_FOUND", "no active travel session found");
    };

    let task_id = node_tasks::enqueue(
        &s.pool,
        &session.node_id,
        Some(&session.id),
        TaskType::StopTravel,
        json!({ "sessionId": session.id, "saveFirst": true }),
    )
    .await;

    audit::record(
        &s.pool,
        &AuditEvent::new(Severity::Info, "Travel", "travel close requested")
            .detail(format!("session={} node={}", session.id, session.node_id)),
    )
    .await;

    let _ = travel_sessions::set_status(&s.pool, &session.id, "closing").await;

    match task_id {
        Ok(tid) => Json(json!({
            "accepted": true,
            "sessionId": session.id,
            "nodeId": session.node_id,
            "taskId": tid,
            "message": "Close task queued. Node will save → backup → stop."
        }))
        .into_response(),
        Err(e) => api_err(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", &e.to_string()),
    }
}

async fn travel_force_close(State(s): State<AppState>, Json(req): Json<TravelCloseReq>) -> impl IntoResponse {
    let session = resolve_session(&s, &req.session_id, &req.node_id).await;
    let Some(session) = session else {
        return api_err(StatusCode::NOT_FOUND, "NOT_FOUND", "no active travel session found");
    };

    let task_id = node_tasks::enqueue(
        &s.pool,
        &session.node_id,
        Some(&session.id),
        TaskType::StopTravel,
        json!({ "sessionId": session.id, "saveFirst": false, "force": true }),
    )
    .await;

    let _ = travel_sessions::set_status(&s.pool, &session.id, "closing").await;

    audit::record(
        &s.pool,
        &AuditEvent::new(Severity::Warn, "Travel", "travel force-close requested")
            .detail(format!("session={} node={}", session.id, session.node_id)),
    )
    .await;

    match task_id {
        Ok(tid) => Json(json!({
            "accepted": true,
            "sessionId": session.id,
            "nodeId": session.node_id,
            "taskId": tid,
            "message": "Force-close task queued. Node will stop immediately (no save)."
        }))
        .into_response(),
        Err(e) => api_err(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", &e.to_string()),
    }
}

// ── config version hash (for node config validation) ─────────────────────────

async fn config_version_hash(State(s): State<AppState>) -> impl IntoResponse {
    let canonical = format!("{}:{}", s.config.cluster.id, s.config.cluster.manager_version);
    let hash = {
        let mut h = Sha256::new();
        h.update(canonical.as_bytes());
        hex::encode(h.finalize())
    };
    Json(json!({
        "hash": hash,
        "clusterId": s.config.cluster.id,
        "managerVersion": s.config.cluster.manager_version
    }))
}

// ── travel request: external node routing ────────────────────────────────────

/// Extended travel request that routes to external nodes.
/// Called from the existing `/api/travel/request` handler when the map is
/// not available as a local slot. Exported so `api.rs` can call it.
pub async fn try_external_travel(
    s: &AppState,
    map_input: &str,
    actor: &str,
    actor_discord_id: &str,
) -> Option<axum::response::Response> {
    let profile = resolve_map_profile(map_input)?;

    // Find node assigned to this Discord user, or any free node
    let node = find_available_node(s, actor_discord_id).await?;

    // Check node is ready
    if node.status == "offline" {
        return Some(api_err(
            StatusCode::SERVICE_UNAVAILABLE,
            "NODE_OFFLINE",
            &format!("Node '{}' is offline.", node.display_name),
        ));
    }
    if node.status == "busy" {
        let current = node.current_map.as_deref().unwrap_or("unknown map");
        return Some(api_err(
            StatusCode::CONFLICT,
            "NODE_BUSY",
            &format!(
                "Blocked: {} is already hosting {}. Return to Home and close it before starting another map on this PC.",
                node.display_name, current
            ),
        ));
    }
    if node.status == "not_ready" {
        let reason = build_not_ready_reason(&node);
        return Some(api_err(
            StatusCode::SERVICE_UNAVAILABLE,
            "NODE_NOT_READY",
            &reason,
        ));
    }

    // Create travel session
    let ports = node_ports_for(&node);
    let session = match travel_sessions::create(
        &s.pool,
        &node.id,
        profile.id,
        profile.name,
        actor_discord_id,
        Some(ports.0),
        Some(ports.1),
        Some(ports.2),
        Some(ports.3),
    )
    .await
    {
        Ok(s) => s,
        Err(e) => {
            return Some(api_err(
                StatusCode::INTERNAL_SERVER_ERROR,
                "SESSION_ERROR",
                &e.to_string(),
            ))
        }
    };

    // Enqueue start_travel task
    let task_payload = json!({
        "sessionId": session.id,
        "mapId": profile.id,
        "mapName": profile.name,
        "arkMapName": profile.ark_map_name,
        "sessionName": session.session_name,
        "gamePort": ports.0,
        "rawPort": ports.1,
        "queryPort": ports.2,
        "rconPort": ports.3,
        "clusterSharePath": "",
        "clusterId": s.config.cluster.id
    });

    let task_id = match node_tasks::enqueue(
        &s.pool,
        &node.id,
        Some(&session.id),
        TaskType::StartTravel,
        task_payload,
    )
    .await
    {
        Ok(id) => id,
        Err(e) => {
            let _ = travel_sessions::set_error(&s.pool, &session.id, &e.to_string()).await;
            return Some(api_err(
                StatusCode::INTERNAL_SERVER_ERROR,
                "TASK_QUEUE_ERROR",
                &e.to_string(),
            ));
        }
    };

    audit::record(
        &s.pool,
        &AuditEvent::new(Severity::Info, "Travel", "external travel requested")
            .actor(actor)
            .detail(format!(
                "map={} node={} session={} task={}",
                profile.name, node.id, session.id, task_id
            )),
    )
    .await;

    let node_ip = if node.tailscale_ip.is_empty() { "pending".to_string() } else { node.tailscale_ip.clone() };
    let connect_addr = format!("{}:{}", node_ip, ports.0);
    let query_addr = format!("{}:{}", node_ip, ports.2);
    let relay_ip = &s.config.server.relay_public_ip;
    let pub_connect = if !relay_ip.is_empty() { format!("{}:{}", relay_ip, ports.0) } else { connect_addr.clone() };
    let pub_query = if !relay_ip.is_empty() { format!("{}:{}", relay_ip, ports.2) } else { query_addr.clone() };
    let display_addr = pub_connect.clone();

    Some(
        Json(json!({
            "accepted": true,
            "status": "starting",
            "requestedMap": map_input,
            "resolvedMap": profile.id,
            "resolvedMapName": profile.name,
            "nodeId": node.id,
            "nodeName": node.display_name,
            "sessionId": session.id,
            "taskId": task_id,
            "connectionAddress": connect_addr,
            "queryAddress": query_addr,
            "publicConnectionAddress": pub_connect,
            "publicQueryAddress": pub_query,
            "connectionAvailable": !node_ip.is_empty() && node_ip != "pending",
            "userMessage": format!(
                "{} is starting on {}. Use terminal 'Join Another Server' once it's ready. Connect: {}",
                profile.name, node.display_name, display_addr
            )
        }))
        .into_response(),
    )
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn resolve_map_profile(input: &str) -> Option<&'static OfficialMapProfile> {
    let lower = input.to_lowercase();
    OFFICIAL_MAPS.iter().find(|p| {
        p.aliases.iter().any(|a| a.to_lowercase() == lower)
            || p.id == lower
            || p.ark_map_name.to_lowercase() == lower
    })
}

async fn find_available_node(s: &AppState, discord_id: &str) -> Option<crate::models::nodes::Node> {
    // Prefer node assigned to this Discord user
    if !discord_id.is_empty() {
        if let Some(n) = nodes::get_by_owner(&s.pool, discord_id).await {
            return Some(n);
        }
    }
    // Fall back to any non-busy, non-offline node
    let all = nodes::list(&s.pool).await;
    all.into_iter().find(|n| n.status == "online")
}

fn build_not_ready_reason(node: &crate::models::nodes::Node) -> String {
    let mut reasons = Vec::new();
    if node.cluster_share_mounted == 0 { reasons.push("cluster share not mounted"); }
    if node.ark_server_installed == 0 { reasons.push("ARK dedicated server not installed"); }
    if node.mods_valid == 0 { reasons.push("mods not validated"); }
    if node.config_valid == 0 { reasons.push("config mismatch"); }
    if node.ports_free == 0 { reasons.push("ports not free"); }
    if reasons.is_empty() {
        format!("Node '{}' is not ready.", node.display_name)
    } else {
        format!("Node '{}' is not ready: {}", node.display_name, reasons.join(", "))
    }
}

/// Default port assignments per node. Real ports come from node config/heartbeat.
fn node_ports_for(node: &crate::models::nodes::Node) -> (i64, i64, i64, i64) {
    // Use second-node ports if this looks like a second node
    if node.id.contains("friend") || node.id.contains("2") {
        (7793, 7794, 27019, 27024)
    } else {
        (7789, 7790, 27018, 27023)
    }
}

async fn resolve_session(
    s: &AppState,
    session_id: &Option<String>,
    node_id: &Option<String>,
) -> Option<crate::models::travel_sessions::TravelSession> {
    if let Some(sid) = session_id {
        return travel_sessions::get(&s.pool, sid).await;
    }
    if let Some(nid) = node_id {
        return travel_sessions::active_for_node(&s.pool, nid).await;
    }
    None
}

fn api_err(status: StatusCode, code: &str, message: &str) -> axum::response::Response {
    (status, Json(json!({ "error": { "code": code, "message": message }, "reason": message }))).into_response()
}

fn not_found(message: &str) -> axum::response::Response {
    api_err(StatusCode::NOT_FOUND, "NOT_FOUND", message)
}
