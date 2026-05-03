-- Claims backfill D3a-2: Backfill legacy dismissal mechanisms 5-8 into
-- intelligence_claims tombstone rows.
--
-- Scope:
--   5. linking_dismissals
--   6. briefing_callouts.dismissed_at
--   7. nudge_dismissals
--   8. triage_snoozes
--
-- D3a-2 also writes duplicate-pair corroborations between mechanism 4
-- (meeting_entity_dismissals, already backfilled by D3a-1) and mechanism 5.

-- ---------------------------------------------------------------------------
-- Mechanism 5 — linking_dismissals
-- Link dismissals become owner-subject tombstone claims when they have no
-- mechanism 4 counterpart or are newer than that counterpart. Older duplicate
-- rows become corroborations against the mechanism 4 winner.
-- ---------------------------------------------------------------------------
INSERT INTO intelligence_claims (
    id, subject_ref, claim_type, field_path, text, dedup_key, item_hash,
    actor, data_source, observed_at, created_at,
    provenance_json, metadata_json,
    claim_state, surfacing_state, retraction_reason, expires_at,
    temporal_scope, sensitivity
)
SELECT
    'm5-' || ld.owner_type || ':' || ld.owner_id || ':' || ld.entity_id || ':' || ld.entity_type,
    json_object(
        'kind',
        CASE ld.owner_type
            WHEN 'meeting' THEN 'Meeting'
            WHEN 'email' THEN 'Email'
            ELSE ld.owner_type
        END,
        'id', ld.owner_id
    ),
    'linking_dismissed',
    ld.entity_type,
    ld.entity_id,
    ld.entity_id || ':' || ld.owner_type || ':' || ld.owner_id || ':linking_dismissed:' || ld.entity_type,
    ld.entity_id,
    'system_backfill',
    'legacy_dismissal',
    ld.created_at,
    ld.created_at,
    json_object(
        'backfill_mechanism', 'linking_dismissals',
        'source_table', 'linking_dismissals',
        'source_id', ld.owner_type || ':' || ld.owner_id || ':' || ld.entity_id || ':' || ld.entity_type
    ),
    json_object(
        'owner_type', ld.owner_type,
        'owner_id', ld.owner_id,
        'entity_id', ld.entity_id,
        'entity_type', ld.entity_type,
        'dismissed_by', ld.dismissed_by
    ),
    'tombstoned',
    'active',
    'user_removal',
    NULL,
    'state',
    'internal'
FROM linking_dismissals ld
LEFT JOIN meeting_entity_dismissals med
  ON ld.owner_type = 'meeting'
 AND med.meeting_id = ld.owner_id
 AND med.entity_id = ld.entity_id
 AND med.entity_type = ld.entity_type
WHERE (med.meeting_id IS NULL OR ld.created_at > med.dismissed_at)
  AND NOT EXISTS (
      SELECT 1
      FROM intelligence_claims ic
      WHERE ic.dedup_key = (
          ld.entity_id || ':' || ld.owner_type || ':' || ld.owner_id || ':linking_dismissed:' || ld.entity_type
      )
  );

-- Mechanism 5 duplicate-pair corroborations — older mechanism 4 rows attached
-- to newer linking_dismissals winners.
INSERT INTO claim_corroborations (
    id, claim_id, data_source, source_mechanism,
    strength, reinforcement_count, last_reinforced_at
)
SELECT
    'm4-m5-corr-' || ld.owner_id || ':' || ld.entity_id || ':' || ld.entity_type,
    'm5-' || ld.owner_type || ':' || ld.owner_id || ':' || ld.entity_id || ':' || ld.entity_type,
    'legacy_dismissal',
    'meeting_entity_dismissals_dup',
    0.5,
    1,
    med.dismissed_at
FROM linking_dismissals ld
JOIN meeting_entity_dismissals med
  ON ld.owner_type = 'meeting'
 AND med.meeting_id = ld.owner_id
 AND med.entity_id = ld.entity_id
 AND med.entity_type = ld.entity_type
WHERE ld.created_at > med.dismissed_at
  AND EXISTS (
      SELECT 1
      FROM intelligence_claims ic
      WHERE ic.id = (
          'm5-' || ld.owner_type || ':' || ld.owner_id || ':' || ld.entity_id || ':' || ld.entity_type
      )
  )
  AND NOT EXISTS (
      SELECT 1
      FROM claim_corroborations cc
      WHERE cc.id = (
          'm4-m5-corr-' || ld.owner_id || ':' || ld.entity_id || ':' || ld.entity_type
      )
  );

-- Mechanism 5 duplicate-pair corroborations — older linking_dismissals rows
-- attached to mechanism 4 winners already backfilled by D3a-1.
INSERT INTO claim_corroborations (
    id, claim_id, data_source, source_mechanism,
    strength, reinforcement_count, last_reinforced_at
)
SELECT
    'm5-m4-corr-' || ld.owner_id || ':' || ld.entity_id || ':' || ld.entity_type,
    'm4-' || ld.owner_id || ':' || ld.entity_id || ':' || ld.entity_type,
    'legacy_dismissal',
    'linking_dismissals_dup',
    0.5,
    1,
    ld.created_at
FROM linking_dismissals ld
JOIN meeting_entity_dismissals med
  ON ld.owner_type = 'meeting'
 AND med.meeting_id = ld.owner_id
 AND med.entity_id = ld.entity_id
 AND med.entity_type = ld.entity_type
WHERE ld.created_at <= med.dismissed_at
  AND EXISTS (
      SELECT 1
      FROM intelligence_claims ic
      WHERE ic.id = ('m4-' || ld.owner_id || ':' || ld.entity_id || ':' || ld.entity_type)
  )
  AND NOT EXISTS (
      SELECT 1
      FROM claim_corroborations cc
      WHERE cc.id = (
          'm5-m4-corr-' || ld.owner_id || ':' || ld.entity_id || ':' || ld.entity_type
      )
  );

-- ---------------------------------------------------------------------------
-- Mechanism 6 — briefing_callouts.dismissed_at
-- Only dismissed briefing callouts become entity-subject tombstone claims.
-- ---------------------------------------------------------------------------
INSERT INTO intelligence_claims (
    id, subject_ref, claim_type, field_path, text, dedup_key, item_hash,
    actor, data_source, observed_at, created_at,
    provenance_json, metadata_json,
    claim_state, surfacing_state, retraction_reason, expires_at,
    temporal_scope, sensitivity
)
SELECT
    'm6-' || id,
    json_object(
        'kind', upper(substr(entity_type, 1, 1)) || substr(entity_type, 2),
        'id', entity_id
    ),
    'briefing_callout_dismissed',
    NULL,
    headline,
    id || ':' || entity_id || ':briefing_callout_dismissed',
    headline,
    'system_backfill',
    'legacy_dismissal',
    dismissed_at,
    dismissed_at,
    json_object(
        'backfill_mechanism', 'briefing_callouts',
        'source_table', 'briefing_callouts',
        'source_id', id
    ),
    json_object(
        'signal_id', signal_id,
        'entity_type', entity_type,
        'entity_id', entity_id,
        'entity_name', entity_name,
        'severity', severity,
        'detail', detail,
        'context_json', context_json,
        'surfaced_at', surfaced_at,
        'created_at', created_at
    ),
    'tombstoned',
    'active',
    'user_removal',
    NULL,
    'state',
    'internal'
FROM briefing_callouts
WHERE dismissed_at IS NOT NULL
  AND NOT EXISTS (
      SELECT 1
      FROM intelligence_claims ic
      WHERE ic.dedup_key = (id || ':' || entity_id || ':briefing_callout_dismissed')
  );

-- ---------------------------------------------------------------------------
-- Mechanism 7 — nudge_dismissals
-- Nudge dismissals become entity-subject tombstone claims.
-- ---------------------------------------------------------------------------
INSERT INTO intelligence_claims (
    id, subject_ref, claim_type, field_path, text, dedup_key, item_hash,
    actor, data_source, observed_at, created_at,
    provenance_json, metadata_json,
    claim_state, surfacing_state, retraction_reason, expires_at,
    temporal_scope, sensitivity
)
SELECT
    'm7-' || entity_type || ':' || entity_id || ':' || nudge_key,
    json_object(
        'kind', upper(substr(entity_type, 1, 1)) || substr(entity_type, 2),
        'id', entity_id
    ),
    'nudge_dismissed',
    entity_type,
    nudge_key,
    nudge_key || ':' || entity_id || ':nudge_dismissed',
    nudge_key,
    'system_backfill',
    'legacy_dismissal',
    dismissed_at,
    dismissed_at,
    json_object(
        'backfill_mechanism', 'nudge_dismissals',
        'source_table', 'nudge_dismissals',
        'source_id', entity_type || ':' || entity_id || ':' || nudge_key
    ),
    json_object(
        'entity_type', entity_type,
        'entity_id', entity_id,
        'nudge_key', nudge_key
    ),
    'tombstoned',
    'active',
    'user_removal',
    NULL,
    'state',
    'internal'
FROM nudge_dismissals
WHERE NOT EXISTS (
    SELECT 1
    FROM intelligence_claims ic
    WHERE ic.dedup_key = (nudge_key || ':' || entity_id || ':nudge_dismissed')
);

-- ---------------------------------------------------------------------------
-- Mechanism 8 — triage_snoozes
-- Active snoozes and resolved triage rows become entity-subject tombstone
-- claims with the legacy snooze expiry preserved as expires_at.
-- ---------------------------------------------------------------------------
INSERT INTO intelligence_claims (
    id, subject_ref, claim_type, field_path, text, dedup_key, item_hash,
    actor, data_source, observed_at, created_at,
    provenance_json, metadata_json,
    claim_state, surfacing_state, retraction_reason, expires_at,
    temporal_scope, sensitivity
)
SELECT
    'm8-' || entity_type || ':' || entity_id || ':' || triage_key,
    json_object(
        'kind', upper(substr(entity_type, 1, 1)) || substr(entity_type, 2),
        'id', entity_id
    ),
    'triage_snooze',
    entity_type,
    triage_key,
    triage_key || ':' || entity_id || ':triage_snooze',
    triage_key,
    'system_backfill',
    'legacy_dismissal',
    coalesce(resolved_at, updated_at, created_at),
    coalesce(resolved_at, updated_at, created_at),
    json_object(
        'backfill_mechanism', 'triage_snoozes',
        'source_table', 'triage_snoozes',
        'source_id', entity_type || ':' || entity_id || ':' || triage_key
    ),
    json_object(
        'entity_type', entity_type,
        'entity_id', entity_id,
        'triage_key', triage_key,
        'snoozed_until', snoozed_until,
        'resolved_at', resolved_at,
        'created_at', created_at,
        'updated_at', updated_at
    ),
    'tombstoned',
    'active',
    CASE
        WHEN resolved_at IS NOT NULL THEN 'user_resolved'
        ELSE 'system_snooze'
    END,
    snoozed_until,
    'state',
    'internal'
FROM triage_snoozes
WHERE (snoozed_until > datetime('now') OR resolved_at IS NOT NULL)
  AND NOT EXISTS (
      SELECT 1
      FROM intelligence_claims ic
      WHERE ic.dedup_key = (triage_key || ':' || entity_id || ':triage_snooze')
  );
