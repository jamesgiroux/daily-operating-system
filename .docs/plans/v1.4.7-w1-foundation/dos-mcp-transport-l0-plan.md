# v1.4.7 — MCP v2 Transport Ingress (rmcp ServerHandler → Gateway) — L0 plan packet

**Wave:** v1.4.7 W1.5 (post-W1-A + W1-B; pre-W2 handlers)
**Linear ticket:** TBD (mint after L0 approve)
**Authoring discipline:** narrow-scoped per K-in lessons. Substrate gaps file as separate Linear tickets.

## Cycle-4 changelog (2026-05-21)

Cycle-3 verdicts: 1 BLOCK (devex) + 3 NEEDS-CHANGES (CSO + architect + challenge). 3/4 reviewers convergent on **identify-subprotocol fundamentally broken**:
- **Devex BLOCK**: standard MCP clients (Claude Desktop, Cursor) call only spec methods — they will never send `dailyos/identify`. Pre-identify lockdown turns the whole server into a brick from a standard client.
- **Architect critical**: single-use 60s identify nonce makes Claude Desktop subprocess restart impossible.
- **CSO**: identify consumes a ledger nonce but doesn't return `next_request_nonce`, pressuring implementers to bypass Gateway nonce verification.

**Cycle-4 architectural pivot: drop the identify subprotocol entirely.** Identity flows via process environment at startup, not via JSON-RPC message:

```
pair CLI emits config snippet with env block:
  env: { DAILYOS_MCP_CLIENT_ID, DAILYOS_MCP_TRANSPORT_KEY }

Subprocess startup reads env, cross-checks against mcp_client_manifest +
keychain. Standard MCP initialize → tools/list (filtered by env client_id's
grants) → tools/call (wrapper still HMAC-verifies + consumes nonce per call
via W1-A Gateway substrate).
```

Threat model collapses to "machine-local trust" (same as transport_key being in env at all). Per-envelope HMAC + nonce ledger still provides replay protection. Works with standard MCP clients without custom protocol extension.

Cycle-4 changes:
1. Drop `dailyos/identify` JSON-RPC method + AC-12 (consume) + AC-13 (pre-identify lockdown) + AC-14 (single-assignment) + AC-15 (server_pid binding)
2. New AC-12: startup env-assertion (read env vars, look up manifest, cross-check transport_key against keychain via `keychain_ref`, fail-fast with operator-readable error)
3. AC-6 unchanged in semantics (per-pairing `tools/list` filtering) but driven by env-asserted client_id at startup, not session identify state
4. AC-2 wire shape unchanged (still `arguments._dailyos_envelope/_signature`)
5. Per-call envelope HMAC canonicalization spec: RFC 8785 JCS, with golden cross-language fixtures (challenge cycle-3 #1)
6. AC-3 typo: `NotFound { resource }` not `{ entity }` (devex cycle-3)
7. AC-6 `_dailyos_envelope` is JSON object schema (challenge cycle-3 + architect cycle-3 alignment)
8. AC-11 hardlinks/copies acknowledgment + binary self-identification fallback (challenge cycle-3 #4)
9. `pair --format claude-desktop` snippet emits env block (not identify_nonce); becomes the standard MCP-client-compatible deliverable

## 1. What this lane ships

The MCP v2 Gateway (W1-A) has no transport ingress — it's a Rust function nobody can call over the wire. Legacy `dailyos-mcp` binary uses `rmcp::ServerHandler` on stdio with a legacy `McpAbilityBridge`. This lane bridges `rmcp` to the v2 `Gateway` so standard MCP clients (Claude Desktop, Cursor, custom) invoke v2 tools through the W1-A trust contract.

### Architecture: env-asserted identity, per-envelope HMAC

Each `dailyos-mcp-v2` stdio process serves one verified pairing for its lifetime. Identity is asserted via environment variables at process startup; per-call envelopes still HMAC-verify and consume the W1-A nonce ledger per invocation (defense in depth).

Lifecycle:

```
1. Operator runs `dailyos-mcp-v2 pair --client-name claude --grant ... --format claude-desktop`
2. Operator pastes the emitted snippet into claude_desktop_config.json:
     {
       "mcpServers": {
         "dailyos": {
           "command": "dailyos-mcp-v2",
           "args": ["serve"],
           "env": {
             "DAILYOS_MCP_CLIENT_ID": "mcp_client_<hex>",
             "DAILYOS_MCP_TRANSPORT_KEY": "<hex>"
           }
         }
       }
     }
3. Claude Desktop spawns `dailyos-mcp-v2 serve` with those env vars set
4. Subprocess startup:
     a. Read DAILYOS_MCP_CLIENT_ID + DAILYOS_MCP_TRANSPORT_KEY from env
     b. Look up `mcp_client_manifest` row for that client_id; verify exists + not revoked
     c. Read transport_key from keychain (via row.keychain_ref); cross-check
        against env-asserted key; reject on mismatch
     d. Store `verified_client_id` in V2ServerHandler (process-scoped, immutable)
     e. Log boot status; begin serving rmcp::ServerHandler
5. Standard MCP `initialize` / `get_info` handshake (no auth state beyond what's
   already set at startup)
6. `tools/list` returns registered handler set FILTERED by `mcp_tool_grant` rows
   for verified_client_id where exposure = Invocable
7. `tools/call` extracts arguments._dailyos_envelope + _dailyos_signature,
   asserts envelope.client_id == verified_client_id + envelope.tool_name ==
   request.name, calls Gateway::handle_tool_call (HMAC + nonce + dispatch per
   W1-A substrate)
8. Process exit = session end; no persistent state
```

### Deliverable 1: `src-tauri/src/services/mcp_v2/transport.rs` (NEW)

```rust
pub struct V2ServerHandler {
    gateway: Arc<Gateway>,
    catalog: Arc<dyn TaxonomyCatalog>,         // requires §4 trait extension
    db: Arc<Mutex<ActionDb>>,
    verified_client_id: McpClientId,           // set at construction; immutable
    server_info: ServerInfo,
}

impl V2ServerHandler {
    /// Constructed AFTER successful env-assertion + manifest lookup +
    /// keychain cross-check in main.rs. Construction failure (any of those
    /// checks fail) exits the process before the rmcp service runs.
    pub fn from_verified_pairing(
        gateway: Arc<Gateway>,
        catalog: Arc<dyn TaxonomyCatalog>,
        db: Arc<Mutex<ActionDb>>,
        verified_client_id: McpClientId,
    ) -> Self { ... }
}

impl rmcp::ServerHandler for V2ServerHandler {
    fn get_info(&self) -> ServerInfo { ... }                    // not initialize
    async fn list_tools(...) -> Result<ListToolsResult, McpError> { ... }
    async fn call_tool(...) -> Result<CallToolResult, McpError> { ... }
}
```

### Deliverable 2: `src-tauri/src/mcp_v2/main.rs` (NEW binary `dailyos-mcp-v2`)

```bash
# Subcommands
dailyos-mcp-v2 serve [--legacy-config-path <p>]   # stdio MCP server
dailyos-mcp-v2 pair --client-name <s>             # pair a new client
                    --grant <tool>:<scope1,scope2>:<exposure>  # repeatable
                    [--format json|claude-desktop]
dailyos-mcp-v2 unpair --client-id <id>             # revoke a pairing
```

`serve` reads env at startup. Missing env vars OR unknown client_id OR revoked pairing OR keychain mismatch → exit code 1 with operator-readable error to stderr.

### Deliverable 3: `tools/call` wire shape

```jsonrpc
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "dailyos.read.account_status",
    "arguments": {
      "subject": "acme",
      "_dailyos_envelope": {
        "client_id": "mcp_client_<hex>",
        "tool_name": "dailyos.read.account_status",
        "request_nonce": "<hex>",
        "params": { "subject": "acme" },
        "conversation_handle": null,
        "tool_grant_id": null
      },
      "_dailyos_signature": "<hex>"
    }
  }
}

// signature = HMAC-SHA256(
//   transport_key,
//   JCS_canonical_json(_dailyos_envelope)
// )
// where JCS_canonical_json is RFC 8785 JSON Canonicalization Scheme:
// UTF-8, sorted keys, no insignificant whitespace, RFC 8259 number form.
//
// Cross-language golden fixtures shipped in tests/fixtures/jcs_*.json
// so SDK authors in Python/TypeScript/Go can verify their implementation.
```

### Deliverable 4: `pair --format claude-desktop` output

```json
{
  "mcpServers": {
    "<client-name>": {
      "command": "dailyos-mcp-v2",
      "args": ["serve"],
      "env": {
        "DAILYOS_MCP_CLIENT_ID": "mcp_client_<hex>",
        "DAILYOS_MCP_TRANSPORT_KEY": "<hex>"
      }
    }
  }
}
```

Stderr (never stdout): `WARNING: DAILYOS_MCP_TRANSPORT_KEY printed ONCE. Record it now — there is no recovery. Re-pair via 'unpair --client-id <id>' then 'pair' regenerates a new key.`

## 2. What this lane does NOT ship

- Per-tool handlers (W2 / W3 / W4)
- Loopback HTTP transport (Phase 2)
- MCP v1 deprecation — legacy binary stays per W1-A AC-10
- Production `BusSignalEmitter` (separate W1.5 lane)
- Operator pairing UI beyond CLI (post-v1.4.7)
- `client_label` schema column for pair-by-name revoke (separate ticket)
- Per-process keychain ACLs to gate transport_key reads (the env-assertion model assumes the operator's machine is trusted; tightening to per-process keychain ACL is post-v1.4.7)

## 3. Frozen substrate citations

- `rmcp = "0.1"` already in Cargo.toml; legacy precedent at `src/mcp/main.rs:973-1031` + `:1312-1379`
- W1-A `Gateway::handle_tool_call` — dispatch entry called per envelope
- W1-A `auth::pair_client`, `auth::load_client_record`, `auth::verify_transport_hmac`, `auth::verify_and_consume_and_preissue` — all reused; no parallel paths
- W1-A keychain via `keychain_ref` in `mcp_client_manifest` — env-asserted key cross-checked against this
- W1-A `mcp_transport_nonce_ledger` — request_nonce ledger consumed per envelope
- W1-B `YamlTaxonomyCatalog::description_for` (will move to trait per §4)
- W1-B `Gateway::seal()`
- ADR-0128 §B — MCP as product surface
- ADR-0102 §C.bis.replay/refresh — per-envelope HMAC + nonce

## 4. K-in + substrate extensions

- **K-in**: grep `docs/solutions/` + `.docs/decisions/`. Legacy `dailyos-mcp` is the rmcp precedent.
- **`TaxonomyCatalog` trait extension** (architect cycle-1): add `fn description_for(&self, name: &ScopedName) -> Option<&ToolDescription>` to the trait. One-line additive.

## 5. Acceptance criteria (cycle-4 normative — supersedes cycle-1/2/3)

- **AC-1 rmcp::ServerHandler implemented.** `V2ServerHandler` implements `get_info` (mirrors legacy `src/mcp/main.rs:973`), `list_tools`, `call_tool`. No custom JSON-RPC methods.
- **AC-2 Wire shape pinned.** `tools/call` extracts `_dailyos_envelope` + `_dailyos_signature` from `arguments` (rmcp 0.1 exposes only name + arguments on CallToolRequestParam). Asserts `request.name == envelope.tool_name`. Asserts `envelope.client_id == verified_client_id`. Reserved-key collision (real param `_dailyos_*`) rejected with uniform `BadParams { detail: "reserved key" }`.
- **AC-3 ToolError → rmcp::Error closed-matrix mapping.** Every `ToolError` variant has a row. Test asserts every variant; CI lint fails on orphan variant.

   | ToolError | rmcp::Error code | public `data.kind` | operator log fields |
   |---|---|---|---|
   | `Unauthorized { missing_scope }` | -32600 invalid_request | `unauthorized` | client_id, tool_name, missing_scope |
   | `BadParams { detail }` | -32602 invalid_params | `bad_params` | client_id, tool_name, detail (server-side log only — never on wire) |
   | `RateLimited { retry_after_seconds }` | -32099 custom | `rate_limited` | client_id, tool_name, retry_after_seconds |
   | `ExposureForbidden { tool_name }` | -32601 method_not_found | `exposure_forbidden` | client_id, tool_name |
   | `PairingRevoked` | -32600 invalid_request | `pairing_revoked` | client_id |
   | `ConversationRevoked` | -32600 invalid_request | `conversation_revoked` | client_id, conversation_handle |
   | `NotFound { resource }` | -32601 method_not_found | `not_found` | client_id, tool_name, resource (opaque ID only — no PII) |
   | `UpstreamFailure { detail }` | -32603 internal_error | `upstream_failure` | trace_id (opaque); detail logged server-side only |
   | `Internal { trace_id }` | -32603 internal_error | `internal` | trace_id |

- **AC-4 Boot logs.** Format: `mcp_v2 boot: pairing <client_id> verified, 0 handlers registered, N catalog entries pending. tools/list will return empty for this build. Expected for W1.5 transport-only.` With embedded YAML (10 tools) and 0 handlers, N = 10. Strict mode env: `DAILYOS_MCP_V2_REQUIRE_HANDLERS=1` fails boot if no handlers registered.
- **AC-5 `pair` CLI:**
  - `dailyos-mcp-v2 pair --client-name <s> --grant <tool>:<scope1>[,<scope2>...]:<invocable|metadata-only> [--grant ...] [--format json|claude-desktop]`
  - Writes `mcp_client_manifest` + `mcp_tool_grant` via `auth::pair_client` (no parallel write path)
  - `--format json` → machine-parsable `PairingResponse` to stdout
  - `--format claude-desktop` → MCP-client-compatible `{"mcpServers": {...}}` snippet (env block with DAILYOS_MCP_CLIENT_ID + DAILYOS_MCP_TRANSPORT_KEY)
  - Stderr: "WARNING: DAILYOS_MCP_TRANSPORT_KEY printed ONCE. Record it now — there is no recovery."
  - **No "same name revokes existing"**: operator must `unpair --client-id <id>` first; separate ticket for `client_label` substrate.
- **AC-6 `tools/list` filtered + composed description.** Returns only registered handlers WHOSE `mcp_tool_grant` row for `verified_client_id` has `exposure = Invocable`. Each `rmcp::Tool` carries:
  - `name` = catalog entry name (ScopedName)
  - `description` = `format!("{summary}\n\nWhen to call:\n{when_to_call}\n\nWhen NOT to call:\n{when_not_to_call}")`
  - `input_schema` = JSON Schema with tool params from `ToolDescription.parameters` + required `_dailyos_envelope` (type: object, opaque to client; reference doc URL) + required `_dailyos_signature` (type: string, hex)
  - Empty filtered set → empty `tools` list (valid MCP response).
- **AC-7 Legacy `dailyos-mcp` binary unchanged.** Build matrix verifies both binaries compile + bin paths don't collide. W1-A AC-10 coexistence preserved.
- **AC-8 Integration tests** (cover AC-1..AC-13):
  - Unit: `V2ServerHandler::call_tool` direct invocation with wrapper extraction + verified_client_id assertion + Gateway dispatch via stub handler
  - **Subprocess test**: spawn `dailyos-mcp-v2 serve` with env vars; perform full MCP `initialize` → `tools/list` → `tools/call` sequence over stdin/stdout; assert stdout is protocol-only (no boot log corruption per legacy `src/mcp/main.rs:1312` precedent)
  - Subprocess negative tests: missing env var → exit 1 with stderr error; unknown client_id env → exit 1; revoked pairing env → exit 1; keychain mismatch → exit 1; tools/call with envelope.client_id != verified_client_id → BadParams; tools/call with wrong HMAC → BadParams; replay (consumed nonce) → BadParams
- **AC-9 (DELETED — per-pairing filtering is now REQUIRED via AC-6, not deferred)**
- **AC-10 Required checks.** `cargo build --features mcp --bin dailyos-mcp-v2` clean. `cargo clippy --features mcp -- -D warnings` clean. `cargo test --features mcp` passes including subprocess tests.
- **AC-11 Legacy executable identity guardrail.** `serve --legacy-config-path <p>` parses the config and refuses to start if it claims v2-owned tool names AND points at a binary that resolves (via filesystem canonicalization following symlinks) to legacy `dailyos-mcp`. Error: `dailyos-mcp-v2: refusing to start. Config at <p> claims v2-owned tools [<list>] routed through legacy binary at <resolved-path>. Run 'dailyos-mcp-v2 migrate-config <p>' to update.` **Hardlinks/copies acknowledged limitation** (per challenge cycle-3): canonical path resolution catches symlinks; hardlinks and copies are indistinguishable from independent binaries at filesystem level. Path-α fallback: binary self-identification via `<binary> --version-id` returning a stable internal identifier; if `--legacy-config-path` is provided AND the configured binary's `--version-id` returns the legacy identifier, refuse. Filed as separate ticket if `which`-based canonicalization proves insufficient in practice.
- **AC-12 Startup env-assertion** (replaces cycle-3 AC-12/13/14/15). `serve` subcommand:
  1. Read `DAILYOS_MCP_CLIENT_ID` + `DAILYOS_MCP_TRANSPORT_KEY` from process env. Missing → exit 1, stderr: `dailyos-mcp-v2: missing required env var <NAME>. See 'dailyos-mcp-v2 pair --format claude-desktop' output.`
  2. Look up `mcp_client_manifest` row for client_id. Not found → exit 1, stderr: `dailyos-mcp-v2: client_id <id> not paired. Run 'pair' first.`
  3. Row.revoked_at != NULL → exit 1, stderr: `dailyos-mcp-v2: pairing for <id> revoked at <ts>. Re-pair.`
  4. Read transport_key from keychain via `row.keychain_ref`; cross-check against env-asserted key (Zeroizing comparison, constant-time). Mismatch → exit 1, stderr: `dailyos-mcp-v2: transport_key mismatch for <id>. Env value does not match keychain. Re-pair to regenerate.`
  5. All checks pass → construct `V2ServerHandler::from_verified_pairing(...)`, begin serving.
  Env vars are wiped from the process's own environment after reading (`unsafe { std::env::remove_var }`) so subprocess inspection via `/proc/self/environ` minimizes the surface time the key is visible.
- **AC-13 Per-envelope replay protection via W1-A substrate (unchanged behavior).** `tools/call` calls `Gateway::handle_tool_call` which already invokes `auth::verify_transport_hmac` + `auth::verify_and_consume_and_preissue`. Per-envelope HMAC + nonce consumption is the defense-in-depth layer beyond startup env-assertion. No new substrate needed.
- **AC-14 RFC 8785 JCS canonicalization** (replaces cycle-3 server_pid binding). HMAC input is RFC 8785 JSON Canonicalization Scheme (JCS) of the `_dailyos_envelope` JSON object. UTF-8, sorted keys, no insignificant whitespace, RFC 8259 number form. Golden cross-language fixtures shipped in `src-tauri/tests/fixtures/jcs_envelope_*.json` covering ASCII + Unicode + escape sequences + nested objects + arrays + integers/floats/nulls so SDK authors in Python/TypeScript/Go can verify their implementation matches.

## 6. Files owned

| File | State | Owner |
|---|---|---|
| `src-tauri/src/services/mcp_v2/transport.rs` | NEW | exclusive |
| `src-tauri/src/services/mcp_v2/taxonomy.rs` | additive — `description_for` moves to trait | shared (one-line) |
| `src-tauri/src/services/mcp_v2/mod.rs` | additive — `pub mod transport;` | shared (one-line) |
| `src-tauri/src/mcp_v2/main.rs` | NEW (binary with serve / pair / unpair) | exclusive |
| `src-tauri/Cargo.toml` | additive — `[[bin]]` block for `dailyos-mcp-v2` matching legacy pattern | shared (additive) |
| `src-tauri/tests/dos_mcp_transport_test.rs` | NEW | exclusive |
| `src-tauri/tests/dos_mcp_transport_subprocess_test.rs` | NEW | exclusive |
| `src-tauri/tests/fixtures/jcs_envelope_*.json` | NEW (5-10 golden fixtures) | exclusive |

## 7. Test plan (per-AC)

- **AC-1**: unit test asserts `get_info` returns expected ServerInfo
- **AC-2**: unit tests for wrapper extraction (well-formed, missing key, reserved-key collision, name mismatch, client_id mismatch)
- **AC-3**: for-each `ToolError` variant assert correct rmcp::Error code; CI lint asserts no orphan variant
- **AC-4**: subprocess boot test parses stderr boot log against exact format
- **AC-5**: subprocess `pair` run asserts stdout parseable, stderr warning present, DB rows match
- **AC-6**: subprocess test pairs with grant for tool A but not B; asserts tools/list returns [A] not [B]; description contains "When to call:" + "When NOT to call:" sections
- **AC-7**: build matrix asserts both binaries compile
- **AC-8**: subprocess full handshake test (covered above)
- **AC-10**: CI script
- **AC-11**: synthetic config triggers refusal; error message matches
- **AC-12**: subprocess tests for each failure mode (missing env, unknown client, revoked pairing, key mismatch); env-wipe verification via `/proc/self/environ` check
- **AC-13**: subprocess test replays consumed nonce → BadParams (already covered by W1-A substrate unit tests; this lane just asserts the integration path)
- **AC-14**: golden fixture tests assert JCS canonicalization matches across Python (reference impl), Rust (this impl), and a manual hex-dump of expected bytes for at least one fixture

## 8. Security gates

`/cso` mandatory. Cycle-4 must verify the env-assertion model (AC-12) is acceptable threat-model-wise: machine-local trust assumption + env-wipe after read + per-envelope HMAC defense-in-depth via W1-A substrate. Specifically verify no degradation vs cycle-3's identify model on attacker capabilities given attacker-in-env.

## 9. Path-α (separate Linear tickets)

- `client_label` schema for pair-by-name revoke (cycle-2)
- `dailyos-mcp-v2 migrate-config` subcommand (AC-11 error reference)
- Binary self-identification via `--version-id` if `which`-canonicalization insufficient (AC-11 challenge cycle-3)
- Per-process keychain ACL to gate transport_key reads (cycle-4 threat-model tightening)
- Loopback HTTP transport (Phase 2)
- Tauri operator pairing UI (post-v1.4.7)
- MCP client SDK code samples + JCS reference implementations

## 10. Depends-on

- W1-A merged (PR #347): Gateway + auth + nonce ledger + keychain substrate
- W1-B merged (PR #347): YamlTaxonomyCatalog + Gateway::seal
- rmcp crate already in deps

## 11. Definition of Done

§5 AC-1..AC-14 met. L0 unanimous APPROVE. L2 unanimous APPROVE bounded by AC. Commit-msg `L2-status: passed`. Pushed to PR #347. CI green including subprocess + JCS golden tests.

## 12. Open questions resolved

| # | Question | Resolution |
|---|---|---|
| Q1 | rmcp wrapper format | `arguments._dailyos_envelope/_dailyos_signature` (cycle-2 4/4) |
| Q2 | pair CLI argparse | clap |
| Q3 | empty handler set at boot | warning + strict-mode env |
| Q4 | HTTP transport | path-α |
| Q5 | identify_nonce expiry | DELETED (cycle-4 pivot to env-assertion; no nonce in startup path) |
| Q6 | server_pid binding | DELETED (cycle-4 pivot; JCS canonicalization replaces) |
| Q7 (cycle-4 NEW) | env-wipe after read | yes (AC-12 step 5; minimizes /proc/self/environ surface time) |
| Q8 (cycle-4 NEW) | JCS canonicalization | RFC 8785 with golden cross-language fixtures |

## 13. Reviewer dispatch

- **CSO** (mandatory; cycle-4 pivot needs fresh security review of env-assertion model)
- **/codex challenge** (adversarial; especially attacker-has-env threat model)
- **architect-reviewer** (env-assertion architecture; constructor pattern)
- **/plan-devex-review** (Claude Desktop config integration UX; JCS SDK fixtures)
