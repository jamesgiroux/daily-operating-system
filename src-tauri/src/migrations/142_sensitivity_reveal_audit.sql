CREATE TABLE IF NOT EXISTS sensitivity_reveal_audit (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    claim_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    revealed_at TEXT NOT NULL,
    FOREIGN KEY (claim_id) REFERENCES intelligence_claims(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_sensitivity_reveal_audit_claim
    ON sensitivity_reveal_audit(claim_id, revealed_at);

CREATE INDEX IF NOT EXISTS idx_sensitivity_reveal_audit_user
    ON sensitivity_reveal_audit(user_id, revealed_at);
