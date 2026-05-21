-- Meeting prep status dismissals.
--
-- Persists user-driven UserSuppressed / UserDismissed transitions for
-- meeting prep status. Distinct from claim-level dismissals
-- (claim_review_deferrals) and from prep generation queue state
-- (existing prep_invalidation queue substrate).
--
-- Lifecycle: a row exists while the user has actively suppressed or
-- dismissed prep status for a given meeting. `resolved_at` is set when
-- the user un-suppresses / un-dismisses (transition back to PrepNeeded)
-- so we keep the audit trail without re-suppressing on next compute.
--
-- ADR-0125 sensitivity classification: `dismissal_reason` is
-- user-authored free text and treated as user_authored_text payload
-- privacy. No PII normalization at write time; callers are responsible
-- for sensitivity classification before persisting.

CREATE TABLE IF NOT EXISTS meeting_prep_status_dismissals (
    id              TEXT PRIMARY KEY,
    meeting_id      TEXT NOT NULL,
    -- 'user_suppressed' (user actively turned prep off) vs
    -- 'user_dismissed' (user dismissed prep for this meeting only).
    dismissal_kind  TEXT NOT NULL
        CHECK (dismissal_kind IN ('user_suppressed', 'user_dismissed')),
    dismissal_reason TEXT,
    actor           TEXT NOT NULL,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,
    resolved_at     TEXT
);

CREATE INDEX IF NOT EXISTS idx_meeting_prep_status_dismissals_meeting
    ON meeting_prep_status_dismissals (meeting_id);

CREATE INDEX IF NOT EXISTS idx_meeting_prep_status_dismissals_active
    ON meeting_prep_status_dismissals (meeting_id, dismissal_kind)
    WHERE resolved_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_meeting_prep_status_dismissals_actor
    ON meeting_prep_status_dismissals (actor, updated_at);
