-- I644: Source-aware account truth — source references + commercial stage separation

CREATE TABLE IF NOT EXISTS account_source_refs (
  id TEXT PRIMARY KEY,
  account_id TEXT NOT NULL,
  field TEXT NOT NULL,
  source_system TEXT NOT NULL,
  source_kind TEXT NOT NULL DEFAULT 'inference',
  source_value TEXT,
  observed_at TEXT NOT NULL,
  source_record_ref TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_source_refs_account_field ON account_source_refs(account_id, field);

-- Separate commercial opportunity stage from timeline renewal stage
ALTER TABLE accounts ADD COLUMN commercial_stage TEXT;
