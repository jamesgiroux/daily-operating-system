ALTER TABLE projection_quarantine
    ADD COLUMN observed_payload_bytes BLOB;

CREATE TABLE IF NOT EXISTS projection_keyring_state (
    state_id TEXT PRIMARY KEY CHECK (state_id = 'projection_keyring'),
    current_version INTEGER NOT NULL CHECK (current_version >= 1),
    updated_at TEXT NOT NULL
);

INSERT OR IGNORE INTO projection_keyring_state
    (state_id, current_version, updated_at)
VALUES ('projection_keyring', 1, datetime('now'));

CREATE TABLE IF NOT EXISTS projection_signature_enforcement_state (
    state_id TEXT PRIMARY KEY CHECK (state_id = 'projection_signature_enforcement'),
    mode TEXT NOT NULL CHECK (mode IN ('shadow', 'enforce', 'disabled')),
    updated_at TEXT NOT NULL,
    actor_kind TEXT NOT NULL CHECK (
        actor_kind IN ('user', 'agent', 'admin', 'system', 'surface_client')
    )
);

INSERT OR IGNORE INTO projection_signature_enforcement_state
    (state_id, mode, updated_at, actor_kind)
VALUES ('projection_signature_enforcement', 'shadow', datetime('now'), 'system');

CREATE TABLE IF NOT EXISTS projection_enforcement_mode_events (
    event_id TEXT PRIMARY KEY,
    previous_mode TEXT NOT NULL CHECK (previous_mode IN ('shadow', 'enforce', 'disabled')),
    next_mode TEXT NOT NULL CHECK (next_mode IN ('shadow', 'enforce', 'disabled')),
    reason TEXT NOT NULL,
    created_at TEXT NOT NULL,
    actor_kind TEXT NOT NULL CHECK (
        actor_kind IN ('user', 'agent', 'admin', 'system', 'surface_client')
    )
);

CREATE INDEX IF NOT EXISTS idx_projection_enforcement_mode_events_created
    ON projection_enforcement_mode_events(created_at);
