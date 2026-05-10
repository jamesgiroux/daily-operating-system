-- Isolate trust-compiler shadow scores from live trust columns.
--
-- The shadow compiler used version 1401003 while writing the live trust_score
-- columns. Preserve those historical shadow results in dedicated columns, then
-- clear the live columns for that shadow-only version so live readers no longer
-- consume shadow output as production trust data.

ALTER TABLE intelligence_claims ADD COLUMN shadow_trust_score REAL;
ALTER TABLE intelligence_claims ADD COLUMN shadow_trust_computed_at TEXT;
ALTER TABLE intelligence_claims ADD COLUMN shadow_trust_version INTEGER;

UPDATE intelligence_claims
   SET shadow_trust_score = trust_score,
       shadow_trust_computed_at = trust_computed_at,
       shadow_trust_version = trust_version,
       trust_score = NULL,
       trust_computed_at = NULL,
       trust_version = NULL
 WHERE trust_version = 1401003
   AND shadow_trust_version IS NULL;

CREATE INDEX IF NOT EXISTS idx_claims_shadow_trust_version
    ON intelligence_claims(shadow_trust_version)
    WHERE shadow_trust_version IS NOT NULL;
