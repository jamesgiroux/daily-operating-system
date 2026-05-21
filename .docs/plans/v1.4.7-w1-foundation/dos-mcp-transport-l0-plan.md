# v1.4.7 — MCP v2 Transport Ingress (rmcp ServerHandler → Gateway) — L0 plan packet

**Wave:** v1.4.7 W1.5 (post-W1-A + W1-B; pre-W2 handlers)
**Linear ticket:** TBD (mint after L0 approve)
**Authoring discipline:** narrow-scoped per K-in lessons. Substrate gaps file as separate Linear tickets.

## Cycle-2 changelog (2026-05-21)

Cycle-1 verdicts: 1 BLOCK (challenge) + 3 NEEDS-CHANGES. Convergent on rmcp wire shape (must be `arguments._dailyos_*`, not top-level wrapper which won't parse) + `ToolError` table incomplete + AC-9 deferral acceptable only while handler set empty + pair CLI re-pair semantics don't fit substrate + empty-boot misstatement. Plus BLOCK on missing `client_id` path.

Cycle-2 fixes (architectural insight from synthesis: **stdio process = pairing session** — first message is identify, subsequent calls inherit verified client_id):

1. **`dailyos/identify` session bootstrap** (closes challenge #1 BLOCK + AC-9 path-α + CSO #2 + devex #5). First MCP message a v2 client sends is a server-namespaced `dailyos/identify` containing `{client_id, identify_nonce, signature}`. Server: looks up client manifest by client_id, verifies HMAC of `(client_id || identify_nonce)` against transport key, sets per-session `verified_client_id` state. All subsequent `tools/list` is filtered by that session's manifest grants (Invocable only); all subsequent `tools/call` envelopes MUST have `client_id == verified_client_id` (else reject) AND still verify HMAC + consume nonce per envelope (defense in depth). Process exit = session end.

2. **Wire shape pinned to `arguments._dailyos_*`** (convergent 4/4). `call_tool` extracts `_dailyos_envelope` + `_dailyos_signature` from `arguments` (rmcp 0.1 only exposes name + arguments). Asserts `request.name == envelope.tool_name`. Reserved-key collision: real param names matching `_dailyos_*` rejected via `BadParams`. `input_schema` in `tools/list` advertises the wrapper convention (closes architect #4).

3. **AC-3 mapping table complete** (CSO #1 + devex #2). Closed matrix of EVERY `ToolError` variant → rmcp::Error code → opaque public `data.kind` → operator-log fields. Includes `NotFound` (→ method_not_found, kind "tool_not_found") and `UpstreamFailure { detail }` (→ internal_error, opaque trace_id, detail logged server-side only). Test asserts every variant; CI lint fails if a new variant lands without a row.

4. **`TaxonomyCatalog` trait gains `description_for`** (architect #1). Additive method `fn description_for(&self, name: &ScopedName) -> Option<&ToolDescription>`. `YamlTaxonomyCatalog` already has this method; trait addition is one-line.

5. **rmcp `get_info` not `initialize`** (architect #3). `V2ServerHandler::get_info(&self) -> ServerInfo` returning `{server_info: {name: "dailyos-mcp-v2", version: ...}, capabilities: {tools: {list_changed: false}, ...}}`. Mirrors legacy `dailyos-mcp` at `src/mcp/main.rs:973`.

6. **rmcp Tool description composes summary + when_to_call + when_NOT_to_call** (devex #4). `list_tools` builds `Tool.description` as `format!("{summary}\n\nWhen to call:\n{when_to_call}\n\nWhen NOT to call:\n{when_NOT_to_call}")` so host model has ADR-0128 §3 product copy directly. Examples and fixtures stay out of wire shape.

7. **Empty-boot misstatement fixed** (architect #2 + devex #6). With embedded YAML (10 tools) + 0 handlers, `gateway.seal()` returns `Vec<ScopedName>` of length 10 (the catalog-only entries). Log line: `mcp_v2 boot: 0 handlers registered, 10 catalog entries pending. tools/list will return empty for this build. Expected for W1.5 transport-only.` Strict mode env hint included.

8. **Pair CLI ergonomics** (devex #3 + CSO #3 + challenge #4). `dailyos-mcp-v2 pair --client-name <s> --grant <tool>:<scope>[,<scope>...]:<exposure> [--grant ...] [--format json|claude-desktop]` — `--grant` is repeatable per-tool to match the `mcp_tool_grant` substrate. `--format claude-desktop` emits a copy-pasteable `mcpServers` JSON snippet. "Transport key printed ONCE; record it now — there is no recovery path" warning. **Drop "same name revokes existing" claim** (CSO #3 + challenge #4): re-pair semantics need `client_label` schema column that doesn't exist; operator must explicitly `unpair --client-id <id>` first. Filed as separate ticket for v1.4.7+ ("DOS-? MCP pair-by-name revoke via client_label").

9. **Subprocess JSON-RPC handshake test** (challenge #5). AC-8 extended: integration test spawns `dailyos-mcp-v2` as a child process, performs full JSON-RPC handshake over stdin/stdout, verifies stdout is protocol-only (no boot log corruption per legacy precedent `src/mcp/main.rs:1312`). Direct `ServerHandler::call_tool` unit test stays as a faster inner-loop check.

10. **Legacy coexistence guardrails** (challenge #6). Cycle-2 adds AC-11: dailyos-mcp-v2 binary refuses to run if `--legacy-config-path <p>` points at a config that still references `dailyos-mcp` for tools v2 owns. Operator gets clear error + remediation. Negative integration test asserts the refusal. Documents post-v1.4.7 deprecation path for legacy binary in `.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md` as a separate ADR amendment (filed ticket).

Cycle-2 commits the architecture insight (`identify` session bootstrap) which collapses multiple findings into one design.

## 1. What this lane ships

The MCP v2 Gateway (W1-A) has no transport ingress yet — it's a Rust function nobody can call over the wire. The legacy `dailyos-mcp` binary (`src/mcp/main.rs`, 1898 LOC) uses the official `rmcp` crate (`rmcp::ServerHandler` trait) on stdio with a legacy `McpAbilityBridge`. This lane bridges `rmcp` to the v2 `Gateway` so MCP clients (Claude Desktop, Cursor, custom) can actually invoke v2 tools through the W1-A trust contract.

Two deliverables:

1. **`src-tauri/src/services/mcp_v2/transport.rs`** (NEW) — `V2ServerHandler` struct implementing `rmcp::ServerHandler`:
   - `list_tools(...)` — returns the registered handler set, filtered by the caller's manifest scope grant (`Invocable` exposure tier only). Description text pulled from `YamlTaxonomyCatalog::description_for`.
   - `call_tool(...)` — translates `rmcp::CallToolRequestParam` to `McpToolRequestEnvelope`, calls `Gateway::handle_tool_call`, translates `McpToolResponseEnvelope` back to `rmcp::CallToolResult`. Maps `ToolError` variants to `rmcp::Error` consistently.
   - `initialize(...)` — standard MCP handshake.
   - Holds: `Arc<Gateway>`, `Arc<dyn TaxonomyCatalog>`, `Arc<Mutex<ActionDb>>` (for auth lookup + audit write).

2. **`src-tauri/src/mcp_v2/main.rs`** (NEW binary `dailyos-mcp-v2`) — entry point that:
   - Loads `Config`
   - Constructs `ActionDb`, `YamlTaxonomyCatalog::load_embedded()`
   - Constructs `Gateway::new()`, registers handlers (W2-W4 fill in; ships with EMPTY handler set initially — pending-tool list logged at boot), `gateway.set_taxonomy(...)`, `gateway.seal()?`
   - Constructs `V2ServerHandler` + serves on stdio via `rmcp::transport::io::stdio()`
   - Optional `--http <addr>` flag for loopback HTTP transport (per ADR-0128 §B; rmcp supports both via `transport-io` + custom transport adapter)

3. **Pairing handshake** is OUT of the standard `rmcp` JSON-RPC channel (the MCP protocol has no native pairing message). Handled by a separate sidecar:
   - Operator runs `dailyos-mcp-v2 pair --client-name "claude-desktop"` (CLI subcommand, NOT the server loop)
   - Writes to `mcp_client_manifest` + `mcp_tool_grant`
   - Prints `{client_id, seed_nonce, transport_key, transport_key_ref}` for operator to paste into the MCP client's config
   - Client uses those values to sign + sequence its rmcp connection
   - This means the rmcp ServerHandler MUST verify HMAC + nonce per envelope it processes (not at connection-open) — every `call_tool` becomes a verified request

## 2. What this lane does NOT ship

- Per-tool handlers (W2 / W3 / W4 scope)
- Loopback HTTP transport (Phase 2; stdio ships first per existing `dailyos-mcp` precedent)
- MCP v1 deprecation — legacy `dailyos-mcp` binary stays unchanged per W1-A AC-10
- Production `BusSignalEmitter` wiring (separate W1.5 lane)
- Operator pairing UI — CLI subcommand is the v1.4.7 ingress; richer UI is post-v1.4.7

## 3. Frozen substrate citations

- **`rmcp = "0.1"`** with features `["server", "client", "transport-io", "transport-child-process", "macros"]` already in `Cargo.toml`. Reused; no new crate.
- **W1-A `Gateway::handle_tool_call(conn, asserted_client_id, envelope, signature)`** — the dispatch entry point this transport calls per invocation.
- **W1-A `auth::pair_client(conn, ...) -> PairingResponse`** — used by the `pair` CLI subcommand.
- **W1-B `YamlTaxonomyCatalog::description_for(name)`** — used by `list_tools` for description text.
- **W1-B `Gateway::seal()`** — called at boot to fail-fast on handler↔catalog mismatch.
- **Legacy `src/mcp/main.rs:1379`** — `server.serve(rmcp::transport::io::stdio()).await?` is the exact pattern v2 mirrors.
- **ADR-0128 §B** — headless DailyOS MCP as product surface; this lane is its concrete substrate.
- **ADR-0102 §C.bis.replay/refresh** — per-envelope HMAC + nonce verification; rmcp message frames carry these in a custom JSON-RPC param wrapper.

## 4. K-in citations (present-on-dev)

- `src/mcp/main.rs:973-1031` — legacy `ServerHandler` impl, full pattern to mirror
- `docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md` — grep substrate type, not proposed name; applied here by reading rmcp surface from existing code rather than guessing API
- `docs/solutions/architecture-patterns/capability-boundary-needs-crate-split-not-grep-2026-05-18.md` — informs the bin-vs-lib split: transport is a binary entry point, not lib-internal

## 5. Acceptance criteria

- **AC-1 `V2ServerHandler` implements `rmcp::ServerHandler`**: `initialize`, `list_tools`, `call_tool` all implemented. `list_tools` returns ALL registered handlers (no per-pairing filtering yet — see AC-9 deferral). `call_tool` translates request → envelope → Gateway → response.
- **AC-2 HMAC + nonce verification per envelope.** The MCP JSON-RPC `tools/call` params carry a custom wrapper: `{ envelope: McpToolRequestEnvelope, signature: hex }`. `call_tool` extracts both, calls `gateway.handle_tool_call`. Reject with `rmcp::Error::invalid_params` if wrapper malformed.
- **AC-3 ToolError → rmcp::Error mapping table.** `ToolError::Unauthorized` → `rmcp::Error::invalid_request`; `ToolError::BadParams` → `rmcp::Error::invalid_params`; `ToolError::RateLimited` → custom code (define one); `ToolError::ExposureForbidden` → `Error::method_not_found`; `ToolError::PairingRevoked` → `Error::invalid_request`; `ToolError::ConversationRevoked` → `Error::invalid_request`; `ToolError::Internal` → `Error::internal_error` carrying opaque `trace_id`. Closed table; test asserts every variant.
- **AC-4 stdio binary boots clean.** `cargo build --features mcp --bin dailyos-mcp-v2` succeeds. Binary spawned via `cargo run --features mcp --bin dailyos-mcp-v2` reads `Config`, constructs DB, opens taxonomy, calls `Gateway::seal()`, logs `0 pending tools` (empty handler set is acceptable), then waits on stdin per `rmcp::transport::io::stdio`.
- **AC-5 `pair` CLI subcommand works.** `dailyos-mcp-v2 pair --client-name <s> --scope <s>...` writes to `mcp_client_manifest` + `mcp_tool_grant` + emits `PairingResponse` as JSON to stdout. Idempotent re-pair (same `--client-name` → revokes existing + issues new).
- **AC-6 `tools/list` reflects taxonomy.** Returns one `rmcp::Tool` per registered handler with `name` from `ScopedName`, `description` from `YamlTaxonomyCatalog::description_for(name).summary`, `input_schema` from `ToolDescription.parameters` translated to JSON Schema. Empty registered set → empty `tools` list (valid MCP response).
- **AC-7 Legacy `dailyos-mcp` binary unchanged.** Build matrix verifies both `dailyos-mcp` AND `dailyos-mcp-v2` compile + their bin paths don't collide. W1-A AC-10 coexistence preserved.
- **AC-8 Integration test `tests/dos_mcp_transport_test.rs`.** Spawns the v2 server in-process via `rmcp::ServerHandler` direct invocation (no actual stdio fork — uses rmcp's test harness if available, else a minimal direct-call test): tests/list returns expected names + register-a-stub-handler+invoke flow round-trips one envelope through Gateway → handler → response.
- **AC-9 Path-α deferred to separate ticket** (not blocking this PR): per-pairing `list_tools` scope filtering. Currently all registered handlers are listed regardless of caller manifest. Tightening to filter by `mcp_tool_grant.exposure = Invocable` for the asserted `client_id` is a separate Linear ticket because it touches the rmcp connection context (currently no way to thread `client_id` through `RequestContext`).
- **AC-10 Required checks.** `cargo build --features mcp --bin dailyos-mcp-v2` clean. `cargo clippy --features mcp -- -D warnings` clean. `cargo test --features mcp dos_mcp_transport_test` passes.

## 6. Files owned

| File | State | Owner |
|---|---|---|
| `src-tauri/src/services/mcp_v2/transport.rs` | NEW | exclusive |
| `src-tauri/src/services/mcp_v2/mod.rs` | additive — add `pub mod transport;` | shared (one-line additive) |
| `src-tauri/src/mcp_v2/main.rs` | NEW (binary entry) | exclusive |
| `src-tauri/Cargo.toml` | additive — add `[[bin]]` block for `dailyos-mcp-v2` matching legacy pattern | shared (additive) |
| `src-tauri/tests/dos_mcp_transport_test.rs` | NEW | exclusive |

## 7. Test plan (covers AC-1..AC-10)

- **Wrapper extraction** (AC-2): malformed wrapper → `rmcp::Error::invalid_params`; well-formed → envelope passed to gateway.
- **ToolError mapping table** (AC-3): for-each `ToolError` variant, assert the rmcp::Error code returned.
- **Boot path** (AC-4): integration test constructs `Gateway::new() + seal() + V2ServerHandler::new()`; smoke-test `list_tools` returns empty + `call_tool("nonexistent", ...)` returns BadParams.
- **pair CLI** (AC-5): subprocess test runs the binary with `pair --client-name test --scope dailyos.read.account_status`; asserts JSON response shape; second invocation with same name asserts re-pair revokes + reissues.
- **tools/list shape** (AC-6): register stub handler, assert `Tool { name, description, input_schema }` matches taxonomy entry.
- **End-to-end round trip** (AC-1+AC-8): register `EchoHandler`, pair a test client, sign an envelope, invoke `call_tool`, assert response envelope round-trips.

## 8. Security gates

`/cso` MANDATORY for this lane — transport ingress is the HTTP/stdio entry point for the v2 trust contract. CSO must verify:
- HMAC verification happens on EVERY call_tool invocation (no per-connection caching)
- Nonce ledger consume+preissue fires per envelope
- `tools/list` does NOT leak unregistered handler names or scope info that bypasses manifest gates
- `pair` CLI writes use the same `auth::pair_client` path as the in-process pair (no parallel write path)
- ToolError → rmcp::Error mapping does NOT leak internal detail (trace_id is opaque per W1-A CSO cycle-3 finding)
- Legacy `dailyos-mcp` binary is NOT a fall-through that bypasses v2's trust contract

## 9. Path-α (file as separate Linear tickets)

- AC-9 per-pairing `list_tools` scope filtering (needs rmcp::RequestContext extension for client_id threading)
- Loopback HTTP transport (Phase 2; stdio ships first)
- Operator pairing UI beyond CLI (Tauri admin surface, post-v1.4.7)
- MCP client SDK examples (Python, TypeScript) — post-v1.4.7 DX

## 10. Depends-on

- **W1-A merged** (in this PR #347): Gateway::handle_tool_call + auth::pair_client substrate
- **W1-B merged** (in this PR #347): YamlTaxonomyCatalog::load_embedded + description_for
- **No external substrate**: rmcp crate already in dependencies

## 11. Definition of Done

§5 AC-1..AC-10 met; L0 unanimous APPROVE; L2 unanimous APPROVE bounded by AC; commit-msg `L2-status: passed`; pushed to PR #347 (same branch as W1-A + W1-B); CI green.

## 12. Open questions for L0 cycle 1

| # | Question | Default |
|---|---|---|
| Q1 | rmcp envelope wrapper format — extend `CallToolRequestParam.arguments` with reserved `_dailyos_envelope` + `_dailyos_signature` keys, OR use rmcp's `meta` field | `arguments._dailyos_envelope/_signature` (visible in protocol; meta is request-level not param-level) |
| Q2 | `pair` subcommand — argparse via `clap` (already in deps) or hand-rolled | clap (matches existing CLI patterns in codebase) |
| Q3 | Empty handler set at boot — fatal or just operator-log warning | warning (W2-W4 land incrementally; v1.4.7 staged rollout means binary ships before all handlers) |
| Q4 | `--http` flag this lane or path-α | path-α (rmcp HTTP transport is separate adapter; stdio is enough for v1.4.7 W1.5 scope) |

## 13. Reviewer dispatch

- **CSO** (mandatory — trust contract entry point)
- **/codex challenge** (adversarial — try to bypass the trust contract via rmcp surface quirks)
- **architect-reviewer** (rmcp integration shape; bin-vs-lib split; envelope wrapper design)
- **/plan-devex-review** (DX of MCP client integration; `pair` CLI ergonomics; ToolError → rmcp::Error mapping operator-readability)
