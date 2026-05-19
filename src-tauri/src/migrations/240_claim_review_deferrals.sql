-- Deferral preferences for review-queue targets. Queue membership itself stays
-- computed from claim/proposal lifecycle state; only user-driven defer/snooze
-- decisions persist here.

CREATE TABLE IF NOT EXISTS claim_review_deferrals (
    id            TEXT PRIMARY KEY,
    target_kind   TEXT NOT NULL CHECK (target_kind IN ('claim', 'proposal', 'candidate')),
    target_id     TEXT NOT NULL,
    surface       TEXT,
    reason        TEXT,
    snoozed_until TEXT,
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL,
    resolved_at   TEXT,
    actor         TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_claim_review_deferrals_target
    ON claim_review_deferrals (target_kind, target_id);

CREATE INDEX IF NOT EXISTS idx_claim_review_deferrals_active
    ON claim_review_deferrals (snoozed_until)
    WHERE resolved_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_claim_review_deferrals_actor
    ON claim_review_deferrals (actor, updated_at);
