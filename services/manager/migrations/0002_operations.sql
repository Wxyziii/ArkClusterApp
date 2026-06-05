-- T1.2 guarded operations foundation.

CREATE TABLE IF NOT EXISTS systemd_actions (
    id              TEXT PRIMARY KEY,
    ts              TEXT NOT NULL DEFAULT (datetime('now')),
    server_id       TEXT NOT NULL,
    slot_id         TEXT NOT NULL,
    unit            TEXT NOT NULL,
    operation       TEXT NOT NULL,
    reason          TEXT NOT NULL,
    result          TEXT NOT NULL,
    message         TEXT NOT NULL DEFAULT '',
    detail          TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS backup_records (
    id              TEXT PRIMARY KEY,
    slot_id         TEXT NOT NULL,
    server_id       TEXT NOT NULL,
    map_key         TEXT NOT NULL,
    type            TEXT NOT NULL,
    reason          TEXT NOT NULL,
    status          TEXT NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at    TEXT,
    size_bytes      INTEGER NOT NULL DEFAULT 0,
    path            TEXT NOT NULL DEFAULT '',
    error           TEXT
);

CREATE TABLE IF NOT EXISTS rcon_status (
    slot_id             TEXT PRIMARY KEY,
    server_id           TEXT NOT NULL DEFAULT '',
    enabled             INTEGER NOT NULL DEFAULT 0,
    configured          INTEGER NOT NULL DEFAULT 0,
    connected           INTEGER NOT NULL DEFAULT 0,
    last_poll_at        TEXT,
    last_error          TEXT,
    player_count        INTEGER NOT NULL DEFAULT 0,
    player_names_json   TEXT NOT NULL DEFAULT '[]',
    updated_at          TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS detected_chat_commands (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    ts              TEXT NOT NULL DEFAULT (datetime('now')),
    slot_id         TEXT NOT NULL,
    player          TEXT NOT NULL DEFAULT '',
    command         TEXT NOT NULL,
    argument        TEXT NOT NULL DEFAULT '',
    mode            TEXT NOT NULL DEFAULT 'detected_only'
);

CREATE INDEX IF NOT EXISTS idx_systemd_actions_ts ON systemd_actions (ts);
CREATE INDEX IF NOT EXISTS idx_backup_records_created ON backup_records (created_at);
CREATE INDEX IF NOT EXISTS idx_detected_chat_commands_ts ON detected_chat_commands (ts);
