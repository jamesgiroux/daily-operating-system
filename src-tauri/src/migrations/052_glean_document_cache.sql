-- Glean document cache for dual-mode context architecture (ADR-0095).
-- Stores cached Glean API responses with TTL-based expiration.

CREATE TABLE IF NOT EXISTS glean_document_cache (
    cache_key   TEXT PRIMARY KEY,
    kind        TEXT NOT NULL DEFAULT 'document',
    content     TEXT NOT NULL,
    cached_at   TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_glean_cache_kind_cached
    ON glean_document_cache(kind, cached_at);

-- Context mode configuration. Stored as a single-row config extension.
-- NULL means Local mode (default). JSON value for Glean mode.
-- Example: {"mode":"Glean","endpoint":"https://...","keychain_key":"...","strategy":"Additive"}
CREATE TABLE IF NOT EXISTS context_mode_config (
    id          INTEGER PRIMARY KEY CHECK (id = 1),
    mode_json   TEXT,
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT OR IGNORE INTO context_mode_config (id, mode_json) VALUES (1, NULL);
