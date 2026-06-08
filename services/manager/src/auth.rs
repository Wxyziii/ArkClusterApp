//! Auth middleware for `/api/*`.
//!
//! - `require_token`: admin Bearer token, matches config.
//! - `require_node_token`: node-specific Bearer token, validated against DB.
//!   Inserts `NodeClaims` extension with the authenticated node_id.
//!
//! Tokens are NEVER logged — failures log only the category of rejection.

use axum::body::Body;
use axum::extract::State;
use axum::http::{header::AUTHORIZATION, Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

use crate::models::nodes;
use crate::state::AppState;

/// Extension injected by `require_node_token`.
#[derive(Clone)]
pub struct NodeClaims {
    pub node_id: String,
}

fn unauthorized(reason: &str) -> Response {
    tracing::warn!("api auth rejected: {reason}");
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({ "error": "unauthorized", "message": reason })),
    )
        .into_response()
}

pub async fn require_token(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let expected = state.config.server.api_token.as_str();
    let Some(token) = extract_bearer(req.headers()) else {
        return unauthorized("missing or malformed Authorization header");
    };
    if constant_time_eq(token.as_bytes(), expected.as_bytes()) {
        next.run(req).await
    } else {
        unauthorized("invalid token")
    }
}

pub async fn require_node_token(
    State(state): State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    let Some(token) = extract_bearer(req.headers()) else {
        return unauthorized("missing or malformed Authorization header");
    };
    let Some(node_id) = nodes::validate_token(&state.pool, token).await else {
        return unauthorized("invalid or revoked node token");
    };
    req.extensions_mut().insert(NodeClaims { node_id });
    next.run(req).await
}

fn extract_bearer(headers: &axum::http::HeaderMap) -> Option<&str> {
    let header = headers.get(AUTHORIZATION)?.to_str().ok()?;
    header
        .strip_prefix("Bearer ")
        .or_else(|| header.strip_prefix("bearer "))
        .map(|t| t.trim())
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::constant_time_eq;

    #[test]
    fn eq_matches() {
        assert!(constant_time_eq(b"secret", b"secret"));
    }

    #[test]
    fn eq_rejects_diff_and_len() {
        assert!(!constant_time_eq(b"secret", b"secreT"));
        assert!(!constant_time_eq(b"secret", b"secret-longer"));
    }
}
