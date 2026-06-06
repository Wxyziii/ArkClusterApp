//! SQLite setup and migrations.
//!
//! The DB file is created if missing (with parent dirs). Migrations from the
//! `migrations/` directory are embedded at compile time and applied on startup.
//! Runtime handlers read backup/activity/config/mod rows from this database.

use std::path::Path;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("failed to create database directory {0}: {1}")]
    Dir(String, std::io::Error),
    #[error("failed to open database: {0}")]
    Connect(#[from] sqlx::Error),
    #[error("failed to run migrations: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
}

/// Open (creating if needed) the SQLite database at `path` and run migrations.
/// Pass `":memory:"` for an ephemeral DB (used in tests).
pub async fn init(path: &str) -> Result<SqlitePool, DbError> {
    let opts = if path == ":memory:" {
        SqliteConnectOptions::new().in_memory(true)
    } else {
        if let Some(parent) = Path::new(path).parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| DbError::Dir(parent.display().to_string(), e))?;
            }
        }
        SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
    };

    // An in-memory SQLite DB lives only inside a single connection, so the pool
    // must hold exactly one or migrations land in a connection later queries
    // never see. File-backed DBs can use the full pool.
    let max_conns = if path == ":memory:" { 1 } else { 5 };

    let pool = SqlitePoolOptions::new()
        .max_connections(max_conns)
        .connect_with(opts)
        .await?;

    MIGRATOR.run(&pool).await?;
    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn in_memory_db_migrates() {
        let pool = init(":memory:").await.expect("init in-memory db");
        // Tables should exist after migration.
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM activity_log")
            .fetch_one(&pool)
            .await
            .expect("query activity_log");
        assert_eq!(row.0, 0);
    }
}
