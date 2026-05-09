BEGIN;

DROP INDEX IF EXISTS idx_entity_engagement_curve_entity_week;

ALTER TABLE entity_engagement_curve RENAME TO entity_engagement_curve_old;

CREATE TABLE entity_engagement_curve (
    entity_type           TEXT NOT NULL DEFAULT 'account',
    entity_id             TEXT NOT NULL,
    week_start            TEXT NOT NULL,
    meetings_count        INTEGER NOT NULL DEFAULT 0 CHECK (meetings_count >= 0),
    emails_count          INTEGER NOT NULL DEFAULT 0 CHECK (emails_count >= 0),
    bidirectional_ratio   REAL NOT NULL DEFAULT 0.0
                           CHECK (bidirectional_ratio >= 0.0 AND bidirectional_ratio <= 1.0),
    source_refs_json      TEXT NOT NULL DEFAULT '[]',
    source_invalidated_at TIMESTAMP NULL,
    PRIMARY KEY (entity_type, entity_id, week_start)
);

INSERT INTO entity_engagement_curve (
    entity_type, entity_id, week_start, meetings_count, emails_count,
    bidirectional_ratio, source_refs_json, source_invalidated_at
)
SELECT
    'account',
    old.entity_id,
    old.week_start,
    old.meetings_count,
    old.emails_count,
    old.bidirectional_ratio,
    old.source_refs_json,
    old.source_invalidated_at
FROM entity_engagement_curve_old old;

DROP TABLE entity_engagement_curve_old;

CREATE INDEX IF NOT EXISTS idx_entity_engagement_curve_entity_week
    ON entity_engagement_curve(entity_type, entity_id, week_start DESC);

DROP INDEX IF EXISTS idx_person_role_progression_entity;
DROP INDEX IF EXISTS idx_person_role_progression_entity_started;

ALTER TABLE person_role_progression RENAME TO person_role_progression_old;

CREATE TABLE person_role_progression (
    entity_type           TEXT NOT NULL DEFAULT 'person',
    entity_id             TEXT NOT NULL,
    started_at            TEXT NOT NULL,
    ended_at              TEXT,
    title                 TEXT NOT NULL,
    org                   TEXT,
    seniority             TEXT,
    source_refs_json      TEXT NOT NULL DEFAULT '[]',
    source_invalidated_at TIMESTAMP NULL,
    PRIMARY KEY (entity_type, entity_id, started_at)
);

INSERT INTO person_role_progression (
    entity_type, entity_id, started_at, ended_at, title, org, seniority,
    source_refs_json, source_invalidated_at
)
SELECT
    'person',
    old.entity_id,
    old.started_at,
    old.ended_at,
    old.title,
    old.org,
    old.seniority,
    old.source_refs_json,
    old.source_invalidated_at
FROM person_role_progression_old old;

DROP TABLE person_role_progression_old;

CREATE INDEX IF NOT EXISTS idx_person_role_progression_entity
    ON person_role_progression(entity_type, entity_id);

CREATE INDEX IF NOT EXISTS idx_person_role_progression_entity_started
    ON person_role_progression(entity_type, entity_id, started_at DESC);

DROP INDEX IF EXISTS idx_temporal_backfill_state_ability;

ALTER TABLE temporal_backfill_state RENAME TO temporal_backfill_state_old;

CREATE TABLE temporal_backfill_state (
    entity_type                  TEXT NOT NULL DEFAULT 'account',
    entity_id                    TEXT NOT NULL,
    ability_id                   TEXT NOT NULL,
    last_completed_week_start    TEXT NOT NULL,
    retention_cutoff             TEXT NOT NULL,
    updated_at                   TEXT NOT NULL,
    PRIMARY KEY (entity_type, entity_id, ability_id)
);

INSERT INTO temporal_backfill_state (
    entity_type, entity_id, ability_id, last_completed_week_start,
    retention_cutoff, updated_at
)
SELECT
    'account',
    old.entity_id,
    old.ability_id,
    old.last_completed_week_start,
    old.retention_cutoff,
    old.updated_at
FROM temporal_backfill_state_old old;

DROP TABLE temporal_backfill_state_old;

CREATE INDEX IF NOT EXISTS idx_temporal_backfill_state_ability
    ON temporal_backfill_state(ability_id, entity_type, updated_at DESC);

COMMIT;
