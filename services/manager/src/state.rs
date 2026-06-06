//! Shared application state passed to every handler.

use std::sync::Arc;
use std::time::Instant;

use sqlx::SqlitePool;
use tokio::sync::RwLock;

use crate::config::Config;
use crate::models::rcon::{RconRuntimeState, SharedRconRuntime};
use crate::models::systemd::{RealSystemd, SystemdController};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub pool: SqlitePool,
    /// systemd reader/controller. T1.1 only calls read-only status methods;
    /// mutating calls return `NotImplemented` and are not exposed via routes.
    pub systemd: Arc<dyn SystemdController>,
    pub rcon_runtime: SharedRconRuntime,
    pub manager_started_at: Instant,
}

impl AppState {
    pub fn new(config: Config, pool: SqlitePool) -> Self {
        Self {
            config: Arc::new(config),
            pool,
            systemd: Arc::new(RealSystemd),
            rcon_runtime: Arc::new(RwLock::new(RconRuntimeState::new())),
            manager_started_at: Instant::now(),
        }
    }
}
