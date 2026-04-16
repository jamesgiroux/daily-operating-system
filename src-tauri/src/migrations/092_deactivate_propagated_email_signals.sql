-- DOS-156: Deactivate propagated email signals (person→account fan-out noise).
-- These signals have person_id set, meaning they were propagated from a person
-- entity to an account. Production data showed 14.6x fan-out (117 emails → 1,713 rows).
UPDATE email_signals
SET deactivated_at = datetime('now')
WHERE person_id IS NOT NULL
  AND entity_type = 'account'
  AND deactivated_at IS NULL;
