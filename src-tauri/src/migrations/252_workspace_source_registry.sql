-- v1.4.5 W1-B — workspace source registry: source-type allowlist + per-entity-type category registry.
-- Per L0 packet V1.3 §6.

CREATE TABLE IF NOT EXISTS workspace_source_registry (
    source_type             TEXT PRIMARY KEY,           -- canonical WorkspaceFileKind serde-tag (snake_case)
    data_source_json        TEXT NOT NULL,              -- JSON-serialized DataSource::WorkspaceFile{kind} (externally-tagged)
    default_sensitivity     TEXT NOT NULL,              -- 'internal' | 'confidential'
    allowed                 INTEGER NOT NULL DEFAULT 1,
    created_at              TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

-- Canonical serde shape: DataSource is externally-tagged with rename_all=snake_case.
-- Granola + Quill transcripts default to confidential per V1.1 fold #11 (PII-bearing).
INSERT OR IGNORE INTO workspace_source_registry (source_type, data_source_json, default_sensitivity) VALUES
    ('inbox',              '{"workspace_file":{"kind":"inbox"}}',              'internal'),
    ('entity_doc',         '{"workspace_file":{"kind":"entity_doc"}}',         'internal'),
    ('drive_sync',         '{"workspace_file":{"kind":"drive_sync"}}',         'internal'),
    ('user_attachment',    '{"workspace_file":{"kind":"user_attachment"}}',    'internal'),
    ('granola_transcript', '{"workspace_file":{"kind":"granola_transcript"}}', 'confidential'),
    ('quill_transcript',   '{"workspace_file":{"kind":"quill_transcript"}}',   'confidential'),
    ('mcp_placement',      '{"workspace_file":{"kind":"mcp_placement"}}',      'internal');

CREATE TABLE IF NOT EXISTS workspace_category_registry (
    entity_type             TEXT NOT NULL,              -- 'account' | 'person' | 'project' (snake_case per crate::entity::EntityType)
    category_slug           TEXT NOT NULL,              -- canonical WorkspaceCategory::as_slug() output
    allowed                 INTEGER NOT NULL DEFAULT 1,
    created_at              TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    PRIMARY KEY (entity_type, category_slug)
);

-- All 6 default categories for each of the 3 entity types per cycle 8 Option B-prime
-- (V1.1 fold #18 corrected V1.0's accidental 4-categories-for-person count).
INSERT OR IGNORE INTO workspace_category_registry (entity_type, category_slug) VALUES
    ('account', 'presentations'), ('account', 'transcripts'), ('account', 'meetings'),
    ('account', 'notes'),         ('account', 'contracts'),   ('account', 'attachments'),
    ('person',  'presentations'), ('person',  'transcripts'), ('person',  'meetings'),
    ('person',  'notes'),         ('person',  'contracts'),   ('person',  'attachments'),
    ('project', 'presentations'), ('project', 'transcripts'), ('project', 'meetings'),
    ('project', 'notes'),         ('project', 'contracts'),   ('project', 'attachments');

CREATE INDEX IF NOT EXISTS idx_wcr_entity_type ON workspace_category_registry (entity_type);
