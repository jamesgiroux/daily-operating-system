-- I585: Persist Value Delivered
-- Adds a separate column for the user-owned, merged value_delivered array.
-- Unlike the existing value_delivered column (AI output, overwritten on re-enrichment),
-- value_delivered_json preserves user-confirmed items across enrichment cycles.
ALTER TABLE entity_assessment ADD COLUMN value_delivered_json TEXT;
