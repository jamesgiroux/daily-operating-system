CREATE TABLE IF NOT EXISTS mcp_target_handles (
  handle_lookup_hash TEXT PRIMARY KEY,
  client_id TEXT NOT NULL,
  conversation_handle_hash TEXT NOT NULL,
  originating_tool TEXT NOT NULL,
  result_item_path TEXT NOT NULL,
  target_kind TEXT NOT NULL CHECK (
    target_kind IN ('claim', 'action', 'entity', 'source_provenance', 'workspace_source')
  ),
  target_ref_ciphertext BLOB NOT NULL,
  target_ref_key_version INTEGER NOT NULL,
  render_policy_version TEXT NOT NULL,
  sensitivity_tier TEXT NOT NULL,
  provenance_hash TEXT NOT NULL,
  target_watermark_hash TEXT NOT NULL,
  created_at TEXT NOT NULL,
  last_used_at TEXT NOT NULL,
  expires_at TEXT NOT NULL,
  revoked_at TEXT,
  revoked_reason_code TEXT
);

CREATE INDEX IF NOT EXISTS idx_mcp_target_handles_client_conversation
  ON mcp_target_handles (client_id, conversation_handle_hash, originating_tool);

CREATE INDEX IF NOT EXISTS idx_mcp_target_handles_expiry
  ON mcp_target_handles (expires_at);

CREATE INDEX IF NOT EXISTS idx_mcp_target_handles_revoked
  ON mcp_target_handles (revoked_at)
  WHERE revoked_at IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_mcp_target_handles_target_watermark
  ON mcp_target_handles (target_kind, target_watermark_hash);

CREATE INDEX IF NOT EXISTS idx_mcp_target_handles_deterministic_target
  ON mcp_target_handles (
    client_id,
    conversation_handle_hash,
    originating_tool,
    result_item_path,
    target_kind,
    provenance_hash,
    target_watermark_hash
  );
