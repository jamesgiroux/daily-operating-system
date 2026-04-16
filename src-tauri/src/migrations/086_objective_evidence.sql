-- DOS-14: Add evidence tracking and AI origin to account_objectives.
-- Evidence accumulates from AI enrichment when statedObjectives match user objectives.
-- ai_origin_id enables deduplication of suggestions.

ALTER TABLE account_objectives ADD COLUMN evidence_json TEXT;
ALTER TABLE account_objectives ADD COLUMN ai_origin_id TEXT;
