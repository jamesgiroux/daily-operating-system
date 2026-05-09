CREATE TABLE IF NOT EXISTS entity_engagement_curve (
    entity_id             TEXT NOT NULL,
    week_start            TEXT NOT NULL,
    meetings_count        INTEGER NOT NULL DEFAULT 0 CHECK (meetings_count >= 0),
    emails_count          INTEGER NOT NULL DEFAULT 0 CHECK (emails_count >= 0),
    bidirectional_ratio   REAL NOT NULL DEFAULT 0.0
                           CHECK (bidirectional_ratio >= 0.0 AND bidirectional_ratio <= 1.0),
    source_refs_json      TEXT NOT NULL DEFAULT '[]',
    PRIMARY KEY (entity_id, week_start)
);

CREATE INDEX IF NOT EXISTS idx_entity_engagement_curve_entity_week
    ON entity_engagement_curve(entity_id, week_start DESC);

CREATE TABLE IF NOT EXISTS person_role_progression (
    entity_id             TEXT NOT NULL,
    started_at            TEXT NOT NULL,
    ended_at              TEXT,
    title                 TEXT NOT NULL,
    org                   TEXT,
    seniority             TEXT,
    source_refs_json      TEXT NOT NULL DEFAULT '[]',
    PRIMARY KEY (entity_id, started_at)
);

CREATE INDEX IF NOT EXISTS idx_person_role_progression_entity
    ON person_role_progression(entity_id);

CREATE INDEX IF NOT EXISTS idx_person_role_progression_entity_started
    ON person_role_progression(entity_id, started_at DESC);
