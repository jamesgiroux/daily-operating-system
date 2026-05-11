-- Tombstoned legacy commitment alias remediation.
--
-- If migration 156 cannot reconstruct a tombstoned legacy bridge row's typed
-- CommitmentClaim identity from immutable source/provenance data, it must not
-- derive an alias from mutable action fields. This table records those rows for
-- manual review and lets runtime commitment sync block resurrection until the
-- row is resolved or discarded.

CREATE TABLE IF NOT EXISTS action_commitment_alias_remediation (
    id                     TEXT PRIMARY KEY,
    legacy_bridge_id       TEXT NOT NULL,
    tombstoned_action_id   TEXT,
    entity_type            TEXT NOT NULL,
    entity_id              TEXT NOT NULL,
    source_commitment_id   TEXT,
    source_type            TEXT,
    source_id              TEXT,
    source_label           TEXT,
    observed_at            TEXT NOT NULL,
    reason                 TEXT NOT NULL,
    remediation_status     TEXT NOT NULL DEFAULT 'pending'
                                 CHECK (remediation_status IN ('pending', 'resolved', 'discarded')),
    created_at             TEXT NOT NULL DEFAULT (datetime('now')),
    resolved_at            TEXT,
    resolution_note        TEXT
);

CREATE INDEX IF NOT EXISTS idx_action_commitment_alias_remediation_pending_entity
    ON action_commitment_alias_remediation(remediation_status, entity_type, entity_id);

CREATE UNIQUE INDEX IF NOT EXISTS idx_action_commitment_alias_remediation_action
    ON action_commitment_alias_remediation(legacy_bridge_id, tombstoned_action_id)
    WHERE tombstoned_action_id IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_action_commitment_alias_remediation_bridge
    ON action_commitment_alias_remediation(legacy_bridge_id)
    WHERE tombstoned_action_id IS NULL;
