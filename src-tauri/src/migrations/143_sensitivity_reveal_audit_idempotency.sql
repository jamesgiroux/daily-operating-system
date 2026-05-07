ALTER TABLE sensitivity_reveal_audit
    ADD COLUMN reveal_session_id TEXT;

CREATE UNIQUE INDEX IF NOT EXISTS idx_sensitivity_reveal_audit_reveal_session
    ON sensitivity_reveal_audit(claim_id, user_id, reveal_session_id)
    WHERE reveal_session_id IS NOT NULL;
