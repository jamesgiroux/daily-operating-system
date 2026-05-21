CREATE TABLE IF NOT EXISTS mcp_transport_nonce_ledger (
    nonce TEXT NOT NULL,
    client_id TEXT NOT NULL,
    issued_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    consumed_at INTEGER NULL,
    UNIQUE (nonce, client_id)
);

CREATE INDEX IF NOT EXISTS idx_mcp_nonce_lookup
    ON mcp_transport_nonce_ledger (client_id, nonce);

CREATE INDEX IF NOT EXISTS idx_mcp_nonce_consumed
    ON mcp_transport_nonce_ledger (consumed_at);

CREATE INDEX IF NOT EXISTS idx_mcp_nonce_expires
    ON mcp_transport_nonce_ledger (expires_at);
