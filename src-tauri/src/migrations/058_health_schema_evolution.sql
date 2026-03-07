-- 058: Structured health schema evolution (I503 / ADR-0097)
-- Adds structured health JSON columns to entity_assessment and backfills from
-- legacy scalar mirrors in entity_quality.

BEGIN IMMEDIATE;

DROP TABLE IF EXISTS entity_assessment_new;
CREATE TABLE entity_assessment_new (
    entity_id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL DEFAULT 'account',
    enriched_at TEXT,
    source_file_count INTEGER DEFAULT 0,
    executive_assessment TEXT,
    risks_json TEXT,
    recent_wins_json TEXT,
    current_state_json TEXT,
    stakeholder_insights_json TEXT,
    next_meeting_readiness_json TEXT,
    company_context_json TEXT,
    value_delivered TEXT,
    success_metrics TEXT,
    open_commitments TEXT,
    relationship_depth TEXT,
    health_json TEXT,
    org_health_json TEXT,
    user_relevance_weight REAL DEFAULT 1.0,
    consistency_status TEXT,
    consistency_findings_json TEXT,
    consistency_checked_at TEXT,
    portfolio_json TEXT,
    network_json TEXT,
    user_edits_json TEXT,
    source_manifest_json TEXT
);

INSERT OR REPLACE INTO entity_assessment_new (
    entity_id, entity_type, enriched_at, source_file_count,
    executive_assessment, risks_json, recent_wins_json,
    current_state_json, stakeholder_insights_json,
    next_meeting_readiness_json, company_context_json,
    value_delivered, success_metrics, open_commitments, relationship_depth,
    health_json, org_health_json, user_relevance_weight, consistency_status,
    consistency_findings_json, consistency_checked_at, portfolio_json,
    network_json, user_edits_json, source_manifest_json
)
SELECT
    ea.entity_id,
    ea.entity_type,
    ea.enriched_at,
    ea.source_file_count,
    ea.executive_assessment,
    ea.risks_json,
    ea.recent_wins_json,
    ea.current_state_json,
    ea.stakeholder_insights_json,
    ea.next_meeting_readiness_json,
    ea.company_context_json,
    ea.value_delivered,
    ea.success_metrics,
    ea.open_commitments,
    ea.relationship_depth,
    CASE
        WHEN eq.health_score IS NULL THEN NULL
        ELSE json_object(
            'score', eq.health_score,
            'band', CASE
                WHEN eq.health_score >= 70 THEN 'green'
                WHEN eq.health_score >= 40 THEN 'yellow'
                ELSE 'red'
            END,
            'source', 'computed',
            'confidence', 0.3,
            'trend', json_object(
                'direction', CASE
                    WHEN json_valid(eq.health_trend) THEN COALESCE(json_extract(eq.health_trend, '$.direction'), 'stable')
                    ELSE 'stable'
                END,
                'rationale', CASE
                    WHEN json_valid(eq.health_trend) THEN json_extract(eq.health_trend, '$.rationale')
                    ELSE NULL
                END,
                'timeframe', '30d',
                'confidence', 0.3
            ),
            'dimensions', json_object(
                'meetingCadence', json_object('score', 0.0, 'weight', 0.0, 'evidence', json('[]'), 'trend', 'stable'),
                'emailEngagement', json_object('score', 0.0, 'weight', 0.0, 'evidence', json('[]'), 'trend', 'stable'),
                'stakeholderCoverage', json_object('score', 0.0, 'weight', 0.0, 'evidence', json('[]'), 'trend', 'stable'),
                'championHealth', json_object('score', 0.0, 'weight', 0.0, 'evidence', json('[]'), 'trend', 'stable'),
                'financialProximity', json_object('score', 0.0, 'weight', 0.0, 'evidence', json('[]'), 'trend', 'stable'),
                'signalMomentum', json_object('score', 0.0, 'weight', 0.0, 'evidence', json('[]'), 'trend', 'stable')
            ),
            'recommendedActions', json('[]')
        )
    END AS health_json,
    NULL AS org_health_json,
    ea.user_relevance_weight,
    ea.consistency_status,
    ea.consistency_findings_json,
    ea.consistency_checked_at,
    ea.portfolio_json,
    ea.network_json,
    ea.user_edits_json,
    ea.source_manifest_json
FROM entity_assessment ea
LEFT JOIN entity_quality eq ON eq.entity_id = ea.entity_id;

DROP TABLE IF EXISTS entity_assessment;
ALTER TABLE entity_assessment_new RENAME TO entity_assessment;
CREATE INDEX IF NOT EXISTS idx_entity_assessment_type ON entity_assessment(entity_type);

COMMIT;
