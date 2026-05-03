-- Tombstone fixture: correctly hidden item.
--
-- A tombstone exists. The legacy projection does NOT include the item
-- (it was correctly hidden when the user dismissed). Reconcile must
-- report ZERO findings.

INSERT INTO intelligence_claims
  (claim_id, subject_ref, claim_type, field_path, dedup_key, item_hash,
   source_asof, created_at, claim_state, superseded_at)
VALUES
  ('claim-1', 'account:acme', 'risk', 'risks[]', 'dedup-renewal-1', 'hash-renewal-1',
   '2026-01-01T00:00:00Z', '2026-02-01T00:00:00Z', 'tombstoned', NULL);

-- legacy_projection_state has no row for this dedup_key — correctly hidden.
-- (Empty insert to be explicit; tests assert absence.)
