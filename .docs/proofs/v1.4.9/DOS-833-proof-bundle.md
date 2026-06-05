# DOS-833 Proof Bundle - W2 MCP Auth Right-Size

**Wave:** v1.4.9 W2 - Security right-size  
**Scope:** Local MCP stdio auth/topology, hostile-input rejection, audit privacy  
**Branch:** `codex/v1.4.9-w2-dos833`

## Implementation Evidence

| Requirement | Evidence |
| --- | --- |
| Server-owned local MCP identity | `src-tauri/src/services/mcp_v2/local_runtime.rs` creates/persists the local stdio client id in macOS Keychain; `src-tauri/src/mcp/main.rs` no longer reads `DAILYOS_MCP_CLIENT_ID`. |
| Local stdio write exposure blocked | `src-tauri/src/mcp/main.rs` maps `Side::Write` to `McpExposure::None`; `dailyos.write.place_document` remains registered but is not locally invocable. |
| ADR-0128 submit-correction trio preserved as allowlist | Local stdio exposure admits `dailyos.submit.note`, `dailyos.submit.action`, and `dailyos.submit.action_status` only when those submit handlers are registered; W2 does not create placeholder submit handlers. |
| Legacy MCP v1 quarantined | `DAILYOS_MCP_LEGACY_V1=1` no longer starts v1 unless the debug-only `DAILYOS_MCP_LEGACY_V1_UNSAFE_DEV=1` escape hatch is also set. |
| ADR-0137 local MCP trust boundary | `.docs/decisions/0137-local-mcp-same-user-trust-boundary.md` supersedes the local-stdio portions of ADR-0102/0128 while preserving `Actor::McpClient` attribution. |
| Conversation continuity is server-owned | Transport accepts only hidden `_dailyos.conversationHandle`, strips it before handler dispatch, and the gateway resolves/mints 24-hour sliding handles through a locked mode-scoped state file. |
| Caller-asserted identity/scope rejected | Gateway rejects `_dailyos`, `clientId`, `actor`, `scope(s)`, `side`, and `sensitivity` if they reach handler params. |
| Read audit payload privacy | Read-class audit rows replace params/responses with keyed HMAC-SHA256 digests; write/submit payload masking remains in place. |
| Read audit fail-closed | Gateway returns `ToolError::Internal { trace_id: "mcp_read_audit_failed" }` when read audit digesting fails. |

## Validation

| Check | Result |
| --- | --- |
| `bash src-tauri/scripts/build-mcp.sh --stub` | Pass |
| `cargo test --manifest-path src-tauri/Cargo.toml services::mcp_v2 --lib` | Pass - 100 tests |
| `cargo test --manifest-path src-tauri/Cargo.toml services::mcp_v2::local_runtime --lib` | Pass - 11 tests covering Keychain first-seed race handling, Keychain timeout, 24-hour conversation remint, revoked-handle, and concurrent-store behavior |
| `cargo test --manifest-path src-tauri/Cargo.toml --test v146_validation mcp_placement_handler_registered_but_not_local_stdio_invocable` | Pass |
| `cargo test --manifest-path src-tauri/Cargo.toml --features mcp --bin dailyos-mcp local_stdio_exposure_keeps_write_handlers_non_invocable` | Pass |
| `cargo test --manifest-path src-tauri/Cargo.toml` | Pass |
| `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` | Pass |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --features mcp --bin dailyos-mcp -- -D warnings` | Pass |
| `pnpm tsc --noEmit` | Pass |
| `git diff --check` | Pass |

## Intelligence Loop Check

W2 does not add a new claim table, claim field, or user-visible intelligence surface. It tightens the MCP transport boundary around existing claim-backed abilities. Provenance attribution remains `Actor::McpClient`; read audit now stores digests instead of raw payloads; correction/feedback semantics remain owned by registered submit-correction handlers and downstream claim-feedback services.
