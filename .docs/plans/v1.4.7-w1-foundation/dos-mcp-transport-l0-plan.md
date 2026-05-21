# v1.4.7 — MCP v2 Transport Ingress (rmcp ServerHandler → Gateway) — L0 plan packet

**Wave:** v1.4.7 W1.5 (post-W1-A + W1-B; pre-W2 handlers)
**Linear ticket:** TBD (mint after L0 approve)
**Authoring discipline:** narrow-scoped per K-in lessons. Substrate gaps file as separate Linear tickets.

## Cycle-3 changelog (2026-05-21)

Cycle-2 verdicts: 1 BLOCK + 3 NEEDS-CHANGES. 4/4 reviewers convergent: cycle-2 only updated the changelog at top, did NOT update the normative AC body — same pattern as DOS-478 cycle-4. Implementers could follow stale AC text and rebuild the rejected design. Plus identify-session needs cryptographic hardening (nonce consume + pre-identify lockdown + single-assignment + legacy executable identity check).

Cycle-3: full rewrite of §1 deliverable spec + §5 AC body + §7 test plan so they reflect the cycle-2 design (identify session bootstrap, arguments._dailyos_*, complete ToolError table, get_info, per-pairing filtering via identify, etc.). Changelog now describes what changed; AC body IS the source of truth.

Identify-session cryptographic hardening added per CSO + challenge cycle-2:
- identify_nonce is consumed (single-use) via existing `mcp_transport_nonce_ledger` substrate
- identify_nonce expires 60s after issuance (operator must use pair output promptly)
- Pre-identify lockdown: only `initialize`/`get_info`/`dailyos/identify` accepted; tools/list and tools/call return uniform `BadParams` until verified_client_id is set
- Single-assignment: second identify in same stdio process → uniform `BadParams`; verified_client_id not mutated
- Legacy coexistence: `--legacy-config-path` refusal uses canonical executable identity (compare resolved binary path against `which dailyos-mcp`), not just config-text matching

## 1. What this lane ships

The MCP v2 Gateway (W1-A) has no transport ingress — it's a Rust function nobody can call over the wire. Legacy `dailyos-mcp` binary uses `rmcp::ServerHandler` on stdio with a legacy `McpAbilityBridge`. This lane bridges `rmcp` to the v2 `Gateway` so MCP clients (Claude Desktop, Cursor, custom) invoke v2 tools through the W1-A trust contract.

### Architecture: stdio process = pairing session

Each `dailyos-mcp-v2` stdio process serves one verified pairing for the lifetime of the process. Lifecycle:

```
1. MCP client spawns `dailyos-mcp-v2` as subprocess (per claude_desktop_config.json)
2. Standard MCP `initialize` / `get_info` handshake (no auth state)
3. Client sends `dailyos/identify` { client_id, identify_nonce, signature } as
   the first non-initialize message
4. Server verifies HMAC, consumes identify_nonce in mcp_transport_nonce_ledger,
   sets `verified_client_id` = client_id, fails any subsequent identify
5. tools/list now returns the registered handler set FILTERED by mcp_tool_grant
   rows for verified_client_id where exposure = Invocable
6. tools/call extracts arguments._dailyos_envelope + _dailyos_signature, asserts
   envelope.client_id == verified_client_id, calls Gateway::handle_tool_call
7. Process exit (stdin EOF or operator kill) = session end; verified_client_id
   is process-scoped state, not persisted
```

### Deliverable 1: `src-tauri/src/services/mcp_v2/transport.rs` (NEW)

```rust
pub struct V2ServerHandler {
    gateway: Arc<Gateway>,
    catalog: Arc<dyn TaxonomyCatalog>,                         // see §4 trait extension
    db: Arc<Mutex<ActionDb>>,
    verified_client_id: Arc<RwLock<Option<McpClientId>>>,      // process-scoped, single-assignment
    server_info: ServerInfo,                                   // for rmcp::get_info
}

impl V2ServerHandler {
    pub fn new(
        gateway: Arc<Gateway>,
        catalog: Arc<dyn TaxonomyCatalog>,
        db: Arc<Mutex<ActionDb>>,
    ) -> Self { ... }
}

impl rmcp::ServerHandler for V2ServerHandler {
    fn get_info(&self) -> ServerInfo { ... }                   // not initialize; mirrors legacy
    async fn list_tools(...) -> Result<ListToolsResult, McpError> { ... }
    async fn call_tool(...) -> Result<CallToolResult, McpError> { ... }
    // No custom dailyos/identify method per rmcp::ServerHandler — see §2 #2
    // identify is handled via a method dispatch override OR a rmcp::ServerHandler
    // trait extension feature (rmcp 0.1 supports custom methods via the macros
    // module — implementation detail to verify at L1).
}
```

### Deliverable 2: `src-tauri/src/mcp_v2/main.rs` (NEW binary `dailyos-mcp-v2`)

```bash
# Subcommands
dailyos-mcp-v2 serve [--legacy-config-path <p>]   # stdio MCP server (default mode)
dailyos-mcp-v2 pair --client-name <s>             # pair a new client; print PairingResponse
                    --grant <tool>:<scope1,scope2>:<exposure>  # repeatable
                    [--format json|claude-desktop]
dailyos-mcp-v2 unpair --client-id <id>             # revoke a pairing
```

### Deliverable 3: `dailyos/identify` wire shape

```jsonrpc
// Request (client → server)
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "dailyos/identify",
  "params": {
    "client_id": "mcp_client_<hex>",
    "identify_nonce": "<hex>",
    "signature": "<hex>"
  }
}
// signature = HMAC-SHA256(transport_key, canonical_json({
//   client_id, identify_nonce, server_pid: <unix-pid-from-getpid>
// }))
// Note: server_pid binds the identify proof to the specific stdio process,
// preventing replay against a parallel session.
```

Response: `{ result: { ok: true } }` on success; uniform `BadParams` on any failure (unknown client, bad signature, nonce already consumed, nonce expired, signature pid mismatch).

### Deliverable 4: `tools/call` wire shape

```jsonrpc
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "dailyos.read.account_status",
    "arguments": {
      "subject": "acme",
      "_dailyos_envelope": { ... McpToolRequestEnvelope ... },
      "_dailyos_signature": "<hex>"
    }
  }
}
```

`_dailyos_envelope.client_id` MUST equal session's `verified_client_id`. `_dailyos_envelope.tool_name` MUST equal `request.name`. Reserved-key collision (real parameter named `_dailyos_*`) rejected with `BadParams { detail: "reserved key" }`.

## 2. What this lane does NOT ship

- Per-tool handlers (W2 / W3 / W4)
- Loopback HTTP transport (Phase 2; stdio first)
- MCP v1 deprecation — legacy binary stays per W1-A AC-10
- Production `BusSignalEmitter` (separate W1.5 lane)
- Operator pairing UI beyond CLI (post-v1.4.7)
- `client_label` schema column for pair-by-name revoke (filed as separate ticket — re-pair requires explicit `unpair --client-id` first)

## 3. Frozen substrate citations

- `rmcp = "0.1"` features `["server", "client", "transport-io", "transport-child-process", "macros"]` — already in Cargo.toml
- W1-A `Gateway::handle_tool_call(conn, asserted_client_id, envelope, signature)`
- W1-A `auth::pair_client(conn, ...) -> PairingResponse`
- W1-A `auth::verify_and_consume_and_preissue` — reused for identify_nonce consumption
- W1-A `mcp_transport_nonce_ledger` schema (v243) — identify_nonce stored same as request nonces
- W1-B `YamlTaxonomyCatalog::load_embedded` + `description_for`
- W1-B `Gateway::seal()` — returns pending catalog entries
- Legacy `src/mcp/main.rs:973-1031` — rmcp::ServerHandler impl precedent
- Legacy `src/mcp/main.rs:1312-1379` — stdio + stdout suppression precedent
- ADR-0128 §B — MCP as product surface
- ADR-0102 §C.bis.replay/refresh — HMAC + nonce per envelope (defense in depth even with identify)

## 4. K-in + substrate extensions

- **K-in**: grep `docs/solutions/` + `.docs/decisions/`. Legacy `dailyos-mcp` is the rmcp precedent; reused.
- **`TaxonomyCatalog` trait extension** (architect cycle-1): add `fn description_for(&self, name: &ScopedName) -> Option<&ToolDescription>` to the trait (it already exists on `YamlTaxonomyCatalog`); transport uses `Arc<dyn TaxonomyCatalog>`. One-line additive trait change.

## 5. Acceptance criteria (cycle-3 normative — supersedes cycle-1/cycle-2 AC text)

- **AC-1 rmcp::ServerHandler implemented.** `V2ServerHandler` implements `get_info` (NOT initialize — mirrors legacy `src/mcp/main.rs:973`), `list_tools`, `call_tool`. Custom `dailyos/identify` method handled per rmcp 0.1 macros pattern (implementation detail validated at L1).
- **AC-2 Wire shape pinned.** `tools/call` extracts `_dailyos_envelope` + `_dailyos_signature` from `arguments` (rmcp 0.1 only exposes name + arguments — `_meta` is not available on `CallToolRequestParam`). Asserts `request.name == envelope.tool_name`. Asserts `envelope.client_id == verified_client_id`. Reserved-key collision (real param named `_dailyos_*`) rejected with uniform `BadParams`.
- **AC-3 ToolError → rmcp::Error closed-matrix mapping.** Every `ToolError` variant has a row. Test asserts every variant; CI lint fails if a new variant lands without a row.

   | ToolError | rmcp::Error code | public `data.kind` | operator log fields |
   |---|---|---|---|
   | `Unauthorized { missing_scope }` | -32600 invalid_request | `unauthorized` | client_id, tool_name, missing_scope |
   | `BadParams { detail }` | -32602 invalid_params | `bad_params` | client_id, tool_name, detail (server-side log only — never on wire) |
   | `RateLimited { retry_after_seconds }` | -32099 custom | `rate_limited` | client_id, tool_name, retry_after_seconds |
   | `ExposureForbidden { tool_name }` | -32601 method_not_found | `exposure_forbidden` | client_id, tool_name |
   | `PairingRevoked` | -32600 invalid_request | `pairing_revoked` | client_id |
   | `ConversationRevoked` | -32600 invalid_request | `conversation_revoked` | client_id, conversation_handle |
   | `NotFound { entity }` | -32601 method_not_found | `not_found` | client_id, tool_name, entity (opaque ID only — no PII) |
   | `UpstreamFailure { detail }` | -32603 internal_error | `upstream_failure` | trace_id (opaque); detail logged server-side only — **never on wire** |
   | `Internal { trace_id }` | -32603 internal_error | `internal` | trace_id |

- **AC-4 Boot logs `mcp_v2 boot: 0 handlers registered, N catalog entries pending. tools/list will return empty for this build. Expected for W1.5 transport-only.`** With the embedded YAML (10 tools) and 0 handlers, N = 10. Strict mode env hint: `set DAILYOS_MCP_V2_REQUIRE_HANDLERS=1 to fail boot when no handlers registered`.
- **AC-5 `pair` CLI:**
  - `dailyos-mcp-v2 pair --client-name <s> --grant <tool>:<scope1>[,<scope2>...]:<invocable|metadata-only> [--grant ...] [--format json|claude-desktop]`
  - Writes `mcp_client_manifest` + `mcp_tool_grant` rows via `auth::pair_client` (no parallel write path)
  - `--format json` → emits `PairingResponse` as JSON to stdout (machine-parsable for piping)
  - `--format claude-desktop` → emits `{ "mcpServers": { "<client-name>": { "command": "dailyos-mcp-v2", "args": ["serve"], "env": { "DAILYOS_MCP_CLIENT_ID": "...", "DAILYOS_MCP_TRANSPORT_KEY": "..." } } } }` snippet
  - Stderr (never stdout): `WARNING: transport_key printed ONCE. Record it now — there is no recovery. Re-pair (via unpair + pair) regenerates a new key.`
  - **No "same name revokes existing"** — substrate lacks client_label column; operator must explicitly `unpair --client-id <id>` first. Filed as separate ticket.
- **AC-6 `tools/list` filtered + composed description.** Returns only registered handlers WHOSE `mcp_tool_grant` row for `verified_client_id` has `exposure = Invocable`. Each `rmcp::Tool` carries:
  - `name` = catalog entry name
  - `description` = `format!("{summary}\n\nWhen to call:\n{when_to_call}\n\nWhen NOT to call:\n{when_not_to_call}")`
  - `input_schema` = JSON Schema for tool params (from `ToolDescription.parameters`) + advertised `_dailyos_envelope` + `_dailyos_signature` properties (both required, both `string` opaque shape; clients construct via documented HMAC procedure)
  - Empty registered set OR no Invocable grants for this pairing → empty `tools` list (valid MCP response)
- **AC-7 Legacy `dailyos-mcp` binary unchanged.** Build matrix verifies both `dailyos-mcp` AND `dailyos-mcp-v2` compile + bin paths don't collide. W1-A AC-10 coexistence preserved.
- **AC-8 Integration tests** (covers AC-1..AC-15):
  - Unit: direct `V2ServerHandler::call_tool` exercises wrapper extraction + identify gate + Gateway dispatch via stub handler
  - **Subprocess JSON-RPC handshake**: spawn `dailyos-mcp-v2 serve` as child process; perform full MCP `initialize` → `dailyos/identify` → `tools/list` → `tools/call` sequence over stdin/stdout; assert stdout is protocol-only (no boot log corruption per legacy `src/mcp/main.rs:1312` precedent)
  - Negative subprocess tests: pre-identify `tools/list` rejected, pre-identify `tools/call` rejected, second identify rejected, identify with wrong server_pid rejected, captured identify replayed against fresh subprocess rejected (nonce already consumed)
- **AC-9 (DELETED — deferred filtering is no longer the design).** Per-pairing `list_tools` filtering is REQUIRED via identify session per AC-6.
- **AC-10 Required checks.** `cargo build --features mcp --bin dailyos-mcp-v2` clean. `cargo clippy --features mcp -- -D warnings` clean. `cargo test --features mcp` passes including subprocess tests.
- **AC-11 Legacy executable identity guardrail.** `dailyos-mcp-v2 serve --legacy-config-path <p>` parses the config and refuses to start if it claims v2-owned tool names AND points at a binary path that resolves to legacy `dailyos-mcp` (compare against canonical resolved path via `which dailyos-mcp` or its absolute equivalent). Error message: `dailyos-mcp-v2: refusing to start. Config at <p> claims v2-owned tools [<list>] routed through legacy binary at <resolved-path>. Run 'dailyos-mcp-v2 migrate-config <p>' to update.` (Migrate-config is a separate ticket; for v1.4.7 W1.5 the refusal alone is sufficient.) Negative test: synthetic config triggers refusal.
- **AC-12 Identify nonce consumed-once + expired.** `dailyos/identify` consumes the identify_nonce via `auth::verify_and_consume_and_preissue` reusing W1-A's `mcp_transport_nonce_ledger` substrate. Nonces issued by `pair` CLI have `expires_at = paired_at + 60s`. Replay of consumed nonce → uniform `BadParams`. Expired nonce → uniform `BadParams`.
- **AC-13 Pre-identify lockdown.** Before `verified_client_id` is set, only `initialize`, `get_info`, and `dailyos/identify` MCP methods are accepted. `tools/list`, `tools/call`, and any other method → uniform `BadParams { detail: "pre-identify" }` (kind: `pre_identify`, not distinguishing why-we-rejected).
- **AC-14 Single-assignment session.** After `verified_client_id` is set, a second `dailyos/identify` request → uniform `BadParams` (kind: `already_identified`); `verified_client_id` is NOT mutated. Same applies to identify for a different `client_id`.
- **AC-15 Identify HMAC includes server_pid.** `signature = HMAC-SHA256(transport_key, canonical_json({ client_id, identify_nonce, server_pid }))` where server_pid is read from `getpid()` at HMAC verification time. Captured identify from another stdio session has a different server_pid and fails verification. Tests: same identify replayed against fresh subprocess (different pid) → BadParams.

## 6. Files owned

| File | State | Owner |
|---|---|---|
| `src-tauri/src/services/mcp_v2/transport.rs` | NEW | exclusive |
| `src-tauri/src/services/mcp_v2/taxonomy.rs` | additive — `description_for` method moves to trait | shared (one-line) |
| `src-tauri/src/services/mcp_v2/mod.rs` | additive — `pub mod transport;` | shared (one-line) |
| `src-tauri/src/mcp_v2/main.rs` | NEW (binary entry, with serve / pair / unpair subcommands) | exclusive |
| `src-tauri/Cargo.toml` | additive — `[[bin]]` block for `dailyos-mcp-v2` matching legacy pattern | shared (additive) |
| `src-tauri/tests/dos_mcp_transport_test.rs` | NEW | exclusive |
| `src-tauri/tests/dos_mcp_transport_subprocess_test.rs` | NEW | exclusive |

## 7. Test plan (per-AC coverage)

- **AC-1**: unit test asserts `get_info` returns expected ServerInfo shape with version + capabilities
- **AC-2**: unit tests for wrapper extraction (well-formed extract success; missing `_dailyos_envelope` → BadParams; reserved-key collision → BadParams; `request.name != envelope.tool_name` → BadParams; `envelope.client_id != verified_client_id` → BadParams)
- **AC-3**: for-each `ToolError` variant assert correct rmcp::Error code + public data.kind (closed matrix; CI lint asserts no orphan variant)
- **AC-4**: subprocess boot test parses stderr boot log line matches exact format
- **AC-5**: subprocess test runs `pair --client-name test --grant dailyos.read.account_status:dailyos.read.account_status:invocable --format claude-desktop` — asserts stdout is parseable claude-desktop JSON, stderr contains warning, `mcp_client_manifest` + `mcp_tool_grant` rows present
- **AC-6**: subprocess test pairs with grant for tool A but not B; both A and B are catalog entries; after identify, `tools/list` returns [A] not [B]; description text contains "When to call:" and "When NOT to call:" sections
- **AC-7**: build matrix asserts both binaries compile + can run from same target/debug dir
- **AC-8**: subprocess full handshake test (covered above)
- **AC-10**: ci script
- **AC-11**: synthetic config triggers refusal; error message matches expected text
- **AC-12**: subprocess identify replay returns BadParams; pair output then 61s sleep then identify → BadParams (expired)
- **AC-13**: pre-identify `tools/list` → BadParams; pre-identify `tools/call` → BadParams
- **AC-14**: subprocess identify twice → second returns BadParams; verified_client_id check via probe
- **AC-15**: captured identify replayed against fresh subprocess (different pid) → BadParams

## 8. Security gates

`/cso` mandatory — trust boundary. Cycle-2 CSO already validated the W1-A substrate holds against transport-layer attacks; cycle-3 must verify the identify-session hardening (AC-12..AC-15) closes the cycle-2 replay/multi-identify/legacy-bypass vectors.

## 9. Path-α (separate Linear tickets)

- `client_label` schema column for pair-by-name revoke (cycle-2 finding)
- `dailyos-mcp-v2 migrate-config` subcommand referenced in AC-11 error message
- Loopback HTTP transport (Phase 2)
- Tauri operator pairing UI (post-v1.4.7)
- MCP client SDK code samples + integration docs

## 10. Depends-on

- W1-A merged (in this PR #347): Gateway + auth + nonce ledger substrate
- W1-B merged (in this PR #347): YamlTaxonomyCatalog + Gateway::seal
- rmcp crate already in dependencies

## 11. Definition of Done

§5 AC-1..AC-15 met. L0 unanimous APPROVE. L2 unanimous APPROVE bounded by AC. Commit-msg `L2-status: passed`. Pushed to PR #347. CI green including new subprocess tests.

## 12. Open questions resolved in cycle 3

| # | Question | Resolution |
|---|---|---|
| Q1 | rmcp wrapper format | `arguments._dailyos_envelope/_dailyos_signature` (cycle-2 4/4 convergent; rmcp 0.1 only exposes arguments on CallToolRequestParam) |
| Q2 | pair CLI argparse | clap (matches existing CLI patterns) |
| Q3 | empty handler set at boot | warning + strict-mode env hint (cycle-2 architect+devex) |
| Q4 | `--http` flag | path-α (rmcp HTTP transport is separate adapter; stdio sufficient for v1.4.7 W1.5) |
| Q5 (cycle-3 NEW) | identify_nonce expiry window | 60s (operator runs `pair` then immediately pastes into client config; longer windows = larger replay window) |
| Q6 (cycle-3 NEW) | server_pid binding | yes (cycle-2 challenge replay attack defense) |

## 13. Reviewer dispatch

- **CSO** (mandatory)
- **/codex challenge** (adversarial)
- **architect-reviewer** (rmcp + bin/lib + identify-session state)
- **/plan-devex-review** (CLI + JSON-RPC shapes + integration DX)
