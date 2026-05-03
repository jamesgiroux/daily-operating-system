-- Add columns to entity_assessment for fields currently stored only in intelligence.json
ALTER TABLE entity_assessment ADD COLUMN portfolio_json TEXT;
ALTER TABLE entity_assessment ADD COLUMN network_json TEXT;
ALTER TABLE entity_assessment ADD COLUMN user_edits_json TEXT;
ALTER TABLE entity_assessment ADD COLUMN source_manifest_json TEXT;

-- App state key-value store (replaces manifest.json + next-morning-flags.json)
CREATE TABLE IF NOT EXISTS app_state_kv (
    key TEXT PRIMARY KEY,
    value_json TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
