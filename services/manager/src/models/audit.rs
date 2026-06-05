//! Audit event model + writer.
//!
//! Audit events are persisted to the `activity_log` SQLite table. Phase 1 uses
//! this for safe operational events (startup, config load, auth failures). The
//! richer activity feed served at `/api/activity` is still mock data; this
//! writer is the foundation the future feed will read from.
//!
//! IMPORTANT: never pass secrets (API token, Discord token) into an audit
//! event. `record` logs via `tracing` at info level and the message/detail are
//! caller-controlled — keep them secret-free.

use serde::Serialize;
use sqlx::SqlitePool;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warn,
    Error,
    Success,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Info => "info",
            Severity::Warn => "warn",
            Severity::Error => "error",
            Severity::Success => "success",
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuditEvent {
    pub severity: Severity,
    pub source: String,
    pub actor: String,
    pub target_map: String,
    pub message: String,
    pub detail: String,
}

impl AuditEvent {
    pub fn new(severity: Severity, source: &str, message: impl Into<String>) -> Self {
        Self {
            severity,
            source: source.to_string(),
            actor: "rust-manager".to_string(),
            target_map: String::new(),
            message: message.into(),
            detail: String::new(),
        }
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = detail.into();
        self
    }

    pub fn actor(mut self, actor: impl Into<String>) -> Self {
        self.actor = actor.into();
        self
    }

    pub fn target(mut self, target: impl Into<String>) -> Self {
        self.target_map = target.into();
        self
    }
}

/// Persist an audit event to SQLite. Errors are logged but not propagated —
/// failing to write an audit row must never take down a request path.
pub async fn record(pool: &SqlitePool, ev: &AuditEvent) {
    let _ = record_with_id(pool, ev).await;
}

pub async fn record_with_id(pool: &SqlitePool, ev: &AuditEvent) -> Option<i64> {
    tracing::info!(
        severity = ev.severity.as_str(),
        source = %ev.source,
        actor = %ev.actor,
        "audit: {}",
        ev.message
    );

    let res = sqlx::query(
        "INSERT INTO activity_log (severity, source, actor, target_map, message, detail) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )
    .bind(ev.severity.as_str())
    .bind(&ev.source)
    .bind(&ev.actor)
    .bind(&ev.target_map)
    .bind(&ev.message)
    .bind(&ev.detail)
    .execute(pool)
    .await;

    match res {
        Ok(done) => Some(done.last_insert_rowid()),
        Err(e) => {
            tracing::warn!("failed to persist audit event: {e}");
            None
        }
    }
}
