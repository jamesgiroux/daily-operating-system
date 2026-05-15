-- W6-A-meta-8: list_open_loops_extract_commitments / empty / empty workspace.
-- The claim table exists and intentionally has zero commitment or open-loop rows.
CREATE TABLE IF NOT EXISTS intelligence_claims (
    id TEXT PRIMARY KEY,
    subject_ref TEXT NOT NULL,
    claim_type TEXT NOT NULL,
    field_path TEXT,
    topic_key TEXT,
    text TEXT NOT NULL,
    actor TEXT NOT NULL,
    data_source TEXT NOT NULL,
    source_ref TEXT,
    source_asof TEXT,
    observed_at TEXT NOT NULL,
    provenance_json TEXT NOT NULL,
    claim_state TEXT NOT NULL DEFAULT 'active',
    surfacing_state TEXT NOT NULL DEFAULT 'active',
    expires_at TEXT,
    trust_score REAL,
    temporal_scope TEXT NOT NULL DEFAULT 'state',
    sensitivity TEXT NOT NULL DEFAULT 'internal',
    verification_state TEXT NOT NULL DEFAULT 'active'
);

-- No open_loop or commitment claims exist in this workspace.
