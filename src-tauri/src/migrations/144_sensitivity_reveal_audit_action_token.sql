ALTER TABLE sensitivity_reveal_audit
    ADD COLUMN reveal_action_id TEXT NOT NULL DEFAULT '';

DROP INDEX IF EXISTS idx_sensitivity_reveal_audit_reveal_session;
DROP INDEX IF EXISTS idx_sensitivity_reveal_audit_audit_bucket;

CREATE UNIQUE INDEX IF NOT EXISTS idx_sensitivity_reveal_audit_action_token
    ON sensitivity_reveal_audit(claim_id, user_id, reveal_action_id)
    WHERE reveal_action_id != '';
