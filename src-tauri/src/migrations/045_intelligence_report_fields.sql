-- Intelligence report fields for CS health tracking
ALTER TABLE entity_intelligence ADD COLUMN health_score REAL;
ALTER TABLE entity_intelligence ADD COLUMN health_trend TEXT;
ALTER TABLE entity_intelligence ADD COLUMN value_delivered TEXT;
ALTER TABLE entity_intelligence ADD COLUMN success_metrics TEXT;
ALTER TABLE entity_intelligence ADD COLUMN open_commitments TEXT;
ALTER TABLE entity_intelligence ADD COLUMN relationship_depth TEXT;
