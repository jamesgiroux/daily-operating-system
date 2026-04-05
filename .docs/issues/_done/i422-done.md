# I422 — Clay Connector — SSE Transport + OAuth Flow

**Status:** Open
**Priority:** P1
**Version:** 0.13.9
**Area:** Backend / Clay

## Summary

Clay's API keys are a beta-locked feature — not publicly available. This means the current stdio path (`npx -y @clayhq/clay-mcp` + `CLAY_API_KEY` env var) is a dead end until Clay opens key access. The correct path is **SSE transport to `https://mcp.clay.earth/mcp`** with a Bearer token obtained via a Clay OAuth flow — the same path used when Clay MCP tools work in Claude Code without a key.

The client.rs already has this documented as a TODO (lines 160–176): the SSE path was designed first but skipped because `rmcp 0.1.5` doesn't include a `transport-sse` feature. The implementation strategy is already correct in the comments — use `reqwest` for raw JSON-RPC over SSE. `reqwest 0.12` is already in `Cargo.toml` but needs the `stream` feature enabled.

This issue: implement the SSE transport, wire a Clay OAuth login flow, remove the dependency on an API key, remove the legacy `emit_signal` path, and validate against the production DB.

## What changed from the original plan

The original I422 assumed a Clay API key would be available. It won't be. The stdio approach is shelved. SSE + OAuth is the implementation.

## Acceptance Criteria

### 1. reqwest stream feature enabled

`reqwest` in `Cargo.toml` gains the `stream` feature: `features = ["json", "rustls-tls", "stream"]`. `cargo build` passes clean. No other reqwest-using code is affected.

### 2. SSE transport implemented in clay/client.rs

`ClayClient::connect_sse(token: &str)` exists and connects to `https://mcp.clay.earth/mcp` using raw reqwest + JSON-RPC-over-SSE:
- POST requests with `Content-Type: application/json` and `Authorization: Bearer <token>`
- Reads the SSE stream response using reqwest's streaming API
- Parses JSON-RPC messages from the `data:` event lines
- `ClayClient::connect(token)` tries SSE first; the stdio path (with API key) is retained as an unreachable fallback for when Clay opens key access, but is not the active path.

Verify: `grep -n "connect_sse\|connect_stdio" src-tauri/src/clay/client.rs` — both exist; `connect` calls `connect_sse`.

### 3. Clay OAuth login flow in Settings

The Clay Settings card no longer shows an API key input. Instead:
- A "Connect Clay" button that opens a Clay OAuth flow (web browser → Clay login → token returned to app via redirect/callback, stored in keychain alongside Google tokens)
- A "Connected as [Clay username]" status when authenticated
- A "Disconnect" button that removes the token from the keychain

The OAuth flow follows the same pattern as the existing Google OAuth (PKCE, local redirect). The token obtained is the Bearer token passed to the SSE endpoint. Clay's OAuth is confirmed available.

### 4. Clay runs in production and emits signals

After authenticating:
- `SELECT count(*) FROM clay_sync_state WHERE state = 'completed'` returns > 0 within 24 hours
- `SELECT count(*) FROM signal_events WHERE source = 'clay'` returns > 0
- Signal type is one of `title_change`, `company_change`, or `profile_update`

### 5. Legacy signal path removed

`enrich_person_from_clay` (the old function using plain `emit_signal`, line ~649 in `enricher.rs`) is removed. All enrichment flows through `enrich_person_from_clay_with_client` which uses `emit_signal_and_propagate`.

`grep -n "enrich_person_from_clay\b" src-tauri/src/clay/enricher.rs` returns only the `_with_client` variant.

### 6. Clay signals propagate to linked accounts

After a `company_change` or `title_change` is emitted for a person, propagation fires to their linked accounts. `SELECT source, entity_type FROM signal_events WHERE source LIKE '%propagat%' ORDER BY created_at DESC LIMIT 5` — at least one row shows propagation to an `account` entity after a Clay sync.

## Dependencies

None from other v0.13.9 issues. Clay's SSE endpoint availability and OAuth flow need to be confirmed with Clay's current auth model before criterion 3 can be fully implemented.

## Notes / Rationale

**Why Clay API keys are blocked:** Clay has a beta program for API access. Keys are not publicly available as of 2026-02-22. The stdio path requires a key (`CLAY_API_KEY` env var passed to `npx @clayhq/clay-mcp`). This path is unusable until Clay opens key access.

**How the VIP workspace uses Clay without a key:** The `mcp__clay__searchContacts` tools in `~/Documents/VIP/.claude/settings.local.json` are Claude Code MCP tools. Clay is available as a first-party MCP integration through Anthropic for Pro/Max subscribers — authenticated through Anthropic's MCP gateway, not a direct Clay API key. DailyOS's Clay connector needs to replicate this: authenticate with Clay directly via their SSE endpoint and OAuth, not through Anthropic's gateway.

**The SSE implementation path:** `reqwest 0.12` is already in the project. Adding `stream` to its features enables `response.bytes_stream()` which is the right tool for SSE parsing. The MCP protocol over SSE is standard JSON-RPC: POST a request, parse `data:` lines from the streaming response. This is what `client.rs` lines 171-174 already describe as the intended approach.

**Transport enum cleanup:** The `Transport::Sse` variant in `client.rs` (line 136, currently "reserved/unused") becomes the active variant once SSE is implemented. `Transport::Stdio` is retained as a fallback for when Clay eventually opens API key access.
