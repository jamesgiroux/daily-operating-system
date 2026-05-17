-- DOS-655 W4-F V3.2: local-to-local read path — data-only migration.
--
-- v169 already defines absolute_expires_at on surface_client_sessions
-- (migrations/169_dos_559_surface_client_pairings.sql:78). This migration
-- does NOT add the column. Per W4-F packet §5 + V3.1 changelog, v180 is
-- data-only:
--
-- 1. For sessions whose absolute_expires_at is in the past at migration
--    apply time (would be rejected by post-v180 validity check), repair
--    the column to COALESCE(issued_at, datetime('now')) + 365 days. This
--    avoids forcing re-pair of legitimate active sessions on the v180
--    rollout.
--
-- 2. inactive_expires_at column is LEFT UNCHANGED (forensic preservation).
--    v169 declares the column NOT NULL; we cannot backfill to NULL. v180
--    stops consulting the column for validity but retains its values so
--    incident response can correlate against pre-v180 row history.
--
-- 3. v179 rollback note (W4-F V3.1 §6.8b): rows whose inactive_expires_at
--    was already past at v180 apply will be rejected by v179 if the binary
--    is rolled back. Mitigation: re-pair. Documented as known v179 rollback
--    footgun; v180 is forward-safe.
--
-- DEPRECATED v180: inactive_expires_at is no longer consulted for session
-- validity. Retained for forensics. The authoritative validity column is
-- absolute_expires_at.

UPDATE surface_client_sessions
   SET absolute_expires_at = strftime(
           '%Y-%m-%dT%H:%M:%fZ',
           COALESCE(datetime(issued_at), datetime('now')),
           '+365 days'
       )
 WHERE datetime(absolute_expires_at) <= datetime('now');

-- Sanity assertion: no surface_client_sessions row has absolute_expires_at
-- in the past after the repair. Trigger ensures future inserts whose
-- absolute_expires_at is past are rejected (defensive; pairing flow at
-- services/surface_pairing.rs ensures correct values, but a stray write
-- would silently break validity).
--
-- NOTE: triggers create a write-time check; this preserves the forward
-- guarantee that absolute_expires_at is the authoritative validity column.
DROP TRIGGER IF EXISTS dos655_v180_assert_absolute_expires_at_future;
CREATE TRIGGER dos655_v180_assert_absolute_expires_at_future
BEFORE INSERT ON surface_client_sessions
FOR EACH ROW
WHEN datetime(NEW.absolute_expires_at) <= datetime('now')
BEGIN
    SELECT RAISE(ABORT, 'dos655_v180: surface_client_sessions.absolute_expires_at must be in the future at insert');
END;
