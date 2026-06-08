-- External node registry: Windows travel PCs, pairing, tokens, task queue, sessions.

CREATE TABLE IF NOT EXISTS nodes (
    id                      TEXT PRIMARY KEY,
    display_name            TEXT NOT NULL,
    owner_discord_id        TEXT NOT NULL DEFAULT '',
    node_type               TEXT NOT NULL DEFAULT 'external-windows',
    status                  TEXT NOT NULL DEFAULT 'offline',
    max_travel_servers      INTEGER NOT NULL DEFAULT 1,
    tailscale_ip            TEXT NOT NULL DEFAULT '',
    version                 TEXT NOT NULL DEFAULT '',
    active_travel_servers   INTEGER NOT NULL DEFAULT 0,
    current_map             TEXT,
    available_ram_mb        INTEGER,
    total_ram_mb            INTEGER,
    cluster_share_mounted   INTEGER NOT NULL DEFAULT 0,
    ark_server_installed    INTEGER NOT NULL DEFAULT 0,
    mods_valid              INTEGER NOT NULL DEFAULT 0,
    config_valid            INTEGER NOT NULL DEFAULT 0,
    ports_free              INTEGER NOT NULL DEFAULT 0,
    rcon_ready              INTEGER NOT NULL DEFAULT 0,
    last_heartbeat          TEXT,
    last_error              TEXT,
    created_at              TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at              TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS node_tokens (
    node_id                 TEXT PRIMARY KEY,
    token_hash              TEXT NOT NULL,
    created_at              TEXT NOT NULL DEFAULT (datetime('now')),
    revoked                 INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS node_pairing_invites (
    code                    TEXT PRIMARY KEY,
    suggested_name          TEXT NOT NULL DEFAULT '',
    created_by              TEXT NOT NULL DEFAULT '',
    expires_at              TEXT NOT NULL,
    used                    INTEGER NOT NULL DEFAULT 0,
    node_id                 TEXT NOT NULL DEFAULT '',
    created_at              TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS travel_sessions (
    id                      TEXT PRIMARY KEY,
    node_id                 TEXT NOT NULL,
    map_id                  TEXT NOT NULL,
    map_name                TEXT NOT NULL DEFAULT '',
    session_name            TEXT NOT NULL DEFAULT '',
    status                  TEXT NOT NULL DEFAULT 'starting',
    requester_discord_id    TEXT NOT NULL DEFAULT '',
    game_port               INTEGER,
    raw_port                INTEGER,
    query_port              INTEGER,
    rcon_port               INTEGER,
    started_at              TEXT NOT NULL DEFAULT (datetime('now')),
    ready_at                TEXT,
    closed_at               TEXT,
    last_error              TEXT
);

CREATE TABLE IF NOT EXISTS node_tasks (
    id                      TEXT PRIMARY KEY,
    node_id                 TEXT NOT NULL,
    session_id              TEXT,
    task_type               TEXT NOT NULL,
    payload                 TEXT NOT NULL DEFAULT '{}',
    status                  TEXT NOT NULL DEFAULT 'pending',
    created_at              TEXT NOT NULL DEFAULT (datetime('now')),
    sent_at                 TEXT,
    completed_at            TEXT,
    result                  TEXT,
    error                   TEXT
);

CREATE INDEX IF NOT EXISTS idx_nodes_status ON nodes (status);
CREATE INDEX IF NOT EXISTS idx_nodes_owner ON nodes (owner_discord_id);
CREATE INDEX IF NOT EXISTS idx_travel_sessions_node ON travel_sessions (node_id);
CREATE INDEX IF NOT EXISTS idx_travel_sessions_status ON travel_sessions (status);
CREATE INDEX IF NOT EXISTS idx_node_tasks_node_status ON node_tasks (node_id, status);
CREATE INDEX IF NOT EXISTS idx_node_pairing_expires ON node_pairing_invites (expires_at);
