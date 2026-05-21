-- v1.4.5 W1-C — ingestion run tracking. Append-only history.
-- Per L0 packet V1.3 §6. Idempotency enforced via UNIQUE partial index on success rows.

CREATE TABLE IF NOT EXISTS document_ingestion_runs (
    id                      INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id                  TEXT NOT NULL UNIQUE,
    file_id                 TEXT NOT NULL,
    mode                    TEXT NOT NULL,                  -- 'initial' | 'incremental' | 'forced' | 'backfill'
    started_at              TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    completed_at            TEXT,
    status                  TEXT NOT NULL DEFAULT 'in_progress',
    content_sha256          TEXT NOT NULL,
    file_size_bytes         INTEGER NOT NULL,
    extractor_version       TEXT NOT NULL,
    claim_count_produced    INTEGER NOT NULL DEFAULT 0,
    error_log               TEXT,
    retry_of_run_id         TEXT,
    FOREIGN KEY (file_id) REFERENCES workspace_file_lifecycle(file_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_dir_file_id ON document_ingestion_runs (file_id);
CREATE INDEX IF NOT EXISTS idx_dir_status ON document_ingestion_runs (status);
-- UNIQUE partial index enforces idempotency at SQL layer (V1.2 fold #4). Forced-mode retries
-- coexist because in-progress rows aren't in the partial scope.
CREATE UNIQUE INDEX IF NOT EXISTS idx_dir_idempotency_unique
    ON document_ingestion_runs (file_id, content_sha256, mode)
    WHERE status = 'success';
