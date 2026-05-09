CREATE TABLE IF NOT EXISTS temporal_backfill_state (
    entity_id                    TEXT NOT NULL,
    ability_id                   TEXT NOT NULL,
    last_completed_week_start    TEXT NOT NULL,
    retention_cutoff             TEXT NOT NULL,
    updated_at                   TEXT NOT NULL,
    PRIMARY KEY (entity_id, ability_id)
);

CREATE INDEX IF NOT EXISTS idx_temporal_backfill_state_ability
    ON temporal_backfill_state(ability_id, updated_at DESC);
