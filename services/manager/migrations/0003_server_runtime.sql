-- Server-side ARK runtime readiness, travel, config, mod, and maintenance state.

CREATE TABLE IF NOT EXISTS travel_requests (
    id              TEXT PRIMARY KEY,
    ts              TEXT NOT NULL DEFAULT (datetime('now')),
    source          TEXT NOT NULL,
    actor           TEXT NOT NULL DEFAULT '',
    requested_map   TEXT NOT NULL,
    resolved_map    TEXT NOT NULL DEFAULT '',
    chosen_slot     TEXT NOT NULL DEFAULT '',
    status          TEXT NOT NULL,
    reason          TEXT NOT NULL DEFAULT '',
    detail          TEXT NOT NULL DEFAULT ''
);

ALTER TABLE travel_requests ADD COLUMN ts TEXT NOT NULL DEFAULT '';
ALTER TABLE travel_requests ADD COLUMN actor TEXT NOT NULL DEFAULT '';
ALTER TABLE travel_requests ADD COLUMN requested_map TEXT NOT NULL DEFAULT '';
ALTER TABLE travel_requests ADD COLUMN resolved_map TEXT NOT NULL DEFAULT '';
ALTER TABLE travel_requests ADD COLUMN chosen_slot TEXT NOT NULL DEFAULT '';
ALTER TABLE travel_requests ADD COLUMN status TEXT NOT NULL DEFAULT '';
ALTER TABLE travel_requests ADD COLUMN detail TEXT NOT NULL DEFAULT '';

CREATE TABLE IF NOT EXISTS slot_idle_state (
    slot_id             TEXT PRIMARY KEY,
    player_count        INTEGER,
    idle_started_at     TEXT,
    last_seen_players_at TEXT,
    shutdown_due_at     TEXT,
    updated_at          TEXT NOT NULL DEFAULT (datetime('now')),
    status              TEXT NOT NULL DEFAULT 'unknown'
);

CREATE TABLE IF NOT EXISTS config_snapshots (
    id              TEXT PRIMARY KEY,
    ts              TEXT NOT NULL DEFAULT (datetime('now')),
    actor           TEXT NOT NULL DEFAULT 'manager',
    file            TEXT NOT NULL,
    reason          TEXT NOT NULL DEFAULT '',
    backup_path     TEXT NOT NULL DEFAULT '',
    status          TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS mod_records (
    workshop_id     TEXT PRIMARY KEY,
    name            TEXT NOT NULL DEFAULT '',
    enabled         INTEGER NOT NULL DEFAULT 0,
    installed       INTEGER NOT NULL DEFAULT 0,
    load_order      INTEGER NOT NULL DEFAULT 0,
    last_updated    TEXT,
    status          TEXT NOT NULL DEFAULT 'known',
    error           TEXT
);

CREATE TABLE IF NOT EXISTS maintenance_jobs (
    id              TEXT PRIMARY KEY,
    ts              TEXT NOT NULL DEFAULT (datetime('now')),
    kind            TEXT NOT NULL,
    status          TEXT NOT NULL,
    reason          TEXT NOT NULL DEFAULT '',
    detail          TEXT NOT NULL DEFAULT ''
);

CREATE INDEX IF NOT EXISTS idx_travel_requests_ts ON travel_requests (ts);
CREATE INDEX IF NOT EXISTS idx_maintenance_jobs_ts ON maintenance_jobs (ts);
