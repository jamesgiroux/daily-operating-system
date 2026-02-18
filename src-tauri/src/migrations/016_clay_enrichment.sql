-- I228: Clay integration — contact & company enrichment

-- New columns on people for Clay enrichment data
ALTER TABLE people ADD COLUMN linkedin_url TEXT;
ALTER TABLE people ADD COLUMN twitter_handle TEXT;
ALTER TABLE people ADD COLUMN phone TEXT;
ALTER TABLE people ADD COLUMN photo_url TEXT;
ALTER TABLE people ADD COLUMN bio TEXT;
ALTER TABLE people ADD COLUMN title_history TEXT;        -- JSON: [{title, company, startDate, endDate}]
ALTER TABLE people ADD COLUMN company_industry TEXT;
ALTER TABLE people ADD COLUMN company_size TEXT;
ALTER TABLE people ADD COLUMN company_hq TEXT;
ALTER TABLE people ADD COLUMN last_enriched_at TEXT;
ALTER TABLE people ADD COLUMN enrichment_sources TEXT;   -- JSON: {fieldName: {source, at}}

-- Cross-entity enrichment audit trail
CREATE TABLE IF NOT EXISTS enrichment_log (
    id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL,        -- 'person' | 'account'
    entity_id TEXT NOT NULL,
    source TEXT NOT NULL,             -- 'clay' | 'gravatar' | 'ai' | 'user'
    event_type TEXT NOT NULL DEFAULT 'enrichment',  -- 'enrichment' | 'signal'
    signal_type TEXT,                 -- 'title_change' | 'company_change' | null
    fields_updated TEXT,             -- JSON array of field names
    raw_payload TEXT,                -- full response for debugging
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_enrichment_log_entity ON enrichment_log(entity_type, entity_id);
CREATE INDEX IF NOT EXISTS idx_enrichment_log_recent ON enrichment_log(created_at);

-- Clay-specific sync queue
CREATE TABLE IF NOT EXISTS clay_sync_state (
    id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL DEFAULT 'person',
    entity_id TEXT NOT NULL,
    state TEXT NOT NULL DEFAULT 'pending',
    attempts INTEGER NOT NULL DEFAULT 0,
    max_attempts INTEGER NOT NULL DEFAULT 3,
    clay_contact_id TEXT,
    last_attempt_at TEXT,
    completed_at TEXT,
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(entity_type, entity_id)
);
CREATE INDEX IF NOT EXISTS idx_clay_sync_state ON clay_sync_state(state);
