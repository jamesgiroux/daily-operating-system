CREATE TABLE IF NOT EXISTS claim_file_projection_runs (
    id TEXT PRIMARY KEY,
    entity_subject_ref_json TEXT NOT NULL,
    entity_subject_compact TEXT NOT NULL,
    projection_root TEXT NOT NULL DEFAULT '_dailyos_claims'
        CHECK (projection_root = '_dailyos_claims'),
    markdown_rel_path TEXT NOT NULL,
    sidecar_rel_path TEXT NOT NULL,
    projection_version INTEGER NOT NULL,
    sidecar_schema_version INTEGER NOT NULL,
    entity_claim_invalidation_version INTEGER NOT NULL DEFAULT 0,
    claim_watermark TEXT NOT NULL,
    markdown_checksum TEXT NOT NULL,
    sidecar_checksum TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('committed', 'failed', 'repaired')),
    error_class TEXT,
    error_detail_hash TEXT,
    attempted_at TEXT NOT NULL,
    succeeded_at TEXT,
    repaired_from_run_id TEXT REFERENCES claim_file_projection_runs(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_claim_file_projection_runs_entity_status
    ON claim_file_projection_runs(entity_subject_compact, status, attempted_at);

CREATE INDEX IF NOT EXISTS idx_claim_file_projection_runs_repair
    ON claim_file_projection_runs(status, attempted_at)
    WHERE status = 'failed';
