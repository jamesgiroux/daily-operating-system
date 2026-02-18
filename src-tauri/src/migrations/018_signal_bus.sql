-- I306: Signal Bus Foundation (ADR-0080 Phase 2)
--
-- Universal signal event log where every data source produces typed,
-- weighted, time-decaying signals. Signals are fused using weighted
-- log-odds Bayesian combination.

CREATE TABLE signal_events (
    id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    signal_type TEXT NOT NULL,
    source TEXT NOT NULL,
    value TEXT,
    confidence REAL DEFAULT 1.0,
    decay_half_life_days INTEGER DEFAULT 90,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    superseded_by TEXT
);

CREATE INDEX idx_signal_events_entity ON signal_events(entity_type, entity_id, created_at DESC);
CREATE INDEX idx_signal_events_source ON signal_events(source, signal_type);

CREATE TABLE signal_weights (
    source TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    signal_type TEXT NOT NULL,
    alpha REAL DEFAULT 1.0,
    beta REAL DEFAULT 1.0,
    update_count INTEGER DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (source, entity_type, signal_type)
);
