-- W6-A-meta-1: get_entity_context / empty / zero claims for subject.
-- The subject exists, but the claim table intentionally has zero rows for it.
CREATE TABLE IF NOT EXISTS entities (
    id TEXT NOT NULL,
    name TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    tracker_path TEXT,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (id, entity_type)
);

CREATE TABLE IF NOT EXISTS intelligence_claims (
    id TEXT PRIMARY KEY,
    subject_ref TEXT NOT NULL,
    claim_type TEXT NOT NULL,
    text TEXT NOT NULL,
    actor TEXT NOT NULL,
    data_source TEXT NOT NULL,
    source_ref TEXT,
    source_asof TEXT,
    observed_at TEXT NOT NULL,
    provenance_json TEXT NOT NULL,
    claim_state TEXT NOT NULL DEFAULT 'active',
    surfacing_state TEXT NOT NULL DEFAULT 'active',
    trust_score REAL,
    temporal_scope TEXT NOT NULL DEFAULT 'state',
    sensitivity TEXT NOT NULL DEFAULT 'internal',
    verification_state TEXT NOT NULL DEFAULT 'active'
);

INSERT INTO entities (id, name, entity_type, tracker_path, updated_at)
VALUES ('account-meta-1-example', 'Account Meta 1 Example', 'account', NULL, '2026-05-15T12:00:00Z');

-- No intelligence_claims rows for {"kind":"account","id":"account-meta-1-example"}.
