ALTER TABLE sensitivity_reveal_audit
    ADD COLUMN audit_bucket TEXT NOT NULL DEFAULT '';

CREATE UNIQUE INDEX IF NOT EXISTS idx_sensitivity_reveal_audit_audit_bucket
    ON sensitivity_reveal_audit(claim_id, user_id, audit_bucket)
    WHERE audit_bucket != '';
