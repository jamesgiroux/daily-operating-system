-- MCP v2 local transport-ceremony cleanup.
-- The nonce ledger belongs to the removed HMAC transport layer; local MCP uses
-- OS/user trust plus tool authorization scopes.
BEGIN IMMEDIATE;

DROP TABLE IF EXISTS mcp_transport_nonce_ledger;

COMMIT;
