BEGIN;

DROP INDEX IF EXISTS idx_signal_events_source;

ALTER TABLE signal_events RENAME COLUMN source TO data_source;

CREATE INDEX IF NOT EXISTS idx_signal_events_data_source
    ON signal_events(data_source, signal_type);

COMMIT;
