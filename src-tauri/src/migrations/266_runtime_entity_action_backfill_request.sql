BEGIN IMMEDIATE;

CREATE TABLE IF NOT EXISTS migration_state (
    key TEXT PRIMARY KEY,
    value INTEGER NOT NULL
);

INSERT OR IGNORE INTO migration_state (key, value) VALUES ('global_claim_epoch', 0);
INSERT OR IGNORE INTO migration_state (key, value) VALUES ('schema_epoch', 1);

-- Service-owned runtime evidence backfill request.
-- SQL marks intent only; Rust services perform claim writes and recompute enqueue.
INSERT OR IGNORE INTO migration_state (key, value)
VALUES ('runtime_evidence_backfill_266_requested_at', CAST(strftime('%s', 'now') AS INTEGER));

COMMIT;
