CREATE TABLE IF NOT EXISTS surface_client_pairings (
    pairing_id TEXT PRIMARY KEY,
    surface_client_id TEXT NOT NULL UNIQUE,
    runtime_anchor_id TEXT NOT NULL,
    pairing_epoch INTEGER NOT NULL,
    lifecycle_state TEXT NOT NULL CHECK (
        lifecycle_state IN ('issued', 'active', 'suspended', 'revoked', 'expired')
    ),
    previous_pairing_id TEXT,
    site_binding_digest TEXT NOT NULL,
    site_binding_claims_json TEXT,
    wp_install_uuid_hash TEXT NOT NULL,
    plugin_instance_uuid_hash TEXT NOT NULL,
    site_nonce TEXT NOT NULL,
    scopes_json TEXT NOT NULL,
    scope_digest TEXT NOT NULL,
    endpoint_version TEXT NOT NULL,
    ability_projection_json TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL,
    activated_at TEXT,
    last_used_at TEXT,
    expires_at TEXT NOT NULL,
    revoked_at TEXT,
    revoked_reason TEXT,
    audit_id TEXT,
    FOREIGN KEY(previous_pairing_id) REFERENCES surface_client_pairings(pairing_id)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_surface_client_pairings_epoch
    ON surface_client_pairings(runtime_anchor_id, site_binding_digest, pairing_epoch);

CREATE INDEX IF NOT EXISTS idx_surface_client_pairings_active
    ON surface_client_pairings(surface_client_id, lifecycle_state, expires_at);

CREATE INDEX IF NOT EXISTS idx_surface_client_pairings_site
    ON surface_client_pairings(runtime_anchor_id, site_binding_digest, lifecycle_state);

CREATE INDEX IF NOT EXISTS idx_surface_client_pairings_retention
    ON surface_client_pairings(lifecycle_state, revoked_at, expires_at);

CREATE TABLE IF NOT EXISTS surface_client_revocations (
    revocation_id TEXT PRIMARY KEY,
    surface_client_id TEXT NOT NULL,
    pairing_epoch INTEGER NOT NULL,
    runtime_anchor_id TEXT NOT NULL,
    site_binding_digest TEXT NOT NULL,
    scope_digest TEXT NOT NULL,
    revoked_at TEXT NOT NULL,
    reason TEXT NOT NULL,
    previous_pairing_id TEXT,
    audit_id TEXT,
    FOREIGN KEY(surface_client_id) REFERENCES surface_client_pairings(surface_client_id)
);

CREATE INDEX IF NOT EXISTS idx_surface_client_revocations_pairing
    ON surface_client_revocations(surface_client_id, revoked_at);

CREATE INDEX IF NOT EXISTS idx_surface_client_revocations_site
    ON surface_client_revocations(runtime_anchor_id, site_binding_digest, pairing_epoch);

CREATE TABLE IF NOT EXISTS surface_client_epoch_floors (
    runtime_anchor_id TEXT NOT NULL,
    site_binding_digest TEXT NOT NULL,
    highest_pairing_epoch INTEGER NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY(runtime_anchor_id, site_binding_digest)
);

CREATE TABLE IF NOT EXISTS surface_client_sessions (
    session_id TEXT PRIMARY KEY,
    surface_client_id TEXT NOT NULL,
    pairing_epoch INTEGER NOT NULL,
    bearer_token_hash TEXT NOT NULL,
    hmac_key_id TEXT NOT NULL,
    issued_at TEXT NOT NULL,
    last_seen_at TEXT,
    inactive_expires_at TEXT NOT NULL,
    absolute_expires_at TEXT NOT NULL,
    throttled_until_at TEXT,
    rotated_at TEXT,
    revoked_at TEXT,
    revoked_reason TEXT,
    scope_digest TEXT NOT NULL,
    site_binding_digest TEXT NOT NULL,
    wp_user_hash TEXT NOT NULL,
    FOREIGN KEY(surface_client_id) REFERENCES surface_client_pairings(surface_client_id)
);

CREATE INDEX IF NOT EXISTS idx_surface_client_sessions_client
    ON surface_client_sessions(surface_client_id, revoked_at, inactive_expires_at, absolute_expires_at);

CREATE INDEX IF NOT EXISTS idx_surface_client_sessions_hmac_key
    ON surface_client_sessions(hmac_key_id);

CREATE TABLE IF NOT EXISTS surface_pairing_codes (
    code_hash TEXT PRIMARY KEY,
    endpoint_startup_id TEXT NOT NULL,
    bound_port INTEGER NOT NULL,
    issued_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    consumed_at TEXT,
    failed_attempt_count INTEGER NOT NULL DEFAULT 0,
    last_failed_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_surface_pairing_codes_endpoint
    ON surface_pairing_codes(endpoint_startup_id, bound_port, expires_at);

CREATE TABLE IF NOT EXISTS surface_client_session_failures (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    surface_client_id TEXT,
    failure_code TEXT NOT NULL,
    occurred_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_surface_client_session_failures_recent
    ON surface_client_session_failures(session_id, failure_code, occurred_at);
