CREATE TABLE IF NOT EXISTS mcp_conversation_handle (
    handle TEXT,
    client_id TEXT,
    mint_at INTEGER NOT NULL,
    last_touched_at INTEGER NOT NULL,
    revoked_at INTEGER NULL,
    UNIQUE (handle, client_id)
);

CREATE INDEX IF NOT EXISTS idx_mcp_conversation_handle_lookup
    ON mcp_conversation_handle (client_id, handle);
