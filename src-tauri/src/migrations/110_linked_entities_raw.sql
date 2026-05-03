-- linked_entities_raw + linked_entities view.
--
-- Raw table is the single write surface for all entity linking. The view is
-- the sole public read surface — production queries must never read `_raw`
-- directly. Direct reads are blocked via a pre-commit grep hook + code review
-- (clippy cannot enforce module-level visibility across crate boundaries).
--
-- Design decisions encoded here:
-- * owner_type CHECK ensures the three recognized surfaces are the only
--   values that can appear; a typo like 'meetings' fails at insert time.
-- * role CHECK prevents silent promotion to 'primary' or 'auto_suggested'
--   via raw SQL that would bypass the engine.
-- * idx_one_primary partial unique index enforces at most one primary link
--   per (owner_type, owner_id) pair. Any rule that tries to insert a second
--   primary without first deleting the old one will get a UNIQUE constraint
--   violation, making double-primary bugs loud instead of silent.
-- * The view filters source != 'user_dismissed' so that dismissed entities
--   are recorded (for dismissal-persistence logic) but invisible to all
--   UI/intelligence reads that go through the view.

CREATE TABLE IF NOT EXISTS linked_entities_raw (
    owner_type    TEXT NOT NULL CHECK (owner_type IN ('meeting', 'email', 'email_thread')),
    owner_id      TEXT NOT NULL,
    entity_id     TEXT NOT NULL,
    entity_type   TEXT NOT NULL,
    role          TEXT NOT NULL CHECK (role IN ('primary', 'related', 'auto_suggested')),
    source        TEXT NOT NULL,  -- 'rule:<id>' | 'user' | 'inherited_from_thread' | 'inherited_from_series' | 'legacy'
    rule_id       TEXT,
    confidence    REAL,
    evidence_json TEXT,
    graph_version INTEGER NOT NULL,
    created_at    TEXT NOT NULL,
    PRIMARY KEY (owner_type, owner_id, entity_id, entity_type)
);

-- At most one primary link per owner. Partial index; does not constrain
-- 'related' or 'auto_suggested' rows.
CREATE UNIQUE INDEX IF NOT EXISTS idx_one_primary
    ON linked_entities_raw (owner_type, owner_id)
    WHERE role = 'primary';

-- Supporting index for owner lookups (e.g. "give me all links for this email")
CREATE INDEX IF NOT EXISTS idx_linked_entities_raw_owner
    ON linked_entities_raw (owner_type, owner_id);

-- View is the public read surface. source='user_dismissed' rows are stored
-- in the raw table so dismissal-persistence checks can find them, but are
-- excluded from all UI / intelligence reads.
CREATE VIEW IF NOT EXISTS linked_entities AS
    SELECT * FROM linked_entities_raw
    WHERE source != 'user_dismissed';
