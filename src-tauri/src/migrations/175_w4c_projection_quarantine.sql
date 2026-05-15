ALTER TABLE projection_ledger
    ADD COLUMN quarantine_state TEXT NOT NULL DEFAULT 'none' CHECK (
        quarantine_state IN ('none', 'suspected', 'quarantined', 'resolved')
    );

ALTER TABLE projection_ledger
    ADD COLUMN last_quarantine_event_at TEXT;

ALTER TABLE projection_ledger
    ADD COLUMN quarantine_event_count INTEGER NOT NULL DEFAULT 0;

CREATE TABLE IF NOT EXISTS projection_quarantine (
    quarantine_id TEXT PRIMARY KEY,
    projection_id TEXT NOT NULL REFERENCES projection_ledger(projection_id),
    surface TEXT NOT NULL CHECK (surface IN ('wordpress_db', 'markdown_file')),
    surface_locator_hash TEXT NOT NULL,
    observed_payload_hash TEXT NOT NULL,
    observed_signature_b64 TEXT,
    expected_signature_id TEXT REFERENCES projection_signatures(signature_id),
    verification_error TEXT NOT NULL,
    field_pointer TEXT,
    byte_range_start INTEGER,
    byte_range_end INTEGER,
    sanitized_observed_excerpt_hash TEXT,
    detected_by TEXT NOT NULL,
    detected_at TEXT NOT NULL,
    last_seen_at TEXT NOT NULL,
    seen_count INTEGER NOT NULL DEFAULT 1 CHECK (seen_count >= 1),
    coalesced_until TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'open' CHECK (
        status IN ('open', 'resolved', 'ignored')
    )
);

CREATE INDEX IF NOT EXISTS idx_projection_quarantine_projection_open
    ON projection_quarantine(projection_id, coalesced_until)
    WHERE status = 'open';

CREATE INDEX IF NOT EXISTS idx_projection_quarantine_status
    ON projection_quarantine(status, detected_at);
