-- Acceptance criterion: Persist user relevance weight on entity_intelligence
ALTER TABLE entity_intelligence ADD COLUMN user_relevance_weight REAL DEFAULT 1.0;
