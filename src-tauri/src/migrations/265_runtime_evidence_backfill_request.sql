-- Request a one-time service-owned runtime evidence backfill.
--
-- This migration intentionally does not write claims or read legacy evidence
-- directly. It only records that startup should run the Rust service backfill
-- path after migrations complete, so claim/provenance/trust behavior stays in
-- services rather than SQL.

BEGIN IMMEDIATE;

CREATE TABLE IF NOT EXISTS migration_state (
    key TEXT PRIMARY KEY,
    value INTEGER NOT NULL
);

INSERT OR IGNORE INTO migration_state (key, value) VALUES ('global_claim_epoch', 0);
INSERT OR IGNORE INTO migration_state (key, value) VALUES ('schema_epoch', 1);

INSERT OR IGNORE INTO migration_state (key, value)
VALUES (
    'runtime_evidence_backfill_265_requested_at',
    CAST(strftime('%s', 'now') AS INTEGER)
);

COMMIT;
