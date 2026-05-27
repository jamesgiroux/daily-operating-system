CREATE TABLE IF NOT EXISTS salience_factors_weights (
    factor_kind TEXT PRIMARY KEY CHECK (
        factor_kind IN (
            'importance',
            'novelty',
            'urgency',
            'timing',
            'userFit',
            'freshness',
            'trust',
            'corroboration',
            'contradiction',
            'openLoopRelevance'
        )
    ),
    default_weight REAL NOT NULL CHECK (default_weight >= 0.0 AND default_weight <= 1.0),
    schema_version INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE IF NOT EXISTS salience_factors (
    id TEXT PRIMARY KEY,
    evaluation_id TEXT NOT NULL,
    claim_id TEXT NOT NULL REFERENCES intelligence_claims(id) ON DELETE CASCADE,
    factor_kind TEXT NOT NULL CHECK (
        factor_kind IN (
            'importance',
            'novelty',
            'urgency',
            'timing',
            'userFit',
            'freshness',
            'trust',
            'corroboration',
            'contradiction',
            'openLoopRelevance'
        )
    ),
    factor_value REAL CHECK (
        factor_value IS NULL
        OR (factor_value >= 0.0 AND factor_value <= 1.0)
    ),
    weight REAL NOT NULL CHECK (weight >= 0.0 AND weight <= 1.0),
    rationale_json TEXT NOT NULL CHECK (json_valid(rationale_json) = 1),
    schema_version INTEGER NOT NULL DEFAULT 1,
    computed_at TEXT NOT NULL,
    UNIQUE (evaluation_id, claim_id, factor_kind)
);

INSERT INTO salience_factors_weights (factor_kind, default_weight, schema_version)
VALUES
    ('importance', 0.20, 1),
    ('novelty', 0.10, 1),
    ('urgency', 0.15, 1),
    ('timing', 0.10, 1),
    ('userFit', 0.10, 1),
    ('freshness', 0.10, 1),
    ('trust', 0.10, 1),
    ('corroboration', 0.05, 1),
    ('contradiction', 0.05, 1),
    ('openLoopRelevance', 0.05, 1)
ON CONFLICT(factor_kind) DO UPDATE SET
    default_weight = excluded.default_weight,
    schema_version = excluded.schema_version,
    updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now');

CREATE INDEX IF NOT EXISTS idx_salience_factors_claim_computed
    ON salience_factors(claim_id, computed_at DESC);

CREATE INDEX IF NOT EXISTS idx_salience_factors_evaluation
    ON salience_factors(evaluation_id, claim_id);

CREATE INDEX IF NOT EXISTS idx_salience_factors_kind
    ON salience_factors(factor_kind, computed_at DESC);
