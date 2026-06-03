//! Shared application state passed to every handler.

use std::sync::Arc;

use sqlx::SqlitePool;

use crate::config::Config;
use crate::models::systemd::{MockSystemd, SystemdController};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub pool: SqlitePool,
    /// systemd controller. Phase 1 wires the read-only mock; mutating calls
    /// return `NotImplemented` and are not exposed via any route.
    pub systemd: Arc<dyn SystemdController>,
}

impl AppState {
    pub fn new(config: Config, pool: SqlitePool) -> Self {
        Self {
            config: Arc::new(config),
            pool,
            systemd: Arc::new(MockSystemd),
        }
    }
}
