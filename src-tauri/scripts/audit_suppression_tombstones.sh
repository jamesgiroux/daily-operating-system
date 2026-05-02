#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 /path/to/dailyos.db" >&2
  exit 64
fi

db_path="$1"

if ! command -v sqlite3 >/dev/null 2>&1; then
  echo "sqlite3 is required" >&2
  exit 69
fi

sqlite3 "$db_path" <<'SQL'
.mode json
WITH
malformed_timestamps AS (
  SELECT id, entity_id, field_key, item_key, item_hash, 'dismissed_at' AS field, dismissed_at AS value
  FROM suppression_tombstones
  WHERE dismissed_at IS NULL OR datetime(dismissed_at) IS NULL

  UNION ALL
  SELECT id, entity_id, field_key, item_key, item_hash, 'expires_at' AS field, expires_at AS value
  FROM suppression_tombstones
  WHERE expires_at IS NOT NULL AND datetime(expires_at) IS NULL

  UNION ALL
  SELECT id, entity_id, field_key, item_key, item_hash,
         'superseded_by_evidence_after' AS field,
         superseded_by_evidence_after AS value
  FROM suppression_tombstones
  WHERE superseded_by_evidence_after IS NOT NULL
    AND datetime(superseded_by_evidence_after) IS NULL

  UNION ALL
  SELECT id, entity_id, field_key, item_key, item_hash, 'expires_at' AS field, expires_at AS value
  FROM suppression_tombstones
  WHERE expires_at IS NOT NULL
    AND datetime(expires_at) IS NOT NULL
    AND datetime(dismissed_at) IS NOT NULL
    AND datetime(expires_at) < datetime(dismissed_at)
),
multi_tombstones AS (
  SELECT entity_id,
         field_key,
         item_key,
         item_hash,
         COUNT(*) AS tombstone_count,
         MAX(datetime(dismissed_at)) AS latest_dismissed_at
  FROM suppression_tombstones
  GROUP BY entity_id,
           field_key,
           COALESCE(item_hash, '__NULL__'),
           COALESCE(item_key, '__NULL__')
  HAVING COUNT(*) > 1
),
expired_active_rows AS (
  SELECT id, entity_id, field_key, item_key, item_hash, dismissed_at, expires_at
  FROM suppression_tombstones
  WHERE expires_at IS NOT NULL
    AND datetime(expires_at) IS NOT NULL
    AND datetime(expires_at) < datetime('now')
)
SELECT json_object(
  'malformed_timestamps',
  COALESCE((
    SELECT json_group_array(json_object(
      'id', id,
      'entity_id', entity_id,
      'field_key', field_key,
      'item_key', item_key,
      'item_hash', item_hash,
      'field', field,
      'value', value
    ))
    FROM malformed_timestamps
  ), json('[]')),
  'multi_tombstones_per_key',
  COALESCE((
    SELECT json_group_array(json_object(
      'entity_id', entity_id,
      'field_key', field_key,
      'item_key', item_key,
      'item_hash', item_hash,
      'tombstone_count', tombstone_count,
      'latest_dismissed_at', latest_dismissed_at
    ))
    FROM multi_tombstones
  ), json('[]')),
  'expired_active_rows',
  COALESCE((
    SELECT json_group_array(json_object(
      'id', id,
      'entity_id', entity_id,
      'field_key', field_key,
      'item_key', item_key,
      'item_hash', item_hash,
      'dismissed_at', dismissed_at,
      'expires_at', expires_at
    ))
    FROM expired_active_rows
  ), json('[]'))
) AS audit_report;
SQL
