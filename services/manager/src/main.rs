//! ARK Smart Cluster Manager — backend entrypoint (Phase 1).
//!
//! Safe skeleton: config + API contract + auth + SQLite + mock data. It does
//! NOT control ARK servers, RCON, systemd, Discord, or mods. All control-plane
//! surfaces are inert models. See README for scope and limitations.

// Phase 1 is a skeleton: several models (systemd controller, RCON listener,
// audit builders, extra config fields) are deliberate forward hooks that are
// defined now but not yet wired to callers. Allow that without warnings.
#![allow(dead_code)]

mod api;
mod auth;
mod config;
mod db;
mod mock;
mod models;
mod state;

use axum::http::{header, Method};
use axum::routing::get;
use axum::{middleware, Json, Router};
use serde_json::json;
use std::path::Path;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::models::audit::{self, AuditEvent, Severity};
use crate::state::AppState;

const DEFAULT_CONFIG: &str = "manager.toml";

#[tokio::main]
async fn main() {
    init_tracing();

    if let Err(code) = run().await {
        std::process::exit(code);
    }
}

async fn run() -> Result<(), i32> {
    // --- config ---
    let config_path = std::env::var("ARK_MANAGER_CONFIG").unwrap_or_else(|_| DEFAULT_CONFIG.into());
    let config = match Config::load(&config_path) {
        Ok(c) => c,
        Err(config::ConfigError::NotFound(p)) => {
            tracing::error!(
                "config file '{p}' not found. Copy manager.example.toml to {DEFAULT_CONFIG} (or set ARK_MANAGER_CONFIG)."
            );
            return Err(2);
        }
        Err(e) => {
            tracing::error!("config error: {e}");
            return Err(2);
        }
    };
    tracing::info!(cluster = %config.cluster.name, "config loaded and validated");

    if !config.bind_is_private() {
        tracing::warn!(
            "bind address {} is NOT a private/Tailscale/LAN address — this dashboard is intended for private access only. Do not expose it publicly.",
            config.server.bind_address
        );
    }

    let addr = config.socket_addr();

    // --- database ---
    let db_path = std::env::var("ARK_MANAGER_DB").unwrap_or_else(|_| "data/manager.db".into());
    let pool = match db::init(&db_path).await {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("database init failed: {e}");
            return Err(3);
        }
    };
    tracing::info!(db = %db_path, "database ready (migrations applied)");

    let state = AppState::new(config, pool.clone());

    // --- startup audit events (safe, no secrets) ---
    audit::record(
        &pool,
        &AuditEvent::new(Severity::Success, "Manager", "Backend started")
            .detail(format!("Phase 1 skeleton. Bind {addr}.")),
    )
    .await;
    audit::record(
        &pool,
        &AuditEvent::new(Severity::Info, "Config", "Config loaded and validated")
            .detail(format!("source: {config_path}")),
    )
    .await;

    // --- routing ---
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::OPTIONS])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
        .allow_origin(Any);

    let api = api::router().route_layer(middleware::from_fn_with_state(
        state.clone(),
        auth::require_token,
    ));

    let web_root = web_root();
    let static_files =
        ServeDir::new(&web_root).fallback(ServeFile::new(format!("{web_root}/index.html")));

    let app = Router::new()
        .route("/health", get(health))
        .nest("/api", api)
        .fallback_service(static_files)
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(cors);

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("failed to bind {addr}: {e}");
            return Err(4);
        }
    };
    tracing::info!(
        "ARK manager listening on http://{addr} (/, /health public; /api/* requires Bearer token)"
    );

    if let Err(e) = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
    {
        tracing::error!("server error: {e}");
        return Err(5);
    }
    Ok(())
}

fn web_root() -> String {
    if let Ok(path) = std::env::var("ARK_MANAGER_WEB_ROOT") {
        return path;
    }
    for candidate in ["build", "../../build", "../build"] {
        if Path::new(candidate).join("index.html").exists() {
            return candidate.into();
        }
    }
    "build".into()
}

/// Unauthenticated health check.
async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok", "service": "ark-manager", "phase": 1 }))
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,tower_http=info,ark_manager=debug"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutdown signal received");
}
