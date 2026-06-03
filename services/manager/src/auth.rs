//! Bearer-token auth middleware for `/api/*`.
//!
//! `/health` is mounted outside this layer and needs no token. Every `/api/*`
//! request must send `Authorization: Bearer <token>` matching the configured
//! API token. The token is NEVER logged — failures log only the reason and the
//! peer-agnostic fact that auth failed.

use axum::body::Body;
use axum::extract::State;
use axum::http::{header::AUTHORIZATION, Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

use crate::state::AppState;

fn unauthorized(reason: &str) -> Response {
    // Reason is a static category, never the token or header contents.
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

    let header = match req.headers().get(AUTHORIZATION) {
        Some(h) => h,
        None => return unauthorized("missing Authorization header"),
    };

    let value = match header.to_str() {
        Ok(v) => v,
        Err(_) => return unauthorized("malformed Authorization header"),
    };

    let token = match value
        .strip_prefix("Bearer ")
        .or_else(|| value.strip_prefix("bearer "))
    {
        Some(t) => t.trim(),
        None => return unauthorized("expected 'Bearer <token>' scheme"),
    };

    if constant_time_eq(token.as_bytes(), expected.as_bytes()) {
        next.run(req).await
    } else {
        unauthorized("invalid token")
    }
}

/// Length-independent constant-time comparison to avoid timing leaks.
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
