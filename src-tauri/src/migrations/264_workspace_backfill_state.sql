-- v1.4.5 W5-A — workspace source backfill state.
--
-- These tables track resumable registration/classification of existing
-- workspace files as conservative pending-review sources. They intentionally do
-- not store raw paths, filenames, file content, claim text, prompts, or output
-- bodies; operational file identity remains in workspace_file_lifecycle.

CREATE TABLE IF NOT EXISTS workspace_backfill_runs (
    run_id                       TEXT PRIMARY KEY,
    mode                         TEXT NOT NULL CHECK (mode IN ('dry_run', 'apply')),
    status                       TEXT NOT NULL CHECK (status IN ('running', 'completed', 'failed', 'aborted')),
    workspace_root_fingerprint   TEXT NOT NULL,
    actor                        TEXT NOT NULL DEFAULT 'system:workspace_backfill:v1',
    reason_counts_json           TEXT NOT NULL DEFAULT '{}',
    source_class_counts_json     TEXT NOT NULL DEFAULT '{}',
    divergence_counts_json       TEXT NOT NULL DEFAULT '{}',
    started_at                   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    completed_at                 TEXT,
    created_at                   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at                   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE INDEX IF NOT EXISTS idx_wbr_status
    ON workspace_backfill_runs (status);

CREATE TABLE IF NOT EXISTS workspace_backfill_items (
    run_id                       TEXT NOT NULL,
    source_handle                TEXT NOT NULL,
    item_handle                  TEXT NOT NULL,
    file_id                      TEXT NOT NULL,
    content_sha256               TEXT,
    duplicate_group_handle       TEXT,
    candidate_kind               TEXT NOT NULL,
    entity_type                  TEXT,
    entity_id                    TEXT,
    category                     TEXT,
    exposure_state               TEXT NOT NULL DEFAULT 'pending_review'
        CHECK (exposure_state IN ('pending_review', 'promoted', 'ignored')),
    source_time_basis            TEXT,
    source_time_confidence       TEXT,
    backfill_observed_at         TEXT NOT NULL,
    status                       TEXT NOT NULL CHECK (status IN ('planned', 'applied', 'skipped', 'failed')),
    reason_code                  TEXT,
    created_at                   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at                   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    PRIMARY KEY (run_id, source_handle),
    FOREIGN KEY (run_id) REFERENCES workspace_backfill_runs(run_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_wbi_source_handle
    ON workspace_backfill_items (source_handle);

CREATE INDEX IF NOT EXISTS idx_wbi_file_id
    ON workspace_backfill_items (file_id);

CREATE INDEX IF NOT EXISTS idx_wbi_status
    ON workspace_backfill_items (status);

CREATE TABLE IF NOT EXISTS workspace_backfill_operations (
    id                           INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id                       TEXT NOT NULL,
    source_handle                TEXT NOT NULL,
    operation_kind               TEXT NOT NULL,
    status                       TEXT NOT NULL CHECK (status IN ('planned', 'applied', 'skipped', 'failed')),
    created_lifecycle            INTEGER NOT NULL DEFAULT 0,
    updated_lifecycle_fields     TEXT NOT NULL DEFAULT '[]',
    created_link_handle          TEXT,
    reason_code                  TEXT,
    created_at                   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at                   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    FOREIGN KEY (run_id, source_handle)
        REFERENCES workspace_backfill_items(run_id, source_handle) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_wbo_run_source
    ON workspace_backfill_operations (run_id, source_handle);

CREATE INDEX IF NOT EXISTS idx_wbo_kind_status
    ON workspace_backfill_operations (operation_kind, status);
