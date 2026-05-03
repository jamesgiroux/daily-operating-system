-- Reconcile pass for ghost resurrection after the claims cutover.
--
-- WHEN THIS RUNS: at step 6 of the claims migration sequence (pre-flight
-- log → bump epoch → drain → backfill → requeue → reconcile → resume), AFTER
-- the claim substrate has populated `intelligence_claims`.
--
-- DEFINITION OF GHOST RESURRECTION: an active item present in any legacy
-- projection (`entity_intelligence` JSON columns, `intelligence.json` file
-- content, `accounts.company_overview`/`strategic_programs`/`notes`) that
-- matches a tombstoned claim by `(subject_ref, claim_type, field_path,
-- dedup_key OR item_hash)` AND has no newer `source_asof` than the
-- tombstone's `dismissed_at`.
--
-- ZERO findings = clean migration. Any findings = stale projection that
-- the projection sweep or this script's `--repair` mode consumes.
--
-- CROSS-MIGRATION NOTE: this SQL references `intelligence_claims`, so it is
-- a static contract until the claims schema exists. The claims migration script
-- consumes this SQL via include_str! when the table is in place.
--
-- The view `legacy_projection_state` is also part of the claims cutover: it
-- materializes a unified shape over `entity_intelligence` JSON columns,
-- `intelligence.json` file content, and account narrative columns. Its
-- columns must include `(subject_ref, claim_type, field_path, dedup_key,
-- sourced_at, projection_target)`.

-- Match-key contract: a projected item resurrects a tombstoned claim when
-- ANY of the following hold (inclusive OR), in addition to the
-- (subject_ref, claim_type, field_path) triple agreeing:
--   1. dedup_key match (canonicalized item key)
--   2. item_hash match (content fingerprint; survives dedup_key changes)
--
-- The item_hash fallback catches items whose dedup_key shifted post-tombstone
-- (e.g., a re-enrichment regenerated the canonical key) but whose content
-- fingerprint still matches the dismissed item. The reconcile contract calls
-- this out explicitly: "(dedup_key OR item_hash)".

WITH tombstoned_claims AS (
    SELECT
        subject_ref,
        claim_type,
        field_path,
        dedup_key,
        item_hash,
        source_asof,
        created_at AS dismissed_at
    FROM intelligence_claims
    WHERE claim_state = 'tombstoned'
      AND superseded_at IS NULL
)
SELECT
    pi.subject_ref,
    pi.claim_type,
    pi.field_path,
    pi.dedup_key,
    pi.item_hash,
    pi.projection_target,
    tc.dismissed_at,
    pi.sourced_at,
    -- Indicate which match path fired so operators can spot data drift.
    CASE
        WHEN pi.dedup_key IS NOT NULL
             AND tc.dedup_key IS NOT NULL
             AND pi.dedup_key = tc.dedup_key THEN 'dedup_key'
        WHEN pi.item_hash IS NOT NULL
             AND tc.item_hash IS NOT NULL
             AND pi.item_hash = tc.item_hash THEN 'item_hash'
        ELSE 'unknown'
    END AS match_path
FROM legacy_projection_state pi
JOIN tombstoned_claims tc
    ON pi.subject_ref = tc.subject_ref
   AND pi.claim_type  = tc.claim_type
   AND pi.field_path  = tc.field_path
   AND (
        (pi.dedup_key IS NOT NULL AND tc.dedup_key IS NOT NULL AND pi.dedup_key = tc.dedup_key)
        OR
        (pi.item_hash IS NOT NULL AND tc.item_hash IS NOT NULL AND pi.item_hash = tc.item_hash)
   )
WHERE pi.sourced_at IS NULL
   OR pi.sourced_at <= tc.dismissed_at;
-- Each row in this result set is a ghost-resurrection finding.
-- A clean migration produces zero rows.
