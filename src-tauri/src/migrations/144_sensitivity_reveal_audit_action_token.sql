PRAGMA foreign_keys = OFF;

DROP TABLE IF EXISTS sensitivity_reveal_audit_new;

DROP INDEX IF EXISTS idx_sensitivity_reveal_audit_reveal_session;
DROP INDEX IF EXISTS idx_sensitivity_reveal_audit_audit_bucket;

CREATE TABLE sensitivity_reveal_audit_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    claim_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    revealed_at TEXT NOT NULL,
    reveal_action_id TEXT NOT NULL DEFAULT '',
    FOREIGN KEY (claim_id) REFERENCES intelligence_claims(id) ON DELETE CASCADE
);

INSERT INTO sensitivity_reveal_audit_new (id, claim_id, user_id, revealed_at, reveal_action_id)
SELECT id, claim_id, user_id, revealed_at, ''
FROM sensitivity_reveal_audit;

DROP TABLE sensitivity_reveal_audit;

ALTER TABLE sensitivity_reveal_audit_new RENAME TO sensitivity_reveal_audit;

CREATE INDEX IF NOT EXISTS idx_sensitivity_reveal_audit_claim
    ON sensitivity_reveal_audit(claim_id, revealed_at);

CREATE INDEX IF NOT EXISTS idx_sensitivity_reveal_audit_user
    ON sensitivity_reveal_audit(user_id, revealed_at);

CREATE UNIQUE INDEX IF NOT EXISTS idx_sensitivity_reveal_audit_action_token
    ON sensitivity_reveal_audit(claim_id, user_id, reveal_action_id)
    WHERE reveal_action_id != '';

PRAGMA foreign_keys = ON;
