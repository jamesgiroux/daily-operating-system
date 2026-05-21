CREATE TABLE IF NOT EXISTS mcp_client_manifest (
    client_id TEXT PRIMARY KEY,
    paired_at INTEGER NOT NULL,
    revoked_at INTEGER NULL,
    transport_key_ref TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS mcp_tool_grant (
    client_id TEXT,
    tool_name TEXT,
    scopes_granted_json TEXT NOT NULL,
    exposure TEXT NOT NULL CHECK (exposure IN ('None','MetadataOnly','Invocable')),
    rate_limit_max INTEGER NOT NULL,
    rate_limit_window_secs INTEGER NOT NULL,
    PRIMARY KEY (client_id, tool_name)
);

CREATE INDEX IF NOT EXISTS idx_mcp_tool_grant_client_tool
    ON mcp_tool_grant (client_id, tool_name);
