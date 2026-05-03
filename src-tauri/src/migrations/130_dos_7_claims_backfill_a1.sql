-- Claims backfill D3a-1: Backfill legacy dismissal mechanisms 1-4 into
-- intelligence_claims tombstone rows.
--
-- Scope:
--   1. suppression_tombstones
--   2. account_stakeholder_roles.dismissed_at
--   3. email_dismissals
--   4. meeting_entity_dismissals
--
-- Mechanisms 5-8 and the DismissedItem JSON-blob cutover are intentionally
-- left for D3a-2/D3b.

-- ---------------------------------------------------------------------------
-- Mechanism 1 — suppression_tombstones
-- Latest dismissed_at wins per legacy suppression key; prior rows become
-- claim_corroborations attached to the winning claim.
-- ---------------------------------------------------------------------------
INSERT INTO intelligence_claims (
    id, subject_ref, claim_type, field_path, text, dedup_key, item_hash,
    actor, data_source, observed_at, created_at,
    provenance_json, metadata_json,
    claim_state, surfacing_state, retraction_reason, expires_at,
    temporal_scope, sensitivity
)
SELECT
    'm1-' || cast(t.id as text),
    json_object('kind', 'Account', 'id', t.entity_id),
    'risk',
    t.field_key,
    coalesce(t.item_key, '<keyless>'),
    coalesce(t.item_hash, t.item_key, '<keyless>') || ':' || t.entity_id || ':risk:' || t.field_key,
    coalesce(t.item_hash, ''),
    'system_backfill',
    'legacy_dismissal',
    coalesce(t.dismissed_at, datetime('now')),
    coalesce(t.dismissed_at, datetime('now')),
    json_object(
        'backfill_mechanism', 'suppression_tombstones',
        'source_table', 'suppression_tombstones',
        'source_id', t.id
    ),
    json_object(
        'item_key', t.item_key,
        'item_hash', t.item_hash,
        'source_scope', t.source_scope,
        'superseded_by_evidence_after', t.superseded_by_evidence_after
    ),
    'tombstoned',
    'active',
    'user_removal',
    t.expires_at,
    'state',
    'internal'
FROM (
    SELECT *
    FROM suppression_tombstones t1
    WHERE NOT EXISTS (
        SELECT 1
        FROM suppression_tombstones t2
        WHERE t2.entity_id = t1.entity_id
          AND t2.field_key = t1.field_key
          AND coalesce(t2.item_key, '') = coalesce(t1.item_key, '')
          AND coalesce(t2.item_hash, '') = coalesce(t1.item_hash, '')
          AND t2.dismissed_at > t1.dismissed_at
    )
) t
WHERE NOT EXISTS (
    SELECT 1
    FROM intelligence_claims ic
    WHERE ic.dedup_key = (
        coalesce(t.item_hash, t.item_key, '<keyless>') || ':' || t.entity_id || ':risk:' || t.field_key
    )
);

-- Mechanism 1 corroborations — prior suppression rows attached to winners.
INSERT INTO claim_corroborations (
    id, claim_id, data_source, source_mechanism,
    strength, reinforcement_count, last_reinforced_at
)
SELECT
    'm1-corr-' || cast(loser.id as text),
    'm1-' || cast(winner.id as text),
    'legacy_dismissal',
    'suppression_tombstones_dup',
    0.5,
    1,
    coalesce(loser.dismissed_at, datetime('now'))
FROM suppression_tombstones loser
JOIN suppression_tombstones winner
  ON winner.entity_id = loser.entity_id
 AND winner.field_key = loser.field_key
 AND coalesce(winner.item_key, '') = coalesce(loser.item_key, '')
 AND coalesce(winner.item_hash, '') = coalesce(loser.item_hash, '')
 AND winner.dismissed_at > loser.dismissed_at
WHERE NOT EXISTS (
    SELECT 1
    FROM claim_corroborations cc
    WHERE cc.id = ('m1-corr-' || cast(loser.id as text))
);

-- ---------------------------------------------------------------------------
-- Mechanism 2 — account_stakeholder_roles.dismissed_at
-- Only dismissed stakeholder roles become Person-subject tombstone claims.
-- ---------------------------------------------------------------------------
INSERT INTO intelligence_claims (
    id, subject_ref, claim_type, field_path, text, dedup_key, item_hash,
    actor, data_source, observed_at, created_at,
    provenance_json, metadata_json,
    claim_state, surfacing_state, retraction_reason, expires_at,
    temporal_scope, sensitivity
)
SELECT
    'm2-' || account_id || ':' || person_id || ':' || role,
    json_object('kind', 'Person', 'id', person_id),
    'stakeholder_role',
    NULL,
    role,
    role || ':' || account_id || ':' || person_id || ':stakeholder_role',
    role,
    'system_backfill',
    'legacy_dismissal',
    dismissed_at,
    dismissed_at,
    json_object(
        'backfill_mechanism', 'account_stakeholder_roles',
        'source_table', 'account_stakeholder_roles',
        'source_id', account_id || ':' || person_id || ':' || role
    ),
    json_object(
        'account_id', account_id,
        'role', role,
        'data_source', data_source,
        'created_at', created_at
    ),
    'tombstoned',
    'active',
    'user_removal',
    NULL,
    'state',
    'internal'
FROM account_stakeholder_roles
WHERE dismissed_at IS NOT NULL
  AND NOT EXISTS (
      SELECT 1
      FROM intelligence_claims ic
      WHERE ic.dedup_key = (
          role || ':' || account_id || ':' || person_id || ':stakeholder_role'
      )
  );

-- ---------------------------------------------------------------------------
-- Mechanism 3 — email_dismissals
-- Email dismissals are Email-subject tombstone claims, even when entity_id is
-- present on the legacy row.
-- ---------------------------------------------------------------------------
INSERT INTO intelligence_claims (
    id, subject_ref, claim_type, field_path, text, dedup_key, item_hash,
    actor, data_source, observed_at, created_at,
    provenance_json, metadata_json,
    claim_state, surfacing_state, retraction_reason, expires_at,
    temporal_scope, sensitivity
)
SELECT
    'm3-' || cast(id as text),
    json_object('kind', 'Email', 'id', email_id),
    'email_dismissed',
    item_type,
    item_text,
    coalesce(item_text, '<empty>') || ':' || email_id || ':email_dismissed:' || item_type,
    coalesce(item_text, ''),
    'system_backfill',
    'legacy_dismissal',
    coalesce(dismissed_at, datetime('now')),
    coalesce(dismissed_at, datetime('now')),
    json_object(
        'backfill_mechanism', 'email_dismissals',
        'source_table', 'email_dismissals',
        'source_id', cast(id as text)
    ),
    json_object(
        'sender_domain', sender_domain,
        'email_type', email_type,
        'entity_id', entity_id,
        'item_type', item_type
    ),
    'tombstoned',
    'active',
    'user_removal',
    NULL,
    'state',
    'internal'
FROM email_dismissals
WHERE NOT EXISTS (
    SELECT 1
    FROM intelligence_claims ic
    WHERE ic.dedup_key = (
        coalesce(item_text, '<empty>') || ':' || email_id || ':email_dismissed:' || item_type
    )
);

-- ---------------------------------------------------------------------------
-- Mechanism 4 — meeting_entity_dismissals
-- D3a-1 backfills every row from this table. Duplicate-pair loser
-- corroborations against linking_dismissals are owned by D3a-2.
-- ---------------------------------------------------------------------------
INSERT INTO intelligence_claims (
    id, subject_ref, claim_type, field_path, text, dedup_key, item_hash,
    actor, data_source, observed_at, created_at,
    provenance_json, metadata_json,
    claim_state, surfacing_state, retraction_reason, expires_at,
    temporal_scope, sensitivity
)
SELECT
    'm4-' || meeting_id || ':' || entity_id || ':' || entity_type,
    json_object('kind', 'Meeting', 'id', meeting_id),
    'meeting_entity_dismissed',
    entity_type,
    entity_id,
    entity_id || ':' || meeting_id || ':meeting_entity_dismissed:' || entity_type,
    entity_id,
    'system_backfill',
    'legacy_dismissal',
    coalesce(dismissed_at, datetime('now')),
    coalesce(dismissed_at, datetime('now')),
    json_object(
        'backfill_mechanism', 'meeting_entity_dismissals',
        'source_table', 'meeting_entity_dismissals',
        'source_id', meeting_id || ':' || entity_id || ':' || entity_type
    ),
    json_object(
        'entity_id', entity_id,
        'entity_type', entity_type,
        'dismissed_by', dismissed_by
    ),
    'tombstoned',
    'active',
    'user_removal',
    NULL,
    'state',
    'internal'
FROM meeting_entity_dismissals
WHERE NOT EXISTS (
    SELECT 1
    FROM intelligence_claims ic
    WHERE ic.dedup_key = (
        entity_id || ':' || meeting_id || ':meeting_entity_dismissed:' || entity_type
    )
);
