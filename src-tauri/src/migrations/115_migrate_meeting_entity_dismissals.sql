-- backfill meeting_entity_dismissals → linking_dismissals.
--
-- Migrates all existing meeting-level entity dismissals into the new
-- cross-surface linking_dismissals table so that the new service can
-- immediately honor user-set dismissals without a data gap.
--
-- meeting_entity_dismissals is intentionally NOT dropped here. It stays
-- until one week post-cutover (confirmed no regressions) so the legacy
-- calendar/email paths can still read it during the overlap window. The
-- drop migration is a separate numbered migration (N+1) filed as a
-- follow-up task in.
--
-- The INSERT OR IGNORE guard makes this backfill idempotent: if the
-- migration is somehow re-applied (e.g., a DB restored from a backup and
-- re-migrated), existing linking_dismissals rows are preserved without
-- duplication or error.

INSERT OR IGNORE INTO linking_dismissals (
    owner_type,
    owner_id,
    entity_id,
    entity_type,
    dismissed_by,
    created_at
)
SELECT
    'meeting',
    meeting_id,
    entity_id,
    entity_type,
    dismissed_by,
    dismissed_at
FROM meeting_entity_dismissals;
