CREATE TABLE IF NOT EXISTS claim_file_projection_run_claims (
    run_id TEXT NOT NULL REFERENCES claim_file_projection_runs(id) ON DELETE CASCADE,
    claim_id TEXT NOT NULL REFERENCES intelligence_claims(id) ON DELETE CASCADE,
    claim_version INTEGER NOT NULL,
    semantic_identity_json TEXT NOT NULL,
    trust_band TEXT NOT NULL,
    sensitivity TEXT NOT NULL,
    PRIMARY KEY (run_id, claim_id)
);

CREATE INDEX IF NOT EXISTS idx_claim_file_projection_run_claims_claim
    ON claim_file_projection_run_claims(claim_id, claim_version);

CREATE INDEX IF NOT EXISTS idx_claim_file_projection_run_claims_run_trust
    ON claim_file_projection_run_claims(run_id, trust_band, sensitivity);
