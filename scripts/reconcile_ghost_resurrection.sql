-- DOS-311: reconcile pass for ghost resurrection after the DOS-7 cutover.
--
-- WHEN THIS RUNS: at step 6 of DOS-7's 7-step migration sequence (pre-flight
-- log → bump epoch → drain → backfill → requeue → reconcile → resume), AFTER
-- DOS-7's 9-mechanism consolidation has populated `intelligence_claims`.
--
-- DEFINITION OF GHOST RESURRECTION: an active item present in any legacy
-- projection (`entity_intelligence` JSON columns, `intelligence.json` file
-- content, `accounts.company_overview`/`strategic_programs`/`notes`) that
-- matches a tombstoned claim by `(subject_ref, claim_type, field_path,
-- dedup_key OR item_hash)` AND has no newer `source_asof` than the
-- tombstone's `dismissed_at`.
--
-- ZERO findings = clean migration. Any findings = stale projection that
-- DOS-301's projection sweep (W3-D) or this script's `--repair` mode
-- consumes.
--
-- CROSS-ISSUE NOTE (DOS-311 W1 ship-time): this SQL references
-- `intelligence_claims` which DOES NOT YET EXIST in the W1 codebase.
-- DOS-7 (W3-C) ships the table; the SQL below is committed as a static
-- asset that DOS-7's migration script consumes via include_str! when the
-- table is in place. Until DOS-7 lands, this SQL is documentation /
-- contract only.
--
-- The view `legacy_projection_state` is also a DOS-7 deliverable: it
-- materializes a unified shape over `entity_intelligence` JSON columns,
-- `intelligence.json` file content, and account narrative columns. Its
-- columns must include `(subject_ref, claim_type, field_path, dedup_key,
-- sourced_at, projection_target)`.

WITH tombstoned_claims AS (
    SELECT
        subject_ref,
        claim_type,
        field_path,
        dedup_key,
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
    pi.projection_target,
    tc.dismissed_at,
    pi.sourced_at
FROM legacy_projection_state pi
JOIN tombstoned_claims tc
    ON pi.subject_ref = tc.subject_ref
   AND pi.claim_type  = tc.claim_type
   AND pi.field_path  = tc.field_path
   AND pi.dedup_key   = tc.dedup_key
WHERE pi.sourced_at IS NULL
   OR pi.sourced_at <= tc.dismissed_at;
-- Each row in this result set is a ghost-resurrection finding.
-- A clean migration produces zero rows.
