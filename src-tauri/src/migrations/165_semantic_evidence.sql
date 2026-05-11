-- Persist full per-variant evidence for semantic claim merges.
-- claim_corroborations intentionally aggregates by source; this table keeps
-- the original source record recoverable for every near-duplicate variant.

CREATE TABLE IF NOT EXISTS claim_semantic_evidence (
    id                 TEXT PRIMARY KEY,
    canonical_claim_id TEXT NOT NULL REFERENCES intelligence_claims(id),
    corroboration_id   TEXT REFERENCES claim_corroborations(id),
    data_source        TEXT NOT NULL,
    source_ref         TEXT,
    source_asof        TEXT,
    provenance_json    TEXT NOT NULL,
    original_text      TEXT NOT NULL,
    actor              TEXT NOT NULL,
    observed_at        TEXT NOT NULL,
    thread_id          TEXT,
    source_mechanism   TEXT NOT NULL,
    created_at         TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_claim_semantic_evidence_claim
    ON claim_semantic_evidence(canonical_claim_id);

CREATE INDEX IF NOT EXISTS idx_claim_semantic_evidence_source_ref
    ON claim_semantic_evidence(source_ref)
    WHERE source_ref IS NOT NULL;
