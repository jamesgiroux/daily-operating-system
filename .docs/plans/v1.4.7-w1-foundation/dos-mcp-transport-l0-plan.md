# v1.4.7 — MCP v2 Transport Ingress (rmcp ServerHandler → Gateway) — L0 plan packet

**Wave:** v1.4.7 W1.5 (post-W1-A + W1-B; pre-W2 handlers)
**Linear ticket:** TBD (mint after L0 approve)
**Scope:** local-to-local same-machine only. No remote MCP transport in scope. Trust boundary is "same user on this machine."
**Authoring discipline:** narrow-scoped per K-in lessons.

## Cycle-5 changelog (2026-05-21)

Cycle-4 verdicts: architect APPROVE, challenge NEEDS-CHANGES, CSO BLOCK, devex BLOCK. CSO + devex convergent on per-envelope HMAC client-incompatibility for standard MCP clients (Claude Desktop, Cursor don't compute HMAC); devex provided the concrete fix.

**Cycle-5 architectural commit (user-directed):** Option A — startup identity is the auth boundary for stdio; transport self-signs internally. Scope explicitly local-to-local same-machine. No remote MCP transport in v1.4.7.

Cycle-5 fixes:

1. **Trust model differentiated by transport class.** Stdio transport (Claude Desktop, Cursor): auth = env-asserted startup identity. Per-envelope HMAC + nonce ledger = transport-internal hygiene (consistent with W1-A substrate; zero attacker value for this transport class given the local-to-local scope). WP adapter (v1.4.2 W3-C) + custom SDK clients: per-envelope HMAC authenticates the middleware/SDK to the gateway (real client auth at the envelope boundary). Gateway substrate unchanged — it always verifies HMAC + consumes nonce; the transport decides where the trust boundary lives.

2. **Transport self-signs internally** (devex BLOCK fix). `tools/call` accepts normal MCP tool params from Claude Desktop / Cursor (no `_dailyos_*` keys in `arguments`). Transport reads `verified_client_id` + transport_key (process-held) + assigns request_nonce (from W1-A preissue ledger), constructs `McpToolRequestEnvelope`, signs with JCS canonicalization, calls `Gateway::handle_tool_call`. `_dailyos_envelope` and `_dailyos_signature` are NOT exposed in public `tools/list` input_schema.

3. **Scope statement: local-to-local same-machine** (user direction). Documented in §0 scope. The /proc/<pid>/environ exposure (challenge + CSO cycle-4) is acceptable because any attacker who can read it is already same-user same-machine and has access to ~/.dailyos/keychain etc. env-wipe via `std::env::remove_var` reworded as best-effort in-process hygiene (limits subsequent `getenv` reads in same process), not a /proc mitigation.

4. **AC-3 add public `message` field** (devex cycle-4). rmcp::Error carries both `code` + `data.kind` + a short human-readable `message` so MCP clients can render a meaningful error.

5. **AC-6 input_schema clean** — only handler params + `_dailyos_*` excluded (devex cycle-4 + transport self-signs).

6. **Cursor compatibility — `"type": "stdio"`** (devex cycle-4). `pair --format claude-desktop` snippet includes `"type": "stdio"` for Cursor compatibility (harmless for Claude Desktop).

7. **JCS hardening** (challenge + CSO cycle-4). Duplicate-key rejection at all object levels; I-JSON-compatible values; golden fixtures expanded to cover ASCII + Unicode (no normalization) + escape sequences + nested arrays/objects + floats + nulls + integer edge cases. Fixtures shipped for SDK authors of WP adapter / custom SDK clients (real auth path).

8. **claude_desktop_config.json file perms note** (challenge cycle-4). `pair --format claude-desktop` stderr warning extended: "Recommend `chmod 600` on claude_desktop_config.json; the transport_key is in the env block." Soft guidance — local-to-local scope means file perms are advisory, not load-bearing.

9. **`McpToolRequestEnvelope` has no client_id** (devex cycle-4 confirmed). Transport passes `verified_client_id` as the `asserted_client_id` argument to `Gateway::handle_tool_call` (existing W1-A API). Envelope shape is unchanged.

10. **Documented trust-model softening for stdio** in §11 DoD. W1-A's "every call HMAC-verified" claim still technically true, but for stdio transport the HMAC is self-signed by the same process that holds the key. Honest framing in proof bundle.

## 0. Scope statement

**In scope:** local stdio MCP transport (Claude Desktop, Cursor, custom local SDK clients) on the same machine as the dailyos-mcp-v2 binary. Trust boundary: same user on this machine.

**Out of scope:** remote MCP transport (HTTP/TLS to external clients). Out-of-machine attackers. Cross-user attackers on the same machine (multi-tenant systems). Network-attached MCP servers. If you have a remote-MCP threat model, this lane is the wrong substrate.

## 1. What this lane ships

The MCP v2 Gateway (W1-A) has no transport ingress — it's a Rust function nobody can call over the wire. Legacy `dailyos-mcp` uses `rmcp::ServerHandler` on stdio. This lane bridges `rmcp` to the v2 `Gateway` so standard MCP clients (Claude Desktop, Cursor, custom) invoke v2 tools through the W1-A trust contract under the §0 scope.

### Architecture: env-asserted startup identity, transport-internal signing

Each `dailyos-mcp-v2` stdio process serves one verified pairing for its lifetime. Identity is asserted via environment variables at process startup; per-call envelopes are constructed + signed BY THE TRANSPORT itself, then dispatched via `Gateway::handle_tool_call` which verifies HMAC + consumes nonce as substrate hygiene.

```
1. pair CLI emits config snippet → operator pastes into claude_desktop_config.json
2. Claude Desktop spawns `dailyos-mcp-v2 serve` with env vars set
3. Subprocess startup:
     a. Read DAILYOS_MCP_CLIENT_ID + DAILYOS_MCP_TRANSPORT_KEY from env
     b. Look up mcp_client_manifest row for client_id; verify exists + not revoked
     c. Read transport_key from keychain via row.keychain_ref; cross-check
        against env-asserted key (Zeroizing comparison, constant-time)
     d. Store verified_client_id + transport_key in V2ServerHandler
     e. Best-effort: std::env::remove_var on env vars (limits subsequent
        in-process getenv reads; does NOT scrub /proc/<pid>/environ — see §0 scope)
     f. Begin serving rmcp::ServerHandler
4. Standard MCP `initialize`/`get_info` handshake
5. `tools/list` returns registered handlers FILTERED by mcp_tool_grant rows
   for verified_client_id where exposure = Invocable. input_schema exposes
   only handler-defined params (no _dailyos_* keys).
6. `tools/call` flow:
     a. Receive { name, arguments } — normal MCP shape, no _dailyos_* keys
     b. Transport assigns request_nonce (W1-A preissue path)
     c. Transport constructs McpToolRequestEnvelope { tool_name=name,
        request_nonce, params=arguments, conversation_handle, tool_grant_id }
     d. Transport canonicalizes envelope via RFC 8785 JCS; signs with
        process-held transport_key using HMAC-SHA256
     e. Transport calls Gateway::handle_tool_call(conn, &verified_client_id,
        envelope, signature)
     f. Gateway substrate (W1-A, unchanged): verify HMAC, consume nonce,
        check manifest scope grant, dispatch handler, audit, signal-emit
     g. Transport unwraps response envelope into rmcp::CallToolResult
7. Process exit = session end; no persistent state
```

### Deliverable 1: `src-tauri/src/services/mcp_v2/transport.rs` (NEW)

```rust
pub struct V2ServerHandler {
    gateway: Arc<Gateway>,
    catalog: Arc<dyn TaxonomyCatalog>,
    db: Arc<Mutex<ActionDb>>,
    verified_client_id: McpClientId,                  // immutable, set at construction
    transport_key: Zeroizing<[u8; 32]>,               // process-held; signs envelopes internally
    server_info: ServerInfo,
}

impl V2ServerHandler {
    /// Constructed AFTER successful env-assertion + manifest lookup +
    /// keychain cross-check in main.rs. Construction failure exits the
    /// process before the rmcp service runs.
    pub fn from_verified_pairing(
        gateway: Arc<Gateway>,
        catalog: Arc<dyn TaxonomyCatalog>,
        db: Arc<Mutex<ActionDb>>,
        verified_client_id: McpClientId,
        transport_key: Zeroizing<[u8; 32]>,
    ) -> Self { ... }
}

impl rmcp::ServerHandler for V2ServerHandler {
    fn get_info(&self) -> ServerInfo { ... }                   // mirrors legacy
    async fn list_tools(...) -> Result<ListToolsResult, McpError> { ... }
    async fn call_tool(...) -> Result<CallToolResult, McpError> { ... }
    // Internal: construct + sign envelope from request before gateway dispatch
}
```

### Deliverable 2: `src-tauri/src/mcp_v2/main.rs` (NEW binary `dailyos-mcp-v2`)

```bash
dailyos-mcp-v2 serve [--legacy-config-path <p>]   # stdio MCP server
dailyos-mcp-v2 pair --client-name <s>             # pair a new client
                    --grant <tool>:<scope1,scope2>:<exposure>  # repeatable
                    [--format json|claude-desktop]
dailyos-mcp-v2 unpair --client-id <id>             # revoke a pairing
```

### Deliverable 3: `pair --format claude-desktop` output

```json
{
  "mcpServers": {
    "<client-name>": {
      "type": "stdio",
      "command": "<resolved-absolute-path>",
      "args": ["serve"],
      "env": {
        "DAILYOS_MCP_CLIENT_ID": "mcp_client_<hex>",
        "DAILYOS_MCP_TRANSPORT_KEY": "<hex>"
      }
    }
  }
}
```

Stderr (never stdout):
```
WARNING: DAILYOS_MCP_TRANSPORT_KEY printed ONCE. Record it now — there is
no recovery. Re-pair via 'unpair --client-id <id>' then 'pair' regenerates
a new key.

Recommended: chmod 600 on claude_desktop_config.json — the env block
contains the transport_key. (Same-user same-machine threat model; advisory
only.)
```

## 2. What this lane does NOT ship

- Per-tool handlers (W2 / W3 / W4)
- Remote MCP transport (HTTP/TLS) — §0 out-of-scope for v1.4.7
- MCP v1 deprecation — legacy binary stays per W1-A AC-10
- Production `BusSignalEmitter` (separate W1.5 lane)
- Operator pairing UI beyond CLI (post-v1.4.7)
- `client_label` schema column for pair-by-name revoke (separate ticket)

## 3. Frozen substrate citations

- `rmcp = "0.1"` already in Cargo.toml; legacy precedent at `src/mcp/main.rs:973-1031` + `:1312-1379`
- W1-A `Gateway::handle_tool_call(conn, &asserted_client_id, envelope, signature)` — dispatch entry, unchanged
- W1-A `auth::pair_client`, `auth::load_client_record`, `auth::verify_transport_hmac`, `auth::verify_and_consume_and_preissue` — all reused; no parallel paths
- W1-A keychain via `keychain_ref` in `mcp_client_manifest` — env-asserted key cross-checked here
- W1-A `mcp_transport_nonce_ledger` — request_nonce ledger consumed per envelope
- W1-B `YamlTaxonomyCatalog::description_for` (will move to trait per §4)
- W1-B `Gateway::seal()`
- ADR-0128 §B — MCP as product surface
- ADR-0102 §C.bis.replay/refresh — per-envelope HMAC + nonce

## 4. K-in + substrate extensions

- **K-in**: grep `docs/solutions/` + `.docs/decisions/`. Legacy `dailyos-mcp` is the rmcp precedent.
- **`TaxonomyCatalog` trait extension**: add `fn description_for(&self, name: &ScopedName) -> Option<&ToolDescription>`. One-line additive.

## 5. Acceptance criteria (cycle-5 normative)

- **AC-1 rmcp::ServerHandler implemented.** `V2ServerHandler` implements `get_info`, `list_tools`, `call_tool`. No custom JSON-RPC methods. Standard MCP clients work without modification.
- **AC-2 Transport self-signs.** `tools/call` receives normal MCP `{ name, arguments }`. Transport (NOT client) constructs `McpToolRequestEnvelope` with `tool_name = name`, `params = arguments`, assigns `request_nonce` from W1-A ledger, signs with process-held `transport_key` over JCS canonicalization. No `_dailyos_*` keys exposed in public input_schema.
- **AC-3 ToolError → rmcp::Error closed-matrix mapping** including public human-readable `message`.

   | ToolError | rmcp::Error code | data.kind | message (public) | log fields (server-side) |
   |---|---|---|---|---|
   | `Unauthorized { missing_scope }` | -32600 invalid_request | `unauthorized` | "tool requires scope your pairing lacks" | client_id, tool_name, missing_scope |
   | `BadParams { detail }` | -32602 invalid_params | `bad_params` | "tool params are malformed" | client_id, tool_name, detail (server-side only) |
   | `RateLimited { retry_after_seconds }` | -32099 custom | `rate_limited` | "too many calls; retry later" | client_id, tool_name, retry_after_seconds |
   | `ExposureForbidden { tool_name }` | -32601 method_not_found | `exposure_forbidden` | "tool is not exposed to your pairing" | client_id, tool_name |
   | `PairingRevoked` | -32600 invalid_request | `pairing_revoked` | "pairing has been revoked" | client_id |
   | `ConversationRevoked` | -32600 invalid_request | `conversation_revoked` | "conversation has been revoked" | client_id, conversation_handle |
   | `NotFound { resource }` | -32601 method_not_found | `not_found` | "requested resource not found" | client_id, tool_name, resource (opaque ID) |
   | `UpstreamFailure { detail }` | -32603 internal_error | `upstream_failure` | "upstream system failure; try again later" | trace_id (opaque); detail server-side only |
   | `Internal { trace_id }` | -32603 internal_error | `internal` | "internal error" | trace_id |

   Test asserts every variant; CI lint fails on orphan variant.
- **AC-4 Boot logs.** Format: `mcp_v2 boot: pairing <client_id> verified, 0 handlers registered, 10 catalog entries pending. tools/list will return empty for this build. Expected for W1.5 transport-only.` Strict mode env: `DAILYOS_MCP_V2_REQUIRE_HANDLERS=1` fails boot if no handlers registered.
- **AC-5 `pair` CLI:**
  - `dailyos-mcp-v2 pair --client-name <s> --grant <tool>:<scope1>[,<scope2>...]:<invocable|metadata-only> [--grant ...] [--format json|claude-desktop]`
  - `--format claude-desktop` → snippet includes `"type": "stdio"` (Cursor compat) + env block (client_id + transport_key). **`command` field MUST be an absolute path** resolved via `std::env::current_exe()` (cycle-5 devex finding: GUI-launched Claude Desktop / Cursor don't reliably inherit shell PATH; bare binary names fail to spawn)
  - Stderr warnings: key-printed-once + chmod 600 advisory (§0 scope acknowledgment)
  - No name-based re-pair (substrate gap filed separately)
- **AC-6 `tools/list` filtered + composed description.** Returns only registered handlers WHOSE `mcp_tool_grant` row for `verified_client_id` has `exposure = Invocable`. Each `rmcp::Tool` carries:
  - `name` = catalog entry name (ScopedName)
  - `description` = `format!("{summary}\n\nWhen to call:\n{when_to_call}\n\nWhen NOT to call:\n{when_not_to_call}")`
  - `input_schema` = JSON Schema for handler params from `ToolDescription.parameters`. **Excludes `_dailyos_*` keys** (transport self-signs; client doesn't construct them).
- **AC-7 Legacy `dailyos-mcp` unchanged.** Build matrix verifies both binaries compile + bin paths don't collide.
- **AC-8 Integration tests** (cover AC-1..AC-14):
  - Unit: `V2ServerHandler::call_tool` direct invocation with stub handler
  - **Subprocess test**: spawn `dailyos-mcp-v2 serve` with env; perform full MCP `initialize` → `tools/list` → `tools/call` over stdin/stdout; assert stdout protocol-only (per legacy `src/mcp/main.rs:1312` precedent)
  - Subprocess negative tests: missing env → exit 1; unknown client_id → exit 1; revoked pairing → exit 1; keychain mismatch → exit 1; tools/call with replay of consumed nonce → BadParams (W1-A substrate via gateway)
- **AC-10 Required checks.** `cargo build --features mcp --bin dailyos-mcp-v2` clean. `cargo clippy --features mcp -- -D warnings` clean. `cargo test --features mcp` passes including subprocess + JCS golden tests.
- **AC-11 Legacy executable identity guardrail.** `serve --legacy-config-path <p>` refuses to start if config claims v2-owned tools AND points (via filesystem canonicalization following symlinks) at the legacy `dailyos-mcp` binary. Error message includes resolved-path + suggested remediation. Hardlinks/copies acknowledged limitation; binary self-ID `--version-id` fallback filed as separate ticket.
- **AC-12 Startup env-assertion** (replaces cycle-3 identify ACs).
  1. Read `DAILYOS_MCP_CLIENT_ID` + `DAILYOS_MCP_TRANSPORT_KEY` from env (into Zeroizing buffers). Missing → exit 1 with operator-readable error.
  2. Look up `mcp_client_manifest` row. Not found → exit 1.
  3. Revoked → exit 1.
  4. Read transport_key from keychain via `row.keychain_ref`; constant-time compare against env-asserted key (Zeroizing). Mismatch → exit 1.
  5. **Best-effort env-wipe**: `unsafe { std::env::remove_var }` on both env vars BEFORE spawning the rmcp service / any threads. Documented as best-effort in-process hygiene — does NOT scrub `/proc/<pid>/environ` (which retains the initial exec environment for process lifetime per Linux kernel behavior). Acceptable under §0 scope (same-user same-machine threat model; attacker reading /proc already has keychain access).
  6. Construct `V2ServerHandler::from_verified_pairing` and begin serving.
- **AC-13 Per-envelope replay protection via W1-A substrate.** `tools/call` calls `Gateway::handle_tool_call` which invokes `auth::verify_transport_hmac` + `auth::verify_and_consume_and_preissue`. Replay (consumed nonce) → BadParams. **Note**: for stdio transport, HMAC verification + nonce consumption are transport-internal hygiene (the same process signs and verifies via the same key). Real client auth for stdio is the startup env-assertion (AC-12). For WP adapter / SDK transports, HMAC + nonce remain real client-to-gateway authentication.
- **AC-14 RFC 8785 JCS canonicalization with hardened spec + golden fixtures.**
  - Duplicate-key rejection at all object levels (UnknownField / DuplicateKey error)
  - I-JSON-compatible values only (no NaN, no Infinity, no -0, no excessive precision)
  - Golden fixtures shipped in `src-tauri/tests/fixtures/jcs_envelope_*.json` covering:
    - ASCII text
    - Unicode (BMP + surrogate pairs)
    - Escape sequences (`\n`, `\t`, `\\`, `ÿ`)
    - Nested objects (depth 5)
    - Arrays (mixed types)
    - Integer edge cases (0, -0 rejection, max safe int, negative)
    - Floats (0.1, 1e10, 1e-10)
    - Null values
    - Empty object / empty array
    - Duplicate-key rejection (input + expected error)

## 6. Files owned

| File | State | Owner |
|---|---|---|
| `src-tauri/src/services/mcp_v2/transport.rs` | NEW | exclusive |
| `src-tauri/src/services/mcp_v2/taxonomy.rs` | additive — `description_for` to trait | shared (one-line) |
| `src-tauri/src/services/mcp_v2/mod.rs` | additive — `pub mod transport;` | shared (one-line) |
| `src-tauri/src/mcp_v2/main.rs` | NEW (binary with serve / pair / unpair) | exclusive |
| `src-tauri/Cargo.toml` | additive — `[[bin]]` block for `dailyos-mcp-v2` | shared (additive) |
| `src-tauri/tests/dos_mcp_transport_test.rs` | NEW | exclusive |
| `src-tauri/tests/dos_mcp_transport_subprocess_test.rs` | NEW | exclusive |
| `src-tauri/tests/fixtures/jcs_envelope_*.json` | NEW (10 golden fixtures per AC-14) | exclusive |

## 7. Test plan (per-AC)

- AC-1: get_info shape assertion
- AC-2: wrapper construction unit test (transport builds envelope from MCP request params, signs, dispatches)
- AC-3: for-each ToolError variant assert code + message + data.kind + log fields; CI lint enforces no orphan
- AC-4: subprocess boot stderr parse against exact format
- AC-5: subprocess pair invocation asserts stdout JSON parseable + "type": "stdio" present + `command` field is an absolute path that resolves to an executable file + stderr warnings + DB rows present
- AC-6: subprocess pair with grant for A not B; tools/list returns [A] with composed description, no _dailyos_* in input_schema
- AC-7: build matrix asserts both binaries compile
- AC-8: full subprocess MCP handshake test; stdout protocol-only assertion
- AC-10: CI
- AC-11: synthetic config triggers refusal with expected error text
- AC-12: subprocess negative tests per failure mode; env-wipe verified via in-process getenv (NOT /proc — acknowledged limitation)
- AC-13: replay test asserts second tools/call with same envelope nonce → BadParams from gateway
- AC-14: golden fixture round-trip tests across all 10 fixtures; duplicate-key fixture asserts error

## 8. Security gates

`/cso` mandatory. Cycle-5 must verify the local-to-local same-machine scope statement (§0) makes the env-asserted identity model acceptable, AND that the trust-model differentiation (stdio = internal hygiene; WP/SDK = real auth) is documented honestly without misleading W1-A's "every call HMAC-verified" framing.

## 9. Path-α (separate Linear tickets)

- `client_label` schema for pair-by-name revoke (cycle-2)
- `dailyos-mcp-v2 migrate-config` subcommand (AC-11 error reference)
- Binary self-identification via `--version-id` (AC-11 challenge cycle-3)
- Per-process keychain ACL gating transport_key reads (post-v1.4.7 hardening)
- Loopback HTTP transport (Phase 2 — out of §0 scope for v1.4.7)
- Tauri operator pairing UI (post-v1.4.7)
- MCP client SDK code samples + JCS reference implementations

## 10. Depends-on

- W1-A merged (PR #347): Gateway + auth + nonce ledger + keychain substrate
- W1-B merged (PR #347): YamlTaxonomyCatalog + Gateway::seal
- rmcp crate already in deps

## 11. Definition of Done

§5 AC-1..AC-14 met. L0 unanimous APPROVE. L2 unanimous APPROVE bounded by AC. Commit-msg `L2-status: passed`. Pushed to PR #347. CI green including subprocess + JCS golden tests.

**Trust-model honest framing in proof bundle:** W1-A claims "every Gateway::handle_tool_call verifies HMAC + consumes nonce" — that remains true. For stdio transport, the signer IS the verifier (same process holds the key). The real client auth is the startup env-assertion + keychain cross-check (AC-12). For non-stdio transports (WP adapter, SDK clients), the HMAC verification authenticates the external signer to the gateway. Both transport classes share the same gateway substrate; their trust boundaries are at different points.

## 12. Open questions resolved

| # | Question | Resolution |
|---|---|---|
| Q1 | rmcp wrapper format | DELETED (cycle-5: transport self-signs internally; no `_dailyos_*` in client params) |
| Q2 | pair CLI argparse | clap |
| Q3 | empty handler set at boot | warning + strict-mode env |
| Q4 | HTTP transport | path-α (§0 scope out for v1.4.7) |
| Q5 (NEW) | trust-model class | Differentiated by transport: stdio = startup identity + internal hygiene; WP/SDK = per-envelope HMAC real auth |
| Q6 (NEW) | local-to-local same-machine scope | locked in §0 (user direction) |

## 13. Reviewer dispatch

- **CSO** (mandatory; verify local-to-local scope statement + honest trust framing)
- **/codex challenge** (adversarial within scope; out-of-scope vectors documented)
- **architect-reviewer** (V2ServerHandler shape; transport-internal signing seam)
- **/plan-devex-review** (Claude Desktop + Cursor compatibility; client integration UX)
