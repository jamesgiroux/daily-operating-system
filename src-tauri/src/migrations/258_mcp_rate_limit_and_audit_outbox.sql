CREATE TABLE IF NOT EXISTS mcp_tool_call_ledger (
    client_id TEXT,
    tool_name TEXT,
    called_at INTEGER
);

CREATE INDEX IF NOT EXISTS idx_mcp_tool_call_ledger_window
    ON mcp_tool_call_ledger (client_id, tool_name, called_at);

CREATE TABLE IF NOT EXISTS mcp_audit_outbox (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event TEXT,
    detail_json TEXT,
    actor_kind TEXT,
    request_id TEXT,
    created_at INTEGER,
    drained_at INTEGER NULL
);
