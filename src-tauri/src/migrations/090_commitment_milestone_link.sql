-- Link captured commitments to milestones and suggested objectives.
ALTER TABLE captured_commitments ADD COLUMN milestone_id TEXT;
ALTER TABLE captured_commitments ADD COLUMN suggested_objective_id TEXT;
