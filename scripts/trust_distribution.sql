-- Trust compiler shadow-mode divergence monitor.
--
-- Intended for the hourly comparison job. It reads only shadow trust columns,
-- never live trust_score, so live trust data cannot mask missing shadow output.

WITH claim_population AS (
    SELECT
        COUNT(*) AS total_claims
    FROM intelligence_claims
    WHERE claim_state = 'active'
),
scored_population AS (
    SELECT
        COUNT(*) AS scored_claims
    FROM intelligence_claims
    WHERE claim_state = 'active'
      AND shadow_trust_version = 1401003
      AND shadow_trust_score IS NOT NULL
),
scored AS (
    SELECT
        shadow_trust_score AS trust_score,
        CASE
            WHEN shadow_trust_score >= 0.75 THEN 'likely_current'
            WHEN shadow_trust_score >= 0.50 THEN 'use_with_caution'
            ELSE 'needs_verification'
        END AS trust_band
    FROM intelligence_claims
    WHERE claim_state = 'active'
      AND shadow_trust_version = 1401003
      AND shadow_trust_score IS NOT NULL
),
distribution AS (
    SELECT
        trust_band,
        COUNT(*) AS claim_count,
        ROUND(AVG(trust_score), 4) AS avg_score,
        ROUND(MIN(trust_score), 4) AS min_score,
        ROUND(MAX(trust_score), 4) AS max_score
    FROM scored
    GROUP BY trust_band
)
SELECT
    'skipped_tiny_db' AS monitor_status,
    claim_population.total_claims,
    scored_population.scored_claims,
    NULL AS trust_band,
    NULL AS claim_count,
    NULL AS scored_pct,
    NULL AS avg_score,
    NULL AS min_score,
    NULL AS max_score
FROM claim_population
CROSS JOIN scored_population
WHERE claim_population.total_claims < 50

UNION ALL

SELECT
    'no_shadow_scores' AS monitor_status,
    claim_population.total_claims,
    scored_population.scored_claims,
    NULL AS trust_band,
    NULL AS claim_count,
    NULL AS scored_pct,
    NULL AS avg_score,
    NULL AS min_score,
    NULL AS max_score
FROM claim_population
CROSS JOIN scored_population
WHERE claim_population.total_claims >= 50
  AND scored_population.scored_claims = 0

UNION ALL

SELECT
    'ok' AS monitor_status,
    claim_population.total_claims,
    scored_population.scored_claims,
    distribution.trust_band,
    distribution.claim_count,
    ROUND(
        100.0 * distribution.claim_count / NULLIF(scored_population.scored_claims, 0),
        2
    ) AS scored_pct,
    distribution.avg_score,
    distribution.min_score,
    distribution.max_score
FROM distribution
CROSS JOIN claim_population
CROSS JOIN scored_population
WHERE claim_population.total_claims >= 50
  AND scored_population.scored_claims > 0
ORDER BY monitor_status, trust_band;
