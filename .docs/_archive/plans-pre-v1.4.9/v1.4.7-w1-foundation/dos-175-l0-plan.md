# DOS-175 — MCP v2 W2-A read handlers (account_status) — L0 plan packet

**Wave:** v1.4.7 W2-A (Phase A; Phase B = daily_briefing follow-up)
**Lane spec:** [DOS-175](https://linear.app/a8c/issue/DOS-175)
**Wave plan reference:** `.docs/plans/v1.4.7-waves.md` §"Agent W2-A — DOS-175"
**Depends on:** W1-A (DOS-168 gateway), W1-B (DOS-478 taxonomy), W1.5 (transport)
**Authoring discipline:** narrow-scoped per K-in lessons; scope confined to wiring one already-authored producer into the W1-A handler seam.

## Cycle-2 changelog (2026-05-21)

Cycle-1 verdicts: all 4 reviewers NEEDS-CHANGES. Convergent findings folded:

1. **Ability name (HIGH architect + codex):** registered name is `"dailyos/account-overview"` not `"account_overview"` (verified `account_overview.rs:84`). Handler dispatches the correct name.
2. **Catalog↔ability schema gap (HIGH architect):** catalog parameter is `subject: string` per ADR-0083; ability requires `{schema_version: u32, account_id: String}`. Cycle-2: handler does a thin "subject → account_id" passthrough + `schema_version` injection. Full resolver deferred as Phase-A.1.
3. **Sync/async runtime bridge (HIGH devex + MED architect):** wrap the sync `gateway.dispatch(...)` in `transport.rs::call_tool` with `tokio::task::spawn_blocking(...)`. Handler holds a `tokio::runtime::Handle` captured at registration; inside `spawn_blocking` (a blocking-pool thread, not a tokio worker), `self.runtime.block_on(...)` is safe. No W1-A trait change.
4. **Don't invent actor projection (MED architect):** use existing `services/mcp_v2/actor_policy.rs::project_actor`.
5. **Wave-scope reconciliation (HIGH consult):** cycle-2 scopes to **`account_status` only**. `daily_briefing` moves to Phase B (separate sub-ticket of DOS-175, fired after L4 demo lands). Wave plan amendment filed separately.
6. **AC/test consistency (MED):** AC-9 exercises only `account_status` end-to-end; cross-tool rejection test removed (single-tool scope obviates it).
7. **ADR-0083 vocabulary leak (LOW devex):** catalog "recent signals" → "recent updates and changes". Included in AC.
8. **K-in expansion:** `docs/solutions/security-issues/prompt-channel-sensitivity-class-sweep-2026-05-18.md` is the relevant prior solution; `account_overview.rs:268` already invokes `prompt_input_sensitivity_allowed` internally on the OUTPUT side of the boundary, so no handler-layer gate needed. §2 cites it.

## 0. Threat topology

`local-to-local single-user`. Claude Desktop on the same machine, same user, loopback stdio MCP transport. Per `engineering-ladder.md` §"Threat-topology framing" + memory `feedback_local_to_local_security_overreach_primary_concern`: reviewers do NOT enforce multi-actor gates, principal differentiation, cross-actor poisoning, or scope-gated redaction of data the user already has filesystem access to. Invariants that survive the topology test: compile correctness, crate boundaries, sensitivity rendering already done in the producer (ADR-0108), indirect-prompt-injection containment from third-party document content (ADR-0093).

## 1. What this unblocks

End-to-end demo: user pairs Claude Desktop against `dailyos-mcp-v2`, asks "give me an account overview for Acme", Claude routes to `dailyos.read.account_status`, the W2-A handler invokes `dailyos/account-overview` against the same `AbilityRegistry` the Tauri UI uses, substrate response returns. This is the v1.4.7 W2-A Phase-A bar — first observable MCP product surface against real DailyOS substrate, comparable side-by-side with Glean's output for the same prompt.

Phase-B (`daily_briefing`) is filed as a sub-ticket of DOS-175 and dispatched once Phase-A is validated in Claude Desktop. The 8 catalog slots without aligned producers are out of scope for W2-A entirely — they require new producer abilities and belong to W3+.

## 2. Substrate path (K-in confirmation)

`docs/solutions/` + `.docs/decisions/` grep — substrate present, no reinvention:

- **`AbilityRegistry`** — single global static at `abilities-runtime/src/abilities/registry.rs:838`. No parallel legacy registry.
- **`crate::abilities` shim** — `src-tauri/src/abilities/mod.rs:3` is `pub use abilities_runtime::abilities::*;`. Tauri commands and MCP bridges consume the same re-export.
- **`invoke_registry_json`** — shared async dispatch core. Called by Tauri (`bridges/tauri.rs:217`), legacy MCP (`bridges/mcp.rs:411-422`), and now W2-A.
- **`McpAbilityBridge::invoke_ability`** at `bridges/mcp.rs:385-430` — canonical pattern for "(actor, ability_name, input_json) → AbilityResponseJson". W2-A handler reuses this pattern.
- **`services/mcp_v2/actor_policy.rs::project_actor`** at line 68 — projects `McpClientId + ToolGrant + Option<OpaqueConversationHandle>` to `abilities_runtime::Actor::McpClient`.
- **ADR-0102 §C.bis** — abilities-as-runtime contract; `mcp_exposure`, `allowed_actors`, attribute amendments.
- **ADR-0108 + ADR-0125** — claim-sensitivity rendering; producers render sensitivity before returning. The `account_overview` producer invokes `prompt_input_sensitivity_allowed` at `account_overview.rs:268`. Handler is a thin JSON wrapper.
- **`docs/solutions/security-issues/prompt-channel-sensitivity-class-sweep-2026-05-18.md`** — centralized sensitivity gate in `services/claims.rs` enumerable across all 5 prompt-channel readers. `account_overview` is on the OUTPUT side and already gated; no fresh OUTPUT-side gate needed in the handler.
- **W1-A handler seam** — `services/mcp_v2/gateway.rs:155` (`handlers: HashMap<ScopedName, Arc<dyn McpToolHandler>>`), `:177` (`register`), `:271` (`dispatch`). Scaffolding `services/mcp_v2/handlers/` exists; we add `tool_account_status.rs` (no placeholder for it today).
- **`McpToolHandler` trait** — `services/mcp_v2/contracts.rs:341`: sync `invoke(&self, &McpActor, params: Value) -> Result<Value, ToolError>`.

## 3. Architecture

### 3.1 Handler shape

```rust
pub struct AccountStatusHandler {
    description: ToolDescription,
    registry: &'static AbilityRegistry,
    workspace_readers: McpWorkspaceReaders,
    runtime: tokio::runtime::Handle,
}
```

- `registry`: captured at construction from `AbilityRegistry::global_checked()`.
- `workspace_readers`: `McpWorkspaceReaders::from_action_db(db)` per `bridges/mcp.rs:53` — needed so `ServiceContext` has readers attached, otherwise `account_overview` produces empty output.
- `runtime`: `tokio::runtime::Handle::current()` captured during `register_v147_handlers` (called from inside the rmcp tokio runtime context).

### 3.2 Async/sync bridge — spawn_blocking at transport boundary

The architectural fix folds at `transport.rs::call_tool` (rmcp's async entry), NOT at each handler:

```rust
// services/mcp_v2/transport.rs::V2ServerHandler::call_tool (existing async fn)
let gateway = self.gateway.clone();
let envelope = build_envelope(...);
let result = tokio::task::spawn_blocking(move || gateway.dispatch(envelope))
    .await
    .map_err(|join_err| McpError::internal_error(format!("blocking task panic: {join_err}"), None))?;
```

Effects:
- `gateway.dispatch` (sync) + `handler.invoke` (sync) now run on a tokio blocking-pool thread, NOT a tokio worker thread.
- Inside `handler.invoke`, `self.runtime.block_on(async_ability_call)` is safe: we're on a blocking-pool thread, the runtime worker pool is free, `block_on` blocks the current thread while the future runs on the worker pool.
- No nested-runtime panic.
- No W1-A trait change. Single-line edit at the transport boundary.

### 3.3 Ability dispatch within the handler

Mirrors `bridges/mcp.rs:391-422`:

```rust
fn invoke(&self, actor: &McpActor, params: Value) -> Result<Value, ToolError> {
    let McpActor::Client { client_id, conversation_handle, granted_scopes } = actor else {
        return Err(ToolError::Unauthorized { reason: "unsupported actor variant".into() });
    };
    // Synthesize a ToolGrant from the already-resolved McpActor scopes
    // (gateway already validated grant before dispatch).
    let synthetic_grant = ToolGrant::synthetic_from_actor(client_id, granted_scopes);
    let runtime_actor = project_actor(client_id, &synthetic_grant, conversation_handle.as_ref());

    self.runtime.block_on(async {
        let clock = SystemClock;
        let rng = SystemRng;
        let external = ExternalClients::default();
        let services = self.workspace_readers
            .attach_to(ServiceContext::new_live(&clock, &rng, &external).with_actor(MCP_ACTOR_LABEL));
        let invocation = InvocationContext {
            actor: BridgeActor::Agent,
            mode: ExecutionMode::Live,
            surface: BridgeSurface::McpTool,
            claim_dismissal_surface: ClaimDismissalSurface::McpTool,
            dry_run: false,
            confirmation: None,
            confirmation_store: None,
        };
        let resolved_input = build_account_overview_input(params)?;
        let response = invoke_registry_json(
            self.registry,
            &services,
            None,  // intelligence provider — not needed for read-only path
            None,  // tracer — W2-A no per-invocation tracing (W3 adds it)
            invocation,
            "dailyos/account-overview",  // registered ability name
            resolved_input,
        )
        .await
        .map_err(map_invoke_error_to_tool_error)?;
        Ok(response.data)
    })
}
```

Note: actor variant naming in the destructure follows the existing `McpActor::Client { client_id, conversation_handle, granted_scopes }` enum at `contracts.rs:267`. The `ToolGrant::synthetic_from_actor` helper is new — small constructor that wraps the actor's granted_scopes into the existing `ToolGrant` shape. **Open question Q1** (§7): does an equivalent synthetic-grant helper already exist that I should reuse instead of authoring a new one?

### 3.4 Input translation — subject → account_id

Catalog declares the parameter as `subject: string` per ADR-0083 product-vocabulary. The ability requires `{schema_version: 1, account_id: String}`. Cycle-1 handler does a thin translation:

```rust
fn build_account_overview_input(params: Value) -> Result<Value, ToolError> {
    let subject = params.get("subject")
        .and_then(Value::as_str)
        .ok_or_else(|| ToolError::BadParams { reason: "missing 'subject' parameter".into() })?
        .trim();
    if subject.is_empty() {
        return Err(ToolError::BadParams { reason: "'subject' must be non-empty".into() });
    }
    Ok(serde_json::json!({
        "schema_version": 1,
        "account_id": subject,
    }))
}
```

Phase-A.1 sub-ticket: replace this passthrough with a real subject-to-account_id resolver (fuzzy match on account name / handle index). Cycle-1 passthrough means the demo works for any `account_id` the user types verbatim (matches the substrate's account_id field).

### 3.5 Registration helper

New `services/mcp_v2/handlers/registration.rs`:

```rust
pub fn register_v147_handlers(
    gateway: &mut Gateway,
    catalog: &Arc<dyn TaxonomyCatalog>,
    db: Arc<ParkingMutex<ActionDb>>,
) -> Result<(), RegistrationError> {
    let registry = AbilityRegistry::global_checked()
        .map_err(RegistrationError::AbilityRegistry)?;
    let workspace_readers = McpWorkspaceReaders::from_action_db(db);
    let runtime = tokio::runtime::Handle::current();

    let account_status_name = ScopedName::parse("dailyos.read.account_status")?;
    let description = catalog.description_for(&account_status_name)
        .ok_or(RegistrationError::CatalogEntryMissing(account_status_name.clone()))?
        .clone();
    gateway.register(Arc::new(AccountStatusHandler::new(
        description,
        registry,
        workspace_readers,
        runtime,
    )));
    Ok(())
}
```

Called from `mcp_v2/main.rs::run_serve` between `gateway.set_taxonomy(catalog.clone())` and `gateway.seal()`. The `db` arg is the existing `Arc<Mutex<ActionDb>>` the binary already constructs via `open_conn(db_path)`.

### 3.6 Ability attribute amendment

**`account_overview` (`abilities-runtime/src/abilities/account_overview.rs:84`):**

```rust
#[ability(
    name = "dailyos/account-overview",
    category = Read,
-   allowed_actors = [User, SurfaceClient],
+   allowed_actors = [User, SurfaceClient, McpClient],
    mcp_exposure = Invocable,
    ...
)]
```

Consistency fix — ability already declared `Invocable`; adding `McpClient` to the allowlist makes the declaration reachable. Local-to-local single-user topology, no fresh CSO escalation.

### 3.7 Output translation

`AbilityResponseJson.data` is already sensitivity-rendered by the producer (`account_overview.rs:268` + ADR-0108). Handler returns `response.data` verbatim. No re-rendering, no redaction layer in the handler.

## 4. Out-of-scope (explicit deferrals)

These are filed as sub-tickets of DOS-175 (Phase B+) or W3+ tickets, NOT path-α maintenance:

1. **Phase B — `dailyos.read.daily_briefing`** → wraps `get_daily_briefing`. Requires DOS-624 amendment apply (already authored, just needs the actor + exposure attribute change applied) and `tool_briefing.rs` body filled in. Dispatched after Phase-A L4 demo validates harness end-to-end.
2. **Phase A.1 — subject→account_id resolver.** Cycle-1 passthrough works for users who know their account_id slug; real resolver requires name/handle index work.
3. **`dailyos.read.meeting_briefing`** — producer mapping ambiguous (DOS-624 §3.4 timing-floor + uniform-unavailable shape may itself be over-engineered for local-to-local single-user per memory `feedback_local_to_local_security_overreach`). Defer; revisit DOS-624 §3.4 framing when wave-plan amendment lands.
4. **Other 7 slots** — no producer ability exists; W3+ work.

## 5. Acceptance criteria

**AC-1.** `account_overview` ability attribute amended: `allowed_actors` includes `McpClient`. `cargo check --features mcp --lib` + `cargo clippy --features mcp --lib --bin dailyos-mcp-v2 -- -D warnings` green.

**AC-2.** New file `services/mcp_v2/handlers/tool_account_status.rs` exports `AccountStatusHandler` implementing `McpToolHandler` for the `dailyos.read.account_status` scoped name. Dispatches to `"dailyos/account-overview"` ability via `invoke_registry_json` per §3.3. Captures `tokio::runtime::Handle` at construction.

**AC-3.** New file `services/mcp_v2/handlers/registration.rs` exports `register_v147_handlers(gateway, catalog, db) -> Result<(), RegistrationError>`. Called from `mcp_v2/main.rs::run_serve` between `set_taxonomy` and `seal`.

**AC-4.** `services/mcp_v2/transport.rs::V2ServerHandler::call_tool` wraps the `gateway.dispatch(...)` call in `tokio::task::spawn_blocking(...)` per §3.2. Single-line behavioral change; existing tests updated.

**AC-5.** `handlers/mod.rs` updated to export `tool_account_status` + `registration`.

**AC-6.** `Gateway::seal()` succeeds with 1 handler registered against the 10-tool catalog. `validate_catalog_against_handlers` returns the 9 unregistered slugs as a `Vec` (per W1-B AC-7 split — not an `Err`). Test asserts.

**AC-7.** Unit test for `AccountStatusHandler::invoke`: given an `McpActor::Client` stub + `{"subject": "acme"}` params, the handler returns a `Value` matching `AccountOverviewOutput` JSON shape (mocked registry via test-only hook OR hermetic test registry with a seeded fake ability).

**AC-8.** Integration test `tests/dos175_w2a_account_status_test.rs`: subprocess `dailyos-mcp-v2 serve` against a temp DB seeded with one Acme account, pair via stdio, issue `tools/list` (assert `dailyos.read.account_status` present), issue `tools/call` for `dailyos.read.account_status` with `{"subject": "acme"}`, assert response is non-error JSON containing the `account_id`, `as_of`, and `status` fields.

**AC-9.** End-to-end manual smoke (L4): `dailyos-mcp-v2 pair --client-name claude --grant dailyos.read.account_status:dailyos.read.account_status:invocable --format claude-desktop` emits a valid claude_desktop_config.json snippet; after pasting + restarting Claude Desktop, the prompt "give me an account overview for Acme" routes to `dailyos.read.account_status` and returns substrate. Screenshot captured for the Glean comparison.

**AC-10.** Catalog edit at `src-tauri/resources/mcp_v2/tool_descriptions.yaml:24` — replace "recent signals" with "recent updates and changes" per ADR-0083.

**AC-11.** No regression: `cargo check --features mcp --lib`, `cargo clippy --features mcp --lib --bin dailyos-mcp-v2 -- -D warnings`, `cargo test --features mcp` all green.

## 6. Test plan

- **Unit** (`tool_account_status.rs::tests`): mock registry dispatch via thread-local injection or generic registry param; assert subject→input translation, actor projection, and response unwrap. ~40 LOC.
- **Seal/parity test** (`tests/dos175_w2a_seal_test.rs`): assert `seal()` succeeds with 1 handler + 10 catalog entries; assert `validate_catalog_against_handlers` returns the expected 9-slug `Vec`.
- **Integration test** (`tests/dos175_w2a_account_status_test.rs`): subprocess + stdio JSON-RPC happy path. Seeded Acme account; assert response shape.
- **No cross-tool tests in cycle-1** (single-tool scope obviates).
- **No concurrent burst tests** (deferred — memory `feedback_local_to_local_security_overreach`).

## 7. Open questions for L0 reviewers (cycle 2)

**Q1.** Does a synthetic `ToolGrant` helper already exist somewhere I should reuse, or do I author `ToolGrant::synthetic_from_actor` fresh? Either way trivial; flag if I'm missing a precedent.

**Q2.** Defer per-session provenance cache to Phase B. V1 bridge maintains a `(InvocationId → RenderedProvenance)` map for detail-pane lookups; Claude Desktop's text rendering doesn't need this. Cycle-2 stance: defer.

## 8. Reviewer dispatch (cycle 2)

Same panel: `/codex challenge`, architect, `/plan-devex-review`, `/codex consult`. K-in mandatory. Unanimous APPROVE required.

**Reviewer instructions for cycle 2:** focus on whether the cycle-1 findings (numbered 1-7 in the changelog) are addressed. Avoid re-litigating settled topology framing. Surface any genuinely new architectural issues only.

## 9. Definition of done

1. Cycle-2 unanimous L0 APPROVE
2. AC-1 through AC-11 validated with real data
3. L2 (codex review + code-reviewer + architect) unanimous APPROVE on the diff, bounded by AC
4. L4 manual smoke (AC-9) executed: Claude Desktop returns substrate for "give me an account overview for Acme"; screenshot captured for Glean comparison
5. Phase B (daily_briefing) filed as DOS-175 sub-ticket, ready for next session
