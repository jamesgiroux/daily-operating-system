-- DOS-258 Lane A: linking_dismissals table.
--
-- Replaces the meeting-surface-only meeting_entity_dismissals with a
-- cross-surface dismissal store keyed on (owner_type, owner_id, entity_id,
-- entity_type). Existing meeting_entity_dismissals rows are backfilled into
-- this table by migration 115 (migrate_meeting_entity_dismissals).
-- meeting_entity_dismissals is dropped one week post-cutover in a separate
-- migration (N+1), not here, to preserve the rollback window.
--
-- The write path (manual_dismiss) writes a row here AND sets
-- source='user_dismissed' on the linked_entities_raw row in the same
-- transaction. Any in-flight recompute that reads no dismissal row before
-- the user dismisses and then tries to INSERT will fail the UNIQUE constraint
-- on linked_entities_raw + see the dismissal row on retry, making
-- dismissal-wins-race the guaranteed outcome.

CREATE TABLE IF NOT EXISTS linking_dismissals (
    owner_type   TEXT NOT NULL,
    owner_id     TEXT NOT NULL,
    entity_id    TEXT NOT NULL,
    entity_type  TEXT NOT NULL,
    dismissed_by TEXT,
    created_at   TEXT NOT NULL,
    PRIMARY KEY (owner_type, owner_id, entity_id, entity_type)
);

-- Owner lookup used at the start of every evaluate() transaction.
CREATE INDEX IF NOT EXISTS idx_linking_dismissals_owner
    ON linking_dismissals (owner_type, owner_id);
