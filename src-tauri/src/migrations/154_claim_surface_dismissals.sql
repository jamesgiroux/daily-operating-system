CREATE TABLE IF NOT EXISTS claim_surface_dismissals (
    claim_id     TEXT NOT NULL REFERENCES intelligence_claims(id) ON DELETE CASCADE,
    surface      TEXT NOT NULL,
    feedback_id  TEXT REFERENCES claim_feedback(id),
    actor        TEXT NOT NULL,
    dismissed_at TEXT NOT NULL,
    PRIMARY KEY (claim_id, surface)
);

CREATE INDEX IF NOT EXISTS idx_claim_surface_dismissals_surface
    ON claim_surface_dismissals(surface, claim_id);
