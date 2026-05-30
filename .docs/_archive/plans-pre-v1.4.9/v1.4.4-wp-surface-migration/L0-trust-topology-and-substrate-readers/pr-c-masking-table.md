# PR C MCP v2 Write-Class Audit Masking Table

`audit::detail_with_attribution` preserves plaintext detail for `Side::Read`.
For `Side::Write` and `Side::SubmitCorrection`, it calls `sanitize_detail`,
which removes the top-level keys in `PARAM_PAYLOAD_KEYS` (`params`,
`parameters`) and `RESPONSE_PAYLOAD_KEYS` before the audit row is appended.

| handler file | declared Side | params field list | fields hit by `PARAM_PAYLOAD_KEYS` mask |
| --- | --- | --- | --- |
| `src-tauri/src/services/mcp_v2/handlers/tool_placement.rs` | `Side::Write` | `topic`, `content` | All params, via top-level `params` removal |
| `src-tauri/src/services/mcp_v2/handlers/tool_note.rs` | `Side::SubmitCorrection` | `text`, `subject` | All params, via top-level `params` removal |
| `src-tauri/src/services/mcp_v2/handlers/tool_create_action.rs` | `Side::SubmitCorrection` | `description`, `due_at`, `subject` | All params, via top-level `params` removal |
| `src-tauri/src/services/mcp_v2/handlers/tool_update_action_status.rs` | `Side::SubmitCorrection` | `action_id`, `status` | All params, via top-level `params` removal |

