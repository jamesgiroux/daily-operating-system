//! MCP v2 audit log writer.
//!
//! Records client_id, conversation_handle, tool_name, params_hash,
//! response_hash, timestamp for every gateway dispatch. Hashes are
//! keyed HMAC-SHA256 over canonical JSON with a per-install audit key —
//! raw param/response data never lands in audit storage.
//! Implementation pending.
