-- I505: Track when a stakeholder was last seen in Glean results.
-- Used for staleness detection (Phase 3c / I493).
ALTER TABLE account_stakeholders ADD COLUMN last_seen_in_glean TEXT;
