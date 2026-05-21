-- v1.4.5 W1-A / DOS-463 — workspace file lifecycle and ownership model.
--
-- Authoritative provenance carrier for every workspace file DailyOS monitors.
-- One row per (file_id) tracks the seven-state lifecycle (pending,
-- pending_entity_assignment, ingesting, ingested, superseded, rejected,
-- quarantined), the entity binding (if known), the FileIdentity triple
-- (canonical_path + device + inode), and the user-override audit trail for
-- corrections that bypass automated transitions.
--
-- W1-A commits no claims; the lifecycle row is the substrate that W2-A's
-- IngestPipeline, W3-A's WorkspaceExtractor, and W4-A's source-management
-- block all consume.
--
-- Per cycle 12 substrate-consumption sweep: source_type stores the serde-tag
-- of canonical abilities_runtime::abilities::provenance::source::WorkspaceFileKind
-- (snake_case string), NOT a separate W1-A enum mirror. data_source stores
-- JSON-serialized DataSource::WorkspaceFile { kind } per ADR-0107 + the W0
-- amendment.

CREATE TABLE IF NOT EXISTS workspace_file_lifecycle (
    id                    INTEGER PRIMARY KEY AUTOINCREMENT,
    file_id               TEXT    NOT NULL UNIQUE,
    canonical_path        TEXT    NOT NULL,
    device                INTEGER NOT NULL,
    inode                 INTEGER NOT NULL,
    source_type           TEXT    NOT NULL,
    data_source           TEXT    NOT NULL,
    lifecycle_state       TEXT    NOT NULL DEFAULT 'pending',
    source_asof           TEXT    NOT NULL,
    entity_id             TEXT,
    entity_type           TEXT,
    content_sha256        TEXT,
    user_override_actor   TEXT,
    user_override_at      TEXT,
    created_at            TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at            TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

-- W2-D inbox-listing query: SELECT … WHERE lifecycle_state IN (…).
CREATE INDEX IF NOT EXISTS idx_wfl_state
    ON workspace_file_lifecycle (lifecycle_state);

-- W4-A entity-source-list query: SELECT … WHERE entity_id = ? AND entity_type = ?.
-- Partial index keeps the lookup tight even as the table grows with
-- pending_entity_assignment rows that lack entity binding.
CREATE INDEX IF NOT EXISTS idx_wfl_entity
    ON workspace_file_lifecycle (entity_id, entity_type)
    WHERE entity_id IS NOT NULL;

-- W2-D badge-count query: SELECT count(*) … WHERE lifecycle_state = 'pending_entity_assignment'.
-- Partial index is the hot path for the inbox UI's pending counter.
CREATE INDEX IF NOT EXISTS idx_wfl_pending_entity
    ON workspace_file_lifecycle (lifecycle_state)
    WHERE lifecycle_state = 'pending_entity_assignment';
