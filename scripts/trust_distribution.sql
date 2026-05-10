-- Trust compiler shadow-mode divergence monitor.
--
-- Intended for the hourly comparison job. It emits one summary row when the
-- database is too small to judge distribution drift, and otherwise returns the
-- current trust-score population by band plus basic spread metrics.

WITH claim_population AS (
    SELECT
        COUNT(*) AS total_claims,
        SUM(CASE WHEN trust_score IS NOT NULL THEN 1 ELSE 0 END) AS scored_claims
    FROM intelligence_claims
    WHERE claim_state = 'active'
),
scored AS (
    SELECT
        trust_score,
        CASE
            WHEN trust_score >= 0.75 THEN 'likely_current'
            WHEN trust_score >= 0.50 THEN 'use_with_caution'
            ELSE 'needs_verification'
        END AS trust_band
    FROM intelligence_claims
    WHERE claim_state = 'active'
      AND trust_score IS NOT NULL
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
    total_claims,
    scored_claims,
    NULL AS trust_band,
    NULL AS claim_count,
    NULL AS scored_pct,
    NULL AS avg_score,
    NULL AS min_score,
    NULL AS max_score
FROM claim_population
WHERE total_claims < 50

UNION ALL

SELECT
    'ok' AS monitor_status,
    claim_population.total_claims,
    claim_population.scored_claims,
    distribution.trust_band,
    distribution.claim_count,
    ROUND(
        100.0 * distribution.claim_count / NULLIF(claim_population.scored_claims, 0),
        2
    ) AS scored_pct,
    distribution.avg_score,
    distribution.min_score,
    distribution.max_score
FROM distribution
CROSS JOIN claim_population
WHERE claim_population.total_claims >= 50
ORDER BY monitor_status, trust_band;
