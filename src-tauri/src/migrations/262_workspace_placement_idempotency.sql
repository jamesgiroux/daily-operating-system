BEGIN IMMEDIATE;

CREATE TABLE IF NOT EXISTS workspace_placement_idempotency (
  idempotency_id TEXT PRIMARY KEY,
  actor_id TEXT NOT NULL,
  entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  content_sha256 TEXT NOT NULL,
  content_type TEXT NOT NULL,
  category_slug TEXT NOT NULL,
  client_dedup_key TEXT NOT NULL DEFAULT '',
  status TEXT NOT NULL CHECK (status IN ('in_progress','succeeded','failed')),
  document_handle TEXT,
  source_handle TEXT,
  file_id TEXT,
  run_id TEXT,
  chosen_filename TEXT,
  source_asof TEXT,
  lifecycle_state TEXT,
  claim_count_produced INTEGER NOT NULL DEFAULT 0,
  error_code TEXT,
  started_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  stale_after TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now', '+1 hour')),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  UNIQUE(actor_id, entity_type, entity_id, content_sha256, content_type, category_slug, client_dedup_key)
);

CREATE TABLE IF NOT EXISTS workspace_placement_rate_ledger (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  actor_id TEXT NOT NULL,
  tool_name TEXT NOT NULL,
  called_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_workspace_placement_rate_window
  ON workspace_placement_rate_ledger(actor_id, tool_name, called_at);

CREATE TABLE IF NOT EXISTS workspace_placement_attempt_audit (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  actor_id TEXT NOT NULL,
  tool_name TEXT NOT NULL,
  target_audit_key TEXT,
  category_slug TEXT,
  dry_run INTEGER NOT NULL CHECK (dry_run IN (0, 1)),
  outcome TEXT NOT NULL CHECK (outcome IN ('succeeded','failed')),
  error_code TEXT,
  occurred_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

COMMIT;
