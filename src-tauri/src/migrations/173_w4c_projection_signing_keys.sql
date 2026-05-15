CREATE TABLE IF NOT EXISTS projection_signing_keys (
    key_id TEXT PRIMARY KEY,
    public_key_b64 TEXT NOT NULL,
    key_status TEXT NOT NULL CHECK (
        key_status IN ('active', 'rotating', 'retired', 'revoked')
    ),
    created_at TEXT NOT NULL,
    valid_from TEXT NOT NULL,
    valid_until TEXT,
    retired_at TEXT,
    revoked_at TEXT,
    replacement_key_id TEXT REFERENCES projection_signing_keys(key_id),
    keychain_service TEXT NOT NULL,
    keychain_account_ref TEXT NOT NULL UNIQUE
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_projection_signing_keys_one_active
    ON projection_signing_keys(key_status)
    WHERE key_status = 'active';

CREATE INDEX IF NOT EXISTS idx_projection_signing_keys_status
    ON projection_signing_keys(key_status, valid_from, valid_until);

CREATE TABLE IF NOT EXISTS projection_key_status_events (
    event_id TEXT PRIMARY KEY,
    key_id TEXT NOT NULL REFERENCES projection_signing_keys(key_id),
    previous_status TEXT CHECK (
        previous_status IS NULL
        OR previous_status IN ('active', 'rotating', 'retired', 'revoked')
    ),
    next_status TEXT NOT NULL CHECK (
        next_status IN ('active', 'rotating', 'retired', 'revoked')
    ),
    reason TEXT NOT NULL,
    created_at TEXT NOT NULL,
    actor_kind TEXT NOT NULL CHECK (
        actor_kind IN ('user', 'agent', 'admin', 'system', 'surface_client')
    )
);

CREATE INDEX IF NOT EXISTS idx_projection_key_status_events_key
    ON projection_key_status_events(key_id, created_at);
