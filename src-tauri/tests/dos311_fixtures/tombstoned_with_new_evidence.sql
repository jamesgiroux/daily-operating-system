-- Tombstone fixture: newer evidence wins.
--
-- A tombstone exists. The legacy projection includes a NEWER version of
-- the item (`sourced_at > tombstone.dismissed_at`). This is legitimate —
-- newer evidence overrides the dismissal. Reconcile must report ZERO
-- findings (the WHERE clause `pi.sourced_at IS NULL OR pi.sourced_at <= tc.dismissed_at`
-- excludes when sourced_at > dismissed_at).

INSERT INTO intelligence_claims
  (claim_id, subject_ref, claim_type, field_path, dedup_key, item_hash,
   source_asof, created_at, claim_state, superseded_at)
VALUES
  ('claim-2', 'account:acme', 'risk', 'risks[]', 'dedup-arr-shift', 'hash-arr-shift',
   '2026-01-01T00:00:00Z', '2026-02-01T00:00:00Z', 'tombstoned', NULL);

-- New evidence in the legacy projection: sourced_at AFTER dismissed_at.
-- Reconcile WHERE clause excludes this row from findings.
INSERT INTO legacy_projection_state
  (subject_ref, claim_type, field_path, dedup_key, item_hash,
   sourced_at, projection_target)
VALUES
  ('account:acme', 'risk', 'risks[]', 'dedup-arr-shift', 'hash-arr-shift',
   '2026-03-01T00:00:00Z',  -- newer than tombstone.dismissed_at (2026-02-01)
   'entity_intelligence');
