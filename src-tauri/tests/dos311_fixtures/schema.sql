-- Fixture schema for tombstone-resurrection reconcile tests.
--
-- TEST-ONLY scaffolding. The production `intelligence_claims` table ships
-- with (W3); this file mirrors enough of that schema for the W1
-- reconcile-pass tests to exercise the SQL in `scripts/reconcile_ghost_resurrection.sql`.
--
--  may migrate this schema during W3 (e.g., adding columns,
-- renaming fields). The fixtures are designed to be regenerable from
-- the canonical  schema once it lands; until then they encode the
-- shape promised by the live  ticket text.
--
-- Columns mirrored from ticket text + ADR-0113:
--   subject_ref       — the claim's subject (stored as JSON string in
--                       fixtures; production may use a different shape)
--   claim_type        — claim type registry key
--   field_path        — the claim's projected field path
--   dedup_key         — canonical dedup key
--   item_hash         — content fingerprint (reconcile fallback)
--   source_asof       — when the source evidence was observed
--   created_at        — claim row creation (also used as tombstone's
--                       dismissed_at for tombstoned-state rows)
--   claim_state       — 'active' | 'tombstoned' | 'superseded'
--   superseded_at     — if non-null, the row is superseded
--
-- The companion `legacy_projection_state` view materializes the legacy
-- pre- projection shape (entity_intelligence JSON, intelligence.json
-- file content, accounts.* narrative columns). For test purposes we
-- fixture it as a simple table.

CREATE TABLE IF NOT EXISTS intelligence_claims (
    claim_id      TEXT PRIMARY KEY,
    subject_ref   TEXT NOT NULL,
    claim_type    TEXT NOT NULL,
    field_path    TEXT NOT NULL,
    dedup_key     TEXT,
    item_hash     TEXT,
    source_asof   TEXT,
    created_at    TEXT NOT NULL,
    claim_state   TEXT NOT NULL CHECK (claim_state IN ('active', 'tombstoned', 'superseded')),
    superseded_at TEXT
);

-- Index pattern that  will mirror on the production table.
CREATE INDEX IF NOT EXISTS idx_claims_subject_state
    ON intelligence_claims(subject_ref, claim_type, claim_state);

-- Test scaffolding for legacy projection state. Production  wires
-- a view over entity_intelligence + accounts.* narrative columns +
-- intelligence.json content; the test fixture shape exposes only the
-- columns the reconcile SQL joins on.
CREATE TABLE IF NOT EXISTS legacy_projection_state (
    subject_ref        TEXT NOT NULL,
    claim_type         TEXT NOT NULL,
    field_path         TEXT NOT NULL,
    dedup_key          TEXT,
    item_hash          TEXT,
    sourced_at         TEXT,
    projection_target  TEXT NOT NULL  -- 'entity_intelligence' | 'intelligence_json' | 'account_narrative'
);
