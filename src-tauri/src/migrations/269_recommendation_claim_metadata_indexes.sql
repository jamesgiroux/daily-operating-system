CREATE INDEX IF NOT EXISTS idx_claims_recommendation_action_kind
    ON intelligence_claims (
        json_extract(metadata_json, '$.recommendation.recommendedAction.kind')
    )
    WHERE claim_type = 'recommendation'
      AND metadata_json IS NOT NULL
      AND json_valid(metadata_json) = 1;

CREATE INDEX IF NOT EXISTS idx_claims_recommendation_feedback_state
    ON intelligence_claims (
        COALESCE(
            json_extract(metadata_json, '$.recommendation.feedbackState.decided.kind'),
            json_extract(metadata_json, '$.recommendation.feedbackState')
        )
    )
    WHERE claim_type = 'recommendation'
      AND metadata_json IS NOT NULL
      AND json_valid(metadata_json) = 1;

CREATE INDEX IF NOT EXISTS idx_claims_recommendation_conversion_state
    ON intelligence_claims (
        json_extract(metadata_json, '$.recommendation.conversionState.kind')
    )
    WHERE claim_type = 'recommendation'
      AND metadata_json IS NOT NULL
      AND json_valid(metadata_json) = 1;
