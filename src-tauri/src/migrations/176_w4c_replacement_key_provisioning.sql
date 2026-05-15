CREATE TABLE IF NOT EXISTS projection_replacement_keys (
    replacement_id TEXT PRIMARY KEY,
    old_key_id TEXT NOT NULL REFERENCES projection_signing_keys(key_id),
    new_key_id TEXT NOT NULL REFERENCES projection_signing_keys(key_id),
    reason TEXT NOT NULL,
    provisioned_at TEXT NOT NULL,
    activated_at TEXT,
    completed_at TEXT,
    recovery_status TEXT NOT NULL DEFAULT 'pending' CHECK (
        recovery_status IN ('pending', 'in_progress', 'completed', 'failed')
    )
);

CREATE INDEX IF NOT EXISTS idx_projection_replacement_keys_old
    ON projection_replacement_keys(old_key_id, recovery_status);

CREATE TABLE IF NOT EXISTS projection_resign_queue (
    queue_id TEXT PRIMARY KEY,
    projection_id TEXT NOT NULL REFERENCES projection_ledger(projection_id),
    old_signature_id TEXT REFERENCES projection_signatures(signature_id),
    old_key_id TEXT NOT NULL REFERENCES projection_signing_keys(key_id),
    new_key_id TEXT NOT NULL REFERENCES projection_signing_keys(key_id),
    reason TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (
        status IN ('pending', 'processing', 'completed', 'failed')
    ),
    attempts INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    max_attempts INTEGER NOT NULL DEFAULT 3 CHECK (max_attempts >= 1),
    last_error TEXT,
    last_resign_at TEXT,
    last_retampered_at TEXT,
    operator_escalated_at TEXT,
    queued_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    completed_at TEXT
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_projection_resign_queue_unique
    ON projection_resign_queue(projection_id, old_signature_id, new_key_id);

CREATE INDEX IF NOT EXISTS idx_projection_resign_queue_status
    ON projection_resign_queue(status, queued_at);
