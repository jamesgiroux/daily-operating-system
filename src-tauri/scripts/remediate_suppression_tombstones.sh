#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || $# -gt 2 ]]; then
  echo "usage: $0 /path/to/dailyos.db [--apply]" >&2
  exit 64
fi

db_path="$1"
mode="${2:---dry-run}"

if [[ "$mode" != "--dry-run" && "$mode" != "--apply" ]]; then
  echo "mode must be --dry-run or --apply" >&2
  exit 64
fi

if ! command -v sqlite3 >/dev/null 2>&1; then
  echo "sqlite3 is required" >&2
  exit 69
fi

resolved_at_ddl=""
quarantine_exists="$(sqlite3 "$db_path" "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'suppression_tombstones_quarantine';")"
if [[ "$quarantine_exists" == "1" ]]; then
  has_resolved_at="$(sqlite3 "$db_path" "SELECT count(*) FROM pragma_table_info('suppression_tombstones_quarantine') WHERE name = 'resolved_at';")"
  if [[ "$has_resolved_at" == "0" ]]; then
    resolved_at_ddl="ALTER TABLE suppression_tombstones_quarantine ADD COLUMN resolved_at TEXT;"
  fi
fi

terminator="ROLLBACK;"
if [[ "$mode" == "--apply" ]]; then
  terminator="COMMIT;"
fi

sqlite3 "$db_path" <<SQL
.mode json
BEGIN IMMEDIATE;

CREATE TABLE IF NOT EXISTS suppression_tombstones_quarantine (
    id INTEGER PRIMARY KEY,
    entity_id TEXT NOT NULL,
    field_key TEXT NOT NULL,
    item_key TEXT,
    item_hash TEXT,
    dismissed_at TEXT,
    source_scope TEXT,
    expires_at TEXT,
    superseded_by_evidence_after TEXT,
    quarantined_at TEXT NOT NULL DEFAULT (datetime('now')),
    quarantine_reason TEXT NOT NULL,
    resolved_at TEXT
);

${resolved_at_ddl}

CREATE INDEX IF NOT EXISTS idx_quarantine_unresolved
    ON suppression_tombstones_quarantine(resolved_at)
    WHERE resolved_at IS NULL;

CREATE TEMP TABLE remediation_log(action TEXT NOT NULL, row_count INTEGER NOT NULL);

WITH malformed AS (
  SELECT id, 'UnparsableTimestamp(dismissed_at)' AS reason
  FROM suppression_tombstones
  WHERE dismissed_at IS NULL OR datetime(dismissed_at) IS NULL

  UNION ALL
  SELECT id, 'UnparsableTimestamp(expires_at)' AS reason
  FROM suppression_tombstones
  WHERE expires_at IS NOT NULL AND datetime(expires_at) IS NULL

  UNION ALL
  SELECT id, 'UnparsableTimestamp(superseded_by_evidence_after)' AS reason
  FROM suppression_tombstones
  WHERE superseded_by_evidence_after IS NOT NULL
    AND datetime(superseded_by_evidence_after) IS NULL

  UNION ALL
  SELECT id, 'InvalidExpiry' AS reason
  FROM suppression_tombstones
  WHERE expires_at IS NOT NULL
    AND datetime(expires_at) IS NOT NULL
    AND datetime(dismissed_at) IS NOT NULL
    AND datetime(expires_at) < datetime(dismissed_at)
),
malformed_ids AS (
  SELECT id, MIN(reason) AS reason
  FROM malformed
  GROUP BY id
)
INSERT OR REPLACE INTO suppression_tombstones_quarantine (
    id,
    entity_id,
    field_key,
    item_key,
    item_hash,
    dismissed_at,
    source_scope,
    expires_at,
    superseded_by_evidence_after,
    quarantine_reason,
    resolved_at
)
SELECT s.id,
       s.entity_id,
       s.field_key,
       s.item_key,
       s.item_hash,
       s.dismissed_at,
       s.source_scope,
       s.expires_at,
       s.superseded_by_evidence_after,
       m.reason,
       datetime('now')
FROM suppression_tombstones AS s
JOIN malformed_ids AS m ON m.id = s.id;

INSERT INTO remediation_log VALUES ('quarantine_malformed', changes());

WITH malformed AS (
  SELECT id
  FROM suppression_tombstones
  WHERE dismissed_at IS NULL OR datetime(dismissed_at) IS NULL

  UNION
  SELECT id
  FROM suppression_tombstones
  WHERE expires_at IS NOT NULL AND datetime(expires_at) IS NULL

  UNION
  SELECT id
  FROM suppression_tombstones
  WHERE superseded_by_evidence_after IS NOT NULL
    AND datetime(superseded_by_evidence_after) IS NULL

  UNION
  SELECT id
  FROM suppression_tombstones
  WHERE expires_at IS NOT NULL
    AND datetime(expires_at) IS NOT NULL
    AND datetime(dismissed_at) IS NOT NULL
    AND datetime(expires_at) < datetime(dismissed_at)
)
DELETE FROM suppression_tombstones
WHERE id IN (SELECT id FROM malformed);

INSERT INTO remediation_log VALUES ('remove_malformed_from_source', changes());

WITH ranked AS (
  SELECT id,
         ROW_NUMBER() OVER (
           PARTITION BY entity_id,
                        field_key,
                        COALESCE(item_hash, '__NULL__'),
                        COALESCE(item_key, '__NULL__')
           ORDER BY datetime(dismissed_at) DESC, id DESC
         ) AS row_rank
  FROM suppression_tombstones
  WHERE datetime(dismissed_at) IS NOT NULL
)
INSERT OR REPLACE INTO suppression_tombstones_quarantine (
    id,
    entity_id,
    field_key,
    item_key,
    item_hash,
    dismissed_at,
    source_scope,
    expires_at,
    superseded_by_evidence_after,
    quarantine_reason,
    resolved_at
)
SELECT s.id,
       s.entity_id,
       s.field_key,
       s.item_key,
       s.item_hash,
       s.dismissed_at,
       s.source_scope,
       s.expires_at,
       s.superseded_by_evidence_after,
       'DuplicateSuperseded TODO(DOS-7): attach claim_corroborations=' ||
       json_object(
         'entity_id', s.entity_id,
         'field_key', s.field_key,
         'item_key', s.item_key,
         'item_hash', s.item_hash,
         'dismissed_at', s.dismissed_at,
         'source_scope', s.source_scope
       ),
       datetime('now')
FROM suppression_tombstones AS s
JOIN ranked AS r ON r.id = s.id
WHERE r.row_rank > 1;

INSERT INTO remediation_log VALUES ('quarantine_duplicate_tombstones', changes());

WITH ranked AS (
  SELECT id,
         ROW_NUMBER() OVER (
           PARTITION BY entity_id,
                        field_key,
                        COALESCE(item_hash, '__NULL__'),
                        COALESCE(item_key, '__NULL__')
           ORDER BY datetime(dismissed_at) DESC, id DESC
         ) AS row_rank
  FROM suppression_tombstones
  WHERE datetime(dismissed_at) IS NOT NULL
)
DELETE FROM suppression_tombstones
WHERE id IN (SELECT id FROM ranked WHERE row_rank > 1);

INSERT INTO remediation_log VALUES ('remove_duplicate_tombstones_from_source', changes());

SELECT '${mode}' AS mode, action, row_count FROM remediation_log;

${terminator}
SQL
