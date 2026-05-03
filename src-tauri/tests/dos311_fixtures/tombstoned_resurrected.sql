-- Tombstone fixture: resurrected item.
--
-- The bug case. A tombstone exists. The legacy projection still includes
-- the OLD item (`sourced_at <= tombstone.dismissed_at`, or `sourced_at IS NULL`).
-- Reconcile MUST report exactly 1 finding. The `--repair` mode
-- consumes that finding and re-applies the tombstone via commit_claim.
--
-- This fixture also exercises the reconcile match contract: `(dedup_key OR item_hash)`.
-- match: the second tombstone's dedup_key has shifted post-creation but
-- item_hash still matches the projection — the reconcile must catch it
-- via the item_hash fallback.

-- Case 1: dedup_key matches exactly.
INSERT INTO intelligence_claims
  (claim_id, subject_ref, claim_type, field_path, dedup_key, item_hash,
   source_asof, created_at, claim_state, superseded_at)
VALUES
  ('claim-3', 'account:acme', 'risk', 'risks[]', 'dedup-stale-renewal', 'hash-stale-renewal',
   '2026-01-01T00:00:00Z', '2026-02-01T00:00:00Z', 'tombstoned', NULL);

-- Stale projection: same dedup_key, sourced_at BEFORE dismissed_at.
INSERT INTO legacy_projection_state
  (subject_ref, claim_type, field_path, dedup_key, item_hash,
   sourced_at, projection_target)
VALUES
  ('account:acme', 'risk', 'risks[]', 'dedup-stale-renewal', 'hash-stale-renewal',
   '2026-01-15T00:00:00Z',  -- before tombstone.dismissed_at (2026-02-01) → resurrection
   'entity_intelligence');

-- Case 2: dedup_key shifted; only item_hash still matches.
INSERT INTO intelligence_claims
  (claim_id, subject_ref, claim_type, field_path, dedup_key, item_hash,
   source_asof, created_at, claim_state, superseded_at)
VALUES
  ('claim-4', 'account:acme', 'risk', 'risks[]', 'dedup-original-key', 'hash-shared-content',
   '2026-01-01T00:00:00Z', '2026-02-01T00:00:00Z', 'tombstoned', NULL);

INSERT INTO legacy_projection_state
  (subject_ref, claim_type, field_path, dedup_key, item_hash,
   sourced_at, projection_target)
VALUES
  ('account:acme', 'risk', 'risks[]', 'dedup-shifted-after-reenrich', 'hash-shared-content',
   NULL,  -- sourced_at NULL → matches reconcile WHERE clause
   'entity_intelligence');
