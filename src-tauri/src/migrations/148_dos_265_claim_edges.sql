-- Claim-derived entity edges.
--
-- claim_edges is a provenance-preserving projection from committed claims.
-- The only writer in this change is the declarative frontmatter map compiler
-- in services::claims::link_map.

CREATE TABLE IF NOT EXISTS claim_edges (
    id              TEXT PRIMARY KEY,
    from_entity_id  TEXT NOT NULL,
    to_entity_id    TEXT NOT NULL,
    edge_type       TEXT NOT NULL,
    origin_claim_id TEXT NOT NULL REFERENCES intelligence_claims(id),
    link_source     TEXT NOT NULL CHECK (link_source IN ('frontmatter_map', 'manual', 'extracted')),
    weight          REAL NOT NULL DEFAULT 1.0,
    confidence      REAL NOT NULL DEFAULT 1.0,
    superseded_by   TEXT,
    tombstoned_at   TEXT,
    created_at      TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_claim_edges_active_unique
    ON claim_edges(from_entity_id, to_entity_id, edge_type, origin_claim_id)
    WHERE superseded_by IS NULL AND tombstoned_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_claim_edges_from_active
    ON claim_edges(from_entity_id, edge_type)
    WHERE superseded_by IS NULL AND tombstoned_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_claim_edges_to_active
    ON claim_edges(to_entity_id, edge_type)
    WHERE superseded_by IS NULL AND tombstoned_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_claim_edges_origin_claim
    ON claim_edges(origin_claim_id);

CREATE VIEW IF NOT EXISTS claim_edges_active AS
    SELECT
        id,
        from_entity_id,
        to_entity_id,
        edge_type,
        origin_claim_id,
        link_source,
        weight,
        confidence,
        superseded_by,
        tombstoned_at,
        created_at
    FROM claim_edges
    WHERE superseded_by IS NULL
      AND tombstoned_at IS NULL;

CREATE VIEW IF NOT EXISTS backlinks AS
    SELECT
        to_entity_id AS entity_id,
        from_entity_id AS backlink_entity_id,
        edge_type,
        origin_claim_id,
        link_source,
        weight,
        confidence,
        created_at
    FROM claim_edges_active;
