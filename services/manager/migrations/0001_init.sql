-- ARK Smart Cluster Manager initial schema.
-- Runtime handlers read and write these SQLite tables for backups, activity,
-- travel decisions, config snapshots, and mod records.

CREATE TABLE IF NOT EXISTS servers (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    alias           TEXT NOT NULL,
    ark_map_name    TEXT NOT NULL,
    systemd_unit    TEXT NOT NULL,
    assignment      TEXT NOT NULL DEFAULT 'Unassigned',
    query_port      INTEGER NOT NULL,
    rcon_port       INTEGER NOT NULL,
    game_port       INTEGER NOT NULL,
    slot_priority   INTEGER NOT NULL DEFAULT 0,
    can_be_home     INTEGER NOT NULL DEFAULT 0,
    can_auto_stop   INTEGER NOT NULL DEFAULT 1,
    can_standby     INTEGER NOT NULL DEFAULT 0,
    state           TEXT NOT NULL DEFAULT 'Offline',
    updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS travel_requests (
    id            TEXT PRIMARY KEY,
    map           TEXT NOT NULL,
    requested_by  TEXT NOT NULL,
    source        TEXT NOT NULL,
    source_raw    TEXT NOT NULL,
    source_map    TEXT NOT NULL,
    step          INTEGER NOT NULL DEFAULT 0,
    result        TEXT NOT NULL,
    reason        TEXT NOT NULL DEFAULT '',
    created_at    TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS activity_log (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    ts          TEXT NOT NULL DEFAULT (datetime('now')),
    severity    TEXT NOT NULL,
    source      TEXT NOT NULL,
    actor       TEXT NOT NULL DEFAULT '',
    target_map  TEXT NOT NULL DEFAULT '',
    message     TEXT NOT NULL,
    detail      TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS backups (
    id          TEXT PRIMARY KEY,
    map         TEXT NOT NULL,
    type        TEXT NOT NULL,
    size_mb     INTEGER NOT NULL DEFAULT 0,
    created     TEXT NOT NULL DEFAULT (datetime('now')),
    reason      TEXT NOT NULL,
    status      TEXT NOT NULL,
    error       TEXT
);

CREATE TABLE IF NOT EXISTS config_versions (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    map         TEXT NOT NULL,
    file        TEXT NOT NULL,            -- 'Game.ini' | 'GameUserSettings.ini'
    content     TEXT NOT NULL,
    author      TEXT NOT NULL DEFAULT '',
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS mods (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL,
    workshop_id   TEXT NOT NULL,
    enabled       INTEGER NOT NULL DEFAULT 1,
    installed     INTEGER NOT NULL DEFAULT 0,
    load_order    INTEGER NOT NULL DEFAULT 0,
    last_updated  TEXT NOT NULL DEFAULT '',
    size_mb       INTEGER NOT NULL DEFAULT 0,
    state         TEXT NOT NULL DEFAULT 'missing'
);

CREATE TABLE IF NOT EXISTS settings (
    key         TEXT PRIMARY KEY,
    value       TEXT NOT NULL,
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_activity_log_ts ON activity_log (ts);
CREATE INDEX IF NOT EXISTS idx_backups_created ON backups (created);
