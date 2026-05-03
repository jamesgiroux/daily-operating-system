DROP INDEX IF EXISTS idx_claim_projection_status_failed;

CREATE INDEX IF NOT EXISTS idx_claim_projection_status_failed_v2
    ON claim_projection_status(projection_target, attempted_at)
    WHERE status = 'failed';
