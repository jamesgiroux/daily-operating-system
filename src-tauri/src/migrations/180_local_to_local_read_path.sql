-- Local-to-local read path: data-only migration.
--
-- v169 already defines absolute_expires_at on surface_client_sessions
-- (migrations/169_surface_client_pairings.sql:78). This migration does
-- NOT add the column. v180 is data-only:
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
-- 3. v179 rollback note: rows whose inactive_expires_at was already past
--    at v180 apply will be rejected by v179 if the binary is rolled back.
--    Mitigation: re-pair. Documented as known v179 rollback footgun;
--    v180 is forward-safe.
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

-- The BEFORE INSERT trigger that previously guarded against past
-- `absolute_expires_at` on insert was removed during integration review.
-- CREATE TRIGGER is a schema-change DDL operation and v180 is committed
-- to data-only. The pairing flow at services/surface_pairing.rs writes
-- only future timestamps, and the exhaustive-match enforcement on
-- SignedSessionFailure prevents future variants from silently inserting
-- past values. If insert-time validation becomes load-bearing, file a
-- separate vNNN schema migration.
