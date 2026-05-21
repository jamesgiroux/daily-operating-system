Reading additional input from stdin...
OpenAI Codex v0.130.0
--------
workdir: /private/tmp/dailyos-v147-w1a
model: gpt-5.5
provider: openai
approval: never
sandbox: read-only
reasoning effort: xhigh
reasoning summaries: none
session id: 019e4b9c-559d-7b83-90c9-9db592d5510e
--------
user
Sanity-check consult on the L0 plan packet at .docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md. You are the 4th panelist — independent read, no specific lens. Look for: anything obviously missing, contradictions with the rest of the v1.4.7 wave plan, scope creep risk, or premature complexity that could be cut.

K-in obligation: grep docs/solutions/ and .docs/decisions/ once for substrate the packet cites. Also read .docs/plans/v1.4.7-w1-foundation/dos-624-cso-amendment.md — confirm whether the §4 deferral of meeting_briefing is internally consistent with DOS-624 (which lists meeting_briefing as W2-A scope).

Threat topology: local-to-local single-user. Don\t enforce multi-actor gates.

User\s stated outcome this session: get end-to-end Claude Desktop -> DailyOS substrate working so "give me an account overview for acme" returns real data, comparable to Glean. Is the packet scoped sharply enough to deliver that, or could it be even tighter (e.g., cut daily_briefing and ship just account_status)?

Output structure:
- One-line VERDICT: APPROVE | NEEDS-CHANGES | BLOCKED
- Findings (severity, concern)
- Recommended scope tightening if applicable

Be terse. ~200 words max.
2026-05-21T17:36:37.870537Z ERROR rmcp::transport::worker: worker quit with fatal: Transport channel closed, when Auth(TokenRefreshFailed("Server returned error response: invalid_grant: Grant not found"))
2026-05-21T17:36:38.263988Z ERROR rmcp::transport::worker: worker quit with fatal: Transport channel closed, when Deserialize(Error("data did not match any variant of untagged enum JsonRpcMessage", line: 0, column: 0))
codex
I’ll read the L0 packet, the DOS-624 amendment, and nearby wave-plan context first, then do the required substrate grep against the cited decision/solution material.
exec
/bin/zsh -lc "sed -n '1,260p' .docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
# DOS-175 — MCP v2 W2-A read handlers (account_status + daily_briefing) — L0 plan packet

**Wave:** v1.4.7 W2-A
**Lane spec:** [DOS-175](https://linear.app/a8c/issue/DOS-175)
**Wave plan reference:** `.docs/plans/v1.4.7-waves.md` §"Agent W2-A — DOS-175"
**Depends on:** W1-A (DOS-168 gateway), W1-B (DOS-478 taxonomy), W1.5 (transport), DOS-624 (CSO amendment for briefing MCP exposure)
**Authoring discipline:** narrow-scoped per K-in lessons; scope confined to wiring already-authored producers into the W1-A handler seam.

## Cycle-1 scope

W2-A ships two read-side MCP tool handlers wrapping abilities that exist today in `abilities-runtime`:

1. **`dailyos.read.account_status`** → wraps `account_overview` ability (`src-tauri/abilities-runtime/src/abilities/account_overview.rs:34`)
2. **`dailyos.read.daily_briefing`** → wraps `get_daily_briefing` ability (`src-tauri/abilities-runtime/src/abilities/get_daily_briefing/mod.rs:27`)

These are the two read tools whose producer abilities exist in the registry and whose MCP exposure is either already declared (`account_overview` carries `mcp_exposure = Invocable`) or explicitly authorized by a prior amendment (`get_daily_briefing` per DOS-624). The remaining 8 slots in the W1-B catalog do not have aligned producers in `abilities-runtime` today and are out of scope for W2-A cycle 1.

**Threat topology:** `local-to-local single-user`. Claude Desktop on the same machine, same user, loopback stdio MCP transport. Per `engineering-ladder.md` §"Threat-topology framing" + memory `feedback_local_to_local_security_overreach_primary_concern`: reviewers do NOT enforce multi-actor gates, principal differentiation, cross-actor poisoning, or scope-gated redaction of data the user already has filesystem access to. The only invariants that survive the topology test are: compile correctness, crate-boundary rules, sensitivity rendering already done in the producer (ADR-0108), and indirect-prompt-injection containment from third-party document content (ADR-0093). Reviewers who flag multi-actor gates on this surface waste a cycle; please scope your threat model to "Claude Desktop run by James on James's machine talking to DailyOS running as James."

## 1. What this unblocks

End-to-end demo path: user pairs Claude Desktop against `dailyos-mcp-v2`, asks "give me an account overview for Acme" or "what's on my plate today", Claude routes to the appropriate DailyOS tool, the W2-A handler invokes the underlying ability against the same `abilities-runtime` registry the Tauri UI uses, and the substrate response is returned to Claude Desktop. This is the v1.4.7 W2-A bar — first observable MCP product surface against real DailyOS substrate.

The other 8 catalog slots (`meeting_briefing`, `portfolio_attention`, `workspace_memory`, `workspace_source_provenance`, `place_document`, `submit.note`, `submit.action`, `submit.action_status`) require either new producer abilities or non-trivial mapping work and are explicitly deferred to W2-A Phase-2 / W3 / W4 lanes. See §7 below.

## 2. Substrate path (K-in confirmation)

`docs/solutions/` + `.docs/decisions/` grep results — substrate already present:

- **`AbilityRegistry`** — single global static. `abilities-runtime/src/abilities/registry.rs:838` exposes `pub fn global_checked() -> Result<&'static Self, ...>` backed by `OnceLock`. There is no parallel legacy registry.
- **`crate::abilities` shim** — `src-tauri/src/abilities/mod.rs:3` is `pub use abilities_runtime::abilities::*;`. Tauri commands and MCP bridges both consume the same crate-level re-export.
- **`invoke_registry_json`** — the shared async dispatch core. Called by Tauri (`bridges/tauri.rs:217`), legacy MCP (`bridges/mcp.rs:411-422`), and now W2-A.
- **`McpAbilityBridge::invoke_ability`** (`src-tauri/src/bridges/mcp.rs:385-430`) — the canonical pattern for "actor + ability_name + input_json → AbilityResponseJson" over the same registry. W2-A handlers reuse this pattern verbatim (no second invocation engine).
- **ADR-0102 §C.bis** — abilities-as-runtime contract; covers `mcp_exposure`, `allowed_actors`, attribute changes via re-approval.
- **ADR-0108** — claim-sensitivity rendering; producer abilities render sensitivity before returning. Handler is a thin JSON wrapper.
- **ADR-0125** — claim-sensitivity rendering depth; same as above.
- **DOS-624 CSO amendment** — explicit MCP exposure authorization for `get_daily_briefing` per AC-507.5.
- **W1-A handler seam** — `gateway.rs:155` (`handlers: HashMap<ScopedName, Arc<dyn McpToolHandler>>`), `gateway.rs:177` (`register`), `gateway.rs:271` (`dispatch`). Scaffolding directory `services/mcp_v2/handlers/` already exists with 1-line placeholder files including `tool_briefing.rs`.
- **`McpToolHandler` trait** — `services/mcp_v2/contracts.rs:341`: sync `invoke(&self, &McpActor, params: Value) -> Result<Value, ToolError>`.

**No substrate reinvention.** W2-A is purely the wiring layer between W1-A's `McpToolHandler` registration seam and the existing `invoke_registry_json` dispatch.

## 3. Architecture

### 3.1 Handler shape

One handler struct per tool, in its own file under `services/mcp_v2/handlers/`. Each handler holds:

```rust
pub struct AccountStatusHandler {
    description: ToolDescription,
    registry: &'static AbilityRegistry,
    runtime: tokio::runtime::Handle,
}
```

The runtime handle is captured at construction time (during gateway registration, when we are inside the tokio runtime that rmcp drives). The sync `invoke()` method calls `self.runtime.block_on(async { ... })` to dispatch the underlying async ability. This resolves the sync/async bridge question from the handoff: **Option 2 from the handoff (carry-handle-in-struct)**, not Option 1 (`Handle::current()` lookup at call time, which risks panic if called outside a runtime context). The legacy MCP bridge implicitly assumes a runtime context because it's only called from rmcp; the W1-A `dispatch` trait is sync, so we must construct an explicit bridge.

Rejected alternatives:
- **Option 1** (`Handle::current()` lookup at invoke time): panics in non-runtime tests; capturing at construction is strictly safer.
- **Option 3** (change trait to async): forces W1-A test stubs + transport layer changes for a problem solvable in the handler.
- **Option 4** (return `Pin<Box<dyn Future>>` from trait): verbose, requires W1-A trait change.

### 3.2 Ability dispatch within the handler

The handler's `invoke` body mirrors `bridges/mcp.rs:391-422` (the legacy MCP bridge's `invoke_ability`):

```rust
fn invoke(&self, actor: &McpActor, params: Value) -> Result<Value, ToolError> {
    self.runtime.block_on(async {
        let clock = SystemClock;
        let rng = SystemRng;
        let external = ExternalClients::default();
        let services = ServiceContext::new_live(&clock, &rng, &external)
            .with_actor(MCP_V2_ACTOR_LABEL);
        let invocation = InvocationContext {
            actor: BridgeActor::from_mcp_actor(actor),
            mode: ExecutionMode::Live,
            surface: BridgeSurface::McpTool,
            claim_dismissal_surface: ClaimDismissalSurface::McpTool,
            dry_run: false,
            confirmation: None,
            confirmation_store: None,
        };
        let response = invoke_registry_json(
            self.registry,
            &services,
            None,  // provider
            None,  // tracer (W2-A: no per-invocation tracing yet; W3 adds it)
            invocation,
            "account_overview",  // or "get_daily_briefing"
            params,
        )
        .await
        .map_err(map_invoke_error_to_tool_error)?;
        Ok(response.data)
    })
}
```

The actor label can reuse the existing `bridges/mcp.rs::MCP_ACTOR_LABEL = "mcp"` — local-to-local single-user, no audit-differentiation requirement that justifies a new constant. The `BridgeActor::from_mcp_actor` projection converts `services::mcp_v2::McpActor::Client { client_id, conversation_handle }` to `abilities_runtime::Actor::McpClient { client_id, conversation_handle }` — same shape, trivial conversion. Most likely lands as a small adapter in `services/mcp_v2/actor_policy.rs::project_actor` or sibling.

### 3.3 Registration helper

New module `services/mcp_v2/handlers/registration.rs`:

```rust
pub fn register_v147_handlers(
    gateway: &mut Gateway,
    catalog: &YamlTaxonomyCatalog,
    registry: &'static AbilityRegistry,
    runtime: tokio::runtime::Handle,
) -> Result<(), RegistrationError> {
    gateway.register(Arc::new(AccountStatusHandler::new(
        catalog.description_for(&account_status_name())?.clone(),
        registry,
        runtime.clone(),
    )));
    gateway.register(Arc::new(DailyBriefingHandler::new(
        catalog.description_for(&daily_briefing_name())?.clone(),
        registry,
        runtime.clone(),
    )));
    Ok(())
}
```

Called from `mcp_v2/main.rs::run_serve` between `Gateway::new()` + `set_taxonomy(...)` and `seal()`. The seal step (W1-B AC-7) validates handler↔catalog parity — if our two handlers are registered but the catalog has 8 unregistered slots, seal logs the gap via `validate_catalog_against_handlers` but does not fail (W1-B AC-7 explicitly: catalog→handler returns a list, not an error).

### 3.4 Ability attribute amendments

**`account_overview` (`abilities-runtime/src/abilities/account_overview.rs:34`):**

Current:
```rust
#[ability(
    name = "account_overview",
    category = Read,
    allowed_actors = [User, SurfaceClient],
    mcp_exposure = Invocable,
    ...
)]
```

Amendment:
```rust
    allowed_actors = [User, SurfaceClient, McpClient],
```

**Justification.** The ability already declares `mcp_exposure = Invocable`, so adding `McpClient` to the allowlist is a consistency fix, not a new exposure decision. Local-to-local single-user topology: no CSO escalation cycle warranted. Just amend.

**`get_daily_briefing` (`abilities-runtime/src/abilities/get_daily_briefing/mod.rs:27`):**

Current:
```rust
#[ability(
    name = "get_daily_briefing",
    category = Read,
    allowed_actors = [User, SurfaceClient],
    mcp_exposure = None,
    ...
)]
```

Amendment:
```rust
    allowed_actors = [User, SurfaceClient, McpClient],
    mcp_exposure = Invocable,
```

**Justification.** Explicitly authorized by DOS-624 §1 and §3 (cycle-2 normative). The DOS-624 amendment is unanimous APPROVE on this branch.

### 3.5 Output translation

Per ADR-0125 + ADR-0108, the producer ability renders claim sensitivity before returning. The handler's translation is `serde_json::to_value(response.data)` — no re-rendering, no redaction layer in the handler. The `AbilityResponseJson` already wraps the rendered output. This matches the legacy `McpAbilityBridge::invoke_ability` pattern (line 411-430): the bridge returns `AbilityResponseJson` directly; the consumer (rmcp layer) serializes for wire. **Open question Q2** (§7): the V1 bridge wraps the response in a session-scoped provenance cache (`insert_provenance` at `bridges/mcp.rs:431-436`). W2-A does not include a per-session provenance cache yet. Reviewers: is this a W2-A AC or W2-B follow-up? Cycle-1 stance: defer to W2-B (the user-facing demo doesn't depend on detail-pane provenance lookup).

## 4. Out-of-scope (explicit deferrals)

These items are NOT in W2-A cycle 1 and have specific deferral reasoning:

1. **`dailyos.read.meeting_briefing`** — DOS-624 §3.4 authorizes MCP exposure and specifies strict protections (uniform unavailable response across 4 unavailable classes, 20ms timing floor with p95 ± 5ms burst test). The producer ability mapping is ambiguous: `prepare_meeting` is `Transform` category, not `Read`, and its output shape doesn't match the catalog's `{meeting_id, attendees, threads}` returns schema. Producer wiring requires either a new `Read`-category meeting_briefing ability or a careful repurpose of `prepare_meeting`'s output. **Defer to W2-A Phase-2** with explicit DOS-175 sub-ticket. The demo outcome (account overview + daily briefing) is unaffected.
2. **`dailyos.read.portfolio_attention`** — producer ability does not exist; `get_entity_intelligence` is per-entity not per-portfolio. Defer to W3.
3. **`dailyos.search.workspace_memory`** — no producer ability; depends on v1.4.5 workspace substrate landing on this branch. Defer.
4. **`dailyos.read.workspace_source_provenance`** — same as above.
5. **`dailyos.write.place_document`** + **`dailyos.submit.{note,action,action_status}`** — Write-side abilities; require additional CSO amendment surface (W3-C parallel-amendment open question per DOS-624 §8 Q3). Defer.

## 5. Acceptance criteria

**AC-1.** `account_overview` ability attribute amended: `allowed_actors` includes `McpClient`. Compile + clippy green.

**AC-2.** `get_daily_briefing` ability attribute amended: `allowed_actors` includes `McpClient`; `mcp_exposure = Invocable`. Compile + clippy green.

**AC-3.** New file `services/mcp_v2/handlers/tool_account_status.rs` implementing `McpToolHandler` for the `dailyos.read.account_status` scoped name. Handler dispatches to `account_overview` ability via `invoke_registry_json` using the §3.2 pattern. Captures `tokio::runtime::Handle` at construction.

**AC-4.** `services/mcp_v2/handlers/tool_briefing.rs` filled in (currently 1-line placeholder) with `DailyBriefingHandler` implementing the same pattern for `dailyos.read.daily_briefing` → `get_daily_briefing`.

**AC-5.** `services/mcp_v2/handlers/registration.rs` exists and exports `register_v147_handlers(&mut Gateway, &YamlTaxonomyCatalog, &'static AbilityRegistry, tokio::runtime::Handle) -> Result<(), RegistrationError>`. Called from `mcp_v2/main.rs::run_serve` between `gateway.set_taxonomy(...)` and `gateway.seal()`. `handlers/mod.rs` updated to export `tool_account_status` + `registration`.

**AC-6.** `Gateway::seal()` succeeds with two handlers registered against a 10-tool catalog (handler↔catalog mismatch returns 8 unregistered slugs as a `Vec`, not an `Err` — per W1-B AC-7 split). Test asserts this.

**AC-7.** Unit test per handler: given a stub `McpActor::Client { client_id, conversation_handle }` and stub params (`{"subject": "acme"}` for account_status; `{}` for daily_briefing), the handler returns a `Value` (mock the registry dispatch via an injected test helper, or run against a hermetic test registry).

**AC-8.** Integration test `tests/dos175_w2a_read_handlers_test.rs` exercises the happy path: spin up a `dailyos-mcp-v2 serve` subprocess against a temp DB, pair via stdio, issue `tools/list` (assert both tool slugs present), issue `tools/call` for `dailyos.read.account_status` with a seeded test account, assert response is a non-error JSON value with the expected fields from `AccountOverviewOutput`. Cross-tool scope rejection is NOT tested here — local-to-local single-user does not justify a defense-in-depth test for a configuration the user controls at pairing time.

**AC-9.** End-to-end manual smoke (L4): `dailyos-mcp-v2 pair --client-name claude --grant dailyos.read.account_status:dailyos.read.account_status:invocable --grant dailyos.read.daily_briefing:dailyos.read.daily_briefing:invocable --format claude-desktop` emits a valid `claude_desktop_config.json` snippet; after pasting + restart, Claude Desktop lists both tools; the prompt "give me an account overview for Acme" routes to `dailyos.read.account_status` and returns the user's real Acme substrate.

**AC-10.** No regression: `cargo check --features mcp --lib`, `cargo clippy --features mcp --lib --bin dailyos-mcp-v2 -- -D warnings`, `cargo test --features mcp` all green. No L1 self-validation gaps before opening the PR.

## 6. Test plan

- **Unit** (per handler): mock registry dispatch (provide test-only fake via cfg(test) hook or thread the registry as a generic param) and assert wire shape conversion. ~30 LOC each.
- **Seal/parity test**: assert `seal()` succeeds with 2 handlers registered, and `validate_catalog_against_handlers` returns the expected 8-slug `Vec` of unregistered tools.
- **Integration test** (single file, `tests/dos175_w2a_read_handlers_test.rs`): subprocess + stdio JSON-RPC; covers tools/list, tools/call success path for both tools, and an unauthorized scope (manifest grants `account_status` only; call `daily_briefing` → expect `ToolError::Unauthorized`).

**Out of scope for cycle-1 tests:** concurrent burst latency (W2-B), provenance cache (W2-B), meeting_briefing existence oracle (deferred per §4).

## 7. Open questions for L0 reviewers

**Q1 (architecture).** Should the `BridgeActor::from_mcp_actor` projection live in `services/mcp_v2/actor_policy.rs` (next to `project_actor`) or in a new `bridges/mcp_v2.rs` mirroring the legacy bridge layout? The actor projection helper is the only cross-crate signature decision in W2-A; everything else follows W1-A precedent.

**Q2 (provenance).** Defer per-session provenance cache to W2-B? V1 bridge maintains a session-scoped `(InvocationId → RenderedProvenance)` map for detail-pane lookups. The end-to-end demo (Claude Desktop rendering substrate as text) doesn't depend on this. Cycle-1 stance: defer.

**Q3 (meeting_briefing deferral).** Is the §4.1 deferral of `meeting_briefing` to W2-A Phase-2 acceptable given DOS-624 §3.4's explicit AC inclusion? Cycle-1 stance: producer mapping work is non-trivial and decoupled from the demo outcome; phase-2 sub-ticket of DOS-175 captures the residual without losing the audit trail. The DOS-624 §3.4 timing-floor / p95-burst test is itself a multi-actor-style defense that the local-to-local topology arguably does not require; reviewers may consider whether the original DOS-624 §3.4 requirement should itself be revisited or whether it's load-bearing for a future remote-to-local surface (W3-C WP MCP adapter).

## 8. Reviewer dispatch (cycle 1)

- `/codex challenge` — adversarial: try to break the handler design, the sync/async bridge, the registration ordering.
- `architect-reviewer` — domain: substrate work, registry coupling, ADR-0102/0108/0125 conformance.
- `/plan-devex-review` — MCP/DX surface: tool exposure, Claude Desktop pairing flow, error shapes.
- `/codex consult` — fifth panelist: independent sanity check.

K-in mandatory for each: grep `docs/solutions/` and `.docs/decisions/` for substrate cited above; cite hits in verdict. Unanimous required.

## 9. Definition of done

The session reaches DoD when:
1. Unanimous L0 APPROVE from all four reviewers
2. AC-1 through AC-10 validated with real data
3. L2 (codex review + code-reviewer + architect-reviewer) unanimous APPROVE on the W2-A diff, bounded by AC
4. L4 manual smoke (AC-9) executed and captured: screenshot of Claude Desktop returning substrate response for "give me an account overview for Acme"

If reviewers convergently push back on the §4 deferrals (especially meeting_briefing), reset scope rather than stack patches per memory `feedback_review_loop_diminishing_returns_means_scope_is_wrong`.

exec
/bin/zsh -lc "sed -n '1,260p' .docs/plans/v1.4.7-w1-foundation/dos-624-cso-amendment.md" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
# DOS-624 — CSO L0 amendment for MCP exposure of daily/meeting briefing intelligence

**Type:** L0 amendment (security-annotated)
**Wave:** v1.4.7 W2-A predecessor (gates DOS-175 implementation)
**Lane spec:** [DOS-624](https://linear.app/a8c/issue/DOS-624)
**Origin:** v1.4.3 carve-out — `get_daily_briefing` shipped as Read-only User actor only. No Agent/MCP exposure without separate CSO-approved L0 amendment. This is that amendment.

## Cycle-2 changelog (2026-05-20)

Cycle-1 verdicts: all 4 NEEDS-CHANGES. **Convergent (2+ reviewers):**
- **meeting_briefing existence oracle / enumeration** (CSO + challenge)
- **W3-C two-paths actor_kind tagging is factually wrong** (architect + challenge + devex)

Cycle-2 fixes:

1. **Architect §4 rewrite — actor_kind correctly distinguishes paths** (architect HIGH + challenge + devex convergent). The cycle-1 amendment claimed "both paths write via Actor::McpClient variants distinguishable by actor_kind." Architect correctly flagged: there's only ONE `Actor::McpClient { client_id, conversation_handle }` variant; `audit_log.rs:215` collapses all McpClient invocations to `actor_kind = "mcp_client"`. The W3-C WordPress MCP Adapter path uses `Actor::SurfaceClient { instance, scopes }` per ADR-0111 §8 + ADR-0129 — audit tags `actor_kind = "surface_client"`. So the two paths ARE distinguishable, just by `surface_client` vs `mcp_client`, not by two McpClient variants. §4 corrected.

2. **§4 W3-C policy store correction** (architect HIGH). ADR-0129 §4: WP uses WP capability + WP MCP Adapter allowlist + `SurfaceClient` scopes per ADR-0111 §8. Not `mcp_client_manifest` / `mcp_tool_grant` (which is W1-A direct-path-only). §4 specifies independent policy stores per path; operator grants briefing access in BOTH places independently. No shared manifest schema.

3. **§3 sensitivity gate composition** (architect MED). The cycle-1 §3.1 protection said "Handler MUST filter to caller's read.entity_names scope." Architect correctly flagged this should COMPOSE the existing centralized claim sensitivity gate (ADR-0125 + per-call ClaimSensitivity render in services::claims::render), NOT copy redaction logic. §3.1 rewritten to require DOS-175 handler routes briefing output through the centralized sensitivity render with caller's manifest scopes; no parallel redaction.

4. **meeting_briefing uniform unavailable response in DOS-175 AC** (CSO HIGH + challenge HIGH convergent). Existence-oracle defense: DOS-175 W2-A must include `dailyos.read.meeting_briefing(meeting_id)` returning a uniform `{status: "unavailable"}` typed result for ALL of: nonexistent meeting_id, manifest scope insufficient for entity_names, filtered by sensitivity, stale beyond freshness threshold. This pushes the "object resolution hygiene" into W2-A AC explicitly. CSO Q2 conditional sign-off requires this.

5. **CSO #2 audit coverage for unavailable responses** — DOS-175 makes unavailable responses successful typed results (per #4), so the W1-A success-path audit (`audit::write` with `event="mcp.tool_invoked"`) covers them naturally. `params_hash` over `(meeting_id)` fingerprints probing patterns; operator detects via duplicate `params_hash` across many `conversation_handle`s.

6. **Q1 sign-off folded — per-pairing only** (CSO + devex confirm). Default-denied per-pairing grants + revocation are sufficient for v1.4.7. Global policy registry remains path-α (separate ticket: "DOS-? v1.4.7+ MCP global briefing policy registry").

7. **Q2 sign-off folded — manifest scope authorization + uniform unavailable response** (CSO conditional sign-off). Combined gate.

8. **Singleton substrate hardening filed as separate tickets, NOT folded into this amendment** (per `feedback_review_loop_diminishing_returns_means_scope_is_wrong` + `feedback_l2_path_alpha_to_maintenance_project`):
   - **DOS-? "MCP pairing credential-theft defense"** (challenge #1) — host/client-key binding, one-time pairing codes, short pairing TTL, operator-visible device identity. Real substrate work for v1.4.7+; not blocking this amendment because the post-pairing trust contract already covers replay (per ADR-0102 §C.bis.replay/refresh/schema/fail-closed which CSO independently verified holds).
   - **DOS-? "MCP per-user global briefing rate-limit budget"** (challenge #4) — path-coalesced anomaly detection across W1-A direct + W3-C WP paths. Substrate work for v1.4.7+.
   - **DOS-? "MCP scope grant UI: no glob, explicit briefing scope, confirmation copy"** (challenge #5) — operator UI work, post-v1.4.7.
   - **DOS-? "MCP response_hash side-channel mitigation"** (challenge #2) — per-row salt or security-admin-only fingerprint querying. Substrate work for v1.4.7+.
   - **DOS-? "MCP operator pairing UX: briefing intelligence section"** (devex #1) — operator-facing docs + manifest UI changes. Post-v1.4.7.
   - **DOS-? "MCP rate-limit config keys + retry-after surfacing"** (devex #2) — DOS-175 should make `rate_limit_max=60`, `rate_limit_window_secs=3600` discoverable; gateway's `ToolError::RateLimited { retry_after_seconds }` already exists in W1-A so this is documentation + config wiring, not substrate.
   - **DOS-? "MCP audit dashboard query for briefing replay anomaly detection"** (devex #3) — explicit SQL query schema for operator dashboards.
   - **DOS-? "MCP two-paths-one-ability operator runbook"** (devex #4) — pre-W2-A docs work; how to grant/revoke each path independently; how to audit both sources via `actor_kind` filter.

   These are real, named, scoped — filed in this amendment rather than folded keeps the amendment as a policy decision (its proper shape) rather than turning it into a substrate-hardening initiative.

---

## 1. What this amendment unblocks (unchanged from cycle 1)

v1.4.7 W2-A (DOS-175) ships two MCP tools wrapping the v1.4.3 `get_daily_briefing` ability:
- `dailyos.read.daily_briefing` — current-day briefing
- `dailyos.read.meeting_briefing(meeting_id)` — prep briefing for a specific meeting

The v1.4.3 carve-out blocks them pending this amendment.

## 2. Why MCP exposure is appropriate now (unchanged + CSO verified)

The v1.4.3 carve-out predates v1.4.7 W1-A's MCP trust contract. Pre-W1-A, MCP exposure meant unauthenticated unbounded ability invocation. v1.4.7 W1-A landed (commits `45b3f53d`..`3e5d7155` on `v1.4.7-w1-foundation`; L2 unanimous APPROVE; CSO confirmed in this cycle's review: "No new transport replay vector found. ADR-0102 nonce consume + per-response refresh + fail-closed semantics are implemented in `auth.rs`/`gateway.rs` as described.") Every MCP-originated invocation now passes through:

1. Pairing handshake → server-issued `McpClientId` + operator-set manifest
2. Transport HMAC-SHA256 signing over canonical-JSON envelope including `request_nonce`; key wrapped in `Zeroizing`
3. Server-side nonce ledger consume-once + fail-closed (per ADR-0102 §C.bis.replay/refresh/schema/fail-closed)
4. Server-side manifest scope authorization (caller-asserted scopes rejected)
5. Per-tool exposure tier (None / MetadataOnly / Invocable; default None)
6. Per `(McpClientId, ScopedName)` rate limit with atomic BEGIN IMMEDIATE
7. Audit attribution with keyed HMAC-SHA256 hashes (no raw payloads)
8. Signal emission via `McpToolInvoked` / `McpInvocationRejected` (NonPiiMetadata)

Pre-W1-A threat model (unauthenticated MCP exposure of personal briefing intelligence) no longer applies.

## 3. Briefing-specific protections (cycle-2 sensitivity gate composed)

Beyond W1-A's universal gates, DOS-175 (W2-A) MUST honor these briefing-specific ACs:

1. **Sensitivity rendering composes the centralized claim sensitivity gate** (cycle-2 fix #3 per architect MED). DOS-175 handler routes briefing output through `services::claims::render` (or the equivalent centralized sensitivity rendering path per ADR-0125 + ADR-0108) using the caller's manifest-resolved scopes. PII-tier claims (file paths, raw entity names, raw claim text in aggregates) redact when caller's manifest does not grant the corresponding read scope (`read.entity_names`). NO parallel redaction logic in the handler.

2. **Per-pairing tool grant default = denied** (unchanged). New pairings receive no briefing access by default. Operator must explicitly grant `dailyos.read.daily_briefing` and `dailyos.read.meeting_briefing` scopes + Invocable exposure tier at pairing time.

3. **Per-invocation rate limit default = 60 calls/hour** (unchanged from cycle 1). Operator-tunable.

4. **Uniform unavailable response for meeting_briefing(meeting_id)** (cycle-2 fix #4 per CSO HIGH + challenge HIGH). DOS-175 handler returns `{status: "unavailable", reason_class: "unavailable"}` (NOT distinguishing nonexistent / scope-insufficient / sensitivity-filtered / stale-beyond-freshness — uniform shape) for ALL of:
   - meeting_id not in caller's accessible meeting set
   - meeting exists but scope insufficient for entity_names disclosure
   - meeting exists, accessible, but ClaimSensitivity gate filters all surfaceable claims
   - meeting exists but briefing freshness exceeds operator-configured threshold (stale)
   
   **Uniform timing floor for unavailable success responses** (cycle-3 CSO normative tightening). The W1-A AC-12 10ms floor applies only to auth-state rejections, NOT to successful typed responses. DOS-175 handler **MUST** call `tokio::time::sleep_until(start + Duration::from_millis(20))` (20ms or greater; not "recommended" — required) before returning ANY `{status: "unavailable"}` response, regardless of which internal branch (nonexistent / scope-insufficient / sensitivity-filtered / stale) produced it. **Concurrent burst test required: p95 latency across all 4 unavailable branches MUST be within ± 5ms of each other** (burst of ≥ 200 calls under concurrent load, asserted in `tests/dos175_meeting_briefing_existence_oracle_test.rs`). Closes CSO cycle-2 + cycle-3 enumeration-via-latency findings.

5. **Audit covers unavailable responses** (cycle-2 fix #5 per CSO #2). Because unavailable is a successful typed result, W1-A's success-path audit naturally fingerprints it via `params_hash` over `(meeting_id)`. Operators detect probing via duplicate `params_hash` across many `conversation_handle`s. No new audit substrate needed.

## 4. Coordination with v1.4.2 W3-C WordPress MCP Adapter (cycle-2 corrected)

Per cycle-2 fix #1 + #2 (architect HIGH convergent):

- **Both paths consume the SAME ability** (`get_daily_briefing`). Producer/renderer split per ADR-0130 supports surface-agnostic producer abilities.
- **Different actor classes, distinguishable in audit**:
  - W1-A direct path (Claude Desktop / Cursor / other agents over loopback HTTP or stdio MCP) → `Actor::McpClient { client_id, conversation_handle }` → audit `actor_kind = "mcp_client"`
  - W3-C WP MCP Adapter path (WordPress block + adapter) → `Actor::SurfaceClient { instance, scopes }` per ADR-0111 §8 + ADR-0129 §4 → audit `actor_kind = "surface_client"`
- **Different policy stores per path** (cycle-2 fix #2):
  - W1-A direct: `mcp_client_manifest` + `mcp_tool_grant` rows (v241 migration; loaded by `services::mcp_v2::auth`)
  - W3-C WP: WP capability + WP MCP Adapter allowlist + `SurfaceClient` scope grants per ADR-0111 §8
  - Operator MUST grant briefing access in BOTH places independently — no shared manifest schema, no implicit propagation.
- **No duplicate audit rows** — each invocation through either path writes ONE audit row attributed to its respective Actor variant; the audit log's `actor_kind` field distinguishes the source.
- **Operator runbook for two-paths coordination** filed as separate ticket (cycle-2 fix #8 — "DOS-? MCP two-paths-one-ability operator runbook").

## 5. Audit + signal payload changes (NONE required) (unchanged)

This amendment adds no new audit event types, no new signal types, no new claim types. The W1-A substrate handles briefing invocations identically to any other tool. The amendment is a scope decision, not a substrate change.

## 6. Acceptance

- **CSO L0 sign-off** on this amendment (Q1 + Q2 cycle-1 sign-offs folded per cycle-2 fix #6 + #7).
- **No code changes in this amendment** — scope/policy only. Implementation lands in DOS-175 (W2-A).
- **W2-A DOS-175 L0 packet** (separate) MUST cite this amendment as predecessor + enumerate §3 ACs in its own AC (especially #1 sensitivity-gate composition + #4 uniform unavailable response + #5 audit fingerprinting).

## 7. Spinoff Linear tickets (cycle-2 fix #8 filed list)

Filed to DailyOS Maintenance project (`b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`) per path-α discipline:

- "MCP pairing credential-theft defense" (challenge #1; substrate)
- "MCP per-user global briefing rate-limit budget" (challenge #4; substrate)
- "MCP scope grant UI: no glob, explicit briefing scope, confirmation copy" (challenge #5; UX)
- "MCP response_hash side-channel mitigation" (challenge #2; substrate)
- "MCP operator pairing UX: briefing intelligence section" (devex #1; docs + UI)
- "MCP rate-limit config keys + retry-after surfacing" (devex #2; docs + DOS-175 AC reference)
- "MCP audit dashboard query for briefing replay anomaly detection" (devex #3; ops dashboards)
- "MCP two-paths-one-ability operator runbook" (devex #4; pre-W2-A docs)

## 8. Open questions — none remaining (CSO Q1 + Q2 signed off in cycle 1+2; cycle-2 folded)

## 9. Reviewer dispatch (cycle 2)

- **CSO** (mandatory primary) — verify Q1 + Q2 sign-offs folded correctly + new §3 #4 uniform unavailable response is sufficient existence-oracle defense
- **/codex challenge** (adversarial) — verify the 8 spinoff tickets actually capture the cycle-1 findings adequately; nothing dropped that should block
- **architect-reviewer** (substrate) — verify cycle-2 fix #1+#2 actor_kind framing + #3 sensitivity composition match ADR-0111/0125/0129/0130
- **/plan-devex-review** (DX) — verify cycle-2 fix #8 ticket-filing-vs-folding split is the right call vs cycle-1 ask

exec
/bin/zsh -lc 'rg --files .docs/plans/v1.4.7-w1-foundation docs/solutions .docs/decisions' in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
docs/solutions/security-issues/prompt-channel-sensitivity-class-sweep-2026-05-18.md
docs/solutions/conventions/migration-filename-version-offset-2026-05-18.md
docs/solutions/README.md
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/consult.md
docs/solutions/tooling-decisions/pre-push-hook-duration-vs-ssh-idle-timeout-2026-05-19.md
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/devex.md
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/codex-challenge.md
.docs/plans/v1.4.7-w1-foundation/reviews/dos-175-cycle-1/architect.md
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md
.docs/plans/v1.4.7-w1-foundation/dos-mcp-transport-l0-plan.md
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md
.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md
.docs/plans/v1.4.7-w1-foundation/dos-624-cso-amendment.md
docs/solutions/tooling-decisions/codex-worktree-isolation-incompatible-with-rescue-forwarder-2026-05-18.md
docs/solutions/tooling-decisions/phpcs-warning-severity-zero-prevents-warning-only-ci-fails-2026-05-19.md
docs/solutions/tooling-decisions/wip-and-no-verify-escape-hatches-under-memory-pressure-2026-05-20.md
docs/solutions/tooling-decisions/gh-pr-merge-delete-branch-multi-worktree-incompatibility-2026-05-19.md
docs/solutions/tooling-decisions/codex-companion-task-worker-hang-finalAnswerSeen-2026-05-20.md
.docs/decisions/0079-role-presets.md
.docs/decisions/0106-prompt-fingerprinting-and-provider-interface.md
.docs/decisions/0092-data-security-at-rest-and-operational-hardening.md
.docs/decisions/0067-resume-latency-and-db-concurrency-guardrails.md
.docs/decisions/0085-email-as-intelligence-input.md
.docs/decisions/0011-sidebar-groups.md
.docs/decisions/0129-commitment-claim-identity.md
.docs/decisions/0068-oauth-pkce-keychain-hardening.md
.docs/decisions/0121-team-intelligence-architecture.md
.docs/decisions/0107-source-taxonomy-alignment.md
.docs/decisions/0122-chapter-enrichment.md
.docs/decisions/0089-user-entity-and-professional-context.md
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md
.docs/decisions/0017-pure-rust-archive.md
.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md
.docs/decisions/0091-intelligence-provider-abstraction.md
.docs/decisions/0025-app-native-governance.md
.docs/decisions/0054-list-page-design-pattern.md
.docs/decisions/0029-three-tier-email-priority.md
.docs/decisions/0028-structured-document-schemas.md
.docs/decisions/0093-prompt-injection-hardening.md
.docs/decisions/0088-people-relationship-network-intelligence.md
.docs/decisions/0111-surface-independent-ability-invocation.md
.docs/decisions/0030-weekly-prep-with-daily-refresh.md
.docs/decisions/0063-meeting-preview-type-architecture.md
.docs/decisions/0132-pill-primitive-dual-existence.md
.docs/decisions/0081-event-driven-meeting-intelligence.md
.docs/decisions/0014-mvp-scope.md
.docs/decisions/0024-email-ai-triage-not-client.md
.docs/decisions/0095-dual-mode-context-architecture.md
.docs/decisions/0008-profile-aware-navigation.md
.docs/decisions/0099-remote-first-server-canonical-architecture.md
.docs/decisions/0051-user-configurable-metadata-settings.md
.docs/decisions/0002-frontend-first-implementation.md
.docs/decisions/0102-abilities-as-runtime-contract.md
.docs/decisions/0005-archives-remain-markdown-only.md
.docs/decisions/0018-hybrid-storage-markdown-sqlite.md
.docs/decisions/0062-briefing-artifacts-vs-live-queries.md
.docs/decisions/0016-defer-post-meeting-to-phase-3.md
.docs/decisions/0009-non-destructive-profile-switching.md
.docs/decisions/0080-signal-intelligence-architecture.md
.docs/decisions/0110-evaluation-harness-for-abilities.md
.docs/decisions/0055-schedule-first-dashboard-layout.md
.docs/decisions/0124-longitudinal-topic-threading.md
.docs/decisions/0036-inbox-processing-in-phase-1.md
.docs/decisions/0072-auto-updater-architecture.md
.docs/decisions/0035-incremental-prep-generation.md
.docs/decisions/0046-entity-mode-architecture.md
.docs/decisions/0037-post-meeting-capture-in-phase-1.md
.docs/decisions/0109-temporal-primitives-in-the-entity-graph.md
.docs/decisions/0023-post-meeting-capture-replaces-wrap.md
.docs/decisions/0052-week-page-redesign.md
.docs/decisions/0100-glean-first-intelligence-architecture.md
.docs/decisions/0116-tenant-control-plane-boundary.md
.docs/decisions/0064-meeting-prep-page-redesign.md
.docs/decisions/0098-data-governance-source-aware-lifecycle.md
.docs/decisions/0034-adaptive-dashboard.md
.docs/decisions/0084-surface-jobs-to-be-done.md
.docs/decisions/0010-simplified-sidebar.md
.docs/decisions/0077-magazine-layout-editorial-redesign.md
.docs/decisions/0007-dashboard-is-the-product.md
.docs/decisions/0097-account-health-scoring-architecture.md
.docs/decisions/0047-entity-dashboard-architecture.md
.docs/decisions/0120-observability-contract.md
.docs/decisions/0066-meeting-intelligence-package.md
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md
.docs/decisions/0130-surface-independent-composition-contract.md
.docs/decisions/0038-cs-first-development.md
.docs/decisions/0021-multi-signal-meeting-classification.md
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md
.docs/decisions/0108-provenance-rendering-and-privacy.md
.docs/decisions/0006-determinism-boundary.md
.docs/decisions/0094-audit-log-and-enterprise-observability.md
.docs/decisions/README.md
.docs/decisions/0003-config-json-no-ui-for-mvp.md
.docs/decisions/0041-two-sided-impact-model.md
.docs/decisions/0126-memory-substrate-invariants.md
.docs/decisions/0013-meeting-detail-drill-down.md
.docs/decisions/0012-profile-indicator-in-sidebar.md
.docs/decisions/0131-structured-embedding-claim-canonicalization.md
.docs/decisions/0040-archive-reconciliation.md
.docs/decisions/0061-calendar-event-id-as-meeting-key.md
.docs/decisions/0060-unified-radix-ui-monorepo.md
.docs/decisions/0086-intelligence-as-shared-service.md
.docs/decisions/0058-proactive-intelligence-maintenance.md
.docs/decisions/0015-defer-inbox-to-phase-2.md
.docs/decisions/0056-parent-child-accounts.md
.docs/decisions/0059-entity-directory-template.md
.docs/decisions/0045-entity-abstraction.md
.docs/decisions/0074-vector-search-entity-content.md
.docs/decisions/0073-editorial-design-language.md
.docs/decisions/0001-use-tauri-over-electron.md
.docs/decisions/0075-conversational-interface-architecture.md
.docs/decisions/0004-hybrid-json-markdown-architecture.md
.docs/decisions/0050-universal-file-extraction.md
.docs/decisions/0096-glean-mode-local-footprint.md
.docs/decisions/0071-schema-migration-framework.md
.docs/decisions/0076-brand-identity.md
.docs/decisions/0123-typed-claim-feedback-semantics.md
docs/solutions/architecture-patterns/capability-boundary-needs-crate-split-not-grep-2026-05-18.md
docs/solutions/architecture-patterns/append-only-jsonl-schema-change-preserves-hash-chain-2026-05-20.md
docs/solutions/architecture-patterns/emit-or-log-wrapper-silent-error-swallow-class-2026-05-18.md
docs/solutions/architecture-patterns/L3-catches-wave-level-bypass-of-sub-ticket-discipline-2026-05-20.md
.docs/decisions/0065-meeting-prep-editability.md
.docs/decisions/0078-nomic-embed-text-model-switch.md
.docs/decisions/0019-reference-approach-for-directives.md
.docs/decisions/0101-service-boundary-enforcement.md
.docs/decisions/0087-entity-hierarchy-intelligence.md
.docs/decisions/0117-publish-boundary-pencil-and-pen.md
.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md
.docs/decisions/0103-maintenance-ability-safety-constraints.md
.docs/decisions/0027-mcp-dual-mode.md
.docs/decisions/0053-dashboard-ux-redesign.md
.docs/decisions/0043-meeting-intelligence-is-core.md
.docs/decisions/0082-entity-generic-prep-pipeline.md
.docs/decisions/0104-execution-mode-and-mode-aware-services.md
.docs/decisions/0039-feature-toggle-architecture.md
.docs/decisions/0026-extension-architecture.md
.docs/decisions/0070-internal-team-entities.md
.docs/decisions/0069-radix-ui-standalone-packages.md
.docs/decisions/0032-calendar-source-of-truth.md
.docs/decisions/0090-user-entity-page-architecture.md
.docs/decisions/0114-scoring-unification.md
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md
.docs/decisions/0119-runtime-evaluator-pass-for-transform-abilities.md
.docs/decisions/0049-eliminate-python-runtime.md
.docs/decisions/0083-product-vocabulary.md
.docs/decisions/0022-proactive-research-unknown-meetings.md
.docs/decisions/0044-meeting-scoped-transcript-intake.md
.docs/decisions/0048-three-tier-data-model.md
.docs/decisions/0057-entity-intelligence-architecture.md
.docs/decisions/0105-provenance-as-first-class-output.md
.docs/decisions/0042-per-operation-pipelines.md
.docs/decisions/0127-presets-as-intelligence-contracts.md
.docs/decisions/0020-profile-dependent-accounts.md
.docs/decisions/0033-meeting-entity-unification.md
.docs/decisions/0031-actions-source-of-truth.md
.docs/decisions/0115-signal-granularity-audit.md
docs/solutions/test-failures/hmac-canonical-change-requires-golden-vector-recompute-2026-05-20.md
docs/solutions/workflow-issues/pr-template-security-auditor-field-regex-collision-2026-05-20.md
docs/solutions/test-failures/parallel-test-singleton-state-flake-2026-05-18.md
docs/solutions/workflow-issues/gh-pr-edit-body-not-picked-up-by-run-rerun-2026-05-20.md
docs/solutions/workflow-issues/worktree-setup-needs-pnpm-install-2026-05-19.md
docs/solutions/workflow-issues/parallel-wave-synced-from-conflicts-2026-05-19.md
docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md
docs/solutions/workflow-issues/substrate-only-landing-needs-l0-amendment-2026-05-18.md
docs/solutions/workflow-issues/node-modules-tracked-symlink-enotdir-pnpm-install-2026-05-19.md
docs/solutions/workflow-issues/parallel-agent-commit-hook-contention-2026-05-20.md
docs/solutions/workflow-issues/l0-review-loop-diminishing-returns-means-scope-is-wrong-2026-05-20.md
docs/solutions/workflow-issues/codex-rescue-destructive-git-reset-on-529-2026-05-20.md
docs/solutions/workflow-issues/premise-check-production-vs-dev-friction-before-scoping-waves-2026-05-20.md
docs/solutions/workflow-issues/codex-agent-dispatched-but-no-file-changes-2026-05-20.md

codex
The first pass found an important tension: DOS-624 explicitly says W2-A includes both `daily_briefing` and `meeting_briefing`, while the L0 packet defers `meeting_briefing`. I’m checking the wave plan and the cited substrate docs now to see whether that is a real blocker or a documented scope reset.
exec
/bin/zsh -lc 'rg -n "DOS-175|W2-A|account_status|daily_briefing|meeting_briefing|get_daily_briefing|account_overview|McpToolHandler|AbilityRegistry|invoke_registry_json|McpAbilityBridge|ADR-0102|ADR-0108|ADR-0125|ADR-0093|ADR-0128|W1-A handler seam" .docs/plans/v1.4.7-waves.md .docs/plans/v1.4.7-w1-foundation/*.md' in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
.docs/plans/v1.4.7-w1-foundation/dos-624-cso-amendment.md:4:**Wave:** v1.4.7 W2-A predecessor (gates DOS-175 implementation)
.docs/plans/v1.4.7-w1-foundation/dos-624-cso-amendment.md:6:**Origin:** v1.4.3 carve-out — `get_daily_briefing` shipped as Read-only User actor only. No Agent/MCP exposure without separate CSO-approved L0 amendment. This is that amendment.
.docs/plans/v1.4.7-w1-foundation/dos-624-cso-amendment.md:11:- **meeting_briefing existence oracle / enumeration** (CSO + challenge)
.docs/plans/v1.4.7-w1-foundation/dos-624-cso-amendment.md:20:3. **§3 sensitivity gate composition** (architect MED). The cycle-1 §3.1 protection said "Handler MUST filter to caller's read.entity_names scope." Architect correctly flagged this should COMPOSE the existing centralized claim sensitivity gate (ADR-0125 + per-call ClaimSensitivity render in services::claims::render), NOT copy redaction logic. §3.1 rewritten to require DOS-175 handler routes briefing output through the centralized sensitivity render with caller's manifest scopes; no parallel redaction.
.docs/plans/v1.4.7-w1-foundation/dos-624-cso-amendment.md:22:4. **meeting_briefing uniform unavailable response in DOS-175 AC** (CSO HIGH + challenge HIGH convergent). Existence-oracle defense: DOS-175 W2-A must include `dailyos.read.meeting_briefing(meeting_id)` returning a uniform `{status: "unavailable"}` typed result for ALL of: nonexistent meeting_id, manifest scope insufficient for entity_names, filtered by sensitivity, stale beyond freshness threshold. This pushes the "object resolution hygiene" into W2-A AC explicitly. CSO Q2 conditional sign-off requires this.
.docs/plans/v1.4.7-w1-foundation/dos-624-cso-amendment.md:24:5. **CSO #2 audit coverage for unavailable responses** — DOS-175 makes unavailable responses successful typed results (per #4), so the W1-A success-path audit (`audit::write` with `event="mcp.tool_invoked"`) covers them naturally. `params_hash` over `(meeting_id)` fingerprints probing patterns; operator detects via duplicate `params_hash` across many `conversation_handle`s.
.docs/plans/v1.4.7-w1-foundation/dos-624-cso-amendment.md:31:   - **DOS-? "MCP pairing credential-theft defense"** (challenge #1) — host/client-key binding, one-time pairing codes, short pairing TTL, operator-visible device identity. Real substrate work for v1.4.7+; not blocking this amendment because the post-pairing trust contract already covers replay (per ADR-0102 §C.bis.replay/refresh/schema/fail-closed which CSO independently verified holds).
.docs/plans/v1.4.7-w1-foundation/dos-624-cso-amendment.md:36:   - **DOS-? "MCP rate-limit config keys + retry-after surfacing"** (devex #2) — DOS-175 should make `rate_limit_max=60`, `rate_limit_window_secs=3600` discoverable; gateway's `ToolError::RateLimited { retry_after_seconds }` already exists in W1-A so this is documentation + config wiring, not substrate.
.docs/plans/v1.4.7-w1-foundation/dos-624-cso-amendment.md:38:   - **DOS-? "MCP two-paths-one-ability operator runbook"** (devex #4) — pre-W2-A docs work; how to grant/revoke each path independently; how to audit both sources via `actor_kind` filter.
.docs/plans/v1.4.7-w1-foundation/dos-624-cso-amendment.md:46:v1.4.7 W2-A (DOS-175) ships two MCP tools wrapping the v1.4.3 `get_daily_briefing` ability:
.docs/plans/v1.4.7-w1-foundation/dos-624-cso-amendment.md:47:- `dailyos.read.daily_briefing` — current-day briefing
.docs/plans/v1.4.7-w1-foundation/dos-624-cso-amendment.md:48:- `dailyos.read.meeting_briefing(meeting_id)` — prep briefing for a specific meeting
.docs/plans/v1.4.7-w1-foundation/dos-624-cso-amendment.md:54:The v1.4.3 carve-out predates v1.4.7 W1-A's MCP trust contract. Pre-W1-A, MCP exposure meant unauthenticated unbounded ability invocation. v1.4.7 W1-A landed (commits `45b3f53d`..`3e5d7155` on `v1.4.7-w1-foundation`; L2 unanimous APPROVE; CSO confirmed in this cycle's review: "No new transport replay vector found. ADR-0102 nonce consume + per-response refresh + fail-closed semantics are implemented in `auth.rs`/`gateway.rs` as described.") Every MCP-originated invocation now passes through:
.docs/plans/v1.4.7-w1-foundation/dos-624-cso-amendment.md:58:3. Server-side nonce ledger consume-once + fail-closed (per ADR-0102 §C.bis.replay/refresh/schema/fail-closed)
.docs/plans/v1.4.7-w1-foundation/dos-624-cso-amendment.md:69:Beyond W1-A's universal gates, DOS-175 (W2-A) MUST honor these briefing-specific ACs:
.docs/plans/v1.4.7-w1-foundation/dos-624-cso-amendment.md:71:1. **Sensitivity rendering composes the centralized claim sensitivity gate** (cycle-2 fix #3 per architect MED). DOS-175 handler routes briefing output through `services::claims::render` (or the equivalent centralized sensitivity rendering path per ADR-0125 + ADR-0108) using the caller's manifest-resolved scopes. PII-tier claims (file paths, raw entity names, raw claim text in aggregates) redact when caller's manifest does not grant the corresponding read scope (`read.entity_names`). NO parallel redaction logic in the handler.
.docs/plans/v1.4.7-w1-foundation/dos-624-cso-amendment.md:73:2. **Per-pairing tool grant default = denied** (unchanged). New pairings receive no briefing access by default. Operator must explicitly grant `dailyos.read.daily_briefing` and `dailyos.read.meeting_briefing` scopes + Invocable exposure tier at pairing time.
.docs/plans/v1.4.7-w1-foundation/dos-624-cso-amendment.md:77:4. **Uniform unavailable response for meeting_briefing(meeting_id)** (cycle-2 fix #4 per CSO HIGH + challenge HIGH). DOS-175 handler returns `{status: "unavailable", reason_class: "unavailable"}` (NOT distinguishing nonexistent / scope-insufficient / sensitivity-filtered / stale-beyond-freshness — uniform shape) for ALL of:
.docs/plans/v1.4.7-w1-foundation/dos-624-cso-amendment.md:83:   **Uniform timing floor for unavailable success responses** (cycle-3 CSO normative tightening). The W1-A AC-12 10ms floor applies only to auth-state rejections, NOT to successful typed responses. DOS-175 handler **MUST** call `tokio::time::sleep_until(start + Duration::from_millis(20))` (20ms or greater; not "recommended" — required) before returning ANY `{status: "unavailable"}` response, regardless of which internal branch (nonexistent / scope-insufficient / sensitivity-filtered / stale) produced it. **Concurrent burst test required: p95 latency across all 4 unavailable branches MUST be within ± 5ms of each other** (burst of ≥ 200 calls under concurrent load, asserted in `tests/dos175_meeting_briefing_existence_oracle_test.rs`). Closes CSO cycle-2 + cycle-3 enumeration-via-latency findings.
.docs/plans/v1.4.7-w1-foundation/dos-624-cso-amendment.md:91:- **Both paths consume the SAME ability** (`get_daily_briefing`). Producer/renderer split per ADR-0130 supports surface-agnostic producer abilities.
.docs/plans/v1.4.7-w1-foundation/dos-624-cso-amendment.md:109:- **No code changes in this amendment** — scope/policy only. Implementation lands in DOS-175 (W2-A).
.docs/plans/v1.4.7-w1-foundation/dos-624-cso-amendment.md:110:- **W2-A DOS-175 L0 packet** (separate) MUST cite this amendment as predecessor + enumerate §3 ACs in its own AC (especially #1 sensitivity-gate composition + #4 uniform unavailable response + #5 audit fingerprinting).
.docs/plans/v1.4.7-w1-foundation/dos-624-cso-amendment.md:121:- "MCP rate-limit config keys + retry-after surfacing" (devex #2; docs + DOS-175 AC reference)
.docs/plans/v1.4.7-w1-foundation/dos-624-cso-amendment.md:123:- "MCP two-paths-one-ability operator runbook" (devex #4; pre-W2-A docs)
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:1:# DOS-175 — MCP v2 W2-A read handlers (account_status + daily_briefing) — L0 plan packet
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:3:**Wave:** v1.4.7 W2-A
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:4:**Lane spec:** [DOS-175](https://linear.app/a8c/issue/DOS-175)
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:5:**Wave plan reference:** `.docs/plans/v1.4.7-waves.md` §"Agent W2-A — DOS-175"
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:7:**Authoring discipline:** narrow-scoped per K-in lessons; scope confined to wiring already-authored producers into the W1-A handler seam.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:11:W2-A ships two read-side MCP tool handlers wrapping abilities that exist today in `abilities-runtime`:
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:13:1. **`dailyos.read.account_status`** → wraps `account_overview` ability (`src-tauri/abilities-runtime/src/abilities/account_overview.rs:34`)
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:14:2. **`dailyos.read.daily_briefing`** → wraps `get_daily_briefing` ability (`src-tauri/abilities-runtime/src/abilities/get_daily_briefing/mod.rs:27`)
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:16:These are the two read tools whose producer abilities exist in the registry and whose MCP exposure is either already declared (`account_overview` carries `mcp_exposure = Invocable`) or explicitly authorized by a prior amendment (`get_daily_briefing` per DOS-624). The remaining 8 slots in the W1-B catalog do not have aligned producers in `abilities-runtime` today and are out of scope for W2-A cycle 1.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:18:**Threat topology:** `local-to-local single-user`. Claude Desktop on the same machine, same user, loopback stdio MCP transport. Per `engineering-ladder.md` §"Threat-topology framing" + memory `feedback_local_to_local_security_overreach_primary_concern`: reviewers do NOT enforce multi-actor gates, principal differentiation, cross-actor poisoning, or scope-gated redaction of data the user already has filesystem access to. The only invariants that survive the topology test are: compile correctness, crate-boundary rules, sensitivity rendering already done in the producer (ADR-0108), and indirect-prompt-injection containment from third-party document content (ADR-0093). Reviewers who flag multi-actor gates on this surface waste a cycle; please scope your threat model to "Claude Desktop run by James on James's machine talking to DailyOS running as James."
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:22:End-to-end demo path: user pairs Claude Desktop against `dailyos-mcp-v2`, asks "give me an account overview for Acme" or "what's on my plate today", Claude routes to the appropriate DailyOS tool, the W2-A handler invokes the underlying ability against the same `abilities-runtime` registry the Tauri UI uses, and the substrate response is returned to Claude Desktop. This is the v1.4.7 W2-A bar — first observable MCP product surface against real DailyOS substrate.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:24:The other 8 catalog slots (`meeting_briefing`, `portfolio_attention`, `workspace_memory`, `workspace_source_provenance`, `place_document`, `submit.note`, `submit.action`, `submit.action_status`) require either new producer abilities or non-trivial mapping work and are explicitly deferred to W2-A Phase-2 / W3 / W4 lanes. See §7 below.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:30:- **`AbilityRegistry`** — single global static. `abilities-runtime/src/abilities/registry.rs:838` exposes `pub fn global_checked() -> Result<&'static Self, ...>` backed by `OnceLock`. There is no parallel legacy registry.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:32:- **`invoke_registry_json`** — the shared async dispatch core. Called by Tauri (`bridges/tauri.rs:217`), legacy MCP (`bridges/mcp.rs:411-422`), and now W2-A.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:33:- **`McpAbilityBridge::invoke_ability`** (`src-tauri/src/bridges/mcp.rs:385-430`) — the canonical pattern for "actor + ability_name + input_json → AbilityResponseJson" over the same registry. W2-A handlers reuse this pattern verbatim (no second invocation engine).
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:34:- **ADR-0102 §C.bis** — abilities-as-runtime contract; covers `mcp_exposure`, `allowed_actors`, attribute changes via re-approval.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:35:- **ADR-0108** — claim-sensitivity rendering; producer abilities render sensitivity before returning. Handler is a thin JSON wrapper.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:36:- **ADR-0125** — claim-sensitivity rendering depth; same as above.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:37:- **DOS-624 CSO amendment** — explicit MCP exposure authorization for `get_daily_briefing` per AC-507.5.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:38:- **W1-A handler seam** — `gateway.rs:155` (`handlers: HashMap<ScopedName, Arc<dyn McpToolHandler>>`), `gateway.rs:177` (`register`), `gateway.rs:271` (`dispatch`). Scaffolding directory `services/mcp_v2/handlers/` already exists with 1-line placeholder files including `tool_briefing.rs`.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:39:- **`McpToolHandler` trait** — `services/mcp_v2/contracts.rs:341`: sync `invoke(&self, &McpActor, params: Value) -> Result<Value, ToolError>`.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:41:**No substrate reinvention.** W2-A is purely the wiring layer between W1-A's `McpToolHandler` registration seam and the existing `invoke_registry_json` dispatch.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:52:    registry: &'static AbilityRegistry,
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:85:        let response = invoke_registry_json(
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:89:            None,  // tracer (W2-A: no per-invocation tracing yet; W3 adds it)
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:91:            "account_overview",  // or "get_daily_briefing"
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:111:    registry: &'static AbilityRegistry,
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:115:        catalog.description_for(&account_status_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:120:        catalog.description_for(&daily_briefing_name())?.clone(),
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:132:**`account_overview` (`abilities-runtime/src/abilities/account_overview.rs:34`):**
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:137:    name = "account_overview",
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:152:**`get_daily_briefing` (`abilities-runtime/src/abilities/get_daily_briefing/mod.rs:27`):**
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:157:    name = "get_daily_briefing",
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:175:Per ADR-0125 + ADR-0108, the producer ability renders claim sensitivity before returning. The handler's translation is `serde_json::to_value(response.data)` — no re-rendering, no redaction layer in the handler. The `AbilityResponseJson` already wraps the rendered output. This matches the legacy `McpAbilityBridge::invoke_ability` pattern (line 411-430): the bridge returns `AbilityResponseJson` directly; the consumer (rmcp layer) serializes for wire. **Open question Q2** (§7): the V1 bridge wraps the response in a session-scoped provenance cache (`insert_provenance` at `bridges/mcp.rs:431-436`). W2-A does not include a per-session provenance cache yet. Reviewers: is this a W2-A AC or W2-B follow-up? Cycle-1 stance: defer to W2-B (the user-facing demo doesn't depend on detail-pane provenance lookup).
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:179:These items are NOT in W2-A cycle 1 and have specific deferral reasoning:
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:181:1. **`dailyos.read.meeting_briefing`** — DOS-624 §3.4 authorizes MCP exposure and specifies strict protections (uniform unavailable response across 4 unavailable classes, 20ms timing floor with p95 ± 5ms burst test). The producer ability mapping is ambiguous: `prepare_meeting` is `Transform` category, not `Read`, and its output shape doesn't match the catalog's `{meeting_id, attendees, threads}` returns schema. Producer wiring requires either a new `Read`-category meeting_briefing ability or a careful repurpose of `prepare_meeting`'s output. **Defer to W2-A Phase-2** with explicit DOS-175 sub-ticket. The demo outcome (account overview + daily briefing) is unaffected.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:189:**AC-1.** `account_overview` ability attribute amended: `allowed_actors` includes `McpClient`. Compile + clippy green.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:191:**AC-2.** `get_daily_briefing` ability attribute amended: `allowed_actors` includes `McpClient`; `mcp_exposure = Invocable`. Compile + clippy green.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:193:**AC-3.** New file `services/mcp_v2/handlers/tool_account_status.rs` implementing `McpToolHandler` for the `dailyos.read.account_status` scoped name. Handler dispatches to `account_overview` ability via `invoke_registry_json` using the §3.2 pattern. Captures `tokio::runtime::Handle` at construction.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:195:**AC-4.** `services/mcp_v2/handlers/tool_briefing.rs` filled in (currently 1-line placeholder) with `DailyBriefingHandler` implementing the same pattern for `dailyos.read.daily_briefing` → `get_daily_briefing`.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:197:**AC-5.** `services/mcp_v2/handlers/registration.rs` exists and exports `register_v147_handlers(&mut Gateway, &YamlTaxonomyCatalog, &'static AbilityRegistry, tokio::runtime::Handle) -> Result<(), RegistrationError>`. Called from `mcp_v2/main.rs::run_serve` between `gateway.set_taxonomy(...)` and `gateway.seal()`. `handlers/mod.rs` updated to export `tool_account_status` + `registration`.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:201:**AC-7.** Unit test per handler: given a stub `McpActor::Client { client_id, conversation_handle }` and stub params (`{"subject": "acme"}` for account_status; `{}` for daily_briefing), the handler returns a `Value` (mock the registry dispatch via an injected test helper, or run against a hermetic test registry).
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:203:**AC-8.** Integration test `tests/dos175_w2a_read_handlers_test.rs` exercises the happy path: spin up a `dailyos-mcp-v2 serve` subprocess against a temp DB, pair via stdio, issue `tools/list` (assert both tool slugs present), issue `tools/call` for `dailyos.read.account_status` with a seeded test account, assert response is a non-error JSON value with the expected fields from `AccountOverviewOutput`. Cross-tool scope rejection is NOT tested here — local-to-local single-user does not justify a defense-in-depth test for a configuration the user controls at pairing time.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:205:**AC-9.** End-to-end manual smoke (L4): `dailyos-mcp-v2 pair --client-name claude --grant dailyos.read.account_status:dailyos.read.account_status:invocable --grant dailyos.read.daily_briefing:dailyos.read.daily_briefing:invocable --format claude-desktop` emits a valid `claude_desktop_config.json` snippet; after pasting + restart, Claude Desktop lists both tools; the prompt "give me an account overview for Acme" routes to `dailyos.read.account_status` and returns the user's real Acme substrate.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:213:- **Integration test** (single file, `tests/dos175_w2a_read_handlers_test.rs`): subprocess + stdio JSON-RPC; covers tools/list, tools/call success path for both tools, and an unauthorized scope (manifest grants `account_status` only; call `daily_briefing` → expect `ToolError::Unauthorized`).
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:215:**Out of scope for cycle-1 tests:** concurrent burst latency (W2-B), provenance cache (W2-B), meeting_briefing existence oracle (deferred per §4).
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:219:**Q1 (architecture).** Should the `BridgeActor::from_mcp_actor` projection live in `services/mcp_v2/actor_policy.rs` (next to `project_actor`) or in a new `bridges/mcp_v2.rs` mirroring the legacy bridge layout? The actor projection helper is the only cross-crate signature decision in W2-A; everything else follows W1-A precedent.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:223:**Q3 (meeting_briefing deferral).** Is the §4.1 deferral of `meeting_briefing` to W2-A Phase-2 acceptable given DOS-624 §3.4's explicit AC inclusion? Cycle-1 stance: producer mapping work is non-trivial and decoupled from the demo outcome; phase-2 sub-ticket of DOS-175 captures the residual without losing the audit trail. The DOS-624 §3.4 timing-floor / p95-burst test is itself a multi-actor-style defense that the local-to-local topology arguably does not require; reviewers may consider whether the original DOS-624 §3.4 requirement should itself be revisited or whether it's load-bearing for a future remote-to-local surface (W3-C WP MCP adapter).
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:228:- `architect-reviewer` — domain: substrate work, registry coupling, ADR-0102/0108/0125 conformance.
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:239:3. L2 (codex review + code-reviewer + architect-reviewer) unanimous APPROVE on the W2-A diff, bounded by AC
.docs/plans/v1.4.7-w1-foundation/dos-175-l0-plan.md:242:If reviewers convergently push back on the §4 deferrals (especially meeting_briefing), reset scope rather than stack patches per memory `feedback_review_loop_diminishing_returns_means_scope_is_wrong`.
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:52:3. Fixture density (challenge + ceo) — ADR-0128 §B requires ≥ 2 positive + ≥ 2 negative per tool; fixed by AC-5 update + per-entry YAML fixture-stub fields
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:54:5. AC-4 weak (ceo + challenge) — ADR-0128 needs BOTH broad-corpus AND adjacent-wrong-tool; fixed by AC-4 split
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:68:   - name: dailyos.read.account_status
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:84:     scopes_required: [dailyos.read.account_status]
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:92:         invocation: { name: dailyos.read.account_status, arguments: { subject: acme } }
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:96:         - { prompt: "What's going on with Acme?", expected_tool: dailyos.read.account_status }
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:97:         - { prompt: "How are things looking with the Hooli deal?", expected_tool: dailyos.read.account_status }
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:126:- ADR-0128 changes (already amended in W0 commit 45b3f53d).
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:133:- **ADR-0102 §E** — scope namespace freeze.
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:134:- **ADR-0128 §3 + §6 + cycle-7 §B amendment** — tool descriptions are product copy; cross-conversation continuity named as affordance; host-selection eval (DOS-481) requires ≥ 2 positive + ≥ 2 negative fixtures per tool with broad-corpus AND adjacent-wrong-tool negatives.
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:149:| 1 | `dailyos.read.account_status` | Read | `dailyos.read.account_status` | W2 (out-of-scope handler — discovery surface from DOS-172 ranked entity lookup) |
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:150:| 2 | `dailyos.read.daily_briefing` | Read | `dailyos.read.daily_briefing` | W2-A (DOS-175) |
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:151:| 3 | `dailyos.read.meeting_briefing` | Read | `dailyos.read.meeting_briefing` | W2-A (DOS-175) |
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:167:- **AC-4 Displacement framing in product vocabulary** (cycle-1 ceo + challenge convergent + cycle-2 ceo tightening). Every entry's `when_NOT_to_call` MUST contain BOTH (a) at least one broad-corpus keyword: `broad`, `enterprise search`, `web search`, `Glean`, `corpus`; AND (b) at least one adjacent-tool framing in PRODUCT LANGUAGE: `different surface`, `not for ... — use ... for`, `personal working`, or natural-language reference to the alternative. **AC-4 ALSO forbids**: substring `dailyos.` AND system identifiers (`Read|SubmitCorrection|Write|McpClient|ToolError|McpToolHandler|Gateway|Scope`) in `when_to_call` / `when_NOT_to_call` natural-language fields per cycle-2 CEO. Tool identifiers appear ONLY in structured `selection_fixtures.*.expected_tool` field.
.docs/plans/v1.4.7-w1-foundation/dos-478-l0-plan.md:169:- **AC-6 Continuity affordance in product vocabulary** (cycle-1 ceo #5 + cycle-2 ceo tightening). Every Read entry's `when_to_call` MUST mention continuity using one of: `cross-conversation`, `persists context`, `updates as work evolves`, `working understanding ... over time`. Per ADR-0128 §6 named-affordance contract. Same vocabulary discipline as AC-4 — no `dailyos.` substrings, no system identifiers.
.docs/plans/v1.4.7-w1-foundation/dos-mcp-transport-l0-plan.md:171:- ADR-0128 §B — MCP as product surface
.docs/plans/v1.4.7-w1-foundation/dos-mcp-transport-l0-plan.md:172:- ADR-0102 §C.bis.replay/refresh — per-envelope HMAC + nonce
.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md:6:**Authoring discipline:** scoped tight; cycle-2 same-shape findings driven by my substrate-grep errors → cycle-3 grounds every claim with the verified path. Cycle-2 architectural questions resolved via small W0 amendment (~30 LOC + ADR-0102 cycle-7 text) folded into this packet as a precondition deliverable.
.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md:15:2. **ADR-0102 cycle-9 amendment** — §C.bis.refresh (per-response nonce refresh; seed at pairing, fresh nonce per response, asymmetric flow), §C.bis.schema (full ledger DDL with `issued_at`/`expires_at`/`consumed_at` + 3 indexes + atomic consume SQL), §C.bis.fail-closed (consumed nonce stays consumed even on downstream failure — fail-closed against TCP-reset replay attack), §C.bis.sweep (background sweep cadence + grace window).
.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md:20:1. **§1 #1 gateway flow narrative** updated to use the new request envelope shape: Gate 0 verifies HMAC over (payload + `envelope.request_nonce`); calls `auth::verify_and_consume_nonce(client_id, envelope.request_nonce)` per ADR-0102 §C.bis.refresh. On success, gateway invokes handler; on response, calls `auth::issue_next_nonce(client_id)` → returns `next_request_nonce` in response envelope.
.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md:25:6. **Migration v243 schema reflects ADR-0102 §C.bis.schema**: full DDL with `nonce, client_id, issued_at, expires_at, consumed_at` + 3 indexes + atomic consume predicate + background sweep + 1h grace window.
.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md:36:**L6-1 (replay guard) = nonce ledger.** Per ADR-0102 §C cycle-8 amendment (this cycle): every MCP message MUST carry a server-issued `request_nonce` signed in the HMAC; server stores recently-seen nonces in `mcp_transport_nonce_ledger` (v243-shifted-up to v244 — see slot reallocation below); replay attempts within window → `ToolError::BadParams { detail: "nonce_replayed" }`. Mirrors `services::surface_nonce::verify_and_consume` pattern verbatim. Closes cycle-5 CSO HIGH.
.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md:54:7. **ADR-0102 §C cycle-8 amendment** (this cycle) documents the nonce ledger requirement: 5-minute window, `services::surface_nonce` precedent, BadParams variants. Lands in Deliverable 0 alongside cycle-7.
.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md:67:2. **ServiceMutationCursor REMOVED from Deliverable 0 / W0 amendment scope** (4/4 convergent). The `McpToolHandler::invoke` W0-frozen trait signature does NOT change. Cursor enforcement happens at the gateway level via JSON inspection: gateway extracts `result_value.get("mutation_cursor")` after handler success for `Side::Write` handlers. If absent for `Side::Write`, gateway logs a Suite-S warning `mcp_write_handler_missing_cursor` (forensic alert; handler effects preserved) and proceeds to audit append with `mutation_cursor` field absent in detail JSON. The cursor convention is documented in W1-A `taxonomy.rs` `TaxonomyCatalog` trait docs (Side::Write handlers SHOULD populate cursor). No new W0 type added.
.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md:86:Convergent NEW HIGH (3/4): AC-2 internal inconsistency between §1 #1 and the AC body on `Unauthorized` vs `ExposureForbidden` branching. **All cycle-3 "STILL-OPEN" findings about Deliverable 0 are now closed by substrate landing** — `contracts.rs` carries `ExposureForbidden` variant + golden parity test; ADR-0102 carries cycle-7 amendment (§C.bis + §D.bis); wave plan stale text fixed at lines 85, 453, 467.
.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md:90:1. **Deliverable 0 has landed in this same branch** (closes the 4/4 "shape-only is a trap" finding). Substrate-truth, not packet promise. Cycle-4 reviewers can grep `ExposureForbidden` in `contracts.rs` + read ADR-0102 cycle-7 amendment + verify wave plan lines 85/453/467 carry the new text.
.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md:98:9. **Wave-plan stale text fixed and verified in same commit** (architect MED): lines 85, 453, 467 now carry the dedicated-variant language per ADR-0102 cycle-7 amendment.
.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md:111:   - New `ToolError::ExposureForbidden { tool_name: ScopedName }` variant. Same shape as `PairingRevoked`/`ConversationRevoked` per ADR-0102 §C — auth-state classes that do not abuse `Scope` newtype. New parity test added.
.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md:112:   - `OpaqueConversationHandle` wire shape unchanged (still opaque string); server-side binding spec amended in ADR-0102 §D — handles bind to `client_id` at mint time, resolve by `(client_id, handle)`, cross-client presentation returns `ConversationRevoked`. Wire layer stays compatible.
.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md:113:   ADR-0102 §C/§D cycle-7 amendment text drafted as part of this commit (~6 lines added to ADR-0102).
.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md:123:11. **Mid-call revocation audit state** (Class X — challenge MED): if revocation lands between manifest load and handler return, audit row's `detail` includes `revocation_state: "revoked_during_invocation"` per ADR-0102 §C.
.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md:148:1. **`src-tauri/src/services/mcp_v2/gateway.rs`** — `Gateway::handle_tool_call(envelope: McpToolRequestEnvelope) -> McpToolResponseEnvelope`. Per-dispatch flow (matches AC-2 verbatim): **Gate 0a** `auth::verify_transport_hmac(payload, sig)` — HMAC failure → `ToolError::BadParams { detail: "invalid_signature" }`. **Gate 0b** `auth::verify_and_consume_nonce(client_id, envelope.request_nonce)` per ADR-0102 §C.bis.refresh (cycle-9 amendment) — atomic SQL `UPDATE mcp_transport_nonce_ledger SET consumed_at = ?now WHERE client_id = ?c AND nonce = ?n AND consumed_at IS NULL AND expires_at >= ?now`; on `rowsAffected = 0` reject with `{ detail: "nonce_replayed" }` (row consumed) or `{ detail: "invalid_signature" }` (row absent/expired — uniform shape). Fail-closed: consumed nonce stays consumed even on downstream failure. Then: `auth::load_client_record(client_id)` → reject if `revoked_at.is_some()` (→ `PairingRevoked`); `auth::resolve_tool_grant(client_id, tool_name)` (indexed lookup, single row) → **absent grant OR `grant.exposure != Invocable` → `ExposureForbidden { tool_name }`**; **grant present AND invocable but `tool.description().scopes_required.is_subset(grant.scopes_granted)` false → `Unauthorized { missing_scope }`**; `auth::resolve_or_mint_handle(client_id, envelope.conversation_handle)` (mismatch → `ConversationRevoked`); enforce rate limit BEFORE dispatch via atomic ledger insert (AC-5 explicit tx order); construct `McpActor::Client`; dispatch via registered `McpToolHandler`; on success → extract `mutation_cursor` from `result_value.get("mutation_cursor")` for `Side::Write` (Suite-S warning if absent for Write — SHOULD not MUST per AC-6); → `audit::write(event, fields, actor)` via emit-or-log path (handler effects preserved on audit failure); emit `SignalType::McpToolInvoked`; **call `auth::issue_next_nonce(client_id)` to mint fresh nonce** → return `next_request_nonce` in response envelope. On any auth-state rejection (BadParams from Gate 0a/0b, PairingRevoked, ExposureForbidden, Unauthorized, ConversationRevoked): NO audit row written; emit `SignalType::McpInvocationRejected` via emit-or-log helper (failure → fallback log + Suite-S alert `mcp_reject_signal_emit_failed` per AC-7); gateway calls `tokio::time::sleep_until(start + Duration::from_millis(10))` for AC-12 timing parity; mint fresh nonce on response anyway (so client can recover).
.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md:149:2. **`src-tauri/src/services/mcp_v2/auth.rs`** — `pair_client(handshake) -> McpClientId` (mirrors `services::surface_pairing` + `services::surface_session_keychain` per ADR-0102 §C); `load_client_record(client_id) -> ClientRecord`; `resolve_tool_grant(client_id, tool_name) -> Option<ToolGrant>` (indexed query); `verify_transport_hmac(payload, sig)` (loads key for one verify, drops); `resolve_or_mint_handle(client_id, presented) -> OpaqueConversationHandle` (mint on None; resolve by `(client_id, handle)`; mismatch → `ConversationRevoked`); `revoke_handle / revoke_client` admin paths.
.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md:150:3. **`src-tauri/src/services/mcp_v2/audit.rs`** — `write(event, actor, params, response_or_error, request_id)`. Computes keyed HMAC-SHA256 hashes per ADR-0102 §C. Calls `AuditLogger::append_with_actor(event = "mcp.tool_invoked", actor = &Actor::McpClient { .. }, fields = AuditFields { category: "security", detail: serde_json!({client_id, conversation_handle, tool_name, params_hash, response_hash, revocation_state?, mutation_cursor?}), wp_user_id: None, wp_user_hash: None, request_id: Some(...) })`. On JSONL append failure → insert into `mcp_audit_outbox` table. On outbox failure → return `ToolError::Internal { trace_id }` to caller + emit Suite-S `mcp_audit_double_failure` alert.
.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md:151:4. **`src-tauri/src/services/mcp_v2/actor_policy.rs`** — `ClientRecord -> AbilityPolicy` projection consumed by gateway. Per ADR-0102 §B asymmetry, `Actor::McpClient` carries no scopes; this module owns the projection.
.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md:152:5. **`src-tauri/src/services/mcp_v2/taxonomy.rs`** — W1-A defines the CATALOG INTERFACE only: `pub trait TaxonomyCatalog { fn validate_against_handlers(&self, handlers: &[&dyn McpToolHandler]) -> Result<(), TaxonomyError> }` + `TaxonomyError::HandlerYamlMismatch { handler: ScopedName, yaml: Option<ScopedName> }`. W1-B fills the YAML loader and invokes `validate_against_handlers` at boot.
.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md:157:   - **v243** (cycle-6 NEW per L6-1; cycle-7 schema completed per ADR-0102 §C.bis.schema) `243_mcp_transport_nonce_ledger.sql` — `mcp_transport_nonce_ledger (nonce TEXT, client_id TEXT, issued_at INTEGER, expires_at INTEGER, consumed_at INTEGER NULL)` with `UNIQUE(nonce, client_id)` + `idx_mcp_nonce_lookup ON (client_id, nonce)` + `idx_mcp_nonce_consumed ON (consumed_at)` + `idx_mcp_nonce_expires ON (expires_at)`. Window: 5 minutes (`expires_at = issued_at + 5min`) per ADR-0102 §C.bis.refresh. Atomic consume per §C.bis.schema: `UPDATE ... SET consumed_at = ?now WHERE client_id = ?c AND nonce = ?n AND consumed_at IS NULL AND expires_at >= ?now`. Background sweep every 5 min deletes rows where `expires_at < now() - 1 hour` (1h grace per §C.bis.sweep). Fail-closed per §C.bis.fail-closed.
.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md:163:The wave plan §W1-A explicitly admits coexistence: v2 introduces a v2-paired ingress for clients that opt into the ADR-0102 §C contract; legacy `src/mcp/main.rs` + `src/bridges/mcp.rs:402` continue serving `Actor::Agent` (pre-pairing substrate). The two paths route by `Actor` variant at the abilities-runtime layer (per ADR-0102 §F `AbilityPolicy.allowed_actors`).
.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md:173:- **W0 contracts (`src-tauri/src/services/mcp_v2/contracts.rs`)** — `ToolDescription`, `Side`, `ScopedName`, `Scope`, `ParamSpec`, `ReturnSpec`, `ToolExample`, `Page<T>`, `OpaqueConversationHandle` (wire envelope, opaque string), `McpClientId`, `McpActor::Client`, `McpToolRequestEnvelope` (no granted_scopes — gateway hydrates), `McpToolResponseEnvelope` (frozen: `conversation_handle` + `result`; no audit warnings), `McpToolResult`, `McpToolHandler` trait, `ToolError` (Deliverable 0 adds `ExposureForbidden`).
.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md:177:- **ADR-0102 §C/§D/§E + cycle-7 amendment (Deliverable 0)** — four-gate trust contract, handle lifecycle with client_id binding, scope namespace.
.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md:181:- **ADR-0128 §6 + §C** — continuity affordance; wire shape defers to ADR-0102 §D.
.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md:220:- **AC-4 Pairing revocation.** Per-dispatch `ClientRecord` load → revoked → `PairingRevoked`. Propagates within ≤ 1 call per ADR-0102 §C.
.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md:222:- **AC-6 Audit append (cycle-6 L6-2: success-only).** Handler success precedes audit append. Gateway calls `AuditLogger::append_with_actor(event="mcp.tool_invoked", actor=&actor, fields=AuditFields::new("security", detail).with_request_id(req_id))`. Detail JSON: `{client_id, conversation_handle, tool_name, params_hash, response_hash, mutation_cursor?, revocation_state?}` — `mutation_cursor` is IDs-only `serde_json::Value` (max depth 4, max 2 KiB; truncate → `"truncated_oversize"`), required for `Side::Write` (handler-filled per `taxonomy.rs` rustdoc), omitted for `Side::Read`; `revocation_state` only present if revocation lands mid-call per ADR-0102 §C. Auth-state rejections do NOT write an audit row — handled via `McpInvocationRejected` signal (AC-7) per L6-2. JSONL append fails → insert into `mcp_audit_outbox` (operator-only access via existing `get_audit_log_records` / `export_audit_log` paths; no MCP exposure, no separate CLI). Outbox insert also fails → `ToolError::Internal { trace_id }` + Suite-S alert `mcp_audit_double_failure`. Raw params/responses NEVER stored. Audit is post-commit attribution, not source-of-order truth.
.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md:233:- **Integration**: stub `McpToolHandler` proving (a) audit row iff handler success, (b) JSONL fail → outbox path, (c) JSONL+outbox fail → Internal + Suite-S alert, (d) signal payload carries only `tool_name + client_id + conversation_handle`, (e) rate-limit ledger consumes per full tuple atomically, (f) cross-session same-client (AC-3b.a), (g) cross-client rejection (AC-3b.b), (h) revocation within 1 call.
.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md:241:Per ADR-0102 §C four gates + W0 cycle-7 amendment + cycle-3 ACs:
.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md:253:12. Mid-call revocation captured in audit `revocation_state` (AC-6 + ADR-0102 §C).
.docs/plans/v1.4.7-w1-foundation/dos-168-l0-plan.md:258:- "v1.4.7 W1.5 — legacy MCP retirement plan" (ADR-0102 §C forward-application clarification + plan to flip clients to v2 or deprecate the legacy path).
.docs/plans/v1.4.7-waves.md:14:- [DOS-624](https://linear.app/a8c/issue/DOS-624) — v1.4.7 W2-A `dailyos.read.daily_briefing` and `dailyos.read.meeting_briefing` MCP tools wrap intelligence the v1.4.3 carve-out reserved as User-only. CSO-approved L0 amendment required before W2-A implementation; amendment must also coordinate with v1.4.2 W3-C MCP path to avoid two parallel MCP exposures of the same intelligence.
.docs/plans/v1.4.7-waves.md:31:1. **`Actor::McpClient` ownership assigned to W0** — `actor.rs` change shipped as part of the ADR-0102 amendment commit, not a separate lane.
.docs/plans/v1.4.7-waves.md:44:8. **Canonical convention: `<dot-namespace>.<verb>.<noun>`** — e.g., `dailyos.read.account_status`, `dailyos.write.place_document`, `dailyos.submit.note`. v1.4.5 already shipped `write.workspace_place_document` for the AI placement ability; v1.4.7 W3-B's MCP tool reuses that exact scope verbatim (no rename). Scope-mapping table added to W0. Closes Challenge HIGH #5 + L0 question #3.
.docs/plans/v1.4.7-waves.md:57:Cycle 2 is heavy — 15 changes across 6 classes — but most are propagation/cleanup. The single substantive new design (Class S MCP client trust boundary) ships as a W0 deliverable. No wave shape changes; no migration slot changes (still v220–v239); no new ADRs beyond the already-planned ADR-0102 + ADR-0128 amendments (Class S folds into ADR-0102 amendment scope).
.docs/plans/v1.4.7-waves.md:66:1. **Tool name propagation sweep.** All lane bodies + release gate now use canonical `dailyos.<verb>.<noun>` names: `dailyos.read.daily_briefing`, `dailyos.read.meeting_briefing`, `dailyos.read.portfolio_attention`, `dailyos.search.workspace_memory`, `dailyos.read.workspace_source_provenance`, `dailyos.write.place_document`, `dailyos.submit.note`, `dailyos.submit.action`, `dailyos.submit.action_status`. W1-B taxonomy text updated to match.
.docs/plans/v1.4.7-waves.md:84:2. **`dailyos.read.briefing` → `dailyos.read.daily_briefing` + `dailyos.read.meeting_briefing`** in W0 scope mapping table (the actual W2-A tool surface ships TWO briefing tools, not one). Closes Eng MED.
.docs/plans/v1.4.7-waves.md:85:3. **OpaqueConversationHandle first-write contradiction resolved** (Eng/DX HIGH cycle 3). Gateway now mints a handle transparently if absent on writes — no `ConversationRequired` error path. The error variant is removed from `ToolError`. Revoked handles return the dedicated `ToolError::ConversationRevoked` variant (added by ADR-0102 cycle-1 amendment; reconciled with cycle-7 amendment `§D.bis` cross-client binding). Reconciles gateway/lifecycle/W3-B security gate to the same single behavior: writes don't fail on missing handle; first-write mints.
.docs/plans/v1.4.7-waves.md:108:- W0 ships: top-level `mod.rs` with all submodule declarations; `contracts.rs` with full shared type set (`ToolDescription`, `ScopedName`, `Scope`, `Verb` enum + `CANONICAL_NAMESPACE` const, `ParamSpec` / `ReturnSpec` / `ToolExample` / `ParamSchema` / `Side` / `Page<T>` / `OpaqueConversationHandle` / `McpClientId` / `McpActor` / `McpToolHandler` trait / `ToolError` / `McpToolRequestEnvelope` / `McpToolResponseEnvelope` / `McpToolResult`); `gateway.rs` / `auth.rs` / `audit.rs` / `actor_policy.rs` / `taxonomy.rs` as empty doc-only placeholders; `handlers/mod.rs` + 10 per-tool handler files as empty placeholders.
.docs/plans/v1.4.7-waves.md:114:**Cycle 6 codex review surfaced four AC-bound mediums** (proc-macro `ActorArg::McpClient` parser arm; `Verb` enum + namespace const not encoded; `ToolError::Unauthorized` abusing `Scope` for auth-state sentinels; missing top-level request/response envelope types) — all four addressed in this same W0 branch before merge. Companion ADR-0102 §C / §D wording reconciled with the new `PairingRevoked` / `ConversationRevoked` `ToolError` variants.
.docs/plans/v1.4.7-waves.md:137:- v1.4.5 waves: `.docs/plans/v1.4.5-waves.md` — workspace memory substrate. **v1.4.7 implementation gates on v1.4.5 merge.** Specifically v1.4.7 consumes: `WorkspaceGraphProjection v1` schema (DOS-489 / W3-C), `PlaceDocumentInput / Receipt / Error` contract (DOS-474 / W4-C), `WorkspaceCategoryRegistry` + `WorkspaceCategory` enum (W1-A `contracts.rs`), `services::workspace_ingestion` mutation surface, `link::override_link` API (DOS-465 / W1-C), `pipeline::quarantine_source` API (W2-A), `WorkspaceFile { kind: WorkspaceFileKind }` `DataSource` variant (ADR-0107 amendment).
.docs/plans/v1.4.7-waves.md:150:1. **Original MCP v2 expansion** (legacy issue set DOS-167–DOS-175): replace stale reads with ability-backed current context, add portfolio attention + ranked discovery + pagination, add controlled feedback/note/action writes through `services::*` with receipts and signal emission.
.docs/plans/v1.4.7-waves.md:162:- `services::claims::commit_claim` (sole writer for `intelligence_claims`); `services::feedback`; `services::actions`; Signal Policy Registry; durable invalidation queue; trust scoring; freshness decay; provenance rendering substrate (ADR-0108).
.docs/plans/v1.4.7-waves.md:163:- `CLAIM_TYPE_REGISTRY` (`abilities-runtime/src/abilities/claims.rs::ClaimType`); `ClaimSensitivity` (ADR-0125).
.docs/plans/v1.4.7-waves.md:175:- `IngestPipeline::run(IngestRequest)` (W2-A; v1.4.7 doesn't call directly — goes through `place_document` ability).
.docs/plans/v1.4.7-waves.md:177:- `services::workspace_ingestion::pipeline::quarantine_source(file_id, reason, actor)` (W2-A; v1.4.7 may surface quarantine as an MCP write).
.docs/plans/v1.4.7-waves.md:197:5. ADR-0128 (headless DailyOS — likely amended in W0) and ADR-0102 (abilities runtime — likely amended in W0).
.docs/plans/v1.4.7-waves.md:220:DX-mandatory lanes (every lane carries `/plan-devex-review` because the entire project is a DX surface, but these have heightened DX scrutiny): DOS-478 (taxonomy + host-selection contract), DOS-481 (tool-description eval), DOS-175 (briefing replacement — change in observable MCP behavior), DOS-172 (pagination/ranking — pagination contract is a permanent shape), DOS-173 (portfolio attention — net-new tool surface).
.docs/plans/v1.4.7-waves.md:246:1. **Issue-body renumbering cleanup.** Several v1.4.7 issues still reference the pre-renumbering `v1.4.9 / v1.4.10` taxonomy: DOS-167, DOS-168, DOS-170, DOS-171, DOS-172, DOS-173, DOS-175, DOS-479, DOS-480, DOS-481, DOS-482 all contain `v1.4.9` (now v1.4.5) and/or `v1.4.10` (now v1.4.7) references in their bodies. W0 includes a one-pass edit on each issue body to substitute current numbering. Doesn't change scope; eliminates a class of reviewer confusion.
.docs/plans/v1.4.7-waves.md:248:2. **Cross-version dependency confirmation — v1.4.5.** Confirm v1.4.5 has merged to `dev` with these specific contracts live and frozen: `WorkspaceGraphProjection v1` + `WorkspaceGraphQueryInput v1` + `WorkspaceGraphResponse v1` (W3-C); `PlaceDocumentInput v1` + `PlaceDocumentReceipt v1` + `PlacementError` typed enum (W4-C); `WorkspaceCategoryRegistry` + `WorkspaceCategory` enum slug serialization (W1-A `contracts.rs`); `pipeline::quarantine_source` (W2-A); `link::override_link(&dyn SignalEmitter, ...)` (W1-C); five new `WorkspaceFile*` SignalType variants + invalidation allowlist additions (W3-B); ADR-0107 `WorkspaceFile { kind: WorkspaceFileKind }` `DataSource` variant. **If any item is not yet merged, v1.4.7 implementation is blocked; document the gap on the affected v1.4.7 issue.**
.docs/plans/v1.4.7-waves.md:251:   - **ADR-0102 (abilities runtime) — expanded scope per cycle 2 Class S.** Covers: (a) `Actor::McpClient` enum variant added to `actor.rs` (the actual code change ships in this same PR — not deferred to W1-A); (b) MCP client authentication + session contract: `McpClientId` issued server-side after pairing handshake, HMAC-SHA256 transport signing, server-side scope manifest (no caller-provided granted scopes), revocation path, per-actor + per-client + per-tool rate-limit registry; (c) scope naming convention pinned (see #4); (d) `mcp_exposure` tri-state `None | MetadataOnly | Invocable` confirmed sufficient (no new variants needed).
.docs/plans/v1.4.7-waves.md:252:   - **ADR-0128 (headless DailyOS)** — for v2 expansion principles: tool-description as product copy, host-selection language, cross-conversation continuity contract (server-minted `OpaqueConversationHandle`, REQUIRED on writes, recommended on reads).
.docs/plans/v1.4.7-waves.md:253:   Both amendments drafted, reviewed, and merged before W1 L0 plans open. **The ADR-0102 amendment PR also lands the `Actor::McpClient` code change in `actor.rs`** — closes Eng cycle 1 HIGH #1 ownership gap. Resolves L0 questions #1, #2, #3, #6.
.docs/plans/v1.4.7-waves.md:263:   | (new) | `dailyos.read.account_status`, `dailyos.read.portfolio_attention`, `dailyos.read.daily_briefing`, `dailyos.read.meeting_briefing` (W2 lanes) |
.docs/plans/v1.4.7-waves.md:270:6. **`Actor::McpClient` introduction.** v1.4.2 introduced `Actor::SurfaceClient`. v1.4.7 needs `Actor::McpClient` for MCP-originated calls — distinct from `SurfaceClient` because the access control matrices, audit attribution, and rate-limit budgets differ. Record as part of ADR-0102 amendment.
.docs/plans/v1.4.7-waves.md:284:| **W2** — Ability-Backed Reads | DOS-172, DOS-173, DOS-175 | 3 | 4–5 days |
.docs/plans/v1.4.7-waves.md:294:- W2 internal staging: DOS-175 (replace stale briefing reads — Bug priority Urgent) ships first as it unblocks downstream parity. DOS-172 (pagination) and DOS-173 (portfolio attention) run in parallel after DOS-175 merges.
.docs/plans/v1.4.7-waves.md:323:- [ ] ADR-0102 amendment drafted (covers `Actor::McpClient` variant + MCP client auth/session contract + scope naming convention)
.docs/plans/v1.4.7-waves.md:324:- [ ] ADR-0128 amendment drafted (covers tool-description as product copy + host-selection language + cross-conversation continuity)
.docs/plans/v1.4.7-waves.md:330:- [ ] Both ADR amendments merged. **The ADR-0102 amendment PR also lands the `Actor::McpClient` enum variant code change in `src-tauri/abilities-runtime/src/actor.rs`** — no other lane is allowed to add this variant (cycle 2 fix for Eng HIGH #1 ownership gap).
.docs/plans/v1.4.7-waves.md:331:- [ ] MCP client authentication + session contract documented in ADR-0102 amendment with concrete pairing handshake spec, HMAC signing spec, server-side scope manifest schema, and revocation path. **No W1-A merge without this in place** (cycle 2 fix for CSO HIGH #1).
.docs/plans/v1.4.7-waves.md:379:          pub name: ScopedName,                // e.g., "dailyos.read.account_status"
.docs/plans/v1.4.7-waves.md:432:      // Actor for MCP-originated calls (cycle 2 Class S — McpClient enum variant lives in actor.rs, shipped as part of W0 ADR-0102 PR)
.docs/plans/v1.4.7-waves.md:445:      pub trait McpToolHandler: Send + Sync {
.docs/plans/v1.4.7-waves.md:455:          ConversationRevoked,                       // also returned when cross-client handle binding violation per ADR-0102 §D.bis
.docs/plans/v1.4.7-waves.md:456:          ExposureForbidden { tool_name: ScopedName }, // absent grant OR non-Invocable exposure per ADR-0102 §C.bis
.docs/plans/v1.4.7-waves.md:470:    - **Revocation:** server-side admin API can revoke a handle; subsequent presentations return the dedicated `ToolError::ConversationRevoked` variant and the caller must restart with a new conversation. (Cross-client binding violations per ADR-0102 §D.bis return the same variant.)
.docs/plans/v1.4.7-waves.md:476:  - ADR-0102 amendment file in `.docs/decisions/0102-*-amendment-v147.md` (W0 deliverable; W1-A inherits the merged version).
.docs/plans/v1.4.7-waves.md:478:- **Don't touch:** `services/claims.rs`, `services/feedback.rs`, `services/actions.rs`, `services/workspace_ingestion/*` (consumer-only); existing MCP server entry points outside `services/mcp_v2/`; `actor.rs` (cycle 1 amendment for `Actor::McpClient` is already covered by the W0 ADR-0102 amendment which lands before W1-A starts).
.docs/plans/v1.4.7-waves.md:486:- **`/plan-devex-review` gate:** the gateway pattern is the substrate every other v1.4.7 tool builds on. The `McpToolHandler` trait shape, error taxonomy, and tool-description type all need to be reviewer-quality DX before subsequent lanes start. `/plan-devex-review` at L0 plan.
.docs/plans/v1.4.7-waves.md:494:- **Done when:** Gateway accepts tool calls, validates actor + scopes, dispatches through registered `McpToolHandler` impls, writes audit + emits `McpToolInvoked` signal; `services/mcp_v2/` skeleton fully populated with empty placeholders; `ToolDescription` + `McpToolHandler` + `ToolError` types in `contracts.rs`; CI lint script `check_mcp_tool_handler_allowlist.sh` in place; all tests green; `/cso` + `/plan-devex-review` L0 plan AND L2 diff approval recorded.
.docs/plans/v1.4.7-waves.md:510:  - Read tools: verb is `read` (default), or one of `search` / `list` / `get` / `prepare` for action-shaped reads. E.g., `dailyos.read.account_status`, `dailyos.read.portfolio_attention`, `dailyos.search.workspace_memory`, `dailyos.read.workspace_source_provenance`.
.docs/plans/v1.4.7-waves.md:541:Three agents. DOS-175 (briefing replacement — Bug priority Urgent) merges first; DOS-172 + DOS-173 run in parallel after.
.docs/plans/v1.4.7-waves.md:543:### Agent W2-A — DOS-175: Replace stale briefing MCP reads with ability-backed current context
.docs/plans/v1.4.7-waves.md:545:- **Spec:** [DOS-175](https://linear.app/a8c/issue/DOS-175).
.docs/plans/v1.4.7-waves.md:547:- **Files owned (exclusive):** `src-tauri/src/services/mcp_v2/handlers/tool_briefing.rs` (W1-A pre-created placeholder; W2-A fills); legacy briefing snapshot read paths to deprecate (enumerated at L0 plan via grep).
.docs/plans/v1.4.7-waves.md:550:- **Done when:** `dailyos.read.daily_briefing` and `dailyos.read.meeting_briefing` tools (canonical naming per cycle 2 #8) dispatch to ability-backed current-context queries; legacy snapshot read code paths removed (or wrapped with deprecation warnings); response shape includes provenance + freshness per ADR-0108; `cargo test` + `cargo clippy -D warnings` clean.
.docs/plans/v1.4.7-waves.md:558:- **Depends on:** W1 merged; W2-A merged (depends on the briefing replacement establishing the response-shape pattern with provenance/freshness for read tools).
.docs/plans/v1.4.7-waves.md:567:- **Depends on:** W1 merged; W2-A merged.
.docs/plans/v1.4.7-waves.md:615:- **Goal:** MCP resources (browsable, URL-shaped) for entity + source profiles. URI scheme `dailyos://account/{handle}`, `dailyos://person/{handle}`, `dailyos://source/{handle}` where `{handle}` is a **server-minted opaque resource handle** (cycle 2 CSO MED #4 — never the underlying entity_id which could be enumerable/PII). Privacy-rendered per ADR-0108 + v1.4.5 sensitivity rules using the SAME redaction helper DOS-479 ships (cycle 2 CSO LOW #5 — DOS-171 must not implement parallel redaction logic; reuse the W3-A helper exactly).
.docs/plans/v1.4.7-waves.md:685:- **Goal:** Build the eval harness that validates v1.4.7 tool descriptions select correctly under realistic host-model prompts. **Cycle 2 fixture density (per DX MED #6): minimum ≥2 positive + ≥2 negative per tool**, where the negative fixtures must include BOTH a broad-corpus negative (web/enterprise search territory) AND an adjacent-DailyOS-but-wrong-tool negative (e.g., a prompt that should select `dailyos.read.account_status` but NOT `dailyos.search.workspace_memory` because they're adjacent). Three test categories:
.docs/plans/v1.4.7-waves.md:688:  3. **Adjacent-tool negative:** prompts that select a DailyOS tool but NOT this one (e.g., for the `dailyos.search.workspace_memory` tool: a prompt like "What's Acme's renewal date?" should select `dailyos.read.account_status`, not workspace memory search).
.docs/plans/v1.4.7-waves.md:759:- [ ] ADR-0102 amendment + ADR-0128 amendment merged
.docs/plans/v1.4.7-waves.md:773:| 1 | Does ADR-0102 need amendment for `Actor::McpClient` introduction + scope naming + MCP client auth? | W0 | **Resolved (cycle 2)** — yes; ADR-0102 amendment scope expanded per Class S to also cover MCP client authentication + session contract; PR also lands `Actor::McpClient` enum variant code change in `actor.rs` |
.docs/plans/v1.4.7-waves.md:774:| 2 | Does ADR-0128 need amendment for v2 expansion principles? | W0 | **Resolved (cycle 2)** — yes; covers tool-description as product copy + host-selection language + cross-conversation continuity contract |
.docs/plans/v1.4.7-waves.md:775:| 3 | Scope naming convention | W0 (folded into ADR-0102) | **Resolved (cycle 2)** — `<dot-namespace>.<verb>.<noun>`; v1.4.5's `write.workspace_place_document` reused verbatim; full mapping table in §"Scope naming convention frozen" |
.docs/plans/v1.4.7-waves.md:781:| 9 | Does `Actor::McpClient` need rate-limit budgets distinct from `Actor::SurfaceClient`? | W1-A (DOS-168) | **Resolved (cycle 2 partially)** — yes, distinct per the ADR-0102 amendment Class S contract (per-actor + per-client + per-tool rate-limit registry); exact budget numbers defer to W1-A L0 plan |
.docs/plans/v1.4.7-waves.md:783:| 11 (new) | What's the MCP pairing handshake protocol shape? Mirror v1.4.2 SurfaceClient pairing or use OAuth-style flow or stdio-transport simpler model? | W0 (ADR-0102 amendment) | **Open** — defer to W0 ADR drafting |
.docs/plans/v1.4.7-waves.md:798:| DX-mandatory lanes | All 13 issues (entire project is a DX surface; heightened scrutiny on DOS-478, DOS-481, DOS-175, DOS-172, DOS-173) |

exec
/bin/zsh -lc 'rg -n "AbilityRegistry|invoke_registry_json|McpAbilityBridge|McpToolHandler|mcp_exposure|allowed_actors|ADR-0102|ADR-0108|ADR-0125|ADR-0093|ADR-0128|claim sensitivity|provenance|briefing|meeting_briefing|get_daily_briefing|account_overview|substrate|handler seam|crate::abilities|abilities-runtime" docs/solutions .docs/decisions' in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
docs/solutions/security-issues/prompt-channel-sensitivity-class-sweep-2026-05-18.md:8:related_adr: ADR-0108
docs/solutions/security-issues/prompt-channel-sensitivity-class-sweep-2026-05-18.md:55:- ADR-0108 (sensitivity rendering audit, OUTPUT boundary mirror at DOS-412 W6)
docs/solutions/README.md:9:**Always at L0 (Plan review).** Before approving a plan, L0 reviewers grep `docs/solutions/` and `.docs/decisions/` for prior solutions and architectural decisions that match the work. Reinventing documented substrate = BLOCKED finding, cite the path.
docs/solutions/README.md:11:**Always at L1 (authoring).** Before writing new substrate, agents grep for the entity / module / pattern being touched. If a prior solution exists, consume it — don't reinvent.
docs/solutions/README.md:19:- **End of every L3 wave retro** — autonomous `/ce-compound` runs for each class-pattern finding, each "substrate-already-existed" finding, and each cross-wave drift signal surfaced by L5.
docs/solutions/README.md:82:The compounding only works if K-out runs every retro and K-in (the substrate-grep obligation at L0 / L1 / L2) is enforced. Without both loops closed, this becomes a doc graveyard.
.docs/decisions/0117-publish-boundary-pencil-and-pen.md:6:**Extends:** [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0103](0103-maintenance-ability-safety-constraints.md)
.docs/decisions/0117-publish-boundary-pencil-and-pen.md:7:**Related:** [ADR-0104](0104-execution-mode-and-mode-aware-services.md), [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0108](0108-provenance-rendering-and-privacy.md), [ADR-0111](0111-surface-independent-ability-invocation.md)
.docs/decisions/0117-publish-boundary-pencil-and-pen.md:11:[ADR-0102](0102-abilities-as-runtime-contract.md) §3 introduces four ability categories: Read, Transform, Publish, and Maintenance. Three of them write to state that DailyOS controls — in-memory, in the local DB, or in an outbox under [ADR-0103](0103-maintenance-ability-safety-constraints.md). A failed Maintenance ability can be retried, rolled back, or compensated. A failed Read or Transform ability simply does not update anything.
.docs/decisions/0117-publish-boundary-pencil-and-pen.md:13:**Publish is different.** A Publish ability writes to a destination DailyOS does not own — a P2 blog post, an external email, a Slack channel, a CRM, an exported file. Once that write succeeds, the side effect is in a system we cannot reliably reverse. Some destinations allow edit or delete (P2 post, Slack message within retention); some do not (email sent, webhook fired, third-party record created). Even when retraction is possible, the effect may have already been observed — the briefing has been read by three stakeholders, the email is in recipients' inboxes, the webhook triggered a downstream automation.
.docs/decisions/0117-publish-boundary-pencil-and-pen.md:15:This asymmetry — reversible inside DailyOS, irreversible once the side effect lands — is architecturally distinct. It requires a different contract from the user and a different implementation discipline from the authoring ability. We have one Publish ability today (`publish_to_p2`), referenced in [ADR-0102](0102-abilities-as-runtime-contract.md) §3 and [ADR-0112](0112-migration-strategy-parallel-run-and-cutover.md) §3. Before new Publish abilities land — and before `publish_to_p2` itself gets wrapped in the v1.4.0 abilities runtime — the class needs a shared protocol.
.docs/decisions/0117-publish-boundary-pencil-and-pen.md:36:    pub provenance: Provenance,
.docs/decisions/0117-publish-boundary-pencil-and-pen.md:47:- A Draft is inspectable. The user or a policy check can see exactly what will be sent, to whom, with what provenance.
.docs/decisions/0117-publish-boundary-pencil-and-pen.md:163:- The **provenance summary** — the top-level sources feeding the content, consistent with [ADR-0108](0108-provenance-rendering-and-privacy.md) rendering rules.
.docs/decisions/0117-publish-boundary-pencil-and-pen.md:179:### 8. Audit and provenance
.docs/decisions/0117-publish-boundary-pencil-and-pen.md:185:- The Provenance envelope ([ADR-0105](0105-provenance-as-first-class-output.md)) on the `AbilityOutput<PublishedRecord>` returned by the Publish ability includes a `publish_context` field with the outbox entry ID and the destination ref, enabling anyone reading the output later to trace exactly what went out and where.
.docs/decisions/0117-publish-boundary-pencil-and-pen.md:225:- **Two new tables and a worker pool add operational surface.** Mitigated by reusing the [ADR-0115](0115-signal-granularity-audit.md) §5 worker infrastructure; minimal net new code for the queue substrate.
.docs/decisions/0117-publish-boundary-pencil-and-pen.md:252:- Both human-readable (Markdown/PDF briefing export) and machine-readable (structured JSON with provenance) payload formats must be first-class. The `PublishPayload` shape (§1) supports both; formalize as required in v1.4.1.
docs/solutions/tooling-decisions/codex-companion-task-worker-hang-finalAnswerSeen-2026-05-20.md:14:v1.4.4 W1 L0 review dispatched 4 parallel `codex:codex-rescue` subagents to do adversarial review + substrate-grep consult against L0 packets. All 4 hung at 12:01-12:02 after 5-15 grep commands deep into the research phase. Underlying codex CLI processes had died silently but the worker processes never gave up. Output files frozen mid-research with last `Command completed` lines.
.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md:6:**Extends:** [ADR-0101](0101-service-boundary-enforcement.md), [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0104](0104-execution-mode-and-mode-aware-services.md)  
.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md:7:**Depends on:** [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0107](0107-source-taxonomy-alignment.md), [ADR-0110](0110-evaluation-harness-for-abilities.md), [ADR-0111](0111-surface-independent-ability-invocation.md)
.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md:11:[ADR-0102](0102-abilities-as-runtime-contract.md) §14 describes per-capability migration but does not specify parallel-run validation, cutover criteria, rollback, or the three specific substrate migrations that ship as part of v1.4.0:
.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md:15:3. Synthesis-marker backfill per [ADR-0107](0107-source-taxonomy-alignment.md) §6 — tagging existing stored AI output so [ADR-0105](0105-provenance-as-first-class-output.md)'s trust model works retroactively.
.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md:17:This ADR specifies the per-capability migration flow, the three substrate migrations, parallel-run validation, cutover criteria, and rollback procedures.
.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md:25:**Stage 0 — Substrate readiness.** The three substrate migrations in §2 must be at Phase 2 or later before a capability can begin its own migration.
.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md:27:**Stage 1 — Ability implementation.** A new ability module is added under `src-tauri/src/abilities/` per [ADR-0102](0102-abilities-as-runtime-contract.md) §2. Includes planner function, apply function (for Maintenance), provenance builder usage per [ADR-0105](0105-provenance-as-first-class-output.md), and declared metadata (`mutates`, `composes`, policy).
.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md:107:- Read paths consult `synthesized_fields` when building `SourceAttribution.synthesis_marker` per [ADR-0105](0105-provenance-as-first-class-output.md) §4.
.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md:116:1. `get_entity_context` — highest read volume; validates substrate across entity-generic pipeline.
.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md:117:2. `prepare_meeting` — highest user-visible value; exercises Transform + composition + provenance.
.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md:150:v1.4.0 ships the planning window (Phase 1 of all three substrate migrations) and begins Phase 2 migrations of top-priority capabilities. Full cutover (Phase 3) of all capabilities is a v1.4.1 or v1.4.2 milestone — not gated on v1.4.0 GA.
.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md:177:2. **Substrate dependencies explicit.** Capability migration cannot begin until substrate readiness is established.
.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md:192:2. **Substrate migration incomplete at Phase 2.** Capabilities begin migrating before mode-aware services are everywhere, leading to false confidence. Mitigation: capability Stage 0 gate checks substrate readiness explicitly.
.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md:200:- [ADR-0102: Abilities as the Runtime Contract](0102-abilities-as-runtime-contract.md) — §14 migration-from-current-architecture refined here.
.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md:202:- [ADR-0104: ExecutionMode and Mode-Aware Services](0104-execution-mode-and-mode-aware-services.md) — §2.1 substrate migration.
.docs/decisions/0112-migration-strategy-parallel-run-and-cutover.md:203:- [ADR-0105: Provenance as First-Class Output](0105-provenance-as-first-class-output.md) — §2.3 synthesis-marker backfill enables `contains_stored_synthesis`.
.docs/decisions/0103-maintenance-ability-safety-constraints.md:6:**Extends:** [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0101](0101-service-boundary-enforcement.md)  
.docs/decisions/0103-maintenance-ability-safety-constraints.md:12:[ADR-0102](0102-abilities-as-runtime-contract.md) introduces four ability categories: Read, Transform, Publish, and Maintenance. Three of the four have naturally bounded risk surfaces:
.docs/decisions/0103-maintenance-ability-safety-constraints.md:25:Service-layer mutations ([ADR-0101](0101-service-boundary-enforcement.md)) are bounded by user action: each mutation corresponds to something a user or agent explicitly did, the blast radius is contained to that action, and errors surface immediately to the caller. Maintenance mutations have none of those natural constraints. A bug in a maintenance ability is the single largest new risk surface introduced by [ADR-0102](0102-abilities-as-runtime-contract.md).
.docs/decisions/0103-maintenance-ability-safety-constraints.md:35:The ability pattern is the right host for maintenance operations — they need the registry, provenance, observability, and composability that abilities provide. But they require structural safety constraints that Read and Transform abilities do not.
.docs/decisions/0103-maintenance-ability-safety-constraints.md:41:Every ability in the Maintenance category operates under all rules of [ADR-0102](0102-abilities-as-runtime-contract.md) plus the eleven constraints below, covering idempotency, transactional boundaries, execution-mode and dry-run semantics, multi-dimensional change budgets, signal containment, actor policy, blast radius with user-confirmed global, operational audit records, snapshot-diff evaluation, scoped observability, and compile-time separation. These constraints are enforced by type system, registry metadata, runtime checks, and review discipline — not by convention.
.docs/decisions/0103-maintenance-ability-safety-constraints.md:139:    // AbilityPolicy fields (canonical schema per ADR-0102 §7.1):
.docs/decisions/0103-maintenance-ability-safety-constraints.md:140:    allowed_actors = [System],
.docs/decisions/0103-maintenance-ability-safety-constraints.md:154:    // Composition metadata (per ADR-0102 §7.1):
.docs/decisions/0103-maintenance-ability-safety-constraints.md:176:Signals emitted during maintenance operations preserve their original `data_source` per [ADR-0098](0098-data-governance-source-aware-lifecycle.md) (e.g., `glean`, `REDACTED`, `user`, `ai`) — **never** overwritten with a synthetic "Maintenance" source, which would violate the data-origin model and break purge-on-revocation (Codex finding #15). Instead, signals emitted from inside a maintenance invocation carry two additional fields that describe the operational context without conflating it with data provenance:
.docs/decisions/0103-maintenance-ability-safety-constraints.md:208:// Canonical AbilityPolicy schema per ADR-0102 §7.1
.docs/decisions/0103-maintenance-ability-safety-constraints.md:210:    pub allowed_actors: Vec<Actor>,
.docs/decisions/0103-maintenance-ability-safety-constraints.md:217:Maintenance abilities default to `allowed_actors: [System]`. Agents and users invoking a maintenance ability through the registry receive `Error::ActorNotAllowed` at the boundary, before the ability executes. Opt-in to `Agent` or `User` invocation requires explicit ADR amendment and a documented justification.
.docs/decisions/0103-maintenance-ability-safety-constraints.md:219:**Why this matters:** An MCP agent enumerates the registry as its tool catalog. Without actor policy, every maintenance operation becomes an agent-invokable tool. Default-deny for agents on maintenance is the safe posture. Registry introspection is actor-filtered per [ADR-0102](0102-abilities-as-runtime-contract.md) §7.4 — agents do not see maintenance ability names or schemas at all.
.docs/decisions/0103-maintenance-ability-safety-constraints.md:221:Actor policy alone is insufficient in-process: a direct Rust import of a maintenance ability function would still execute regardless of the registry policy. Compile-time separation (§11) closes that gap post-v1.4.0. Until then, review discipline and the typed registry invocation paths from [ADR-0102](0102-abilities-as-runtime-contract.md) §7.2 prevent accidental bypass.
.docs/decisions/0103-maintenance-ability-safety-constraints.md:266:    pub provenance: ProvenanceOrMasked,          // Full provenance or masked marker if sources revoked
.docs/decisions/0103-maintenance-ability-safety-constraints.md:282:// Either full provenance, or a mask record replacing it after source revocation.
.docs/decisions/0103-maintenance-ability-safety-constraints.md:283:// Masking rules and shape defined by ADR-0108.
.docs/decisions/0103-maintenance-ability-safety-constraints.md:337:Every maintenance invocation produces structured telemetry captured by the `AbilityTracer` from [ADR-0102](0102-abilities-as-runtime-contract.md) §5, with maintenance-specific fields persisted to the `maintenance_audit` table (see §8):
.docs/decisions/0103-maintenance-ability-safety-constraints.md:372:pub struct AbilityRegistry {
.docs/decisions/0103-maintenance-ability-safety-constraints.md:409:2. **Mis-categorized abilities.** A transform-shaped operation gets filed as maintenance (or vice versa) to bypass or escape category-specific discipline. Mitigation: category determined by call-graph effect per [ADR-0102](0102-abilities-as-runtime-contract.md) §3 — static check at registration.
.docs/decisions/0103-maintenance-ability-safety-constraints.md:421:- [ADR-0102: Abilities as the Runtime Contract](0102-abilities-as-runtime-contract.md) — Establishes the ability pattern this ADR constrains. Maintenance is one of four categories; this ADR defines its safety envelope.
.docs/decisions/0103-maintenance-ability-safety-constraints.md:426:- **ADR-0105 (forthcoming): Provenance as First-Class Output** — Defines the provenance shape that maintenance abilities populate (separately from the operational audit records in §8 of this ADR).
.docs/decisions/0027-mcp-dual-mode.md:8:DailyOS has data (workspace, briefings, actions) that external AI tools want to access. It also needs to consume external services (Clay, potentially Slack, Linear).
.docs/decisions/0027-mcp-dual-mode.md:14:**Server mode exposes:** workspace structure, today's briefing, account dashboards, action lists, meeting schedule, processing queue status.
docs/solutions/architecture-patterns/L3-catches-wave-level-bypass-of-sub-ticket-discipline-2026-05-20.md:14:v1.4.4 W1 wave (10 substrate sub-tickets) passed L2 unanimously after 3 cycles. All 3 named L2 reviewers (`/review` + `code-reviewer` + `/cso`) approved cycle-2; codex cycle-3 P2s routed to path-α (DOS-749, DOS-750). Then L3 codex challenge surfaced 5 REAL defects that NONE of the L2 cycles had caught:
docs/solutions/architecture-patterns/L3-catches-wave-level-bypass-of-sub-ticket-discipline-2026-05-20.md:17:2. Touchpoints bypass `build_receipt_for_audience` — raw meeting titles to AgentMcp (ADR-0108 violation)
docs/solutions/architecture-patterns/L3-catches-wave-level-bypass-of-sub-ticket-discipline-2026-05-20.md:19:4. Envelope cache lacks principal binding — cross-actor poisoning (ADR-0125 violation)
docs/solutions/architecture-patterns/L3-catches-wave-level-bypass-of-sub-ticket-discipline-2026-05-20.md:37:- Identify cross-sub-ticket coupling points BEFORE dispatch (signals across tickets, shared substrate, composable primitives, CI gate contracts)
docs/solutions/architecture-patterns/L3-catches-wave-level-bypass-of-sub-ticket-discipline-2026-05-20.md:43:- They are NOT path-α candidates (path-α is for theoretical hardening / not-AC-violations); L3 cross-cutting defects ARE substrate-fundamental
docs/solutions/architecture-patterns/L3-catches-wave-level-bypass-of-sub-ticket-discipline-2026-05-20.md:44:- Per `feedback_zoom_out_for_class_pattern_in_l2_loop`: if multiple cycle-1 L3 findings share a class-pattern (e.g., "wave-level bypass of per-sub-ticket discipline"), document the class and audit the entire substrate before patching, don't just patch the listed instances
.docs/decisions/0053-dashboard-ux-redesign.md:8:A full UX review of the daily briefing/dashboard page identified several information architecture problems:
docs/solutions/test-failures/parallel-test-singleton-state-flake-2026-05-18.md:12:`intelligence::write_fence::tests::dos311_substrate_migration_sequence_end_to_end` passes in isolation but flakes intermittently in the full `cargo test --lib` run. The failure mode looks like a regression introduced by the most recent commit, masking diagnosis during W3-C/H Phase 3 and Phase 5 work (~5 min misdiagnosis cost each time).
docs/solutions/test-failures/parallel-test-singleton-state-flake-2026-05-18.md:16:- Test passes alone (`cargo test --lib dos311_substrate_migration_sequence_end_to_end`).
docs/solutions/test-failures/parallel-test-singleton-state-flake-2026-05-18.md:32:fn dos311_substrate_migration_sequence_end_to_end() {
.docs/decisions/0082-entity-generic-prep-pipeline.md:194:| **1:1 meeting prep** | Relationship briefing: what you discussed last time, what's outstanding, what they care about, how to approach this conversation | `intelligence.json` (full relationship intelligence) |
.docs/decisions/0104-execution-mode-and-mode-aware-services.md:7:**Prerequisite for:** [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0103](0103-maintenance-ability-safety-constraints.md)
.docs/decisions/0104-execution-mode-and-mode-aware-services.md:11:The abilities layer introduced by [ADR-0102](0102-abilities-as-runtime-contract.md) and the maintenance safety envelope in [ADR-0103](0103-maintenance-ability-safety-constraints.md) both depend on execution mode being a first-class, enforced property of the runtime. Three requirements motivate this ADR:
.docs/decisions/0104-execution-mode-and-mode-aware-services.md:130:    pub provenance_ref: ProvenanceRef,    // Reference into ability's provenance envelope per ADR-0105 §8
.docs/decisions/0104-execution-mode-and-mode-aware-services.md:237:However, ability composition is transaction-propagating, not transaction-nesting (per [ADR-0102](0102-abilities-as-runtime-contract.md) §11.2). A parent maintenance ability that holds a transaction may invoke a child ability; the child inherits the parent's `TxCtx` via `AbilityContext.services` and its service calls participate in the same transaction. This is the inheritance pattern, not a new transaction. The child does not call `with_transaction_async` itself — it uses the parent's open scope. If a child ability attempts its own `with_transaction_async` while a parent transaction is active, it fails with `NestedTransactionsForbidden`.
.docs/decisions/0104-execution-mode-and-mode-aware-services.md:353:- [ADR-0101: Service Boundary Enforcement](0101-service-boundary-enforcement.md) — This ADR amends `ServiceContext` from that document. All ADR-0101 rules (services own mutations, every mutation emits a signal, multi-write transactionality, error propagation, reads don't mutate) remain in force; this ADR adds mode-awareness as a prerequisite for [ADR-0102](0102-abilities-as-runtime-contract.md) and [ADR-0103](0103-maintenance-ability-safety-constraints.md).
.docs/decisions/0104-execution-mode-and-mode-aware-services.md:354:- [ADR-0102: Abilities as the Runtime Contract](0102-abilities-as-runtime-contract.md) — Consumes `AbilityContext` which wraps `ServiceContext` with its mode. §5 and §11 of that ADR require the contract established here.
.docs/decisions/0104-execution-mode-and-mode-aware-services.md:383:**Tracking:** [DOS-209](https://linear.app/a8c/issue/DOS-209) (Substrate: ExecutionMode and Mode-Aware Services) has been amended 2026-04-20 to name `ServiceContext` explicitly as the Phase 0 deliverable. Everything downstream (ADR-0102, 0103, 0105, 0110, 0113, 0115, 0116, 0117, 0119) waits on DOS-209 Phase 0.
docs/solutions/workflow-issues/codex-agent-dispatched-but-no-file-changes-2026-05-20.md:14:v1.4.3 W6 needed substrate work on 3 tickets (DOS-742 HMAC signing, DOS-743 MCP invoke audit migrate, DOS-576 11-site migration) plus DOS-741 done manually first. Per memory `feedback_parallel_agent_sizing`, mechanical work + tight rule + non-overlapping files → fan out parallel codex agents.
docs/solutions/workflow-issues/codex-agent-dispatched-but-no-file-changes-2026-05-20.md:61:- Implemented both manually in the DOS-576-migrate worktree (bundling all 4 substrate tickets into one PR).
.docs/decisions/0026-extension-architecture.md:8:Domain-specific features (Customer Success workflows, Professional Development tracking, CRM integration) shouldn't be forced on all users. But they share infrastructure (dashboard, actions, briefing).
docs/solutions/workflow-issues/premise-check-production-vs-dev-friction-before-scoping-waves-2026-05-20.md:58:4. **File that as a focused ticket.** Wave-level scope is for substrate work that genuinely needs multi-ticket coordination, not for dev-ergonomics dressed up as user-facing.
docs/solutions/workflow-issues/premise-check-production-vs-dev-friction-before-scoping-waves-2026-05-20.md:63:- Don't conflate "the substrate could be more robust here" with "users are hitting this." Robustness improvements file as path-α or maintenance tickets; user-visible UX fixes file as focused tickets.
docs/solutions/workflow-issues/premise-check-production-vs-dev-friction-before-scoping-waves-2026-05-20.md:69:- **Pair with [[feedback_check_substrate_before_authoring_primitives]]**: substrate often already does most of what the wave thinks needs net-new work.
.docs/decisions/0032-calendar-source-of-truth.md:9:- `schedule.json`: Generated at briefing time (6 AM). Stale within hours. Has AI enrichment (prep, classification, context).
.docs/decisions/0032-calendar-source-of-truth.md:20:- Meeting in briefing but not live → cancelled, hide or grey out
.docs/decisions/0032-calendar-source-of-truth.md:21:- Meeting in live but not briefing → new, show bare with "No prep" indicator
.docs/decisions/0032-calendar-source-of-truth.md:22:- Meeting in both → live timing + briefing enrichment
docs/solutions/workflow-issues/l0-review-loop-diminishing-returns-means-scope-is-wrong-2026-05-20.md:2:title: "L0 review-loop diminishing returns (5+ findings/cycle, 80+ line revisions) = ticket scope is wrong, not reviewers — reset scope and split substrate into separate tickets"
docs/solutions/workflow-issues/l0-review-loop-diminishing-returns-means-scope-is-wrong-2026-05-20.md:6:tags: [l0, review-loop, scope, packet-authoring, substrate-split, dos-576, dos-741, dos-742, dos-743]
docs/solutions/workflow-issues/l0-review-loop-diminishing-returns-means-scope-is-wrong-2026-05-20.md:14:DOS-576 (v1.4.3 W6 audit forensic validation) was scoped as a **validation + migration** ticket against existing W1-A0 substrate. The L0 packet (Packet H) ran three review cycles:
docs/solutions/workflow-issues/l0-review-loop-diminishing-returns-means-scope-is-wrong-2026-05-20.md:22:Each cycle's revisions absorbed substrate work into what was scoped as a validation ticket:
docs/solutions/workflow-issues/l0-review-loop-diminishing-returns-means-scope-is-wrong-2026-05-20.md:24:- V1.1 added first-class `request_id` field on `AuditRecord` + `AuditFields` (substrate)
docs/solutions/workflow-issues/l0-review-loop-diminishing-returns-means-scope-is-wrong-2026-05-20.md:25:- V1.2 added HMAC canonical signing of `X-DailyOS-Request-Id` (substrate)
docs/solutions/workflow-issues/l0-review-loop-diminishing-returns-means-scope-is-wrong-2026-05-20.md:26:- V1.2 added custom clippy lint crate (`tools/clippy-lints-dailyos/`) (substrate)
docs/solutions/workflow-issues/l0-review-loop-diminishing-returns-means-scope-is-wrong-2026-05-20.md:27:- V1.2 added `AuditError::SurfaceClientMissingRequestId` contract layer (substrate)
docs/solutions/workflow-issues/l0-review-loop-diminishing-returns-means-scope-is-wrong-2026-05-20.md:28:- V1.2 added `emit_pairing_audit_event_legacy` deprecation cascade (substrate)
docs/solutions/workflow-issues/l0-review-loop-diminishing-returns-means-scope-is-wrong-2026-05-20.md:29:- V1.2 changed the JSONL hash-chain spec (substrate)
docs/solutions/workflow-issues/l0-review-loop-diminishing-returns-means-scope-is-wrong-2026-05-20.md:31:By cycle 3, the discretionary "fold vs path-α" question dominated per-finding triage instead of "AC violation vs not." The packet had drifted from validation-ticket framing into substrate-overhaul territory.
docs/solutions/workflow-issues/l0-review-loop-diminishing-returns-means-scope-is-wrong-2026-05-20.md:40:2. **Reviewers flag substrate-additions** (new fields, new helpers, contract changes) rather than tightening existing AC bullets.
docs/solutions/workflow-issues/l0-review-loop-diminishing-returns-means-scope-is-wrong-2026-05-20.md:50:- **Packet H reset to V2.0** — went from 287 lines to 77 lines. Scoped back to its original intent: forensic validation + migration on existing substrate. No substrate work.
docs/solutions/workflow-issues/l0-review-loop-diminishing-returns-means-scope-is-wrong-2026-05-20.md:66:- **Reset ticket scope** to what it actually validates against existing substrate.
docs/solutions/workflow-issues/l0-review-loop-diminishing-returns-means-scope-is-wrong-2026-05-20.md:67:- **File substrate gaps as separate tickets** with their own L0 cycles. Cross-link as predecessors.
.docs/decisions/0090-user-entity-page-architecture.md:94:- Downstream effect: quarterly priorities appear directly in the daily briefing's focus directive. They represent the briefing's understanding of what the user is actively moving forward.
docs/solutions/workflow-issues/codex-rescue-destructive-git-reset-on-529-2026-05-20.md:32:1. **Commit before dispatching** any `--write`-capable codex agent when there is uncommitted substrate work in the worktree. Even a checkpoint commit with `L2-status: not-run-acknowledged` is fine — recoverable, doesn't destroy work.
.docs/decisions/0114-scoring-unification.md:5:**Target:** v1.4.0 substrate (shared factor library, trust extractor) / v1.6.0 (health composer migration)
.docs/decisions/0114-scoring-unification.md:12:DailyOS has two composers that produce scores from entity signals: the **Trust Compiler** ([DOS-5](https://linear.app/a8c/issue/DOS-5)) which scores per-claim trustworthiness for the provenance envelope, and the **Account Health Composer** ([ADR-0097](0097-account-health-scoring-architecture.md)) which scores per-dimension health for briefings. As drafted, both reimplement the same primitive calculations — source reliability from Bayesian posteriors, freshness decay, corroboration boosts, contradiction penalties, user-feedback modulation. The math is identical; only the composition differs (weighted geometric mean for trust, lifecycle-weighted arithmetic for health).
.docs/decisions/0114-scoring-unification.md:167:UI surfaces render `ConfidenceEvidence` uniformly: a score, a band, an expandable breakdown, and a list of caveats. Bands differ per composer; the rendering shell is common. This is what [ADR-0108](0108-provenance-rendering-and-privacy.md) consumes.
.docs/decisions/0114-scoring-unification.md:271:Callers invoke both in sequence. Tests invoke only the pure variant. This matches the abilities contract in [ADR-0102](0102-abilities-as-runtime-contract.md) §3: read/transform abilities do not mutate.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:1:# ADR-0128 — Headless DailyOS: MCP as a co-equal product surface
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:6:**Amended:** 2026-05-19 — same-date companion amendment to ADR-0102; codifies tool description as product copy (DOS-481 eval coverage), cross-conversation continuity wire shape (defers to ADR-0102 §D OpaqueConversationHandle lifecycle), restated v1.4.7 W4 write surface, displacement-use-case framing extended to v1.4.7 W5 host-selection eval, and Linear link refresh to the post-renumber v1.4.7 project. See section appended at end of doc.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:8:**Relates to:** [ADR-0027](0027-mcp-dual-mode.md), [ADR-0083](0083-product-vocabulary.md), [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0111](0111-surface-independent-ability-invocation.md), [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md), [ADR-0118](0118-dailyos-as-ai-harness-principles-and-residual-gaps.md)
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:13:[ADR-0027](0027-mcp-dual-mode.md) established DailyOS as both an MCP server (exposing workspace tools and resources to external agents) and an MCP client (consuming external services). The v1.5.0 project description is sharp on guarantees: tools must route through abilities, not parallel paths; writes must capture structured intent and provenance; reads must return current substrate state; claim-worthy outputs must explain source, freshness, confidence, and salience.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:17:This ADR fills that gap. It does not change the v1.5.0 substrate scope. It frames it.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:19:The frame is straightforward: **MCP is not an export pipe; it is a co-equal head over the same intelligence substrate as the Tauri app.** The Tauri app and the MCP head are two different consumption optimizations of one claim graph.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:21:- The **Tauri head is opinionated**: briefing → triage → prep, with hierarchy and callouts the user reads visually. Designed for the daily ritual.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:24:Same substrate, different consumption shapes for different moments.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:32:The MCP server is the **headless head** of DailyOS. It exposes the same intelligence substrate as the Tauri app, optimized for a different consumption surface (host-model conversation rather than direct visual reading).
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:34:Architectural implication: the substrate is the product; the heads are surfaces over it. Adding a new surface (CLI, mobile, browser extension, future agent shell) follows the same pattern. This generalizes [ADR-0111](0111-surface-independent-ability-invocation.md): a Surface invokes abilities; a Surface is not a feature implementation.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:42:- Preferred: `account_status(name)`, `find_open_commitments(person)`, `recent_signals(account, since)` — each returning synthesized output that exercises the abilities runtime per [ADR-0102](0102-abilities-as-runtime-contract.md).
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:60:### 4. Claim-shaped output, with provenance leaking through
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:62:Tool outputs are claim-shaped, not document-shaped. Each user-meaningful field carries inline trust band and source attribution per [ADR-0105](0105-provenance-as-first-class-output.md). The output structure should make it natural for a host model to render hedging when trust is low — *"according to a transcript from last Thursday, though this hasn't been confirmed elsewhere…"* — without explicit prompting.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:64:The substrate leaking through into the conversation *is* the product differentiator. It is what makes a DailyOS-mediated answer feel different from a flat retrieval over a corpus.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:78:This preserves the v1.5.0 user outcome ("writes capture structured intent and provenance") in spirit by routing all writes through the same propose/commit boundary the Tauri app uses, with the same actor distinctions per [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md).
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:82:Host-model conversations are stateless. DailyOS is not. A user asking the same question across two Claude Desktop conversations hits the same evolving substrate.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:84:This is a quiet superpower and should be made explicit in the surface itself, not left as an implementation detail. Tool descriptions name it. Returned objects, where appropriate, surface it ("last verified Tuesday; no contradicting signals since"). The host model is given the language to communicate substrate persistence to the user.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:88:The same headless framing extends to a CLI surface. CLI is a third head over the substrate, not a separate substrate consumer.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:92:**Render.** CLI's distinct contribution is the output shape. Three renders over one substrate:
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:94:- Tauri: substrate → React components
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:95:- MCP: substrate → claim-shaped JSON
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:96:- CLI: substrate → markdown
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:98:Markdown is the right CLI render because it is both terminal-readable and vault-compatible. A `dailyos brief` command emits markdown a user can read in the terminal, pipe to a file, or paste into Obsidian. This actively reinforces the "you always keep your files" defensibility position established by the v0.0.1 PARA + markdown foundation — the substrate's intelligence is reachable as markdown, not only as a GUI.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:105:**Scope discipline is unchanged.** CLI commands mirror the **app loop** (briefing, prep, account status, commitments, action review), not the **org corpus** (Glean's territory). The displacement framing in §The displacement use case applies regardless of transport.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:112:These predate the abilities-first MCP surface. They should be refreshed once v1.5.0 lands so they consume the new tool surface and inherit its descriptions, output shape, and feedback semantics. This is a v1.5.x follow-on, not v1.5.0 substrate work — but it is the natural beneficiary of v1.5.0 once the MCP head is solid.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:126:| Corpus | Org-wide documents and messages | The user's working substrate |
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:132:DailyOS does not replace Glean. Glean remains the right substrate for org-wide policy, how-to, and broad-corpus search. The two are complements with a clean seam:
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:144:- **Not a wrapper for Claude.** DailyOS does not host conversation. The user converses in their host model of choice; DailyOS is the substrate the host model consults.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:170:- It does not change the v1.5.0 substrate scope. It is consumed by v1.5.0 issue review and tool surface design.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:179:- [ADR-0102](0102-abilities-as-runtime-contract.md) — Abilities as runtime contract
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:180:- [ADR-0105](0105-provenance-as-first-class-output.md) — Provenance as first-class output
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:192:This amendment records the v1.4.7 operational consequences of the same-date [ADR-0102](0102-abilities-as-runtime-contract.md) amendment. It does not reopen the architecture. ADR-0128 remains the product-surface frame for headless DailyOS; ADR-0102 remains the runtime contract for MCP-originated ability invocation.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:208:The lifecycle is intentionally not restated here. Expiry is 24 hours from last touch, revocation is server-side, and raw caller-provided conversation IDs are rejected; the authoritative mint, echo, expiry, first-write, revocation, and "no raw IDs" contract lives in [ADR-0102](0102-abilities-as-runtime-contract.md) §D.
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:222:### §F. Trust boundary — defer to ADR-0102 amendment
.docs/decisions/0128-headless-dailyos-mcp-as-product-surface.md:224:The MCP client trust boundary is codified in the same-date [ADR-0102](0102-abilities-as-runtime-contract.md) amendment, especially §C: MCP client authentication, transport HMAC signing, server-side scope manifest authorization, per-actor × per-client × per-tool rate limits, revocation, and audit attribution with keyed HMAC parameter and response hashes. This ADR cites that contract because ADR-0128 is the product-surface frame, not the transport or authorization spec.
docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md:2:title: "K-in grep should query substrate-type, not proposed-name — V1.0 packets miss existing services when the query mirrors the new module name"
docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md:6:tags: [k-in, l0, substrate-discovery, packet-authoring, grep-discipline, w4, dos-683]
docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md:9:related_memories: [check-substrate-before-authoring-primitives, ground-first-drafts-in-real-codebase]
docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md:14:L0 Packet F (v1.4.3 W4 Feedback Write Infrastructure, DOS-683) authored a parallel `services::surface_feedback` service + migration v181 `surface_feedback_nonces` table + new audit event family `pairing.feedback.*`. Cycle-1 reviewers BLOCKed twice because the substrate already existed:
docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md:32:But the substrate that does the work was named differently. The right query was a **substrate-pattern type** search:
docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md:42:This is the second K-in defect in v1.4.3 (W3 V1.0 deferred substrate audits to "cycle 2", retro-captured in W3 cycle-2 fold). The class pattern recurs — name the gate.
docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md:44:## Pattern: query by substrate-type, not proposed-name
docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md:46:When authoring an L0 packet that introduces "new" substrate:
docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md:49:2. **Grep for the substrate-type primitives.** For a nonce service: `nonce`, `issue/verify`, `consume`. For a feedback writer: `record_claim_feedback`, `ClaimFeedbackInput`, `FeedbackAction`. For a CPT: `register_post_type`. For a migration: scan `src-tauri/src/migrations/` end-to-end.
docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md:63:- **L0 packet template** should require a "substrate-type grep evidence" subsection separate from "proposed-name grep evidence". Both empty → suspicious.
docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md:64:- **K-in reviewer obligation** should explicitly verify: "did the K-in grep cover the substrate-type primitives, not just the proposed module name?" If only proposed-name was grep'd, BLOCK at L0 cycle 1.
docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md:65:- **Memory `check-substrate-before-authoring-primitives.md`** captures the principle. This entry captures the **specific failure mode** (query-shape) so future agents can recognize it before dispatching reviewers.
docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md:71:- Memory: `feedback_check_substrate_before_authoring_primitives.md`
docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md:74:- Existing substrate this missed: `src-tauri/src/services/surface_nonce.rs`
docs/solutions/workflow-issues/substrate-only-landing-needs-l0-amendment-2026-05-18.md:13:W3-C through W3-H (six substrate tickets) landed five commits on `dev` without per-PR L2 review. The catalyst was the appeal of "ship the shape, defer the wiring" — each substrate looked safe in isolation because no consumer was wired through it yet. Four substrates shipped substrate-only with significant deferrals.
docs/solutions/workflow-issues/substrate-only-landing-needs-l0-amendment-2026-05-18.md:23:- **"Shape-only is safe" is a trap.** A single shape-only substrate may be defensible in isolation. Four shape-only substrates simultaneously lock in shapes their future writers can't comfortably adopt. The integration becomes a liability before any consumer wires through.
docs/solutions/workflow-issues/substrate-only-landing-needs-l0-amendment-2026-05-18.md:34:- Any wave where 2+ tickets are "substrate-only with deferred consumer."
docs/solutions/workflow-issues/substrate-only-landing-needs-l0-amendment-2026-05-18.md:55:- Memory: `feedback_wire_existing_substrate_not_future_producer`
.docs/decisions/0119-runtime-evaluator-pass-for-transform-abilities.md:6:**Extends:** [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0110](0110-evaluation-harness-for-abilities.md)
.docs/decisions/0119-runtime-evaluator-pass-for-transform-abilities.md:7:**Related:** [ADR-0104](0104-execution-mode-and-mode-aware-services.md), [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0106](0106-prompt-fingerprinting-and-provider-interface.md), [ADR-0118](0118-dailyos-as-ai-harness-principles-and-residual-gaps.md) (Gap A)
.docs/decisions/0119-runtime-evaluator-pass-for-transform-abilities.md:137:The retry's provenance envelope ([ADR-0105](0105-provenance-as-first-class-output.md)) records:
.docs/decisions/0119-runtime-evaluator-pass-for-transform-abilities.md:194:- Evaluator composite is stored on the ability output (not per-claim) via a new `output_quality_score` field in the Provenance envelope ([ADR-0105](0105-provenance-as-first-class-output.md) amendment required).
.docs/decisions/0119-runtime-evaluator-pass-for-transform-abilities.md:197:The amendment to [ADR-0105](0105-provenance-as-first-class-output.md) adds:
.docs/decisions/0119-runtime-evaluator-pass-for-transform-abilities.md:236:- **Retry with critique is structural.** Not ad-hoc re-prompting — the retry is fingerprinted, provenanced, and traced like any other invocation.
.docs/decisions/0119-runtime-evaluator-pass-for-transform-abilities.md:247:- **`output_quality_score` amendment to [ADR-0105](0105-provenance-as-first-class-output.md).** Tracked as a required follow-up amendment; must land before v1.4.1 evaluator infrastructure.
.docs/decisions/0049-eliminate-python-runtime.md:20:- AI enrichment (emails, briefing, preps, inbox, transcripts, accounts) runs 100% through Rust → Claude Code CLI
.docs/decisions/0079-role-presets.md:22:4. **The daily briefing is universal.** Hero, Focus, The Meeting, Schedule, Priorities — none of these are CS-specific. What changes is how the AI frames urgency: renewal proximity (CS) vs close date (Sales) vs deadline (Agency/PM).
.docs/decisions/0079-role-presets.md:38:3. **Prioritization signals** — What urgency means and how the briefing ranks attention
.docs/decisions/0079-role-presets.md:41:6. **Briefing emphasis** — What the daily briefing prioritizes for this role
.docs/decisions/0079-role-presets.md:45:- The daily briefing structure (Hero, Focus, The Meeting, Schedule, Priorities, Finis)
.docs/decisions/0079-role-presets.md:117:  "briefingEmphasis": "Accounts needing attention, upcoming renewals, relationship gaps, prep for customer calls"
.docs/decisions/0131-structured-embedding-claim-canonicalization.md:6:**Relates to:** [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0108](0108-provenance-rendering-and-privacy.md), [ADR-0113](0113-claim-anatomy-and-dedup.md), [ADR-0114](0114-scoring-unification.md), [ADR-0125](0125-claim-anatomy-temporal-sensitivity-typeregistry.md)
.docs/decisions/0131-structured-embedding-claim-canonicalization.md:28:| Legacy migration recovery (provenance scanning, partial states) | c4, c14, c15 |
.docs/decisions/0131-structured-embedding-claim-canonicalization.md:160:**Claim-type-specific salience overrides** are pinned in the `CLAIM_TYPE_REGISTRY` per ADR-0125. Examples (initial):
.docs/decisions/0131-structured-embedding-claim-canonicalization.md:237:  field_provenance      JSONB NOT NULL, -- per-field source attribution preserved across merge — see conflict-resolution rule below
.docs/decisions/0131-structured-embedding-claim-canonicalization.md:286:**Mode-aware replay contract:** Any operation that consumes `canonicalization_decisions` for the purpose of mutating claim state (canonical_id, dedup_key, contradiction edges, merged provenance) MUST filter on `mode = 'live'` (or equivalently `is_authoritative = true`). Shadow decisions are observational only and have no authority to mutate downstream state under any code path, including replay, ambiguous-pair lifecycle, and re-canonicalization migrations.
.docs/decisions/0131-structured-embedding-claim-canonicalization.md:288:**Per-field provenance preservation on Merge.** `field_provenance` is a JSONB array of `{ field, source_id, source_asof, trust_score }` records, one entry per contributing source. **Conflict-resolution rule** when both source claims contribute a value to the same field at merge time:
.docs/decisions/0131-structured-embedding-claim-canonicalization.md:434:- Registry lives in `abilities-runtime/src/predicates/registry.rs` with explicit enum-like definitions per claim-type.
.docs/decisions/0131-structured-embedding-claim-canonicalization.md:440:- Adding a new predicate requires an ADR amendment (bounded surface — analogous to claim-type addition per ADR-0125).
.docs/decisions/0131-structured-embedding-claim-canonicalization.md:460:- `canonical_merge_applied { merged_claim_id, source_claim_ids, decision_id }` — Phase C only; corroboration count + merged provenance changed
.docs/decisions/0131-structured-embedding-claim-canonicalization.md:479:- User corroboration on a merged claim → strengthens the `field_provenance` confidence; counts toward trust-band uplift per W3-B factor library
.docs/decisions/0131-structured-embedding-claim-canonicalization.md:521:| Per-field provenance lost on merge | `field_provenance` JSONB column (§6); `ClaimEnvelope::field_attribution()` API |
.docs/decisions/0131-structured-embedding-claim-canonicalization.md:529:- Implement `StructuredClaim` types in `abilities-runtime`
.docs/decisions/0131-structured-embedding-claim-canonicalization.md:557:This ADR is the right shape for v1.4.x's substrate thesis:
.docs/decisions/0131-structured-embedding-claim-canonicalization.md:567:- Does NOT change the claim substrate's contradiction-vs-merge semantics (per [ADR-0113](0113-claim-anatomy-and-dedup.md))
.docs/decisions/0131-structured-embedding-claim-canonicalization.md:568:- Does NOT change provenance attribution or trust scoring (per [ADR-0105](0105-provenance-as-first-class-output.md) / [ADR-0114](0114-scoring-unification.md)) — but it DOES enrich per-field provenance via `field_provenance` (§6)
.docs/decisions/0131-structured-embedding-claim-canonicalization.md:569:- Does NOT change sensitivity/temporal_scope gating (per [ADR-0108](0108-provenance-rendering-and-privacy.md) / [ADR-0125](0125-claim-anatomy-temporal-sensitivity-typeregistry.md)) — but it DOES move tier gating earlier in the pipeline via CandidateFilter (§5)
.docs/decisions/0083-product-vocabulary.md:58:| Intelligence (on a meeting) | **Briefing** | "Your meeting briefing." Familiar, implies preparation on your behalf. The meeting detail page is the briefing. |
.docs/decisions/0083-product-vocabulary.md:64:| Prep / prep file | **Briefing** | The output is a briefing, not a task the user needs to do. "Needs prep" becomes a quality indicator (see Section 3). |
.docs/decisions/0083-product-vocabulary.md:78:| Meeting Detail Page | **Meeting Briefing** | It's the briefing for that meeting. Not a "detail page." |
.docs/decisions/0083-product-vocabulary.md:86:| Run Briefing | **Refresh** (when intelligence exists) / **Prepare my day** (cold start) | "Run" is a developer verb. You refresh a briefing. On first open, the system prepares your day. |
.docs/decisions/0086-intelligence-as-shared-service.md:9:Entity intelligence (`intelligence.json`) is the shared enrichment layer. Meeting briefings are mechanical consumers of that intelligence, not independent AI enrichment targets.
.docs/decisions/0086-intelligence-as-shared-service.md:37:4. **Meeting prep is mechanical.** `gather_meeting_context()` reads `intelligence.json` from disk and assembles the briefing from: executive assessment, risks, stakeholder insights, readiness items, open actions, meeting history, email signals. No AI call needed.
.docs/decisions/0086-intelligence-as-shared-service.md:55:- The same intelligence serves the account dashboard, person detail, and meeting briefing
.docs/decisions/0086-intelligence-as-shared-service.md:60:- Meeting briefing quality is bounded by entity intelligence quality
.docs/decisions/0086-intelligence-as-shared-service.md:63:- Entity intelligence enrichment is the lever for improving meeting briefing quality
.docs/decisions/0123-typed-claim-feedback-semantics.md:5:**Target:** v1.4.0 substrate (FeedbackAction enum + ClaimFeedback row + Trust-compiler effects) / v1.4.2 (UI surfaces beyond inline)
.docs/decisions/0123-typed-claim-feedback-semantics.md:6:**Extends:** [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md), [ADR-0114](0114-scoring-unification.md), [ADR-0105](0105-provenance-as-first-class-output.md) (SubjectAttribution amendment)
.docs/decisions/0123-typed-claim-feedback-semantics.md:15:substrate-correct:
.docs/decisions/0123-typed-claim-feedback-semantics.md:39:The v1.4.0 Claim Feedback and Prompt Granularity Review identified that a 5-point accuracy scale or yes/no boolean cannot drive distinct trust, provenance, and repair behavior. Different user intents require different DB writes:
.docs/decisions/0123-typed-claim-feedback-semantics.md:105:    ///   no source penalty; feeds ADR-0108 surface privacy policy.
.docs/decisions/0123-typed-claim-feedback-semantics.md:185:`WrongSubject { corrected_to }` is the user-side complement to ADR-0105's `SubjectAttribution` substrate primitive. Behavior:
.docs/decisions/0123-typed-claim-feedback-semantics.md:188:- `corrected_to: Some(SubjectRef)` — "this content belongs to that entity instead." Tombstone at the asserted subject; propose (not commit) a new claim at the corrected subject with the same claim_text, full provenance preserved, `actor = 'user_correction'`. The corrected claim enters via the standard propose/commit path so it gets the same gate as any other claim.
.docs/decisions/0123-typed-claim-feedback-semantics.md:218:- `WrongSubject` becomes a substrate-level operation pairing cleanly with ADR-0105 `SubjectAttribution`. Subject mistakes do not punish sources.
.docs/decisions/0123-typed-claim-feedback-semantics.md:236:- [ADR-0105: Provenance as First-Class Output](0105-provenance-as-first-class-output.md) — `SubjectAttribution` amendment supplies `SubjectRef` consumed by `WrongSubject`.
.docs/decisions/0106-prompt-fingerprinting-and-provider-interface.md:6:**Extends:** [ADR-0091](0091-intelligence-provider-abstraction.md), [ADR-0105](0105-provenance-as-first-class-output.md)  
.docs/decisions/0106-prompt-fingerprinting-and-provider-interface.md:12:[ADR-0105](0105-provenance-as-first-class-output.md) §1 declares that every ability that invokes the intelligence provider records a `PromptFingerprint` in its provenance envelope. [ADR-0110](0110-evaluation-harness-for-abilities.md) (forthcoming) uses the fingerprint to distinguish prompt regressions from input regressions during evaluation. [ADR-0104](0104-execution-mode-and-mode-aware-services.md) §6 requires replay providers to produce deterministic completions in `Evaluate` mode.
.docs/decisions/0106-prompt-fingerprinting-and-provider-interface.md:14:[ADR-0091](0091-intelligence-provider-abstraction.md) defines `IntelligenceProvider::complete(prompt, tier) -> String`. This signature does not expose the metadata — provider name, model, token counts, seed, prompt template version — that `PromptFingerprint` requires. ADR-0091 was scoped to the intel queue; ADR-0102 expanded provider use to all Transform abilities. This ADR formally amends ADR-0091's trait to expose fingerprint-capable metadata and specifies the fingerprint shape and canonicalization rules.
.docs/decisions/0106-prompt-fingerprinting-and-provider-interface.md:78:`PromptInput` carries the template ID, version, and pre-materialized input values so the provider can compute `tokens_input` accurately. Provider implementations produce `FingerprintMetadata` from their actual invocation; abilities combine it with the prompt-template hash to produce the full `PromptFingerprint` stored in provenance.
.docs/decisions/0106-prompt-fingerprinting-and-provider-interface.md:104:Prompt changes are independent of ability schema versions ([ADR-0102](0102-abilities-as-runtime-contract.md) §8). A prompt bump produces a new fingerprint without bumping the ability version. `prompt_template_version` in the fingerprint lets consumers distinguish prompt-driven output changes from schema-driven ones.
.docs/decisions/0106-prompt-fingerprinting-and-provider-interface.md:143:- [ADR-0102: Abilities as the Runtime Contract](0102-abilities-as-runtime-contract.md) — Prompt changes do not bump ability versions; version discipline in §8.
.docs/decisions/0106-prompt-fingerprinting-and-provider-interface.md:145:- [ADR-0105: Provenance as First-Class Output](0105-provenance-as-first-class-output.md) — §1 `PromptFingerprint` shape referenced; this ADR provides the definition.
.docs/decisions/0098-data-governance-source-aware-lifecycle.md:39:Every record in DailyOS tracks its provenance source. When a source's authorization is revoked, all data from that source is purged. This applies uniformly across all data sources — not just Glean.
.docs/decisions/0098-data-governance-source-aware-lifecycle.md:49:Every record tracks its source via existing provenance mechanisms:
.docs/decisions/0098-data-governance-source-aware-lifecycle.md:52:- `source` field on `entity_intelligence` provenance
.docs/decisions/0098-data-governance-source-aware-lifecycle.md:141:- `people.enrichment_sources` — JSON per-field provenance (already exists)
.docs/decisions/0098-data-governance-source-aware-lifecycle.md:156:4. **I500 (Glean org-score parsing)** stores OrgHealthData with source provenance. Purge clears the baseline score, and the health engine falls back to computed baseline.
.docs/decisions/0098-data-governance-source-aware-lifecycle.md:165:- Builds on existing provenance infrastructure (enrichment_sources, signal source, audit log)
.docs/decisions/0058-proactive-intelligence-maintenance.md:112:The user sees the results in their morning briefing:
.docs/decisions/0058-proactive-intelligence-maintenance.md:168:1. **Relationship drift detection.** Person temperature went from "hot" to "cold" (meeting frequency dropped). The daily briefing surfaces: "You haven't met with Sarah Chen in 6 weeks — she was previously a weekly contact."
.docs/decisions/0058-proactive-intelligence-maintenance.md:172:3. **Portfolio balance alerts.** Entity meeting distribution is skewed — 80% of meetings are with 2 of 12 accounts. The weekly briefing could note: "5 accounts have had no contact in the last 30 days."
.docs/decisions/0058-proactive-intelligence-maintenance.md:176:5. **Prep completeness audit.** Before the daily briefing generates, scan today's meetings: do all external meetings have entity intelligence? Do all attendees have resolved profiles? Surface gaps as "3 of 5 external meetings are fully prepped."
.docs/decisions/0058-proactive-intelligence-maintenance.md:190:- Morning briefing includes a "system health" signal: the user knows the system is working for them
.docs/decisions/0044-meeting-scoped-transcript-intake.md:38:- A transcript is a point-in-time record of a conversation. Once processed, it is **not re-processed** on briefing re-runs.
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:5:**Target:** v1.4.0 substrate (actor taxonomy, propose/commit data model) / v1.5.0 implementation (Analysis Inbox, agent trust ledger, active surfaces)
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:6:**Extends:** [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0103](0103-maintenance-ability-safety-constraints.md), [ADR-0104](0104-execution-mode-and-mode-aware-services.md), [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0107](0107-source-taxonomy-alignment.md)
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:14:1. [ADR-0102](0102-abilities-as-runtime-contract.md) makes every capability a typed, versioned ability. Many of those are Transform abilities that produce claims. The product will ship with multiple ability authors — `prepare_meeting`, `detect_risk_shift`, `generate_weekly_narrative`, future MCP agents — each asserting claims into the same substrate.
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:16:3. The v1.4.1 enrichment refactor ([DOS-241](https://linear.app/a8c/issue/DOS-241)) explicitly asks: "what should enrichment look like when claims have provenance, trust scores, and a propose/commit boundary?"
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:18:Underneath these three is a harder question the substrate as drafted does not answer: **who is allowed to commit a claim, and when does a claim become load-bearing versus merely under consideration?** Today the answer is implicit: whoever writes it. That works when DailyOS is one engineer's enrichment pipeline. It stops working the moment an LLM-backed agent tries to assert "the champion is departing" based on a faint signal, and the briefing tomorrow morning treats that as ground truth.
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:22:This ADR specifies the actor taxonomy, the propose/commit boundary, the contradiction-handling rules, the agent trust ledger, and the deduplication policy that collectively make humans and agents first-class, equal-structured claim sources. The substrate pieces needed in v1.4.0 are limited: an `actor` column and a `claim_state` column on `intelligence_claims`, plus the propose/commit state machine. The user-facing surfaces (Analysis Inbox, agent trust dashboard, contradiction resolution UI) ship in v1.5.0+ and are out of scope for v1.4.0.
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:28:Every claim in `intelligence_claims` carries an `actor` string with one of the following canonical shapes. The `actor` column is non-nullable. Provenance's `Actor` type ([ADR-0105](0105-provenance-as-first-class-output.md) §1) uses the same shape.
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:39:- The `:<version>` suffix on `agent:` is mandatory and matches `ability_version` from provenance ([ADR-0102](0102-abilities-as-runtime-contract.md) §8).
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:51:| `committed` | Claim is authoritative. Live in briefings, MCP, chat. | Included in default reads. |
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:90:Claims that do not pass the gate transition to `proposed` and become visible in the Analysis Inbox (v1.5.0 surface). They do not appear in briefings, MCP responses, or any default-read surface.
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:96:- **Transform abilities** produce `AbilityOutput<ClaimProposal>` — synthesized assertions with full provenance, but no side effect on `intelligence_claims`.
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:99:This matches [ADR-0102](0102-abilities-as-runtime-contract.md) §3's call-graph distinction — Transform abilities must not mutate. The propose/commit split is the structural enforcement of that boundary for claim production.
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:201:Provenance ([ADR-0105](0105-provenance-as-first-class-output.md)) is extended:
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:205:- When a claim is committed via corroboration, its provenance `children[]` includes the corroborating claims' provenance, making the corroboration chain auditable.
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:213:The substrate shipping in v1.4.0 is intentionally minimal:
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:237:- **Propose/commit makes trust operational.** Below-threshold agent output is isolated in the Analysis Inbox rather than leaking into briefings.
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:238:- **Humans and agents share one substrate.** A human analyst's claim and an agent's claim share schema, provenance shape, and trust semantics. Any future multi-agent or multi-analyst scenario composes on this foundation.
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:245:- **Commit policy tuning requires real data.** The auto-commit trust threshold, corroboration threshold, tombstone window, and agent trust floor are all empirical. Expect a 60-day shadow-run post-substrate before the config is stable. During that period, agents run `strategy = "immediate"` to preserve current behavior; propose/commit is observational.
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:315:### R1.5 Actor representation — align with ADR-0102 `Actor` enum
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:317:The original §1 declared actor as a string with canonical shapes. [ADR-0102](0102-abilities-as-runtime-contract.md) §1 defines `Actor` as an enum (`User | Agent | System`). These conflict.
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:332:Serialization produces the same canonical strings the original §1 proposed. Rust code uses the enum; the column stores the rendered string. This preserves ADR-0102's typing while accepting that a columnar DB stores serialized forms.
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:334:The `Actor` enum in ADR-0102 §1 is extended by this ADR (not replaced). The `Agent` variant gains `name` + `version` fields; `Human` is new; `External` is new. This extension requires an amendment to ADR-0102; tracked.
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:338:Original §8 said: identical re-assertion within the window updates `corroboration_count` in place. Codex flagged: this mutates the claim row and loses per-asserter provenance.
.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:365:- No abilities runtime code exists yet. ADR-0102 is also still doc-only. v1.4.0 project work creates both; this ADR's implementation cannot precede ADR-0102's skeleton landing.
.docs/decisions/0076-brand-identity.md:83:Each entity type in DailyOS owns one accent color. This is the color of its accent bar on meeting cards, its icon tint, its page-level accent, and its presence in briefing content. Entity color is **identity** — it answers "what kind of thing is this?" State colors (sage for success, terracotta for attention) still apply independently as **status** — they answer "what state is this thing in?"
.docs/decisions/0076-brand-identity.md:96:Briefing surfaces (daily briefing, weekly briefing, meeting intelligence reports) are the primary reading experience. They use a **constrained subset** of the full palette to maintain editorial calm. The full 14-color system is the brand; the briefing palette is its most disciplined application.
.docs/decisions/0076-brand-identity.md:110:**Not on briefing surfaces:** Linen, Ink, Espresso, Saffron, Chili, Olive, Rosemary. These exist for entity detail pages, settings, error states, and extended UI — not the core reading experience.
.docs/decisions/0076-brand-identity.md:112:**The effect:** Every briefing page feels like the same publication. You never wonder "am I in a different app?" when moving from daily → weekly → intelligence report. The constrained palette creates the visual consistency of a single editorial voice. Entity detail pages can be richer — they're reference material, not briefings.
.docs/decisions/0076-brand-identity.md:139:**3. The sunrise.** Visually, an asterisk is radiant lines from a center point. A starburst. A dawn. DailyOS runs at 6am. Your day starts when the briefing lands. The mark is the moment the sun breaks the horizon and your day is ready.
.docs/decisions/0076-brand-identity.md:166:- **The wildcard:** `*` as a recurring motif beyond the logo. Section breaks. List markers. The finite briefing end-marker could be `* * *` (the typographic section break that also reads as three wildcards).
.docs/decisions/0076-brand-identity.md:180:- Finite briefing pattern
.docs/decisions/0018-hybrid-storage-markdown-sqlite.md:19:- Two data stores means keeping them in sync — the briefing workflow is the synchronization point
.docs/decisions/0037-post-meeting-capture-in-phase-1.md:26:- I17 (captured outcomes don't resurface in briefings) becomes a higher-priority gap — the capture exists but the loop isn't closed
.docs/decisions/0092-data-security-at-rest-and-operational-hardening.md:10:DailyOS stores organizationally sensitive data locally: account intelligence (ARR, renewal risk, health scores), meeting briefings, relationship graphs, email summaries, and enriched PII (names, emails, roles, work history). This is not just personal data -- it is corporate intelligence that belongs to the user's employer.
.docs/decisions/0092-data-security-at-rest-and-operational-hardening.md:268:> DailyOS stores meeting briefings and account intelligence in your workspace folder, which appears to be inside an iCloud-synced directory. This means prep documents, account health data, and relationship notes may be uploading to Apple's servers.
.docs/decisions/0015-defer-inbox-to-phase-2.md:8:Inbox processing (file classification, routing, two-tier with/without AI) adds significant complexity. The core value is passive consumption of the morning briefing.
.docs/decisions/0110-evaluation-harness-for-abilities.md:6:**Extends:** [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0104](0104-execution-mode-and-mode-aware-services.md)  
.docs/decisions/0110-evaluation-harness-for-abilities.md:7:**Depends on:** [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0106](0106-prompt-fingerprinting-and-provider-interface.md), [ADR-0107](0107-source-taxonomy-alignment.md)  
.docs/decisions/0110-evaluation-harness-for-abilities.md:8:**Consumed by:** [ADR-0103](0103-maintenance-ability-safety-constraints.md) §9 (snapshot-diff evaluation), [ADR-0102](0102-abilities-as-runtime-contract.md) §13 (Phase 3 regression gate)
.docs/decisions/0110-evaluation-harness-for-abilities.md:12:[ADR-0102](0102-abilities-as-runtime-contract.md) §10 Enforcement Phase 3 requires a regression gate before abilities become authoritative; Risk #5 declares the evaluation harness a non-negotiable co-dependency. [ADR-0103](0103-maintenance-ability-safety-constraints.md) §9 specifies snapshot-diff evaluation for maintenance abilities. [ADR-0104](0104-execution-mode-and-mode-aware-services.md) §6 requires replay providers in `Evaluate` mode. [ADR-0106](0106-prompt-fingerprinting-and-provider-interface.md) §6 uses fingerprints to classify regression types.
.docs/decisions/0110-evaluation-harness-for-abilities.md:33:│   │   └── expected_provenance.json # Expected provenance envelope (with field-level attributions)
.docs/decisions/0110-evaluation-harness-for-abilities.md:100:   - Compares actual output, provenance, and planned mutations to expected.
.docs/decisions/0110-evaluation-harness-for-abilities.md:141:4. Source IDs retained but flagged as "anonymized" so provenance still works; real IDs not present.
.docs/decisions/0110-evaluation-harness-for-abilities.md:157:[ADR-0102](0102-abilities-as-runtime-contract.md) §13 Phase 3 cutover requires the regression gate to be active before the registry becomes authoritative. The gate's entry requirement is:
.docs/decisions/0110-evaluation-harness-for-abilities.md:245:- [ADR-0102: Abilities as the Runtime Contract](0102-abilities-as-runtime-contract.md) — Phase 3 regression gate depends on this harness.
.docs/decisions/0110-evaluation-harness-for-abilities.md:248:- [ADR-0105: Provenance as First-Class Output](0105-provenance-as-first-class-output.md) — Expected provenance is part of every fixture; attribution accuracy gauged via LLM self-eval.
.docs/decisions/0102-abilities-as-runtime-contract.md:1:# ADR-0102: Abilities as the Runtime Contract
.docs/decisions/0102-abilities-as-runtime-contract.md:5:**Amended:** 2026-05-10 — extended `AbilityPolicy.allowed_actors` to include `SurfaceClient` (defined in [ADR-0111](0111-surface-independent-ability-invocation.md) §8); added §7.6 specifying SurfaceClient policy and introspection semantics. Per the formal L0 panel on [DOS-546](https://linear.app/a8c/issue/DOS-546).
.docs/decisions/0102-abilities-as-runtime-contract.md:6:**Amended (later same day):** 2026-05-10 — extended `AbilityPolicy` schema with `required_scopes: Vec<SurfaceClientScope>` and `mcp_exposure: bool` as first-class fields per the formal L0 Cycle 2 unanimous finding. Two-level SurfaceClient enforcement is now substrate-enforceable from the canonical policy, not just prose in §7.6. Default policy expanded: `required_scopes = vec![]`, `mcp_exposure = false`.
.docs/decisions/0102-abilities-as-runtime-contract.md:7:**Amended (later same day):** 2026-05-10 — promoted `mcp_exposure` from `bool` to tri-state `McpExposure { None | MetadataOnly | Invocable }` enum per Phase 0 artifact 05 lines 383-412; retained `client_side_executable: bool` as an orthogonal separate field for the WordPress 7.0 client-side `executeAbility()` JS path. The two fields govern different trust boundaries — `mcp_exposure` controls the network-facing MCP tool surface (discovery + invocation), `client_side_executable` controls whether a trusted in-product SurfaceClient may invoke the ability after WordPress capability + runtime scope + actor checks pass. The fields are independent; either may be true while the other is false. Default policy is `mcp_exposure = McpExposure::None` + `client_side_executable = false`, opt-in per ability. This cycle-3 reversal of the cycle-2 collapse was made after re-reading artifact 05's explicit warning against the symmetric `client_side_exposure` framing.  
.docs/decisions/0102-abilities-as-runtime-contract.md:8:**Amended:** 2026-05-19 — added `Actor::McpClient` actor variant + `ActorKind::McpClient` discriminator for MCP-originated calls; pinned MCP client authentication + session contract; froze the v1.4.7 scope namespace convention. See §"Amendment 2026-05-19 — `Actor::McpClient` + MCP client trust boundary + scope namespace" appended at end of doc. Companion ADR-0128 amendment of the same date documents the headless-head consequences (tool description as product copy, cross-conversation continuity, displacement-use-case framing applied to v1.4.7 W5 eval).
.docs/decisions/0102-abilities-as-runtime-contract.md:15:DailyOS delivers depth-of-context before every interaction. A user walking into a meeting receives not a summary of their calendar but a briefing shaped by the full state of the relationships, accounts, commitments, and signals relevant to that moment. This is the product's differentiation and its moat: depth compounds over time as more signals flow in, more context accumulates, and more relationships are understood.
.docs/decisions/0102-abilities-as-runtime-contract.md:22:- Prompts, synthesis logic, and provenance are distributed across the codebase rather than owned by a capability.
.docs/decisions/0102-abilities-as-runtime-contract.md:24:The substrate ADRs — services ([ADR-0101](0101-service-boundary-enforcement.md)), signals ([ADR-0080](0080-signal-intelligence-architecture.md)), entities ([ADR-0082](0082-entity-generic-prep-pipeline.md)), providers ([ADR-0091](0091-intelligence-provider-abstraction.md)), data governance ([ADR-0098](0098-data-governance-source-aware-lifecycle.md)), Glean-first retrieval ([ADR-0100](0100-glean-first-intelligence-architecture.md)) — establish how state is mutated, how signals flow, how entities are modeled, and how intelligence is retrieved. None of them establishes how a **product capability** is shaped, named, versioned, or exposed. That layer is missing.
.docs/decisions/0102-abilities-as-runtime-contract.md:38:- Returns a typed output wrapped in `AbilityOutput<T>`, which carries the domain output alongside a mandatory provenance envelope. Provenance shape is specified in [ADR-0105](0105-provenance-as-first-class-output.md). The domain output itself does not re-embed provenance.
.docs/decisions/0102-abilities-as-runtime-contract.md:41:**Boundary scope.** The abilities layer governs synthesized user-facing and agent-facing outputs — briefings, narratives, entity context, risk detection, curated publications. Purely mechanical operations (list entity names for a picker, format a timestamp, read a single field for a toolbar badge) remain in their owning service or command and are not abilities. A good heuristic: if an output is structured, composed from multiple sources, or involves synthesis, it is an ability. If it is a single read or a trivial projection, it is not.
.docs/decisions/0102-abilities-as-runtime-contract.md:57:├── provenance.rs               # Provenance types (detail in ADR-0105)
.docs/decisions/0102-abilities-as-runtime-contract.md:111:    // 3. Attribute: attach provenance to each output field
.docs/decisions/0102-abilities-as-runtime-contract.md:112:    // 4. Return: typed output + provenance
.docs/decisions/0102-abilities-as-runtime-contract.md:116:`AbilityResult<T>` is `Result<AbilityOutput<T>, AbilityError>`, where `AbilityOutput<T>` wraps the domain output with required provenance, version metadata, and diagnostics.
.docs/decisions/0102-abilities-as-runtime-contract.md:144:Every ability declares typed input and output structs in its own module. The output is always wrapped in `AbilityOutput<T>`, which carries provenance exactly once — the domain output does not re-embed it.
.docs/decisions/0102-abilities-as-runtime-contract.md:164:    // Note: no `provenance` field here. It lives in the AbilityOutput<T> wrapper below.
.docs/decisions/0102-abilities-as-runtime-contract.md:172:    pub provenance: Provenance,           // Canonical location — see ADR-0105
.docs/decisions/0102-abilities-as-runtime-contract.md:180:- Provenance lives on `AbilityOutput<T>`, exactly one place. Domain output types never re-declare a `provenance` field. Reviewers reject such declarations at registration.
.docs/decisions/0102-abilities-as-runtime-contract.md:181:- Field-level provenance attribution (how `suggested_outcomes[0]` ties back to specific signals) is specified in [ADR-0105](0105-provenance-as-first-class-output.md). This ADR requires that the mechanism exist; 0105 defines the shape.
.docs/decisions/0102-abilities-as-runtime-contract.md:198:    allowed_actors = [User, Agent, System],
.docs/decisions/0102-abilities-as-runtime-contract.md:217:    pub allowed_actors: Vec<Actor>,             // Who can invoke: User | Agent | SurfaceClient | System (see §7.6)
.docs/decisions/0102-abilities-as-runtime-contract.md:222:                                                  // SurfaceClient permitted by allowed_actors with no specific scope check. Non-empty means:
.docs/decisions/0102-abilities-as-runtime-contract.md:225:    pub mcp_exposure: McpExposure,              // Tri-state exposure tier for MCP — both the Rust runtime's MCP server AND any
.docs/decisions/0102-abilities-as-runtime-contract.md:231:                                                  // scope + actor checks pass. Orthogonal to mcp_exposure: either field may be true
.docs/decisions/0102-abilities-as-runtime-contract.md:248:pub struct SurfaceClientScope(pub String);      // Scope identifier — e.g., "read.account_overview", "submit.feedback",
.docs/decisions/0102-abilities-as-runtime-contract.md:278:- MCP tool registration. Each ability whose `AbilityPolicy.allowed_actors` includes `Agent` becomes an MCP tool. The MCP handler looks up the ability by name and invokes the erased wrapper. Schema drives MCP tool description.
.docs/decisions/0102-abilities-as-runtime-contract.md:285:Registry enumeration is actor-scoped. An MCP client enumerating available tools sees only abilities whose `AbilityPolicy.allowed_actors` includes `Agent` AND whose `mcp_exposure` is `McpExposure::MetadataOnly` or `McpExposure::Invocable`; abilities with `mcp_exposure: None` are filtered. `MetadataOnly` abilities are enumerated with name + description only — the input/output schema and any blast-radius hints are withheld until promotion to `Invocable`; their invocation handler is not registered with the MCP server. `Invocable` abilities are enumerated with full schema and an invocation handler. Introspection does not leak ability names, input schemas, or blast radius to callers that are not authorized to invoke.
.docs/decisions/0102-abilities-as-runtime-contract.md:287:The Tauri bridge, invoked by the first-party app, sees the full registry subject to `User` and `System` filtering; the `mcp_exposure` and `client_side_executable` fields do not gate the first-party Tauri bridge because that bridge is the in-process path for the desktop app, not an MCP-mediated or SurfaceClient-mediated surface. Background workers see the `System` filter.
.docs/decisions/0102-abilities-as-runtime-contract.md:295:[ADR-0111](0111-surface-independent-ability-invocation.md) §8 introduces `SurfaceClient` as a fourth canonical actor class for third-party local surfaces (WordPress, Obsidian, future browser extensions / PWA / mobile). The `AbilityPolicy.allowed_actors` field accepts `SurfaceClient` as a valid value alongside `User`, `Agent`, and `System`.
.docs/decisions/0102-abilities-as-runtime-contract.md:297:**Multi-level enforcement.** Unlike `User`/`Agent`/`System` — where the actor *class* is the unit of policy enforcement — `SurfaceClient` enforces at multiple levels. All checks are against canonical fields on `AbilityPolicy` (per §7.1, extended 2026-05-10 cycle 2, refined later the same day to promote `mcp_exposure` to the `McpExposure` tri-state and add the orthogonal `client_side_executable` field). Invocation is permitted only if ALL applicable checks pass:
.docs/decisions/0102-abilities-as-runtime-contract.md:299:1. The ability's `allowed_actors` includes `SurfaceClient`.
.docs/decisions/0102-abilities-as-runtime-contract.md:301:3. If the invocation arrives via an MCP-mediated path (the Rust runtime's MCP server OR a SurfaceClient-mediated MCP server per [ADR-0129](0129-composable-surfaces-wordpress-studio-as-primary-surface.md) §4), the ability's `mcp_exposure` is `McpExposure::Invocable`. If `mcp_exposure` is `McpExposure::None`, MCP-mediated discovery AND invocation are rejected at the registry boundary. If `mcp_exposure` is `McpExposure::MetadataOnly`, MCP-mediated discovery enumerates name + description only (no input/output schema), and MCP-mediated invocation is rejected at the registry boundary. The tier check happens regardless of `allowed_actors`.
.docs/decisions/0102-abilities-as-runtime-contract.md:302:4. If the invocation arrives via a client-side SurfaceClient JS path (e.g., a WordPress 7.0 block invoking `executeAbility()`), the ability's `client_side_executable` is `true`. If `client_side_executable` is `false`, the SurfaceClient bridge rejects the invocation before registry lookup regardless of `allowed_actors`, `required_scopes`, or `mcp_exposure`. This check is orthogonal to the MCP-mediated tier check — the two fields govern different trust boundaries (per Phase 0 artifact 05 lines 383-412).
.docs/decisions/0102-abilities-as-runtime-contract.md:304:Each check rejects from the substrate (the registry or the SurfaceClient bridge) before the ability body runs. The enforcement is **substrate-enforceable**, not prose-only: a SurfaceClient instance granted only `read.account_overview` cannot invoke an ability whose `required_scopes` includes `submit.feedback`; an MCP client cannot invoke an ability whose `mcp_exposure` is `None` or `MetadataOnly` even if its `allowed_actors` permits the caller's actor class; a WordPress block's `executeAbility()` call cannot reach an ability whose `client_side_executable` is `false` even if it is fully `Invocable` via MCP.
.docs/decisions/0102-abilities-as-runtime-contract.md:308:- A SurfaceClient enumerating available abilities via an MCP-mediated path sees only abilities whose policy: (a) permits `SurfaceClient` in `allowed_actors`, (b) has every `required_scopes` entry covered by the instance's granted scopes, AND (c) has `mcp_exposure` of `MetadataOnly` or `Invocable`. Abilities with `mcp_exposure: None` are not enumerated. Abilities with `mcp_exposure: MetadataOnly` are enumerated with name + description only (no input/output schema); the invocation handler is not registered.
.docs/decisions/0102-abilities-as-runtime-contract.md:309:- A SurfaceClient enumerating available abilities via a client-side JS path (e.g., WordPress 7.0 `executeAbility()` enumeration) sees only abilities whose policy: (a) permits `SurfaceClient` in `allowed_actors`, (b) has every `required_scopes` entry covered by the instance's granted scopes, AND (d) has `client_side_executable: true`. Abilities with `client_side_executable: false` are not exposed to the client-side enumeration path even when fully `Invocable` via MCP.
.docs/decisions/0102-abilities-as-runtime-contract.md:312:- A SurfaceClient does NOT see ability names, schemas, or blast radius for abilities outside its scope grants, for `mcp_exposure: None` abilities on MCP-mediated paths, or for `client_side_executable: false` abilities on client-side JS paths. Unauthorized introspection is closed.
.docs/decisions/0102-abilities-as-runtime-contract.md:314:**Default policy.** Abilities default to `allowed_actors = [User]`, `required_scopes = vec![]`, `mcp_exposure = McpExposure::None`, `client_side_executable = false`. `SurfaceClient` exposure is opt-in per ability — the macro attribute must declare `allowed_actors` including `SurfaceClient`, plus `required_scopes` AND `mcp_exposure` AND `client_side_executable` explicitly (or `no_scope_required` in place of an empty `required_scopes` when the absence is intentional; see §7.1 macro). Most existing abilities do NOT need to be exposed to SurfaceClient instances; ability authors must consciously opt each one in. The two exposure fields govern different trust boundaries (MCP tool surface vs. client-side JS invocation) per Phase 0 artifact 05 lines 383-412 — either may be opted in while the other remains closed.
.docs/decisions/0102-abilities-as-runtime-contract.md:316:**Compile-error gate (substrate, not code-review discipline).** The `#[ability]` proc-macro emits a compile-time error if `allowed_actors` includes `SurfaceClient` AND `required_scopes` is empty AND the `no_scope_required` opt-out is absent. The gate fires on `allowed_actors` membership regardless of `client_side_executable` value, because actor membership is what makes the ability reachable from a SurfaceClient at all. This makes empty-scope SurfaceClient abilities a build-time failure, not a runtime gap or a reviewer-trust convention — drift is structurally impossible.
.docs/decisions/0102-abilities-as-runtime-contract.md:320:**Audit attribution.** Every substrate operation invoked by a SurfaceClient is logged with the SurfaceClient instance identity (per ADR-0111 §8). Reads and writes both. This makes "which surface invoked this" forensically answerable.
.docs/decisions/0102-abilities-as-runtime-contract.md:360:**Rule 5 — Every ability output carries provenance via `AbilityOutput<T>`.**  
.docs/decisions/0102-abilities-as-runtime-contract.md:361:Provenance lives exactly once, on the wrapper. Domain output types never re-declare it. The mechanism is what makes depth explainable and quality measurable. Shape specified in [ADR-0105](0105-provenance-as-first-class-output.md).
.docs/decisions/0102-abilities-as-runtime-contract.md:383:- Transform outputs carry a `trust: Trust::Untrusted` marker in their provenance ([ADR-0105](0105-provenance-as-first-class-output.md)).
.docs/decisions/0102-abilities-as-runtime-contract.md:395:Abilities compose other abilities to build richer capabilities. Composition introduces risks — cycles, transaction propagation, provenance merging, trace nesting — that must be specified.
.docs/decisions/0102-abilities-as-runtime-contract.md:405:The type of `AbilityContext.services` does not change during a transaction; it remains `&ServiceContext<'a>`. What changes is the internal state: `ServiceContext` holds an `Option<TxHandle>` that is `Some` during an active transaction and `None` otherwise. Service mutation functions consult this internal handle: if present, they route the write through it; if absent, they write directly to the DB. This keeps the public type stable for composition (children just receive the same `&ServiceContext` reference) while letting the substrate route correctly. `with_transaction_async` sets the internal handle on entry and clears it on commit/rollback. This is propagation, not reentrancy: the child does not open a new transaction; it uses the parent's.
.docs/decisions/0102-abilities-as-runtime-contract.md:415:When ability A invokes ability B, A's provenance envelope includes B's provenance as a nested child. The full provenance tree shows the composition. Deduplication rules and merge semantics specified in [ADR-0105](0105-provenance-as-first-class-output.md).
.docs/decisions/0102-abilities-as-runtime-contract.md:423:Two substrate ADRs constrain how abilities consume intelligence: [ADR-0091](0091-intelligence-provider-abstraction.md) (pluggable LLM providers) and [ADR-0100](0100-glean-first-intelligence-architecture.md) (Glean is the primary intelligence engine when connected).
.docs/decisions/0102-abilities-as-runtime-contract.md:434:2. Compose that state into a user-facing or agent-facing synthesis (briefing, narrative, prep document) that is ability-specific, not engine-specific.
.docs/decisions/0102-abilities-as-runtime-contract.md:476:4. **Explainability is a product feature, not an afterthought.** Every output carries provenance. Users and agents see why a thing surfaced. Trust is legible.
.docs/decisions/0102-abilities-as-runtime-contract.md:478:6. **Agents receive full product quality.** MCP tools are thin wrappers over abilities, not reduced subsets. When an agent invokes DailyOS, it receives the same briefing the app renders.
.docs/decisions/0102-abilities-as-runtime-contract.md:486:2. **Contract surface area increases.** Every capability now has a named input struct, output struct, provenance, and eval fixtures. This is more code to maintain.
.docs/decisions/0102-abilities-as-runtime-contract.md:505:- [ADR-0082: Entity-Generic Prep Pipeline](0082-entity-generic-prep-pipeline.md) — The entity model is the substrate on which abilities operate. `get_entity_context` generalizes the prep pipeline.
.docs/decisions/0102-abilities-as-runtime-contract.md:507:- [ADR-0098: Data Governance — Source-Aware Lifecycle](0098-data-governance-source-aware-lifecycle.md) — Data source tagging flows through into ability provenance.
.docs/decisions/0102-abilities-as-runtime-contract.md:512:- **ADR-0105 (forthcoming): Provenance as First-Class Output** — Specifies the provenance shape, field-level attribution, trust markers, merge semantics, and rendering contract referenced throughout this ADR.
.docs/decisions/0102-abilities-as-runtime-contract.md:515:- **ADR-0108 (forthcoming): Surface-Independent Ability Invocation** — Specifies MCP and Tauri binding rules concretely (which follows from §7.3).
.docs/decisions/0102-abilities-as-runtime-contract.md:530:1. **Hard error** — ability returns `Err(AbilityError::...)`. Caller sees no output. Surface decides user-visible message per [ADR-0108](0108-provenance-rendering-and-privacy.md). No partial data leaks.
.docs/decisions/0102-abilities-as-runtime-contract.md:545:The `warnings[]` field on Provenance is the canonical record of "what went soft." Surfaces render it per [ADR-0108](0108-provenance-rendering-and-privacy.md).
.docs/decisions/0102-abilities-as-runtime-contract.md:603:`McpClientId` is a server-minted opaque handle issued during the MCP client pairing handshake (§C). Newtype lives at the abilities-runtime layer alongside `SurfaceClientId`; non-PII, non-enumerable, stable for the life of the pairing.
.docs/decisions/0102-abilities-as-runtime-contract.md:607:The corresponding `ActorKind::McpClient` discriminator joins the existing four kinds and is the const-friendly token used in `AbilityPolicy.allowed_actors`. `Actor::kind()` gains a `McpClient { .. } => ActorKind::McpClient` arm.
.docs/decisions/0102-abilities-as-runtime-contract.md:614:- **McpClient** does not carry scopes because the MCP gateway (v1.4.7 `services/mcp_v2/gateway.rs`) enforces scope authorization against a **server-side per-client manifest** loaded by `services/mcp_v2/auth.rs` from the pairing record (§C). Caller-provided scope claims are rejected on principle; the gateway is the single source of truth. Once the gateway admits the call, the abilities runtime trusts the gateway's check and gates only on `AbilityPolicy.allowed_actors.contains(&ActorKind::McpClient)`.
.docs/decisions/0102-abilities-as-runtime-contract.md:627:**Revocation.** Server-side admin API (W1-A scope) can revoke an `McpClientId`. Subsequent invocations from a revoked client fail the manifest-load step and return `ToolError::PairingRevoked`. Revocation propagates within one in-flight call; in-progress invocations are not interrupted but their audit row records the revoked state if revocation happens mid-call. (`PairingRevoked` is a dedicated `ToolError` variant rather than `Unauthorized` with a sentinel `missing_scope` because the `Scope` newtype is reserved for `<namespace>.<verb>.<noun>` / substrate-shipped values per §E — auth-state revocation is not a scope-deficit case.)
.docs/decisions/0102-abilities-as-runtime-contract.md:633:Host-model conversations are stateless; DailyOS is not. Cross-conversation continuity (per ADR-0128 §6) is implemented via a server-minted opaque token:
.docs/decisions/0102-abilities-as-runtime-contract.md:651:- **Examples.** `dailyos.read.account_status`, `dailyos.read.daily_briefing`, `dailyos.read.portfolio_attention`, `dailyos.search.workspace_memory`, `dailyos.write.place_document`, `dailyos.submit.note`, `dailyos.submit.action`.
.docs/decisions/0102-abilities-as-runtime-contract.md:652:- **v1.4.5-reused scopes stay unprefixed.** Scopes shipped by v1.4.5 substrate (e.g. `write.workspace_place_document`, `read.workspace_graph`, `read.entity_names`) are kept verbatim. v1.4.7 MCP tools that reuse them present the unprefixed scope to the manifest. The manifest validation logic accepts both forms but rejects any scope outside the canonical allowlist.
.docs/decisions/0102-abilities-as-runtime-contract.md:655:### F. `AbilityPolicy.allowed_actors`
.docs/decisions/0102-abilities-as-runtime-contract.md:657:`AbilityPolicy.allowed_actors` may now include `ActorKind::McpClient` alongside the existing four kinds. Defaults are unchanged (`&[ActorKind::User]`, least-privilege floor); abilities opt in to MCP exposure by adding `ActorKind::McpClient` to their `allowed_actors` slice.
.docs/decisions/0102-abilities-as-runtime-contract.md:679:### I. Companion ADR-0128 amendment
.docs/decisions/0102-abilities-as-runtime-contract.md:681:ADR-0128 receives a same-date amendment recording the v1.4.7-specific consequences of this trust-boundary contract: tool description as product copy with eval coverage (DOS-481), cross-conversation continuity surfaced as a named affordance per ADR-0128 §6 with the wire-level shape defined here in §D, and extension of the displacement-use-case framing to v1.4.7 W5 host-selection evaluation. ADR-0128 §5 ("Feedback is the only write") narrows in v1.4.7's headless head: writes are restricted to the v1.4.7 W4 surface (note/observation, create_action, update_action_status), all routed through approved `services::*` per ADR-0101.
.docs/decisions/0102-abilities-as-runtime-contract.md:689:The `Scope` newtype is reserved for `<namespace>.<verb>.<noun>` (or pre-amendment substrate-shipped) values per §E. Auth-state rejection paths therefore live as their own `ToolError` variants rather than abusing `Unauthorized::missing_scope` with sentinel `Scope` strings.
.docs/decisions/0102-abilities-as-runtime-contract.md:702:- **Resolution is keyed on `(client_id, handle)`.** A presented handle that exists but was issued to a different `McpClientId` resolves to "no matching binding" and returns `ToolError::ConversationRevoked` (the substrate's existing handle-revocation path; from the caller's perspective the handle is no longer valid and a fresh conversation must be started).
.docs/decisions/0048-three-tier-data-model.md:37:| **Filesystem** (markdown + JSON) | The durable, portable layer. What survives app deletion. What the ecosystem consumes. What the user owns. | Permanent — user's files | Archive, dashboard.md, dashboard.json, transcripts, user notes, config.json, briefing data |
.docs/decisions/0048-three-tier-data-model.md:70:| Action items (incomplete) | Archive markdown files, briefing JSON |
.docs/decisions/0084-surface-jobs-to-be-done.md:40:| Lead Story | Highlight the one meeting that most needs awareness | A brief intelligence narrative — "why this meeting matters" — plus bridge link to the full briefing |
.docs/decisions/0084-surface-jobs-to-be-done.md:67:**Note:** "Your Meetings" and "Commitments" chapters are removed. They duplicated the daily briefing and actions page with less capability.
.docs/decisions/0084-surface-jobs-to-be-done.md:86:| Act II: The Room | Comprehensive stakeholder briefing | "Who will I be talking to?" |
.docs/decisions/0084-surface-jobs-to-be-done.md:175:| B1 | EntityPicker on daily briefing | User prefers keeping editing affordances accessible on the reading surface |
.docs/decisions/0084-surface-jobs-to-be-done.md:192:The expanded meeting card on the daily briefing schedule is a **pre-meeting orientation layer**. Its job is to answer "what is this meeting and what are we covering" — not to surface strategic context or account intelligence. That distinction keeps the card focused and the information hierarchy clear.
.docs/decisions/0084-surface-jobs-to-be-done.md:195:- Meeting agenda (read-only, sourced from the meeting briefing)
.docs/decisions/0084-surface-jobs-to-be-done.md:201:- Wins, risks, talking points, account health, prep notes — these belong exclusively to the meeting briefing. The expanded card deliberately excludes this layer so it doesn't compete with the briefing and stays scannable.
.docs/decisions/0084-surface-jobs-to-be-done.md:203:**Agenda as shared data:** The meeting briefing is the single source of truth for the agenda. The expanded card reads from it. If the agenda is updated or generated in the briefing, the card reflects that change. Editing the agenda happens in the briefing only — not inline on the card.
.docs/decisions/0084-surface-jobs-to-be-done.md:208:- The **Lead Story** is a brief intelligence narrative (the "why this meeting matters" headline) + bridge link to the full briefing. No prep grid, no key people, no action checklist.
.docs/decisions/0084-surface-jobs-to-be-done.md:209:- The **schedule expansion panel** shows meeting facts (agenda, description, attendees, docs). No intelligence content. Zero overlap with the meeting briefing.
.docs/decisions/0084-surface-jobs-to-be-done.md:210:- The **"Before This Meeting"** items (D6) are owned by the briefing. The card doesn't surface them.
.docs/decisions/0084-surface-jobs-to-be-done.md:240:- **The daily briefing stays in its lane.** Today only. No weekly leakage.
.docs/decisions/0084-surface-jobs-to-be-done.md:251:- **Lead Story compaction** could reduce the daily briefing's perceived value if the intelligence narrative isn't compelling enough on its own. Mitigate by ensuring the narrative is high-quality, not just the first paragraph of the meeting prep.
.docs/decisions/0108-provenance-rendering-and-privacy.md:1:# ADR-0108: Provenance Rendering and Privacy
.docs/decisions/0108-provenance-rendering-and-privacy.md:6:**Extends:** [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0107](0107-source-taxonomy-alignment.md)  
.docs/decisions/0108-provenance-rendering-and-privacy.md:7:**Related:** [ADR-0094](0094-audit-log-and-enterprise-observability.md), [ADR-0093](0093-prompt-injection-hardening.md), [ADR-0111](0111-surface-independent-ability-invocation.md), [ADR-0130](0130-surface-independent-composition-contract.md) §2 (block-level `ProvenanceRef` references this ADR's actor-filtered envelope; the ref shape avoids exceeding the rendering-side payload caps this ADR governs)
.docs/decisions/0108-provenance-rendering-and-privacy.md:11:[ADR-0105](0105-provenance-as-first-class-output.md) defines the `Provenance` envelope and requires surfaces to render it. It defers three concerns to this ADR:
.docs/decisions/0108-provenance-rendering-and-privacy.md:15:3. **The `ProvenanceMasked` shape** referenced by [ADR-0105](0105-provenance-as-first-class-output.md) §8 and used when sources are revoked.
.docs/decisions/0108-provenance-rendering-and-privacy.md:17:[ADR-0094](0094-audit-log-and-enterprise-observability.md) governs the append-only JSONL security audit with strict PII hygiene. [ADR-0093](0093-prompt-injection-hardening.md) established that LLM outputs can contain adversarial content. This ADR integrates both: provenance must be renderable to users and agents without leaking internal graph structure or PII, and sanitization of LLM-generated `explanation` text must prevent re-injection.
.docs/decisions/0108-provenance-rendering-and-privacy.md:28:- A third-level "Full provenance" expansion shows the JSON envelope for power users and debugging.
.docs/decisions/0108-provenance-rendering-and-privacy.md:33:- Every tool response includes provenance by default. The `Provenance` envelope serializes as a structured field alongside the domain output.
.docs/decisions/0108-provenance-rendering-and-privacy.md:34:- Actor-filtered contents: the MCP wrapper strips content from provenance that is not permitted for `Actor::Agent`:
.docs/decisions/0108-provenance-rendering-and-privacy.md:36:  - Child provenance trees are collapsed to `ChildElided { ability_name, data_sources_summary }` beyond depth 2.
.docs/decisions/0108-provenance-rendering-and-privacy.md:39:- An MCP tool `get_provenance(invocation_id)` provides a fuller view for authorized agents if needed; default response is summary-sized.
.docs/decisions/0108-provenance-rendering-and-privacy.md:43:- Publications carry a provenance footnote listing the top-level sources by `data_source` class (e.g., "Based on REDACTED, recent meetings, and team activity") — **no source identifiers, no entity names, no attribution text**.
.docs/decisions/0108-provenance-rendering-and-privacy.md:51:- Application logs reference `invocation_id` and do not include provenance content; to retrieve a run's provenance, query the audit table.
.docs/decisions/0108-provenance-rendering-and-privacy.md:59:pub fn render_provenance_for(prov: &Provenance, actor: Actor, surface: Surface) -> RenderedProvenance { ... }
.docs/decisions/0108-provenance-rendering-and-privacy.md:64:    McpToolDetail,    // Actor-filtered, full-detail (invoked via get_provenance)
.docs/decisions/0108-provenance-rendering-and-privacy.md:70:The renderer consults `actor` and `surface` to determine what to include. A user rendering provenance in the app sees the full graph; an agent accessing the same ability's output via MCP sees a filtered version; a P2 publication sees source classes only.
.docs/decisions/0108-provenance-rendering-and-privacy.md:76:LLM-generated `explanation` text is untrusted per [ADR-0093](0093-prompt-injection-hardening.md). It can contain prompt injection attempts, PII from other sources, or links to attacker-controlled content. Every explanation is passed through a sanitizer before rendering:
.docs/decisions/0108-provenance-rendering-and-privacy.md:88:When a source is revoked and its associated provenance records are masked per [ADR-0107](0107-source-taxonomy-alignment.md) §5:
.docs/decisions/0108-provenance-rendering-and-privacy.md:110:**Masking is irreversible.** Once a provenance is masked, the original content is discarded. Re-running the ability after the source is re-connected produces a new invocation with its own provenance. This is the cost of the mask — diagnosis of past runs becomes unavailable for revoked sources, but user privacy is protected.
.docs/decisions/0108-provenance-rendering-and-privacy.md:114:[ADR-0105](0105-provenance-as-first-class-output.md) §9 specifies storage size budgets. Rendering has its own:
.docs/decisions/0108-provenance-rendering-and-privacy.md:117:- **MCP default response:** provenance payload ≤ 10KB. Full detail available via `get_provenance(invocation_id)`.
.docs/decisions/0108-provenance-rendering-and-privacy.md:124:[ADR-0105](0105-provenance-as-first-class-output.md) §7 declared "every surface that displays ability output MUST be capable of rendering provenance." This ADR narrows the "MUST" to outputs where provenance rendering is meaningful:
.docs/decisions/0108-provenance-rendering-and-privacy.md:128:- Meeting prep, daily briefing, weekly narrative, risk assessment, entity context detail pages
.docs/decisions/0108-provenance-rendering-and-privacy.md:137:- Settings list views (each row can show a "source count" indicator; full provenance on tap)
.docs/decisions/0108-provenance-rendering-and-privacy.md:139:The test: if the output is a *composed* or *synthesized* artifact the user will reason about, render provenance. If it is a *presentation affordance* showing a value, a count, or a name, the rendering is optional (but the provenance still exists in storage and can be queried).
.docs/decisions/0108-provenance-rendering-and-privacy.md:149:5. **Size budgets bounded per surface.** Rendering cost is predictable; runaway provenance payloads are truncated with an explicit warning.
.docs/decisions/0108-provenance-rendering-and-privacy.md:154:1. **Renderer complexity grows.** A shared `render_provenance_for(prov, actor, surface)` function with branching logic per surface and actor is non-trivial. Worth it.
.docs/decisions/0108-provenance-rendering-and-privacy.md:156:3. **MCP `get_provenance(invocation_id)` tool adds surface.** Agents can request detail they did not receive by default.
.docs/decisions/0108-provenance-rendering-and-privacy.md:162:2. **Renderer drift across surfaces.** The Tauri app diverges from MCP in how it interprets provenance. Mitigation: `render_provenance_for` is the single entry point; surfaces cannot bypass it.
.docs/decisions/0108-provenance-rendering-and-privacy.md:163:3. **Masked provenance hides legitimate debugging.** User reports data issue; the relevant maintenance audit is masked. Mitigation: masking is irreversible as a matter of privacy. Live re-run reproduces the data issue with fresh provenance.
.docs/decisions/0108-provenance-rendering-and-privacy.md:169:- [ADR-0105: Provenance as First-Class Output](0105-provenance-as-first-class-output.md) — Defines the envelope; this ADR specifies rendering.
.docs/decisions/0108-provenance-rendering-and-privacy.md:171:- [ADR-0094: Audit Log and Enterprise Observability](0094-audit-log-and-enterprise-observability.md) — Security audit remains separate from provenance rendering.
.docs/decisions/0108-provenance-rendering-and-privacy.md:172:- [ADR-0093: Prompt Injection Hardening](0093-prompt-injection-hardening.md) — Explanation sanitizer aligns with ADR-0093's untrusted-content rules.
.docs/decisions/0108-provenance-rendering-and-privacy.md:178:## Amendment — 2026-04-20 — Enforce 64 KB provenance size cap
.docs/decisions/0108-provenance-rendering-and-privacy.md:182:§6 originally introduced size budgets as a soft guideline. In practice a deeply-composed ability (meeting prep composing context which composes claims which compose trajectories) can construct a provenance envelope of several hundred KB. Tauri IPC has serialization limits; this will hit them. Persona review flagged: named limits without enforcement become silent bloat.
.docs/decisions/0108-provenance-rendering-and-privacy.md:184:**Hard cap: 64 KB serialized provenance per ability output.**
.docs/decisions/0108-provenance-rendering-and-privacy.md:188:- The cap is enforced at `AbilityOutput<T>` construction time. The error surfaces to the caller as a hard error (per [ADR-0102](0102-abilities-as-runtime-contract.md) error-handling amendment).
.docs/decisions/0108-provenance-rendering-and-privacy.md:192:1. **Provenance summarization** — collapse deep composition trees into summary nodes marked `ProvenanceWarning::DepthElided`. [ADR-0105](0105-provenance-as-first-class-output.md) §1 already declares this warning class; this amendment activates it.
.docs/decisions/0108-provenance-rendering-and-privacy.md:196:**Starting value of 64 KB is conservative.** Measure during the first end-to-end slice (strategy doc § Path forward action 1). If real meeting-prep provenance is larger than 64 KB on representative entities, raise the cap via amendment with the measurement as justification. Do not raise silently.
.docs/decisions/0062-briefing-artifacts-vs-live-queries.md:15:The two paradigms overlap at the dashboard: `get_dashboard_data()` merges schedule.json (briefing enrichment) with live calendar events (current truth) via `calendar_merge.rs`. This works because the merge is explicit and the consumer (dashboard) expects both signals.
.docs/decisions/0062-briefing-artifacts-vs-live-queries.md:19:1. **Staleness.** schedule.json is a morning snapshot. A meeting added at 11am would not update the computed fields until the next briefing run. Users would see stale capacity data.
.docs/decisions/0062-briefing-artifacts-vs-live-queries.md:80:- **`get_dashboard_data()`** continues merging briefing + live calendar for the dashboard. It's a consumer of both paradigms and that's correct — the dashboard shows enriched content (briefing) overlaid on current timing (live).
.docs/decisions/0062-briefing-artifacts-vs-live-queries.md:82:- **`deliver_schedule()`** continues producing schedule.json as a briefing document. No changes to the delivery pipeline.
.docs/decisions/0062-briefing-artifacts-vs-live-queries.md:83:- **Week overview** (`build_day_shapes()` in orchestrate.rs) already computes gap/density data during weekly prep. This is correct — weekly briefing is a document, computed at generation time. The query module handles intra-day freshness for the daily view.
.docs/decisions/0062-briefing-artifacts-vs-live-queries.md:95:- Two code paths for "schedule data": briefing (schedule.json, for AI content) and live (query functions, for current state). Developers must know which to use. The boundary table above is the guide.
.docs/decisions/0062-briefing-artifacts-vs-live-queries.md:96:- Dashboard already merges both paradigms. Adding a third consumer (focus page) that uses query functions but not briefing artifacts may feel inconsistent — but it's correct because the focus page needs live truth, not morning enrichment.
.docs/decisions/0062-briefing-artifacts-vs-live-queries.md:99:- Accepting that briefing content goes stale is a feature, not a bug. "Your morning briefing said X" is a valid statement even if the schedule has since changed. The briefing is a document you received; the live query is the current reality. Both are useful.
.docs/decisions/0109-temporal-primitives-in-the-entity-graph.md:7:**Consumed by:** [ADR-0102](0102-abilities-as-runtime-contract.md) (Read abilities), [ADR-0105](0105-provenance-as-first-class-output.md) (provenance references trajectory snapshots)
.docs/decisions/0109-temporal-primitives-in-the-entity-graph.md:49:    pub source_refs: Vec<SourceRef>,      // Points into provenance sources per ADR-0105
.docs/decisions/0109-temporal-primitives-in-the-entity-graph.md:120:Each `DataPoint<T>` carries `source_refs` pointing to the signals and entities that contributed to it. When an ability composes a trajectory and uses it in synthesis, its provenance includes the trajectory's source references transitively via `SourceRef::Child` per [ADR-0105](0105-provenance-as-first-class-output.md) §5.
.docs/decisions/0109-temporal-primitives-in-the-entity-graph.md:148:3. **Explicit temporal intelligence.** The product's "continuity over time" promise has a concrete substrate, not just verbal claims.
.docs/decisions/0109-temporal-primitives-in-the-entity-graph.md:150:5. **Source-attributable.** Each data point carries source refs, integrating cleanly with provenance.
.docs/decisions/0109-temporal-primitives-in-the-entity-graph.md:171:- [ADR-0102: Abilities as the Runtime Contract](0102-abilities-as-runtime-contract.md) — `get_entity_context` consumes trajectory bundle.
.docs/decisions/0109-temporal-primitives-in-the-entity-graph.md:172:- [ADR-0105: Provenance as First-Class Output](0105-provenance-as-first-class-output.md) — `DataPoint.source_refs` integrate with provenance.
.docs/decisions/0126-memory-substrate-invariants.md:10:The v1.4.0 claim substrate (DOS-7 `intelligence_claims`, DOS-5 Trust Compiler, DOS-10 freshness decay, DOS-211 Provenance, DOS-212 DataSource taxonomy, DOS-213 prompt fingerprinting, DOS-263 invalidation_jobs, DOS-265 claim_edges, DOS-280 canonicalization) is large, interlocking, and built by multiple contributors over multiple releases. Without a unifying set of invariants, each contributor independently makes small decisions that drift apart over time. The result: two code paths that could in principle do the same thing end up doing subtly different things, ghost-resurrection bugs re-emerge after they were fixed, and every new feature ends up re-solving the "how do I store this?" problem.
.docs/decisions/0126-memory-substrate-invariants.md:12:A memory substrate that CSMs trust over many quarters needs a few structural guarantees that no single ticket enforces on its own. This ADR lists them so future tickets can point here instead of re-asserting them piecemeal.
.docs/decisions/0126-memory-substrate-invariants.md:14:The spirit is also aspirational: we looked at neuroscience's memory mechanisms (Hebbian strengthening, synaptic pruning, offline consolidation, schema-based assimilation, reconsolidation on retrieval) as a source of structural ideas. We adopt the mechanisms. We explicitly reject the failure modes the brain has because of its biological constraints — confabulation, irreversible forgetting, retrieval-induced distortion, schema-suppression of anomalies, one-winner contradiction resolution. Software has different constraints and can do better. This ADR locks down which side of that line the substrate sits on.
.docs/decisions/0126-memory-substrate-invariants.md:18:The memory substrate is governed by the following invariants. Each is enforced in code (CI lint or test) where possible. Each feature ticket either honors these invariants or proposes an amendment to this ADR.
.docs/decisions/0126-memory-substrate-invariants.md:46:Passive user engagement (view, dwell, skip) produces events that affect ranking and surfacing. It does not modify trust, does not rewrite claim text, does not alter provenance.
.docs/decisions/0126-memory-substrate-invariants.md:54:When a new signal fits an existing claim schema (ADR-0125 claim type registry), it assimilates: strengthens corroborations, bumps freshness, does not produce a new surface notification.
.docs/decisions/0126-memory-substrate-invariants.md:56:When a new signal does NOT fit the schema (missing required claim, stale past typical freshness, unexpected presence, cadence break, out-of-range value), it surfaces as a deviation. Deviations are the foreground of the briefing, not the background.
.docs/decisions/0126-memory-substrate-invariants.md:64:When a new signal directly contradicts an existing claim (same subject + claim_type, incompatible value, existing aggregate strength ≥ 0.4), the substrate creates a `claim_contradictions` row with `branch_kind='contradiction'`. Both claims stay active and queryable. Neither is authoritative for default display until reconciled.
.docs/decisions/0126-memory-substrate-invariants.md:83:Budgets, if used, are per claim_type + sensitivity + surface. A critical-sensitivity health claim does not compete for the same attention slot as a routine adoption-metric. The claim type registry (ADR-0125) carries the per-type budget policy, not a global constant.
.docs/decisions/0126-memory-substrate-invariants.md:96:Drift in terminology across tickets is drift in implementation. The substrate uses a fixed vocabulary:
.docs/decisions/0126-memory-substrate-invariants.md:103:- **signal** — runtime event per ADR-0115 emitted on substrate writes
.docs/decisions/0126-memory-substrate-invariants.md:119:- **Hard-coded thresholds (`HOMEOSTATIC_BUDGET=100`, `DORMANT_THRESHOLD=0.15`, `STALE_WINDOW=180`).** Policy lives in the ADR-0125 claim type registry or role preset config (DOS-178), never magic numbers in code.
.docs/decisions/0126-memory-substrate-invariants.md:130:- The substrate accumulates structural properties over releases instead of accumulating exceptions.
.docs/decisions/0126-memory-substrate-invariants.md:133:- New features that would be easy to slam into a single column on `intelligence_claims` have to route through the right substrate layer. Some features will take slightly longer to ship as a result; the tradeoff is that the substrate doesn't decay over time.
.docs/decisions/0126-memory-substrate-invariants.md:138:- Claim types + registry: ADR-0125.
.docs/decisions/0055-schedule-first-dashboard-layout.md:9:ADR-0053 shipped the readiness strip, sidebar flip, greeting removal, and maxVisible improvements. But the daily briefing page still buries the meeting schedule behind 440-560px of preamble (date heading, focus callout, summary card, readiness strip, intelligence card). On a 13" MacBook, the schedule can be completely below the fold on a loaded day.
.docs/decisions/0055-schedule-first-dashboard-layout.md:15:Restructure the daily briefing page from a sequential top-to-bottom flow to a two-column layout:
.docs/decisions/0057-entity-intelligence-architecture.md:12:This is the same disease the weekly page had before ADR-0052: a dashboard when it should be a briefing. The account detail page is a Grafana panel. It should be an intelligence document.
.docs/decisions/0057-entity-intelligence-architecture.md:21:- **Briefings consume entity intelligence.** The daily briefing, weekly briefing, and meeting prep are rendering surfaces. They should pull from per-entity intelligence, not generate their own. Cross-entity synthesis ("your busiest account this week is Nexus Analytics, but CloudCo's renewal is closer") is the briefing layer's job.
.docs/decisions/0057-entity-intelligence-architecture.md:199:The daily and weekly briefings remain separate enrichment surfaces but consume per-entity intelligence as input:
.docs/decisions/0057-entity-intelligence-architecture.md:201:- **Daily briefing**: Pulls intelligence.json for entities with meetings today. Uses executive assessments and nextMeetingReadiness for meeting prep context. Adds cross-entity synthesis: "Your three meetings today span two accounts — Nexus Analytics and CloudCo — both in renewal window."
.docs/decisions/0057-entity-intelligence-architecture.md:202:- **Weekly briefing**: Pulls intelligence.json for all active entities. Uses risks, wins, and readiness to compose the weekly narrative. Adds cross-entity synthesis: "Your busiest account this week is Nexus Analytics, but CloudCo's renewal is closer."
.docs/decisions/0057-entity-intelligence-architecture.md:231:The page layout follows the weekly briefing patterns (ADR-0052 research): conclusions before evidence, tapering word count, "why now?" framing. This layout applies to all entity types. The example below uses accounts; projects follow the same structure with project-specific metrics (milestone progress, blockers, target date countdown instead of renewal countdown).
.docs/decisions/0057-entity-intelligence-architecture.md:267:| **Retrieve** | Detail page render, briefing enrichment, meeting prep | Read from intelligence.json + SQLite |
.docs/decisions/0057-entity-intelligence-architecture.md:294:- Chose entity-scoped intelligence consumed by briefings over briefing-scoped intelligence — single source of truth, but briefings need their own synthesis layer for cross-entity narrative. This is the right boundary.
.docs/decisions/0031-actions-source-of-truth.md:15:| actions.json | Daily snapshot for dashboard. Read-only. | Ephemeral — regenerated each briefing |
.docs/decisions/0031-actions-source-of-truth.md:20:Action completion flow: user clicks checkbox → SQLite `complete_action()` sets status + timestamp → `upsert_action_if_not_completed()` prevents re-briefing from overwriting user completions.
.docs/decisions/0031-actions-source-of-truth.md:29:- Actions persist across briefing cycles — completion status survives re-briefing
.docs/decisions/0031-actions-source-of-truth.md:32:- Open gaps: markdown writeback hooks, cross-briefing deduplication in `prepare_today.py`, manual action creation from UI
.docs/decisions/0077-magazine-layout-editorial-redesign.md:13:The DailyOS app has achieved product-market fit with current functionality (briefing, actions, calendar, intelligence enrichment). However, the visual language and layout patterns feel instrumental rather than editorial. Users interact with dashboards and card-based layouts — tools, not publications.
.docs/decisions/0077-magazine-layout-editorial-redesign.md:48:- **Asterisk watermark:** 420px Montserrat 800, rotated 12°, positioned behind hero content at -z-1. Opacity ~7%. Page-specific color: turmeric (briefing/account/meeting), terracotta (actions), larkspur (weekly).
.docs/decisions/0077-magazine-layout-editorial-redesign.md:73:- Padding: 80px top (hero), 72px bottom (briefing pages), 56px between sections
.docs/decisions/0077-magazine-layout-editorial-redesign.md:91:The actions list groups by when (overdue → today → this week → upcoming), not by who. The daily briefing features the most important meeting. The weekly forecast organizes by day-of-week and theme, not just listing all events.
.docs/decisions/0077-magazine-layout-editorial-redesign.md:97:Every page has an explicit end. Daily briefing ends with `* * *` and "You're briefed. Go get it." Weekly forecast ends with `* * *` and "Plan your week." Actions list ends with a status footer. No infinite scroll. When you've read it, you know.
.docs/decisions/0077-magazine-layout-editorial-redesign.md:193:**No schema changes required.** Existing SQLite tables, briefing JSON artifacts, and entity intelligence files work as-is.
.docs/decisions/0077-magazine-layout-editorial-redesign.md:201:Switching from dashboards to editorial layout changes how users consume information. Less "I need to scan this board for the one important thing" and more "I'm reading a briefing that prioritizes for me."
.docs/decisions/0077-magazine-layout-editorial-redesign.md:204:No new capabilities. All existing workflows (briefing delivery, action tracking, meeting prep) work exactly the same. The UI is a reskin that honors the brand.
.docs/decisions/0077-magazine-layout-editorial-redesign.md:239:- `/Users/jamesgiroux/Desktop/dailyos-magazine-layout.html` (Daily briefing)
.docs/decisions/README.md:75:| [0052](0052-week-page-redesign.md) | Week page redesign — consumption-first weekly briefing | Accepted |
.docs/decisions/README.md:124:| [0105](0105-provenance-as-first-class-output.md) | Provenance as first-class output (v1.4.0 trust/attribution/reproducibility) | Proposed |
.docs/decisions/README.md:127:| [0108](0108-provenance-rendering-and-privacy.md) | Provenance rendering, per-surface rules, and privacy (v1.4.0) | Proposed |
.docs/decisions/README.md:131:| [0112](0112-migration-strategy-parallel-run-and-cutover.md) | Migration strategy — parallel run, cutover, substrate migrations (v1.4.0) | Proposed |
.docs/decisions/README.md:132:| [0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md) | Human and agent analysis as first-class claim sources (v1.4.0 substrate / v1.5.0 surfaces) | Proposed |
.docs/decisions/README.md:143:| [0124](0124-longitudinal-topic-threading.md) | Longitudinal topic threading — substrate allowance and deferred decision | Accepted |
.docs/decisions/README.md:145:| [0126](0126-memory-substrate-invariants.md) | Memory substrate invariants | Proposed |
.docs/decisions/0124-longitudinal-topic-threading.md:3:**Status:** Accepted (substrate allowance for v1.4.0 spine; design decision deferred to v1.4.2)
.docs/decisions/0124-longitudinal-topic-threading.md:5:**Target:** v1.4.0 substrate (schema + envelope allowance) / v1.4.2 (decision + implementation)
.docs/decisions/0124-longitudinal-topic-threading.md:6:**Extends:** [ADR-0105](0105-provenance-as-first-class-output.md) (SubjectAttribution amendment), [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md) (claims)
.docs/decisions/0124-longitudinal-topic-threading.md:7:**Consumed by:** v1.4.3 briefing prep, v1.5.x recommendations layer
.docs/decisions/0124-longitudinal-topic-threading.md:13:The substrate as drafted has:
.docs/decisions/0124-longitudinal-topic-threading.md:16:- Multi-subject claims spanning Account / Project / Person / Meeting ([ADR-0105](0105-provenance-as-first-class-output.md) SubjectAttribution amendment).
.docs/decisions/0124-longitudinal-topic-threading.md:21:Two solution shapes are plausible (detailed in §3 below). The decision between them needs design work and pilot data that is not v1.4.0 spine scope. But choosing later is expensive if the substrate has no place to put a thread reference. This ADR resolves that mismatch: ship the substrate allowance now, defer the meaning to v1.4.2.
.docs/decisions/0124-longitudinal-topic-threading.md:25:### 1. v1.4.0 substrate allowance (spine)
.docs/decisions/0124-longitudinal-topic-threading.md:47:- No retrieval logic. No UI. No thread creation. The substrate ships the field; v1.4.2 ships the meaning.
.docs/decisions/0124-longitudinal-topic-threading.md:48:- Provenance envelope `provenance_schema_version` stays at `1` — the new field is additive, optional, default-empty (forward-compatible per ADR-0105 §1).
.docs/decisions/0124-longitudinal-topic-threading.md:64:**Option C — hybrid.** Auto-generated `thread_id`s (Option B) AND user-named themes (Option A) populate the same field; themes are an optional layer on top of the thread substrate.
.docs/decisions/0124-longitudinal-topic-threading.md:77:The v1.4.0 single-`thread_id` column is sufficient for the substrate allowance. If v1.4.2 chooses Option C or discovers that claims legitimately participate in multiple threads (which the Provenance envelope's `thread_ids: Vec<ThreadId>` already supports), v1.4.2 adds:
.docs/decisions/0124-longitudinal-topic-threading.md:102:Per project memory `recommendations_layer_vision.md`, post-v1.4.x recommendation claims attach to threads when applicable. A recommendation on the consolidation thread can reference 8 months of prior reasoning rather than the latest email alone. No further substrate ADR needed; the allowance from §1 is the foundation.
.docs/decisions/0124-longitudinal-topic-threading.md:127:- `RecommendationClaim` (v1.5.x) can reference threads on the same substrate without further allowance.
.docs/decisions/0124-longitudinal-topic-threading.md:132:- Two opaque-id types (`SubjectRef` and `ThreadId`) increase substrate cognitive load slightly. Mitigated by clear naming and the fact that abilities ignore `thread_id` until v1.4.2.
.docs/decisions/0124-longitudinal-topic-threading.md:141:- [ADR-0105: Provenance as First-Class Output](0105-provenance-as-first-class-output.md) — `SubjectRef::Multi` is what enables a single claim to span the subjects a thread typically does.
.docs/decisions/0094-audit-log-and-enterprise-observability.md:6:**Relates to:** ADR-0092 (Data Security), ADR-0093 (Prompt Injection Hardening)
.docs/decisions/0094-audit-log-and-enterprise-observability.md:13:2. **`enrichment_log` SQLite table** -- tracks enrichment events per entity with source, fields updated, and raw payload. Good for data provenance.
.docs/decisions/0094-audit-log-and-enterprise-observability.md:213:**`enrichment_log` SQLite table:** Retained for data provenance queries (what fields were updated by which source, when). The audit log's `ai_operation` events complement it with timing and validation outcomes. The SQLite tables are mutable and not AU-9 compliant -- they are for operational queries, not compliance audit.
.docs/decisions/0052-week-page-redesign.md:1:# ADR-0052: Week page redesign — consumption-first weekly briefing
.docs/decisions/0052-week-page-redesign.md:16:Weekly prep answers different questions than the daily briefing:
.docs/decisions/0052-week-page-redesign.md:27:### 1. Replace the calendar grid with a weekly briefing
.docs/decisions/0052-week-page-redesign.md:29:The Week page becomes a consumption-first briefing, not a calendar view. Six sections in priority order:
.docs/decisions/0052-week-page-redesign.md:58:| **Suggestions only** (default) | Week briefing shows available time blocks with suggested tasks. No calendar writes. |
.docs/decisions/0052-week-page-redesign.md:160:2. **I201** — Live proactive suggestions via query-layer pattern from ADR-0062 (no briefing artifact rewrites).
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:7:**Relates to:** [ADR-0027](0027-mcp-dual-mode.md), [ADR-0083](0083-product-vocabulary.md), [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0111](0111-surface-independent-ability-invocation.md), [ADR-0118](0118-dailyos-as-ai-harness-principles-and-residual-gaps.md), [ADR-0127](0127-presets-as-intelligence-contracts.md), [ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md)
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:8:**Linear:** [v1.4.1 — Abilities Runtime Completion](https://linear.app/a8c/project/v141-abilities-runtime-completion-19eb22af50e7), [v1.4.2 — Entity Intelligence](https://linear.app/a8c/project/v142-entity-intelligence-accounts-projects-people-a448d2e072ad), [v1.4.7 — MCP Server v2](https://linear.app/a8c/project/v147-mcp-server-v2-abilities-first-6e12027c36c9), [v1.4.9 — Self-Healing v2](https://linear.app/a8c/project/v149-self-healing-v2-da4bc5769164), [DOS-540](https://linear.app/a8c/issue/DOS-540)
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:12:[ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md) established the architectural frame: *the substrate is the product; the heads are surfaces over it.* It named two heads — the Tauri app (visual, opinionated, daily ritual) and the MCP server (responsive, headless, on-demand) — and explicitly anticipated more: *"Adding a new surface (CLI, mobile, browser extension, future agent shell) follows the same pattern."*
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:18:- v1.4.0 (Abilities Runtime Spine) shipped. v1.4.1 (Abilities Runtime Completion) is in flight and finishing the substrate.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:20:- The current primary surface is a Tauri (Rust + React) macOS app rendering a magazine reading surface. The substrate work in v1.4.0/v1.4.1 is largely surface-agnostic; the substantial surface investment is still ahead, not behind.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:34:The economic argument is the load-bearing one. v1.4.2 onward will spend significant design and engineering effort on entity surfaces, briefing experience, claim/trust UI, salience surfaces, and reports rendering. **Those redesign tokens are spent once.** Spending them on a Tauri React magazine surface and re-spending them on a WordPress block surface six to twelve months later is paying twice. Spending them on the WordPress block surface from the start is paying once.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:38:### Why this ADR (vs. ADR-0128's existing framing)
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:40:ADR-0128 framed the architecture as surface-pluggable. It treated the Tauri app as the primary visual surface and MCP as a co-equal headless head. This ADR commits to the next step in that architecture: **WordPress Studio is added as a new head and is promoted to primary**, with the Tauri app reorienting toward runtime-host duties and the MCP head elevating from a late-program feature to the foundation contract every v1.4.2+ wave consumes.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:42:This ADR does not contradict ADR-0128. It is the operationalization of ADR-0128's architectural thesis at the moment the substrate has matured enough to make pluggable surfaces real.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:48:The product is the personal-intelligence runtime: substrate (claims, trust, lifecycle, signals, provenance), abilities runtime, agent-backend mediation, local-first data sovereignty. Surfaces are clients of the runtime. No surface is the product; no surface is sacred.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:50:This generalizes ADR-0128 §1 from *"the substrate is the product; the heads are surfaces over it"* to: *"the runtime is the product. The substrate is its memory; abilities are its capability surface; surfaces are how users compose against it."*
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:59:- A custom block library that renders substrate types (entity blocks, claim blocks with trust-band rendering, briefing blocks, callout blocks, prep blocks, report blocks, agentic blocks).
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:61:- Custom post types for the entity model (Account, Project, Person, Meeting, Briefing, Report) as projections over the substrate.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:62:- A plugin that mediates the substrate-to-WordPress integration: read APIs, write APIs (corrections / dismissals / corroborations), event subscriptions, identity bridge.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:63:- Composition templates for daily briefings, account pages, meeting prep, and report drafts — anchored shapes the user can deviate from. Layouts may also be **agent-composed on the fly** based on substrate salience, not bound to fixed templates.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:65:Local-first holds. Privacy holds. Open-format holds. The substrate remains in user files; WordPress is the composition layer, not the storage of record.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:67:### 3. The substrate-to-surface contract is foundation work, not a late-program feature
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:69:The contract between the substrate and any surface — typed read API, typed write API, event subscription, composition primitives, identity and scope — is the load-bearing layer for every v1.4.2+ wave. It cannot be a late-program feature.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:71:This ADR reorders the program: the substrate-to-surface contract (currently scoped inside v1.4.7) is **promoted to a foundation wave between v1.4.1 and v1.4.2**. v1.4.7's MCP server work continues as the first external implementation of the contract; the contract itself ships earlier so that v1.4.2 entity surfaces, v1.4.3 briefing surfaces, v1.4.4 claim/trust surfaces, and v1.4.5 salience surfaces all consume the same substrate-to-surface API.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:73:### 4. WordPress accesses the substrate via its own Abilities API + MCP Adapter; non-WP surfaces use the Rust runtime's MCP server directly
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:77:The runtime exposes substrate-backed abilities through two paths, both consuming the same substrate:
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:80:- **Non-WP path.** The Rust runtime exposes its own MCP server for surfaces that don't sit inside WordPress (Claude Desktop direct without a WP install, Cursor, future PWA, future mobile). Same substrate, two MCP servers — one WP-mediated for WP-consuming clients, one direct for everything else.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:84:This preserves [ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md) §7's discipline (single ingress to the abilities runtime) — every external invocation, whether routed through the WP MCP server or the runtime's direct MCP server, ultimately resolves to one substrate access path inside the runtime. The runtime exposes one substrate access protocol; clients vary in how they render and compose against it.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:86:**MCP Adapter exposure policy.** Substrate-backed abilities must NOT be exposed by the *default* WP MCP server. DailyOS ships a custom MCP server configuration with an explicit ability allowlist, a dedicated low-capability WP user for substrate access, and read-mostly defaults. Permission callbacks check both WP capabilities and DailyOS `SurfaceClient(WordPress)` scopes (per [ADR-0111](0111-surface-independent-ability-invocation.md)); site-wide WP capabilities alone are not the isolation boundary.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:93:- **Paid.** Hosted multi-tenant substrate. Shared surfaces across teams. Enterprise performance characteristics (hosted vector DB, knowledge-graph-shaped reads). Optional DailyOS-hosted agent backend for users who don't want to manage one. Multisite-shaped team boundaries. The WordPress VIP playbook applied to personal-becoming-team intelligence.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:103:- **Templated layouts.** Curated page templates (daily briefing, account page, meeting prep, report) that the user can deviate from. Anchored shapes.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:104:- **On-the-fly composition.** The agent composes block sets at runtime based on substrate salience. Today's account page may differ from yesterday's because what's salient changed. The user can pin templates as anchors but the system isn't bound to them.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:107:This is the rendering posture the HTML-over-markdown trajectory points toward, and it composes naturally with the typed substrate the v1.4.x program has been building.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:116:The Rust substrate engine, abilities runtime, signal propagation, claim feedback path, and MCP server all continue exactly as v1.4.0/v1.4.1 designed them. None of that work is wasted. The investment has always been in the runtime; this ADR makes that explicit.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:124:This composes with §4: agents reach the substrate via MCP; surfaces reach the substrate via MCP; agent backend is pluggable; surface is pluggable; runtime is the constant.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:130:- **v1.4.1** — Abilities Runtime Completion. Finishes as planned. Surface-agnostic substrate work.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:131:- **(New foundation wave between v1.4.1 and v1.4.2)** — substrate-to-surface contract. Formerly part of v1.4.7's scope. Typed read/write/subscribe APIs, composition primitives, identity bridge. MCP server is the first external implementation.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:133:- **v1.4.3** — Briefing Experience, WP-shaped. Composable briefing as a Gutenberg page template / on-the-fly composition.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:134:- **v1.4.4** — Claim Experience & Trust UI, WP-shaped. Block-level claim primitives with trust-band rendering; corrections feed substrate via the contract.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:137:- **v1.4.7** — MCP Server v2. Repurposed: the contract foundation has shipped earlier; v1.4.7 now focuses on the rich tool surface, displacement-test discipline (per ADR-0128), and the cross-surface story.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:149:- **Instant-launch feel.** A WordPress Studio briefing render with realistic claim volume — does it hit the *"open the app, your day is ready"* feel? Cold-load < 2s, warm-load < 500ms, with appropriate caching and pre-rendering. Empirical answer beats armchair.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:150:- **Markdown filesystem ↔ WordPress DB write path.** Gutenberg block save → substrate event → markdown projection. Does the three-view consistency model (substrate of record, WP DB as projection, filesystem as durable archive) hold up under realistic editing patterns?
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:156:- **Not abandoning the Tauri runtime.** The Rust substrate, abilities runtime, signal propagation, MCP server, cron, and local privacy gate continue exactly as v1.4.0/v1.4.1 design. The runtime is the product.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:159:- **Not committing to multi-tenant hosted substrate as v1.4.x scope.** Paid-tier hosting is a future architectural commitment, not v1.4.x work. The free-tier capability proves the substrate; the paid-tier engineering follows once free-tier customers exist.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:160:- **Not changing local-first, privacy, BYOM, or the substrate's typed model.** Trust as substrate property, claims with provenance and lifecycle, personal-not-organizational — all preserved.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:161:- **Not deprecating ADR-0128.** This ADR extends ADR-0128's surface-pluggable thesis with a concrete primary-surface choice. The headless / co-equal-heads frame from ADR-0128 stays canonical.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:169:- **Block composability is the right rendering substrate for AI-generated, user-editable intelligence.** Gutenberg blocks bridge generation and editing in a way React components do not.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:170:- **Significant infrastructure leverage from WordPress.** REST and GraphQL APIs satisfy the substrate-to-surface contract on the read side without bespoke transport. Remote Data Blocks ship the "block reads from external source with auth" pattern. Multisite gives team boundaries for paid tier. WP.com sync gives shareable-output publishing. VIP infrastructure gives the enterprise-hosting playbook.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:171:- **Cross-surface story becomes coherent.** WordPress for primary composition; Claude Desktop and future agent shells for headless conversation; substrate sovereign. Each surface picks the questions and ritual it serves best, per ADR-0128 §1.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:188:- **No substrate code changes.** The runtime continues as designed. Surface work is additive (new WP integration plugin/theme) and reorientative (v1.4.2+ targets a different render stack).
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:189:- **ADR-0128 is preserved and extended.** The surface-pluggable thesis stays canonical; this ADR commits to the next step.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:190:- **The wave program structure stays similar.** Wave count, scope shape, and substrate trajectory are mostly unchanged. Implementation tech changes for surface-touching waves.
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:199:- [ADR-0102](0102-abilities-as-runtime-contract.md) — Abilities as runtime contract
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:200:- [ADR-0105](0105-provenance-as-first-class-output.md) — Provenance as first-class output
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:204:- [ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md) — Headless DailyOS: MCP as a co-equal product surface (this ADR extends)
.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md:206:- [v1.4.1 Linear project](https://linear.app/a8c/project/v141-abilities-runtime-completion-19eb22af50e7)
.docs/decisions/0129-commitment-claim-identity.md:17:`abilities-runtime::abilities::extractors::commitment`.
.docs/decisions/0073-editorial-design-language.md:9:DailyOS v0.7.x uses the right color palette (cream, charcoal, gold, sage, peach) and the right philosophical foundations (briefing > dashboard, consumption over production, conclusions before evidence). But the visual execution doesn't live up to the intent. The current UI is:
.docs/decisions/0073-editorial-design-language.md:12:- **Generic.** Could be any SaaS dashboard. Nothing about the visual design says "this is a personal briefing tool, not a database interface."
.docs/decisions/0073-editorial-design-language.md:21:### Design Principle: "A beautifully typeset daily briefing laid on a warm desk."
.docs/decisions/0073-editorial-design-language.md:59:Space is not wasted — it's the primary differentiator between "productivity dashboard" and "personal briefing."
.docs/decisions/0073-editorial-design-language.md:79:- Priority items in the weekly briefing (numbered, featured)
.docs/decisions/0073-editorial-design-language.md:110:Every briefing page (daily, weekly, intelligence report) ends with an explicit terminal marker:
.docs/decisions/0073-editorial-design-language.md:113:- "You're briefed. Go get it." / "End of weekly briefing." / "Intelligence current as of today."
.docs/decisions/0073-editorial-design-language.md:116:This reinforces Principle 1 (Zero-Guilt) and Principle 7 (Consumption) — the briefing is finite, not an infinite feed.
.docs/decisions/0073-editorial-design-language.md:146:- Finite briefing pattern reinforces zero-guilt philosophy
.docs/decisions/0073-editorial-design-language.md:161:- Sprint 27: I221 (Focus page), I222 (Weekly briefing), I223 (Entity lists), I224 (Entity details)
.docs/decisions/0073-editorial-design-language.md:162:- Dashboard (daily briefing) — already partially aligned via ADR-0055, Sprint 27 refines
.docs/decisions/0066-meeting-intelligence-package.md:163:| Daily briefing runs | Prep enriched, `_today/data/preps/*.json` created | Disk (ephemeral) |
.docs/decisions/0066-meeting-intelligence-package.md:224:- **Lifecycle starts at detection, not day-of.** This ADR's persistence lifecycle shows "Daily briefing runs → Prep enriched." ADR-0081 shifts this: intelligence enriches when the meeting is first detected on the calendar (via weekly run or calendar polling), not on the day of the meeting. The `MeetingIntelligence` record exists days to weeks before the meeting.
.docs/decisions/0120-observability-contract.md:5:**Target:** v1.4.0 (must land before any ability substrate ships)
.docs/decisions/0120-observability-contract.md:6:**Extends:** [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0104](0104-execution-mode-and-mode-aware-services.md), [ADR-0105](0105-provenance-as-first-class-output.md)
.docs/decisions/0120-observability-contract.md:7:**Related:** [ADR-0106](0106-prompt-fingerprinting-and-provider-interface.md), [ADR-0110](0110-evaluation-harness-for-abilities.md), [ADR-0108](0108-provenance-rendering-and-privacy.md), [ADR-0119](0119-runtime-evaluator-pass-for-transform-abilities.md)
.docs/decisions/0120-observability-contract.md:13:Twenty v1.4.0 substrate ADRs land claims, provenance, trust scoring, signal propagation, and the runtime evaluator — all of which produce telemetry in some form. Each ADR defines its own telemetry locally: [ADR-0105](0105-provenance-as-first-class-output.md) carries the `Provenance` envelope; [ADR-0106](0106-prompt-fingerprinting-and-provider-interface.md) produces `PromptFingerprint`; [ADR-0110](0110-evaluation-harness-for-abilities.md) generates eval run data; [ADR-0119](0119-runtime-evaluator-pass-for-transform-abilities.md) writes `evaluation_traces`.
.docs/decisions/0120-observability-contract.md:15:What does not exist is a **cross-cutting contract** that every ability, every service function, and every background worker emits uniformly. The aggregate effect: when something goes wrong in production, an engineer (or Claude Code session) debugging the issue has to stitch together provenance envelopes, ad-hoc `println!`-style logs, `signal_events` rows, and maybe a `tracing` span here and there — every ADR made its own choice and no contract ties them together.
.docs/decisions/0120-observability-contract.md:21:- Correlation across an invocation — `invocation_id` exists in provenance but doesn't thread through log lines, signal events, or evaluation traces — is manual.
.docs/decisions/0120-observability-contract.md:22:- Retrofitting observability after substrate lands is an order of magnitude more expensive than baking it in now.
.docs/decisions/0120-observability-contract.md:91:- Provenance envelope ([ADR-0105](0105-provenance-as-first-class-output.md)) already carries `invocation_id`. This ADR reaffirms it as the stable correlation key.
.docs/decisions/0120-observability-contract.md:128:The observability contract honors [ADR-0108](0108-provenance-rendering-and-privacy.md) masking rules. Specifically:
.docs/decisions/0120-observability-contract.md:135:The distinction: logs carry **the shape of what happened**, not **the content of what happened**. Content lives in the encrypted database with the provenance envelopes and `evaluation_traces`. Debug flows pull content at display time through [ADR-0108](0108-provenance-rendering-and-privacy.md) renderer, not by reading raw logs.
.docs/decisions/0120-observability-contract.md:232:- **Correlation across substrate is structural.** "Show me everything caused by invocation X" is a single SQL query across claims + signals + eval traces.
.docs/decisions/0120-observability-contract.md:233:- **Retrofitting avoided.** Adding this after substrate lands is expensive; adding it alongside is cheap.
.docs/decisions/0120-observability-contract.md:240:- **`#[instrument]` is boilerplate.** Every ability, every service mutation, every worker. Mitigated by the `#[ability]` macro ([ADR-0102](0102-abilities-as-runtime-contract.md) §7) expanding to include it; service functions can adopt `#[instrument(skip(ctx))]` by convention.
.docs/decisions/0120-observability-contract.md:255:- The `#[ability]` macro from [ADR-0102](0102-abilities-as-runtime-contract.md) §7 expands to open the root span with `invocation_id` + `ability_name` + `ability_version`.
.docs/decisions/0105-provenance-as-first-class-output.md:6:**Extends:** [ADR-0102](0102-abilities-as-runtime-contract.md)  
.docs/decisions/0105-provenance-as-first-class-output.md:8:**Required by:** [ADR-0103](0103-maintenance-ability-safety-constraints.md) §8 (audit), [ADR-0102](0102-abilities-as-runtime-contract.md) §10 (trust boundary)  
.docs/decisions/0105-provenance-as-first-class-output.md:9:**Companions:** [ADR-0106](0106-prompt-fingerprinting-and-provider-interface.md) (prompt fingerprint details), [ADR-0107](0107-source-taxonomy-alignment.md) (`DataSource` taxonomy), [ADR-0108](0108-provenance-rendering-and-privacy.md) (surface rendering and safety), [ADR-0130](0130-surface-independent-composition-contract.md) §2 (block-level `ProvenanceRef` preserves the §8 lives-once invariant by referencing the canonical envelope rather than copying it)
.docs/decisions/0105-provenance-as-first-class-output.md:13:[ADR-0102](0102-abilities-as-runtime-contract.md) §9 Rule 5 declares that every ability output carries provenance via `AbilityOutput<T>`. §10 declares Transform outputs are untrusted for mutation authorization. §11.3 declares composition produces nested provenance trees. [ADR-0103](0103-maintenance-ability-safety-constraints.md) §8 requires maintenance audit records to carry provenance. All of these contracts depend on one shape: the `Provenance` envelope. ADR-0102 references it but does not specify it.
.docs/decisions/0105-provenance-as-first-class-output.md:15:This ADR defines the envelope shape, trust classification, and composition semantics. It deliberately does not define: prompt fingerprinting (see [ADR-0106](0106-prompt-fingerprinting-and-provider-interface.md)), the authoritative `DataSource` taxonomy (see [ADR-0107](0107-source-taxonomy-alignment.md)), or per-surface rendering rules (see [ADR-0108](0108-provenance-rendering-and-privacy.md)). Splitting these concerns keeps each ADR implementable atomically and testable in isolation.
.docs/decisions/0105-provenance-as-first-class-output.md:19:Every ability output carries a `Provenance` envelope via the `AbilityOutput<T>` wrapper from [ADR-0102](0102-abilities-as-runtime-contract.md) §6. The envelope captures identity, temporal context, trust classification, source attribution, composition tree, and field-level attribution.
.docs/decisions/0105-provenance-as-first-class-output.md:26:    pub provenance_schema_version: u32,
.docs/decisions/0105-provenance-as-first-class-output.md:31:    pub ability_schema_version: SchemaVersion,   // The ability's I/O schema version, per ADR-0102 §8
.docs/decisions/0105-provenance-as-first-class-output.md:51:    // Composition tree — child provenance from composed abilities
.docs/decisions/0105-provenance-as-first-class-output.md:70:`provenance_schema_version` is the schema version of this ADR's envelope structure. A consumer seeing a higher version must parse forward-compatibly — unknown fields are ignored; known fields retain their meaning. The initial version is `1`. Any breaking change requires a new version and a migration path.
.docs/decisions/0105-provenance-as-first-class-output.md:72:`ability_schema_version` is the ability's I/O schema version per [ADR-0102](0102-abilities-as-runtime-contract.md) §8 — a different concept, retained here for completeness so a consumer reading provenance knows exactly which ability contract produced the output.
.docs/decisions/0105-provenance-as-first-class-output.md:103:Trust is a structured record, not a collapsed nested enum. It encodes the weakest trust encountered, the mixing history, and whether the output is safe to authorize mutation on without a trust signal per [ADR-0102](0102-abilities-as-runtime-contract.md) §10.
.docs/decisions/0105-provenance-as-first-class-output.md:125:    /// policy, or schema-bounded value per ADR-0102 §10.
.docs/decisions/0105-provenance-as-first-class-output.md:158:**`effective` is computed by the provenance builder, not the author.** The author cannot set it; the registry rejects manually-constructed `Provenance` with mismatched `effective` and `contributions`.
.docs/decisions/0105-provenance-as-first-class-output.md:193:**`evidence_weight` is a contribution weight, not a reliability score.** It is author-supplied (by the ability's planner or synthesizer) and indicates how much this particular source mattered for this specific output. It is not a stored per-source reliability — those are defined by [ADR-0080](0080-signal-intelligence-architecture.md) and consumed separately. We deliberately do not duplicate ADR-0080's reliability into every provenance envelope.
.docs/decisions/0105-provenance-as-first-class-output.md:210:    pub explanation: Option<SanitizedExplanation>,  // Sanitization rules in ADR-0108
.docs/decisions/0105-provenance-as-first-class-output.md:245:When ability A invokes ability B, A's provenance includes B's as a nested `child`:
.docs/decisions/0105-provenance-as-first-class-output.md:264:6. Cycle guard: the registry rejects ability composition cycles at registration time per [ADR-0102](0102-abilities-as-runtime-contract.md) §11.1, so nested `Provenance` trees are guaranteed acyclic.
.docs/decisions/0105-provenance-as-first-class-output.md:266:**`SourceRef::Child(child_idx, field_path)` uses a stable `composition_id`.** To avoid the brittleness of positional `child_idx`, the `children` vector is indexed by a stable `CompositionId` per declared `composes` entry in the ability's metadata (per [ADR-0102](0102-abilities-as-runtime-contract.md) §7.1). The `child_idx` is the index into the `composes` list, not the runtime invocation order. A `SourceRef::Child(child_idx, field_path)` with a `child_idx` outside the declared `composes` list is a registration error.
.docs/decisions/0105-provenance-as-first-class-output.md:270:Contributors produce provenance via a builder that enforces the rules in §3 and §5:
.docs/decisions/0105-provenance-as-first-class-output.md:289:    let account_child_idx = prov.compose(&account_ctx.provenance);
.docs/decisions/0105-provenance-as-first-class-output.md:310:- Wraps the domain output in `AbilityOutput<T>` with the built provenance.
.docs/decisions/0105-provenance-as-first-class-output.md:317:1. **Maintenance audit table.** [ADR-0103](0103-maintenance-ability-safety-constraints.md) §8 `MaintenanceAuditRecord` adds a `provenance: Provenance` field. Source revocation triggers a masking pass: a `Provenance` whose sources reference a revoked `DataSource` is replaced with a `ProvenanceMasked` marker per [ADR-0108](0108-provenance-rendering-and-privacy.md). **Interaction with [ADR-0094](0094-audit-log-and-enterprise-observability.md):** ADR-0094's append-only JSONL security audit remains unchanged. The `maintenance_audit` SQLite table is a separate operational log for runtime diagnostics. Provenance content in the operational log is subject to masking; the JSONL security log is unchanged.
.docs/decisions/0105-provenance-as-first-class-output.md:319:2. **Outbox table.** [ADR-0103](0103-maintenance-ability-safety-constraints.md) §2 outbox entries add a `provenance: Provenance` field. Outbox provenance captures the ability's full output-time provenance, including the invocation that generated the pending external mutation.
.docs/decisions/0105-provenance-as-first-class-output.md:321:3. **Planned mutations.** [ADR-0104](0104-execution-mode-and-mode-aware-services.md) §3 `PlannedMutation` adds a `provenance_ref: InvocationId` field pointing into the ability's provenance envelope. Plans do not carry a full duplicated provenance; they reference the envelope by invocation ID and field path, preventing storage duplication.
.docs/decisions/0105-provenance-as-first-class-output.md:325:**Storage format.** Provenance is stored as JSON in a single column per containing record. Indexes on `invocation_id`, `ability_name`, `produced_at`, and `data_source` (via a normalized xref table) support the queries [ADR-0103](0103-maintenance-ability-safety-constraints.md) §8 requires. A full JSON-column approach is feasible at the six-user scale; if scale changes, a normalized `provenance` / `provenance_sources` split can be introduced without changing the envelope contract.
.docs/decisions/0105-provenance-as-first-class-output.md:329:A typical Read ability produces ~500B–5KB of provenance. Transform abilities with field attributions can reach ~50KB. Deeply composed Maintenance invocations with >8 composition levels or >50 sources can exceed 500KB, which is excessive for per-invocation storage.
.docs/decisions/0105-provenance-as-first-class-output.md:331:**Soft budget.** Abilities SHOULD keep provenance under 100KB. The registry warns at build time when provenance exceeds the soft budget.
.docs/decisions/0105-provenance-as-first-class-output.md:333:**Hard truncation.** When a provenance envelope exceeds 1MB at finalize time, composition subtrees are collapsed from deepest first into a `ProvenanceWarning::DepthElided` summary carrying aggregate source counts and trust classification. Truncation is a finalize-time concern; the full tree never leaves the ability in its raw form above the hard limit.
.docs/decisions/0105-provenance-as-first-class-output.md:340:2. **Trust boundary is enforceable and closes the laundering hole.** [ADR-0102](0102-abilities-as-runtime-contract.md) §10 rejects mutations authorized by `Untrusted` output at the registry layer, and `contains_stored_synthesis` catches the case where stored AI output is surfaced by a Read ability.
.docs/decisions/0105-provenance-as-first-class-output.md:342:4. **Composition is explicit.** Nested provenance makes composition visible in output, not hidden in call stacks, with a stable `composition_id` avoiding positional brittleness.
.docs/decisions/0105-provenance-as-first-class-output.md:343:5. **Source revocation can cascade correctly.** Provenance references `DataSource` via [ADR-0107](0107-source-taxonomy-alignment.md), and lifecycle behavior (purge vs. flag) is governed by that ADR. Masking shape is governed by [ADR-0108](0108-provenance-rendering-and-privacy.md).
.docs/decisions/0105-provenance-as-first-class-output.md:344:6. **Storage integration is explicit.** Maintenance audit, outbox, and plans each know exactly how they carry or reference provenance. Plans use references to avoid duplication.
.docs/decisions/0105-provenance-as-first-class-output.md:357:3. **Storage volume growth.** Provenance retained indefinitely alongside outputs accumulates. Mitigation: same retention as output; no separate provenance retention policy.
.docs/decisions/0105-provenance-as-first-class-output.md:358:4. **`composition_id` drift.** An ability's declared `composes` list changes between versions, invalidating stored `SourceRef::Child` references. Mitigation: registry's version-gating in [ADR-0102](0102-abilities-as-runtime-contract.md) §8 applies — major version bumps for breaking changes.
.docs/decisions/0105-provenance-as-first-class-output.md:360:6. **Forward-compatibility regressions.** A consumer written against `provenance_schema_version = 1` breaks when version `2` adds a required variant. Mitigation: breaking changes require a new `provenance_schema_version` and a deprecation window for consumers.
.docs/decisions/0105-provenance-as-first-class-output.md:364:- [ADR-0102: Abilities as the Runtime Contract](0102-abilities-as-runtime-contract.md) — Defines `AbilityOutput<T>` wrapper; §9 Rule 5 requires provenance; §10 consumes `TrustAssessment`; §11.3 requires composition merge.
.docs/decisions/0105-provenance-as-first-class-output.md:365:- [ADR-0103: Maintenance Ability Safety Constraints](0103-maintenance-ability-safety-constraints.md) — §8 maintenance audit records add a `provenance: Provenance` field per §8 of this ADR.
.docs/decisions/0105-provenance-as-first-class-output.md:366:- [ADR-0104: ExecutionMode and Mode-Aware Services](0104-execution-mode-and-mode-aware-services.md) — `mode` and clock injection for `produced_at`; `PlannedMutation` adds a `provenance_ref` field.
.docs/decisions/0105-provenance-as-first-class-output.md:367:- [ADR-0094: Audit Log and Enterprise Observability](0094-audit-log-and-enterprise-observability.md) — Coexistence: ADR-0094's append-only JSONL security audit is unchanged; this ADR's provenance lives in operational `maintenance_audit` SQLite table with masking.
.docs/decisions/0105-provenance-as-first-class-output.md:368:- [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0098](0098-data-governance-source-aware-lifecycle.md) — Trust classification and source lifecycle background.
.docs/decisions/0105-provenance-as-first-class-output.md:371:- **[ADR-0108: Provenance Rendering and Privacy](0108-provenance-rendering-and-privacy.md)** — Per-surface rendering rules, actor-filtered display, `ProvenanceMasked` shape, sanitization of `explanation` text, and P2 publication safety.
.docs/decisions/0105-provenance-as-first-class-output.md:372:- **ADR-0109 (forthcoming): Temporal Primitives in the Entity Graph** — Defines trajectory types referenced from provenance when an ability consumes trajectory inputs.
.docs/decisions/0105-provenance-as-first-class-output.md:374:- **ADR-0111 (forthcoming): Surface-Independent Ability Invocation** — References the rendering rules in [ADR-0108](0108-provenance-rendering-and-privacy.md).
.docs/decisions/0105-provenance-as-first-class-output.md:420:Surfaces (per ADR-0108) render `SourceTimestampUnknown` so the system can see when freshness is being approximated rather than measured.
.docs/decisions/0105-provenance-as-first-class-output.md:489:  - quarantined claims are queryable via `intelligence_claims WHERE provenance_warnings @> 'SourceTimestampImplausible'`
.docs/decisions/0115-signal-granularity-audit.md:7:**Related:** [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0104](0104-execution-mode-and-mode-aware-services.md), [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0110](0110-evaluation-harness-for-abilities.md), [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md), [ADR-0114](0114-scoring-unification.md)
.docs/decisions/0115-signal-granularity-audit.md:12:[ADR-0080](0080-signal-intelligence-architecture.md) established the signal bus as the backbone of DailyOS intelligence: events fire, listeners react, Bayesian weights update, briefings recompute. It was designed when signals operated at entity granularity — "this account changed," "this person's title moved." Three in-flight changes break that granularity model:
.docs/decisions/0115-signal-granularity-audit.md:15:2. **Ability-output events.** [ADR-0102](0102-abilities-as-runtime-contract.md) makes every capability a named, versioned ability producing an `AbilityOutput<T>`. Each output is cache-keyed and invalidation-tracked. "This output changed" is a new event class operating at ability-output granularity.
.docs/decisions/0115-signal-granularity-audit.md:21:- **Depth-limited drops.** The original invalidation design capped propagation depth and dropped signals past the cap to avoid runaway cascades. That is correctness-unsafe — a dropped invalidation leaves briefings, scores, and claims silently stale. The adversarial review flagged this.
.docs/decisions/0115-signal-granularity-audit.md:173:The event log (audit trail of every signal ever emitted) is separate from the invalidation queue (work to be done). The log is append-only, retains all signals including coalesced ones, and is used for replay, telemetry, and provenance. The queue is transient — jobs complete, fail, or dead-letter, and are purged after a configurable retention period (default 90 days).
.docs/decisions/0115-signal-granularity-audit.md:217:- [ADR-0080](0080-signal-intelligence-architecture.md) is not replaced; it is the substrate this ADR refines. The Bayesian weight-update semantics remain as specified there.
.docs/decisions/0115-signal-granularity-audit.md:317:Codex flagged: the ADR consumes an `OutputId` concept that doesn't exist in code yet ([ADR-0102](0102-abilities-as-runtime-contract.md)/[ADR-0105](0105-provenance-as-first-class-output.md) are also doc-only).
.docs/decisions/0115-signal-granularity-audit.md:319:**Revised:** `OutputId` is defined in this ADR as a dependency placeholder and implemented in the abilities runtime work ([ADR-0102](0102-abilities-as-runtime-contract.md) implementation). Shape:
.docs/decisions/0115-signal-granularity-audit.md:325:    pub input_key: Hash,  // Canonical hash of ability inputs + composing provenance
.docs/decisions/0115-signal-granularity-audit.md:329:This ADR's Phase 0 is blocked on the `AbilityOutput` + `Provenance` types landing from [ADR-0102](0102-abilities-as-runtime-contract.md) / [ADR-0105](0105-provenance-as-first-class-output.md) implementation. Document that dependency explicitly in [DOS-235](https://linear.app/a8c/issue/DOS-235)+.
.docs/decisions/0115-signal-granularity-audit.md:346:R1 established signal bus as fire-and-forget after `emit_signal`. For most flows this is correct — eventual consistency is the right model for an intelligence synthesis app. For **some** flows, it breaks the user-visible behavior. Specifically: a user corrects a claim, clicks "save," and expects the very next briefing to reflect that correction. If the invalidation that would re-render the briefing runs async after `commit_claim` returns, the UI may read stale state on the immediate subsequent render.
.docs/decisions/0115-signal-granularity-audit.md:361:- `PropagateSync { await_completion: true }` — emit the signal inline, write the invalidation job inline, **and wait for the invalidation job to complete** before returning from `emit_signal`. Used when the caller intends to render state that depends on the propagation having finished — the user-correction → next-briefing path is the canonical case.
.docs/decisions/0115-signal-granularity-audit.md:379:**Under `Evaluate` mode:** `await_completion: true` behaves the same as `false` because fixtures test intent, not timing. A fixture that asserts "after commit, briefing reflects correction" drives the invalidation explicitly via the test helper introduced in R1.7.
.docs/decisions/0115-signal-granularity-audit.md:381:**Test case:** integration test that commits a user correction with `PropagateSync { await_completion: true }` and immediately reads the affected briefing; assert the briefing reflects the correction without any explicit wait in test code.
.docs/decisions/0075-conversational-interface-architecture.md:11:DailyOS currently has no chat surface. The app is consumption-first (P7) — users read briefings, not query databases. But conversational exploration is complementary to consumption: the briefing tells you what you need to know proactively; chat lets you dig deeper reactively.
.docs/decisions/0075-conversational-interface-architecture.md:13:**Key tension:** Building an in-app chat panel is a significant UI investment that risks violating P7 (Consumption Over Production) if users spend more time chatting than reading briefings. But ignoring conversational interaction means losing a validated user need.
.docs/decisions/0075-conversational-interface-architecture.md:30:3. `chat_get_briefing()` — Returns today's briefing data. Answers "what's on my plate today?"
.docs/decisions/0075-conversational-interface-architecture.md:100:- **Complements briefings** — proactive (briefing) + reactive (chat) covers both user modes
.docs/decisions/0036-inbox-processing-in-phase-1.md:9:ADR-0015 deferred inbox processing to Phase 2 to reduce MVP scope. During Phase 1 implementation, inbox processing was built alongside the core briefing workflow because:
.docs/decisions/0130-surface-independent-composition-contract.md:5:**Amended:** 2026-05-10 — added explicit references to existing substrate retrieval primitives (ADR-0074 vector search, ADR-0078 embed model) in §2 (Salience computation) and §3 (Custom block fallback) to prevent composition-producing abilities from reinventing retrieval.
.docs/decisions/0130-surface-independent-composition-contract.md:6:**Amended:** 2026-05-10 — §2 `Block.provenance` retyped from `ProvenanceEnvelope` (envelope copy) to `ProvenanceRef` (compact reference into the canonical envelope on `AbilityOutput<Composition>`). Preserves the ADR-0105 "lives once" invariant and keeps block-heavy compositions under ADR-0108's 64KB serialized-provenance cap. Source: Phase 0 artifact `06-composition-provenance-ref.md`.
.docs/decisions/0130-surface-independent-composition-contract.md:7:**Amended:** 2026-05-10 — §3 fallback rewritten from embed-model nearest-known + "rendered as nearest known type" indicator to deterministic schema-bounded projection at JSON-Pointer granularity. Unknown payload fields are dropped, not displayed; `claim_refs` and `provenance_ref` are preserved; rendered block degrades to `needs_verification` and carries a non-dismissible banner. Source: Phase 0 artifact `07-custom-block-fallback-projection.md`.
.docs/decisions/0130-surface-independent-composition-contract.md:9:**Relates to:** [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0108](0108-provenance-rendering-and-privacy.md), [ADR-0111](0111-surface-independent-ability-invocation.md), [ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md), [ADR-0129](0129-composable-surfaces-wordpress-studio-as-primary-surface.md)
.docs/decisions/0130-surface-independent-composition-contract.md:14:DailyOS has converged on a runtime + composable surfaces architecture (ADR-0128 surface-pluggable thesis; ADR-0129 WordPress as a leading concrete surface; ADR-0111 §8 generalized SurfaceClient as the fourth actor class for third-party local surfaces). The substrate-side contracts are in place:
.docs/decisions/0130-surface-independent-composition-contract.md:16:- **Abilities** ([ADR-0102](0102-abilities-as-runtime-contract.md)) — typed runtime capabilities.
.docs/decisions/0130-surface-independent-composition-contract.md:17:- **Provenance** ([ADR-0105](0105-provenance-as-first-class-output.md)) — claim-shaped output with source attribution and trust bands.
.docs/decisions/0130-surface-independent-composition-contract.md:19:- **Rendering** ([ADR-0108](0108-provenance-rendering-and-privacy.md)) — provenance rendering per surface/actor.
.docs/decisions/0130-surface-independent-composition-contract.md:21:What's missing is the contract for *composition*: how a substrate-produced briefing, account page, prep pack, or report is described independently of any rendering technology. ADR-0129 §6 commits to Gutenberg blocks as "the right shape" for composed surfaces, but does not separate the *generic* shape (typed blocks composed into structured pages) from the *implementation* (Gutenberg). Without that separation, WordPress quietly becomes more than a surface — it becomes the document model. That cuts against ADR-0129 §1's own claim that no surface is sacred.
.docs/decisions/0130-surface-independent-composition-contract.md:27:### 1. The substrate owns composition
.docs/decisions/0130-surface-independent-composition-contract.md:29:Composed surfaces (briefings, entity pages, prep packs, reports, callouts, agentic-block compositions) are described by substrate primitives independent of any rendering technology. Surfaces render the composition; surfaces do not author it.
.docs/decisions/0130-surface-independent-composition-contract.md:31:This generalizes [ADR-0111](0111-surface-independent-ability-invocation.md) §1 from "surfaces invoke abilities" to "surfaces consume substrate output, including composed pages." It generalizes [ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md) §1 from "the substrate is the product; the heads are surfaces over it" to apply at the *page* level, not just the *claim* level.
.docs/decisions/0130-surface-independent-composition-contract.md:33:#### Consumed substrate primitives
.docs/decisions/0130-surface-independent-composition-contract.md:35:The Composition contract is a producer/consumer seam on top of existing v1.4.0/v1.4.1 substrate. Composition-producing abilities and surface renderers CONSUME the following primitives; they do not reinvent them:
.docs/decisions/0130-surface-independent-composition-contract.md:37:- **Retrieval and salience** — Hybrid vector search ([ADR-0074](0074-vector-search-entity-content.md)) over the embed model ([ADR-0078](0078-nomic-embed-text-model-switch.md)). The substrate owns the relevance question. The Composition contract names `Salience.weight` as a producer field; the score itself flows from the retrieval primitives.
.docs/decisions/0130-surface-independent-composition-contract.md:38:- **Trust scoring** — Per-claim trust assessment per [ADR-0105](0105-provenance-as-first-class-output.md). Renderers surface trust bands from the resolved provenance envelope; they do not compute trust.
.docs/decisions/0130-surface-independent-composition-contract.md:40:- **Abilities runtime** — Producer-side execution per [ADR-0102](0102-abilities-as-runtime-contract.md). Composition-producing abilities are normal abilities returning `AbilityOutput<Composition>`.
.docs/decisions/0130-surface-independent-composition-contract.md:43:This rule prevents the failure mode where an ability author rebuilds retrieval, trust, signals, or invocation plumbing from scratch without realizing the substrate already has it. New work CONSUMES the named primitive; it does not duplicate it.
.docs/decisions/0130-surface-independent-composition-contract.md:47:The substrate emits composed output as instances of a typed `Composition` model. A `Composition` is the `T` inside `AbilityOutput<Composition>` per [ADR-0102](0102-abilities-as-runtime-contract.md) §6 — the canonical `Provenance` envelope lives exactly once on the wrapper, not on the domain output. The Composition model itself carries no envelope copy; blocks carry compact references into the wrapper envelope. The model:
.docs/decisions/0130-surface-independent-composition-contract.md:72:    pub provenance: ProvenanceRef, // Compact reference into AbilityOutput<Composition>.provenance
.docs/decisions/0130-surface-independent-composition-contract.md:77:/// Reference into the canonical provenance envelope that lives once on
.docs/decisions/0130-surface-independent-composition-contract.md:78:/// AbilityOutput<Composition>.provenance per ADR-0102 §6 and ADR-0105 §8.
.docs/decisions/0130-surface-independent-composition-contract.md:95:**Why `ProvenanceRef`, not `ProvenanceEnvelope`.** Embedding a full envelope on every block creates a second provenance carrier inside the domain output, which violates [ADR-0102](0102-abilities-as-runtime-contract.md) §6 + §9 Rule 5 (provenance lives exactly once on `AbilityOutput<T>`). It also defeats [ADR-0108](0108-provenance-rendering-and-privacy.md)'s 64KB serialized-provenance cap: a 24-block composition with typical 12KB Transform envelopes serializes to ~288KB of duplicated provenance, 4.5× the cap before any block content is counted. A `ProvenanceRef` is ~80-200 bytes serialized; a 60-block long-form composition spends ~13KB on block refs, well inside the cap. The reference shape mirrors [ADR-0105](0105-provenance-as-first-class-output.md) §8's planned-mutation pattern — plans reference the envelope by invocation id and field path rather than duplicating it.
.docs/decisions/0130-surface-independent-composition-contract.md:97:**Resolution.** Renderers resolve a `ProvenanceRef` by fetching the canonical envelope for `invocation_id` from the runtime provenance store (already addressable per ADR-0105/ADR-0108), reading the `FieldAttribution` at `field_path`, and passing the resolved envelope through [ADR-0108](0108-provenance-rendering-and-privacy.md) §2's actor-filtered `render_provenance_for(prov, actor, surface)`. Resolution can fail (garbage-collected envelope, unknown invocation across environments, unauthorized actor, masked provenance, missing field attribution); failure MUST degrade visibly — the block renders with a provenance-unavailable banner or downgraded trust band, never as fully trusted. When the envelope resolves but the exact `field_path` does not, the surface MAY fall back to invocation-level provenance, but MUST label the fallback as less specific.
.docs/decisions/0130-surface-independent-composition-contract.md:99:**Block-builder validation.** Block constructors MUST validate that `field_path` resolves into `output.provenance.field_attributions` at construction time, or fail. This prevents accidental broad attribution.
.docs/decisions/0130-surface-independent-composition-contract.md:101:**Composition-size guard.** Block provenance refs SHOULD stay under 48KB serialized across a single composition. Pathologically block-heavy compositions (>~200 blocks) that exceed the guard MUST collapse to section-level refs rather than embedding envelopes. The 48KB guard preserves headroom under ADR-0108's 64KB cap for surrounding composition metadata.
.docs/decisions/0130-surface-independent-composition-contract.md:103:Provenance and trust bands still flow through the composition at the block level, not as a footer. A renderer that strips provenance refs (or resolves them and discards trust treatment) is rendering wrong; the contract treats provenance as part of the composition's meaning, not its decoration.
.docs/decisions/0130-surface-independent-composition-contract.md:105:Source: Phase 0 artifact [`06-composition-provenance-ref.md`](../plans/dos-546/phase-0/06-composition-provenance-ref.md).
.docs/decisions/0130-surface-independent-composition-contract.md:107:**Salience computation uses existing substrate primitives.** The `Salience.weight` field is typically computed via the substrate's hybrid vector search ([ADR-0074](0074-vector-search-entity-content.md)) over the embed model ([ADR-0078](0078-nomic-embed-text-model-switch.md)), combined with signal correlation, recency, and claim-trust factors. The Composition contract does not mandate the scoring algorithm — it names the substrate's existing retrieval primitives as the canonical inputs. **Composition-producing abilities MUST NOT reinvent retrieval.** They consume the substrate's embed-model-backed search; the substrate owns the relevance question. This rule prevents the failure mode where an ability author rebuilds salience scoring from scratch without realizing the substrate already has it.
.docs/decisions/0130-surface-independent-composition-contract.md:111:The block type taxonomy is canonical and lives in the substrate. Initial types (extensible):
.docs/decisions/0130-surface-independent-composition-contract.md:114:- `ClaimSummary` — one or more claims rendered with provenance and trust
.docs/decisions/0130-surface-independent-composition-contract.md:122:New block types are added by registering them with the substrate's block-type registry.
.docs/decisions/0130-surface-independent-composition-contract.md:126:Surfaces that do not know a `Custom` block type MUST render a **schema-bounded projection** onto the nearest known block type. Renderers MUST NOT render unknown block payload fields directly. The previous "raw payload available" fallback is removed — an unknown block type is a privacy boundary, not a debugging opportunity. Unknown payload fields may include internal notes, source excerpts, email addresses, prompt context, debug carriers, or fields whose sensitivity tier is not allowed on the current surface (per [ADR-0125](0125-claim-anatomy-temporal-sensitivity-typeregistry.md)). Ability metadata is also model-facing and browser-facing API surface per [ADR-0102](0102-abilities-as-runtime-contract.md) §7.6; raw payload disclosure is an info-disclosure vector.
.docs/decisions/0130-surface-independent-composition-contract.md:134:5. **Preserve `claim_refs` exactly.** These are not payload — they are stable references into the claim substrate and are required for downstream link resolution.
.docs/decisions/0130-surface-independent-composition-contract.md:135:6. **Preserve `provenance` (the `ProvenanceRef`) exactly.** This is not payload — it is a reference into the [ADR-0105](0105-provenance-as-first-class-output.md) envelope and renders only through [ADR-0108](0108-provenance-rendering-and-privacy.md) actor-filtered rules. The fallback path MUST NOT dereference `claim_refs` or `provenance` to backfill dropped payload fields.
.docs/decisions/0130-surface-independent-composition-contract.md:136:7. **Render as the selected nearest-known type** using the projected payload, preserved `claim_refs`, and preserved `provenance`.
.docs/decisions/0130-surface-independent-composition-contract.md:137:8. **Show a non-dismissible banner on the block:** `Rendered as <nearest-known-type> — payload may be incomplete`. The banner uses product-facing vocabulary; internal terms (`enrichment`, `intelligence pipeline`, `substrate`, `schema projection`) MUST NOT appear in user-visible banner copy.
.docs/decisions/0130-surface-independent-composition-contract.md:140:If no candidate has a positive deterministic score, or if the selected candidate has no eligible schema intersection with the unknown schema, the renderer MUST render the block as generic `dailyos/text` with an empty payload, preserved `claim_refs`, preserved `provenance`, the banner `Rendered as dailyos/text — payload may be incomplete`, and trust band `needs_verification`.
.docs/decisions/0130-surface-independent-composition-contract.md:144:**Substrate-side count guard.** The substrate SHOULD cap unknown-block count per composition (the exact threshold is a producer-side guard, not a renderer concern). A composition dominated by unknown blocks is a producer bug, not a normal fallback case.
.docs/decisions/0130-surface-independent-composition-contract.md:156:| MCP head (headless) | `JsonBlockRenderer` | Claim-shaped JSON per ADR-0128 §4 |
.docs/decisions/0130-surface-independent-composition-contract.md:157:| CLI head | `MarkdownBlockRenderer` | Markdown per ADR-0128 §7 |
.docs/decisions/0130-surface-independent-composition-contract.md:166:Abilities that produce composed pages (briefings, entity pages, prep packs, reports) return `Composition` as their output. The composition is authored by the ability with full provenance attribution per [ADR-0105](0105-provenance-as-first-class-output.md). Surfaces never construct compositions; they receive them through the normal ability-invocation path and render them.
.docs/decisions/0130-surface-independent-composition-contract.md:170:- A `briefing.daily` ability produces a `Composition { kind: Briefing, sections: [...], ... }`.
.docs/decisions/0130-surface-independent-composition-contract.md:171:- An `entity.account_overview` ability produces a `Composition { kind: EntityPage, subject: Some(account_ref), ... }`.
.docs/decisions/0130-surface-independent-composition-contract.md:175:This is the cleanest expression of the AI-native principle from [PHILOSOPHY.md](../../design/product/PHILOSOPHY.md): AI produces, users consume. Authorship is substrate-side; consumption is surface-side; the contract between them is the `Composition` model.
.docs/decisions/0130-surface-independent-composition-contract.md:183:3. Rendering blocks via its surface-specific renderer. For each block, resolve `provenance: ProvenanceRef` against the wrapper envelope. Batch lookups by unique `invocation_id` on first paint to avoid per-block round trips; the first visible viewport SHOULD complete within ~50ms p95 for local-runtime lookups and ~150ms p95 for remote-runtime lookups. Content MAY render immediately with a pending trust band when the lookup is slower.
.docs/decisions/0130-surface-independent-composition-contract.md:186:6. Applying [ADR-0108](0108-provenance-rendering-and-privacy.md) §2 `render_provenance_for(prov, actor, surface)` to every resolved envelope. The `ProvenanceRef` does not bypass actor/surface filtering; it only locates the canonical envelope to which those rules apply.
.docs/decisions/0130-surface-independent-composition-contract.md:188:The same composition rendered by the WordPress SurfaceClient, Tauri React surface, and MCP head produces equivalent intelligence — different paint, same content, same provenance.
.docs/decisions/0130-surface-independent-composition-contract.md:192:Per [ADR-0129](0129-composable-surfaces-wordpress-studio-as-primary-surface.md) §6, layouts can be agent-composed at runtime based on substrate salience rather than fixed templates. Under this contract, on-the-fly composition is an ability behavior: the producing ability emits a `Composition` whose `sections` and `blocks` are chosen at invocation time based on what's salient. The contract is unchanged; the ability's logic does the choosing.
.docs/decisions/0130-surface-independent-composition-contract.md:200:- **Editing semantics.** A user editing a block's value (claim correction) routes through the existing feedback path per [ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md) §5; this ADR does not redesign feedback.
.docs/decisions/0130-surface-independent-composition-contract.md:201:- **Persistence of compositions.** Whether the substrate stores composed outputs or regenerates them on demand is an implementation question; the contract is the same either way.
.docs/decisions/0130-surface-independent-composition-contract.md:202:- **The composition lifecycle (versioning, invalidation, refresh).** Lives in the existing substrate work on claim lifecycle and signal propagation; compositions inherit those primitives via their `claim_refs`.
.docs/decisions/0130-surface-independent-composition-contract.md:208:- **Surfaces are genuinely interchangeable.** A new SurfaceClient instance (Obsidian, browser extension, mobile, future-thing) ships a renderer and consumes the same compositions. No substrate change required to add a surface.
.docs/decisions/0130-surface-independent-composition-contract.md:209:- **The Composition Contract is the seam DOS-546 actually validates.** The spike validates that the WordPress SurfaceClient renderer can render substrate-authored compositions; it doesn't have to invent the composition model.
.docs/decisions/0130-surface-independent-composition-contract.md:210:- **WordPress doesn't become the document model.** Gutenberg blocks are one render target. The substrate retains authorship; WordPress projects.
.docs/decisions/0130-surface-independent-composition-contract.md:212:- **Provenance lives once.** `ProvenanceRef` preserves ADR-0102 §6 + ADR-0105 §8's "lives once" invariant. Block-heavy compositions stay inside ADR-0108's 64KB serialized-provenance cap — a 60-block long-form composition spends ~13KB on refs vs. ~720KB if envelopes were copied per block.
.docs/decisions/0130-surface-independent-composition-contract.md:219:- **Existing compositions are implicit.** The Tauri React magazine builds composed pages today via React components, not via a `Composition` model. Migration to producing `Composition` from the substrate side is real work (likely in v1.4.2-v1.4.4 wave program).
.docs/decisions/0130-surface-independent-composition-contract.md:221:- **Provenance-resolver retention.** Compositions that outlive runtime provenance retention need either longer retention for published-composition invocations or a durable exported provenance bundle keyed by `invocation_id`. The block ref shape is the right address either way; what changes is where the envelope is fetched from.
.docs/decisions/0130-surface-independent-composition-contract.md:222:- **MCP head's composition rendering** (per ADR-0128 §7) becomes more specified than today's "claim-shaped JSON" framing. The MCP renderer maps blocks to claim-JSON envelopes; existing MCP clients consume the envelopes as before, but the underlying contract is the Composition model.
.docs/decisions/0130-surface-independent-composition-contract.md:228:- ADR-0128 §1 is preserved — substrate is the product, surfaces are heads over it.
.docs/decisions/0130-surface-independent-composition-contract.md:229:- ADR-0102 §6/§9 Rule 5 and ADR-0105 §8 are preserved structurally — provenance lives once, on the wrapper, and references address it.
.docs/decisions/0130-surface-independent-composition-contract.md:235:- [ADR-0074](0074-vector-search-entity-content.md) — Hybrid vector search; substrate retrieval primitive consumed for salience
.docs/decisions/0130-surface-independent-composition-contract.md:238:- [ADR-0102](0102-abilities-as-runtime-contract.md) — Abilities runtime contract; §6 + §9 Rule 5 establish provenance-lives-once on `AbilityOutput<T>`; §7.1 canonical AbilityPolicy schema; §7.6 governs SurfaceClient-reachable ability metadata
.docs/decisions/0130-surface-independent-composition-contract.md:239:- [ADR-0105](0105-provenance-as-first-class-output.md) — Provenance envelope shape; §8 establishes the reference pattern that `ProvenanceRef` mirrors
.docs/decisions/0130-surface-independent-composition-contract.md:240:- [ADR-0108](0108-provenance-rendering-and-privacy.md) — Actor-filtered provenance rendering; 64KB serialized-provenance cap that motivates the ref shape
.docs/decisions/0130-surface-independent-composition-contract.md:242:- [ADR-0125](0125-claim-anatomy-temporal-sensitivity-typeregistry.md) — Sensitivity tiers (`Public`, `Internal`, `Confidential`, `UserOnly`) consumed by schema-bounded fallback projection
.docs/decisions/0130-surface-independent-composition-contract.md:243:- [ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md) — Headless DailyOS; MCP as co-equal surface
.docs/decisions/0130-surface-independent-composition-contract.md:245:- [Phase 0 artifact 06](../plans/dos-546/phase-0/06-composition-provenance-ref.md) — Source for the `ProvenanceRef` shape (§2 amendment)
.docs/decisions/0097-account-health-scoring-architecture.md:272:| **Meeting briefing hero** | Account health band + any divergence alert | `AccountHealth.band` + `divergence` |
.docs/decisions/0097-account-health-scoring-architecture.md:273:| **Meeting briefing risks** | Health-derived risks (declining cadence, champion disengagement) | From `dimensions` where score < threshold |
.docs/decisions/0097-account-health-scoring-architecture.md:274:| **Daily briefing attention** | "N accounts with declining relationship signals" | Aggregated from all accounts' `trend.direction == "declining"` |
.docs/decisions/0116-tenant-control-plane-boundary.md:33:This ADR establishes the boundary now, before the control plane exists, so that every future control-plane feature is measured against a hard rule. It also introduces the one substrate change that v1.4.0 must land so the boundary is physically enforceable later: a `DbKeyProvider` trait seam that lets enterprise BYOK slot in without a database schema migration.
.docs/decisions/0116-tenant-control-plane-boundary.md:86:- **Feature convenience.** "Just cache the latest briefing on the server so users can preview it from the web." No — render it locally when the user opens the web surface, or accept that the web surface is login-only.
.docs/decisions/0116-tenant-control-plane-boundary.md:108:### 4. DbKeyProvider — the substrate seam for v1.4.0
.docs/decisions/0116-tenant-control-plane-boundary.md:210:- Both human-readable (briefing Markdown / PDF) and machine-readable (structured JSON with provenance) output formats.
.docs/decisions/0116-tenant-control-plane-boundary.md:220:The codex outside voice on the aggregate v1.4.0 plan surfaced a sharper challenge: **publish is reporting, not team intelligence.** If DailyOS's substrate succeeds and produces trusted, provenanced, per-user customer intelligence, the enterprise demand will not be "let me export a snapshot" — it will be "let our team see the same account state." Publish (user-initiated, per-user, snapshot-based) does not solve that. Per-user SQLite forever (D1) + publish-as-export forecloses the shared-operational-truth path structurally.
.docs/decisions/0116-tenant-control-plane-boundary.md:364:### R1.7 Metadata boundary clarification — provenance and publish records
.docs/decisions/0116-tenant-control-plane-boundary.md:366:Codex flagged: [ADR-0105](0105-provenance-as-first-class-output.md) provenance includes actor metadata; [ADR-0117](0117-publish-boundary-pencil-and-pen.md) publish records include destination refs. This ADR didn't classify whether those are content, control-plane metadata, or local-only metadata.
.docs/decisions/0116-tenant-control-plane-boundary.md:368:**Revised §3 clarification:** provenance and publish records are **local-only metadata**. They live on the device, encrypted like any other claim data, and never traverse the control plane. "Local-only metadata" is a third category distinct from "permitted control-plane metadata" and "forbidden content." The principle: metadata *about a user action* (auth state, capability grants) is permitted on the control plane; metadata *about an entity or claim* is local-only and follows content encryption rules.
.docs/decisions/0042-per-operation-pipelines.md:8:The monolithic three-phase pattern (Prepare → Enrich → Deliver) was designed for the CLI, where one command produced the full daily briefing. The app has scheduling, background execution, and progressive UI rendering — it doesn't need everything in one shot.
.docs/decisions/0042-per-operation-pipelines.md:28:- `briefing:generate` → Claude call (reads delivered JSON files) → briefing narrative
.docs/decisions/0042-per-operation-pipelines.md:53:- More Claude Code invocations per briefing cycle (cost/concurrency management)
.docs/decisions/0040-archive-reconciliation.md:14:4. **Gap flagging** — Unprocessed transcripts and incomplete capture get surfaced in the next morning's briefing as attention items.
.docs/decisions/0040-archive-reconciliation.md:24:3. **Flag attention items** — Write unresolved items (missing transcripts, unprocessed inbox files) to a `next-morning-flags.json` that the next briefing's Phase 1 can read and surface.
.docs/decisions/0040-archive-reconciliation.md:34:- Next-morning flags create a feedback loop: today's gaps surface in tomorrow's briefing
.docs/decisions/0089-user-entity-and-professional-context.md:25:**The existing `user_focus` field does not solve this.** It is a single freeform string ("What's important right now") — useful for the daily briefing's focus directive, but not structured enough to inform intelligence framing, signal prioritisation, or report generation.
.docs/decisions/0089-user-entity-and-professional-context.md:88:When `current_priorities` contains "expand Crestview Media account", signals from Crestview Media-domain entities get a relevance multiplier. When `value_proposition` mentions security compliance, security-related signals from accounts rank higher in the briefing's attention section.
.docs/decisions/0089-user-entity-and-professional-context.md:118:The user entity is personal professional context. It is never shared, never exposed via the MCP server to external tools, and never included in any outbound integration. The MCP sidecar's four read-only tools (`get_briefing`, `query_entity`, `list_entities`, `search_meetings`) do not expose user entity fields — they are for workspace intelligence, not the user's own professional context.
.docs/decisions/0089-user-entity-and-professional-context.md:129:- Users who update `current_priorities` should see their briefing attention section shift within one signal scoring cycle. This creates observable, personal responsiveness — the app demonstrably knows what matters to the user right now.
.docs/decisions/0093-prompt-injection-hardening.md:1:# ADR-0093: Prompt Injection Hardening
.docs/decisions/0093-prompt-injection-hardening.md:52:This is called at most prompt construction sites in `intelligence/prompts.rs`, `risk_briefing.rs`, `workflow/deliver.rs`, `processor/transcript.rs`, `processor/enrich.rs`, and `accounts.rs`.
.docs/decisions/0093-prompt-injection-hardening.md:192:Must be present in: `intelligence/prompts.rs`, `risk_briefing.rs`, `prepare/email_enrich.rs`, `workflow/deliver.rs`, `processor/transcript.rs`, `processor/enrich.rs`, `accounts.rs`.
.docs/decisions/0081-event-driven-meeting-intelligence.md:44:The user articulated this precisely: "Not every meeting gets an intelligence briefing and it should. An intelligence report, by its definition, is everything someone would need to know to create an agenda if they needed to. It's so they can walk into the room with confidence."
.docs/decisions/0081-event-driven-meeting-intelligence.md:191:The daily briefing's relationship with meeting intelligence inverts. Today it *generates* intelligence. In the new model, it *assembles* and *narrates* pre-existing intelligence.
.docs/decisions/0081-event-driven-meeting-intelligence.md:201:| Stale intelligence | Can't refresh without re-running entire briefing | Per-meeting refresh available — tap a meeting, tap refresh |
.docs/decisions/0081-event-driven-meeting-intelligence.md:205:In the event-based model, there's less need for a big-bang briefing run. Intelligence accumulates continuously. The daily run still has value for:
.docs/decisions/0081-event-driven-meeting-intelligence.md:215:- **Explicit refresh:** A subtle "Refresh" action in the briefing header for on-demand signal incorporation. Not a "generate from scratch" action, but a "check for new signals and update."
.docs/decisions/0081-event-driven-meeting-intelligence.md:218:The "no briefing yet" landing page remains necessary for first-of-day cold start (email hasn't been fetched, actions haven't synced, narrative hasn't been written). But if intelligence accumulates continuously, the app is never truly "empty" — meetings always have their intelligence records even before the morning run.
.docs/decisions/0081-event-driven-meeting-intelligence.md:280:- **The daily run gets faster.** Assembling from pre-computed intelligence is cheaper than generating from scratch. The morning briefing becomes a 30-60 second narrative refresh, not a 3-5 minute pipeline.
.docs/decisions/0081-event-driven-meeting-intelligence.md:282:- **The app is never empty.** With continuous intelligence accumulation, opening DailyOS always shows something — meeting intelligence exists from the moment meetings appear on the calendar. The "no briefing yet" state is limited to first-of-day narrative/email/action synthesis, not meeting intelligence.
.docs/decisions/0081-event-driven-meeting-intelligence.md:295:- **Weekly forecast as primary surface vs. daily briefing.** The weekly forecast becomes the strategic view (intelligence across the week); the daily briefing becomes the tactical view (what's happening today with overnight updates). Neither is deprecated — they serve different time horizons.
.docs/decisions/0081-event-driven-meeting-intelligence.md:303:| **I220** (Meeting forecast 4-5 days ahead) | **Superseded by I327** | I220 proposed a forecast section bolted onto the daily briefing. I327 makes advance intelligence generation the default for all meetings — not an add-on, the core model. |
.docs/decisions/0081-event-driven-meeting-intelligence.md:320:| **I331** | Daily briefing intelligence assembly (diff model, fast refresh) | Surfaces |
.docs/decisions/0121-team-intelligence-architecture.md:17:> Publish is not team intelligence. It is reporting. If the substrate succeeds, DailyOS creates high-value personal truth graphs trapped in per-user SQLite. Enterprise buyers will want shared operational truth, not scheduled user exports to S3/SharePoint. The plan has converted "team intelligence" into "users manually or periodically push summaries elsewhere." That is a product downgrade hidden as an architecture principle.
.docs/decisions/0121-team-intelligence-architecture.md:108:Until then: the substrate continues its current trajectory. Publish framework ships. Per-user SQLite persists. Team intelligence is acknowledged as an open strategic question, not a solved problem.
.docs/decisions/0121-team-intelligence-architecture.md:115:- Preserves substrate investment. Per-user SQLite, claims substrate, provenance, Trust Compiler — all valuable regardless of which option class eventually lands for team intelligence.
.docs/decisions/0121-team-intelligence-architecture.md:122:- "Open" status has a half-life. If this ADR stays Open for 18 months while substrate proceeds, the cost of picking an option grows (more patterns to retrofit). Trigger #3 (v1.5.0+ planning cycle) is the hard deadline for reassessment.
.docs/decisions/0121-team-intelligence-architecture.md:126:- v1.4.0 substrate work is unaffected. This is strategic framing, not architectural dependency.
.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:1:# ADR-0125: Claim Anatomy v2 — Temporal Scope, Sensitivity, Claim Type Registry
.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:3:**Status:** Accepted (substrate primitives for v1.4.0 spine; downstream enforcement in v1.4.1+)
.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:5:**Amended:** 2026-05-19 — added §5 `RecommendationClaim` variant for v1.4.6 Salience & Recommendations. Registers `ClaimType::Recommendation` in `CLAIM_TYPE_REGISTRY` (canonical persisted name `"recommendation"`) with `State` temporal scope, `Internal` sensitivity, `Medium` freshness decay, `Replace` commit policy, agent-only authorship, attaches to `Account` / `Project` / `Person` subjects. Salience factors, user-feedback state (`Accepted` / `Dismissed` / `NotUseful` / `TooNoisy` / `Converted`), conversion state, and supersession + dismissal lifecycle are declared at the substrate level; concrete DTOs land in v1.4.6 W1-A `services/recommendations/contracts.rs`. Per the v1.4.6 W0 hardening cycle in [`.docs/plans/v1.4.6-waves.md`](../plans/v1.4.6-waves.md).
.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:7:**Extends:** [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md), [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0114](0114-scoring-unification.md)
.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:12:The 2026-04-24 Claim Anatomy Review (`.docs/plans/claim-anatomy-review-2026-04-24.md`) walked every dimension a claim should express in the v1.4.x substrate. Three dimensions were under-scrutinized at the substrate level and warrant a substrate primitive in v1.4.0 spine:
.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:18:This ADR is the substrate allowance: schema fields + types + registry mechanism. Downstream enforcement (freshness scoring, render gates) is named per-dimension and lands in v1.4.1+.
.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:65:    /// Fine on user's own surfaces, briefings, MCP. Never customer-facing or shared/published.
.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:70:    /// Fine on the user's own surfaces only. Not on MCP, not on Publish, not on briefings shared with others.
.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:160:| Sensitivity | column + enum + serde | DOS-214 render layer enforces | entity surfaces enforce ceiling | briefing surfaces enforce ceiling | `Recommendation` defaults `Internal` |
.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:165:**Decision (2026-05-19, W0 of v1.4.6 Salience & Recommendations).** Recommendations are first-class claims, not a parallel substrate. They register in `CLAIM_TYPE_REGISTRY` and ride the same provenance, trust, supersession, and sensitivity primitives every other claim type already inherits from this ADR. The W0 amendment fixes the substrate-level shape; concrete DTOs land in v1.4.6 W1-A `services/recommendations/contracts.rs`.
.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:167:**Why a claim-type, not a separate table.** A recommendation says *"this action is recommended for this subject as of this moment, with this evidence and this expected impact"* — that is a claim. Trust band, freshness decay, contradiction handling, and the existing claim lifecycle (assert → corroborate → contradict → supersede → retract) already apply. Making it a claim means every existing intelligence-loop surface (entity context, briefing prep, surface render) consumes recommendations through the same render path as every other claim, with the same provenance receipt and the same trust signaling. A parallel `recommendation_claims` table would have re-invented all of that.
.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:180:**Lifecycle additions beyond the base claim envelope.** Recommendations carry three substrate fields above what other claim types use, declared at the ADR level here and frozen as wire format in W1-A `contracts.rs`:
.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:183:- **User-feedback state.** Closed enum `{ Accepted | Dismissed | NotUseful | TooNoisy | Converted }`. Default `None` (no feedback yet). State transitions are append-only at the `claim_feedback` level (per existing v1.4.0 substrate); the denormalized current-state field on the recommendation claim row is rebuilt from the feedback log by the W4-A feedback loop on each `ClaimFeedbackRecorded` signal.
.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:184:- **Conversion state.** Closed enum `{ NotConverted | InProgress | Converted | Abandoned }`. Default `NotConverted`. Updated when a downstream action (commitment, meeting outcome, account event) is causally linked back to the recommendation. v1.4.6 W4-A wires the link; v1.5.x causal-lineage substrate (deferred per §"Non-goals", anatomy review §14) makes the link first-class.
.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:191:**Surface render contract (v1.4.6 W3 consumers).** The `Recommendation` claim's render output goes through the same Shared Receipt DTO (`services::claim_receipt::contracts::ClaimReceipt`) every other claim uses. Per the v1.4.6 W0 close gate cross-version row, this ADR amendment is unblocked only after v1.4.4 W1-A ships the `ClaimReceipt` substrate.
.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:202:**ADR-0102 (abilities runtime) assessment.** v1.4.6's `recommend_for_entity` ability targets the existing `mcp_exposure: McpExposure::MetadataOnly` tri-state value (added in the 2026-05-10 amendment). No further ADR-0102 amendment needed for v1.4.6 W0. v1.4.7 may amend ADR-0102 to flip `mcp_exposure` to `Invocable` once the MCP `dailyos.read.portfolio_attention` consumer ships.
.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:221:- **Sensitivity** makes surface-leak prevention substrate-level instead of surface-policy convention. Surfaces that respect the sensitivity ceiling cannot leak Internal content into Public surfaces by construction.
.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:238:- [ADR-0105: Provenance as First-Class Output](0105-provenance-as-first-class-output.md) — `Provenance.warnings` interacts with sensitivity at render-time masking.
.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:241:- [ADR-0124: Longitudinal Topic Threading](0124-longitudinal-topic-threading.md) — same substrate-allowance pattern.
.docs/decisions/0038-cs-first-development.md:26:- General profile users get core features (briefing, calendar, inbox, actions) but no CS-specific enrichment
.docs/decisions/0047-entity-dashboard-architecture.md:128:- After briefing delivery (new meetings, actions)
.docs/decisions/0047-entity-dashboard-architecture.md:175:Enrichment is **on-demand only** in v1. No scheduled enrichment in the morning briefing. Future: enrichment for accounts with meetings today could be triggered as part of prep generation.
.docs/decisions/0047-entity-dashboard-architecture.md:214:- Chose on-demand enrichment over scheduled — avoids morning briefing latency, accepts staler company data
.docs/decisions/0064-meeting-prep-page-redesign.md:43:Every section is a Card with an icon, title, and copy button. There's no narrative hierarchy — risks and talking points get the same visual weight as key principles. The page reads like a database dump, not a briefing.
.docs/decisions/0064-meeting-prep-page-redesign.md:45:From the UX research (2026-02-08): "The curation IS the value. Ruthless prioritization — intelligence experts condense all signals into a few pages. The briefing has a beginning, a middle, and an end."
.docs/decisions/0064-meeting-prep-page-redesign.md:82:- If no agenda exists: AI-generated proposed agenda (current behavior) or a structured signal summary (risks + talking points + questions as a consolidated briefing section)
.docs/decisions/0064-meeting-prep-page-redesign.md:164:- Prep page reads as a briefing document, not a database query
.docs/decisions/0064-meeting-prep-page-redesign.md:198:- Remove Card wrappers from Tier 1 (briefing feel, not dashboard feel)
.docs/decisions/0099-remote-first-server-canonical-architecture.md:16:DailyOS was built local-first for good reasons: privacy, offline capability, zero infrastructure, direct user ownership. These principles produced a working product through v0.16.0 — briefings, meeting intelligence, reports, signal bus, editorial design. The local-first bet was correct for the first 98 ADRs.
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md:6:**Relates to:** [ADR-0080](0080-signal-intelligence-architecture.md), [ADR-0091](0091-intelligence-provider-abstraction.md), [ADR-0095](0095-dual-mode-context-architecture.md), [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0104](0104-execution-mode-and-mode-aware-services.md), [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0106](0106-prompt-fingerprinting-and-provider-interface.md), [ADR-0107](0107-source-taxonomy-alignment.md), [ADR-0108](0108-provenance-rendering-and-privacy.md), [ADR-0109](0109-temporal-primitives-in-the-entity-graph.md), [ADR-0110](0110-evaluation-harness-for-abilities.md), [ADR-0111](0111-surface-independent-ability-invocation.md), [ADR-0112](0112-migration-strategy-parallel-run-and-cutover.md), [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md), [ADR-0114](0114-scoring-unification.md), [ADR-0115](0115-signal-granularity-audit.md), [ADR-0119](0119-runtime-evaluator-pass-for-transform-abilities.md)
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md:7:**Linear:** [v1.4.0 — Abilities Runtime](https://linear.app/a8c/project/v140-abilities-runtime-8267614bba99), [DOS-5 Trust Compiler](https://linear.app/a8c/issue/DOS-5), [DOS-7 intelligence_claims persistence](https://linear.app/a8c/issue/DOS-7)
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md:15:DailyOS has been building a harness without naming it. The OpenClaw audit (`.docs/research/2026-02-14-openclaw-learnings.md`) was effectively a harness comparison done in different vocabulary. More importantly: the v1.4.0 Abilities Runtime project (ADRs 0102–0115) *is* the harness made explicit — typed capability contracts, structural provenance, mode-aware services, temporal primitives, trust scoring, evaluation harness, claims as first-class. The work already in flight is the answer. What has been missing is the framing.
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md:20:2. Maps harness best practices from the 2025–2026 literature onto existing ADRs, so contributors can see which harness principle each substrate decision satisfies.
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md:21:3. Isolates the residual gaps — places where the harness literature points at something the v1.4.0 substrate does not fully address, with explicit scoping for each.
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md:23:This ADR is principles + cross-reference + gap list. It does **not** propose new substrate. Where a gap needs closing, this ADR points at the existing ADR it should land in (amendment) or names it as a candidate future ADR.
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md:29:The term **harness** is adopted as canonical for DailyOS's synthesis and intelligence layer. Preferred over "pipeline," "agent," "enrichment system," or "intelligence engine" in architectural discussion. Rationale: *pipeline* undersells the memory and verification surface; *agent* oversells — DailyOS explicitly does not route across sub-agents, does not do recursive tool use, and does not run persistent background reasoning (see [ADR-0102](0102-abilities-as-runtime-contract.md) §1 and `.docs/research/2026-02-18-event-driven-intelligence-vision.md`). *Harness* is accurate: typed capability surface, durable external state, verification before output, feedback learning.
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md:33:### 2. Harness principles and how the v1.4.0 substrate implements them
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md:39:| **P1 — Context engineering over context stuffing.** Dynamically select what the model sees per call; prefer smaller scoped prompts over monolithic ones. | Dimension split (I576), ability composition tree ([ADR-0102](0102-abilities-as-runtime-contract.md) §11.3), Glean-first retrieval ([ADR-0100](0100-glean-first-intelligence-architecture.md)), hybrid vector search ([ADR-0074](0074-vector-search-entity-content.md)). |
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md:41:| **P3 — Verification before trust.** Outputs are validated against deterministic ground truth before reaching the user; high-severity failures trigger repair, not user-visible error. | `validation.rs` (schema + anomaly), `consistency.rs` (fact-grounding), `TrustAssessment` in the Provenance envelope ([ADR-0105](0105-provenance-as-first-class-output.md) §3), Trust Compiler six-factor score ([DOS-5](https://linear.app/a8c/issue/DOS-5)). |
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md:44:| **P6 — Typed capability surface.** Tools / capabilities are versioned, schema-validated, category-classified with effect inference. | Abilities runtime contract — name, version, schema, category ([ADR-0102](0102-abilities-as-runtime-contract.md) §1–§3), service boundary enforcement ([ADR-0101](0101-service-boundary-enforcement.md)), mode-aware services ([ADR-0104](0104-execution-mode-and-mode-aware-services.md)). |
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md:45:| **P7 — Observability per invocation.** Trace tool calls, context contents, prompt, response, verification result, evaluator score — for every call. | Provenance envelope ([ADR-0105](0105-provenance-as-first-class-output.md)), prompt fingerprinting ([ADR-0106](0106-prompt-fingerprinting-and-provider-interface.md)), invocation ID + inputs snapshot. **Incomplete**: debug UI consuming this data (Gap E). |
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md:47:| **P9 — Source attribution as output.** Every generated field carries attribution so corrections can flow back to the originating source's reliability weight. | `field_attributions` map in the Provenance envelope ([ADR-0105](0105-provenance-as-first-class-output.md) §5), authoritative source taxonomy ([ADR-0107](0107-source-taxonomy-alignment.md)), provenance rendering + privacy ([ADR-0108](0108-provenance-rendering-and-privacy.md)). |
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md:50:**Takeaway.** Nine of ten principles are substantively implemented by the v1.4.0 substrate or in-flight issues. P4 and P7 are partially implemented — each has a residual gap detailed below. P2 has a gap around claim *history*. No principle is absent. The v1.4.0 Abilities Runtime is not a set of scattered improvements — it is a harness, read in a particular vocabulary.
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md:54:Five gaps remain after v1.4.0 substrate lands. Listed by expected leverage.
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md:64:**Dependencies.** [ADR-0102](0102-abilities-as-runtime-contract.md) (invocation hook on Transform ability completion), [ADR-0106](0106-prompt-fingerprinting-and-provider-interface.md) (evaluator's own prompt is fingerprinted), [ADR-0110](0110-evaluation-harness-for-abilities.md) (rubric source of truth).
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md:99:**Dependencies.** [DOS-7](https://linear.app/a8c/issue/DOS-7) (the right place to decide this), [ADR-0105](0105-provenance-as-first-class-output.md) (`ProvenanceWarning::SourceStale` can extend to claim-stale), [ADR-0110](0110-evaluation-harness-for-abilities.md) (stability rubric dimension), [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md) (propose/commit semantics).
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md:119:**What's missing.** Provenance envelopes ([ADR-0105](0105-provenance-as-first-class-output.md)), prompt fingerprints ([ADR-0106](0106-prompt-fingerprinting-and-provider-interface.md)), and the eval harness ([ADR-0110](0110-evaluation-harness-for-abilities.md)) collectively produce all the data needed to answer "why did the AI say this" — assembled context, prompt text, response, validation flags, consistency report, trust score, composition tree. No UI consumes this data. Debugging a bad output today requires stitching logs by hand.
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md:121:**Proposal.** A devtools-style panel on the entity page (dev mode only, gated by [ADR-0108](0108-provenance-rendering-and-privacy.md) privacy rules) showing the last N invocations per entity: context hash + resolvable pointer, prompt fingerprint + resolvable template, provider response, validation + consistency + trust output, composition tree. Not new storage — a new consumer of existing storage.
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md:123:**Dependencies.** [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0106](0106-prompt-fingerprinting-and-provider-interface.md), [ADR-0108](0108-provenance-rendering-and-privacy.md) (the panel must honor masking rules and privacy tiers), [ADR-0111](0111-surface-independent-ability-invocation.md) (panel is a Surface per ADR-0111's definition; use the invocation endpoint, not a bespoke query).
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md:125:**Scope.** v1.5.0 or later. Not substrate-blocking; pure product surface work.
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md:127:**Status:** Tracked as Linear issue for v1.5.0+ product work. Substantively empowered by the addition of `evaluation_traces` table in [ADR-0119](0119-runtime-evaluator-pass-for-transform-abilities.md) §6, which gives the panel rich data beyond provenance alone. No new ADR required.
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md:139:| E — Debug trace surface | Tracked as v1.5.0+ product work | Linear issue (filed 2026-04-19); consumes `evaluation_traces` + provenance |
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md:141:All residual gaps identified in the original ADR either closed by specific ADR work or tracked as product-surface issues with substrate dependencies satisfied.
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md:147:- **`init.sh`-style session bootstrap.** Anthropic's pattern targets multi-session coding agents. DailyOS sessions are per-ability invocations; cross-session continuity is served by the claims layer + temporal primitives + briefing callouts. No additional bootstrap artifact is needed.
.docs/decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md:197:- [v1.4.0 — Abilities Runtime](https://linear.app/a8c/project/v140-abilities-runtime-8267614bba99) project brief
.docs/decisions/0122-chapter-enrichment.md:225:4. **Should it trigger briefing callouts via `CALLOUT_SIGNAL_TYPES`?** No. Enrichment scheduling is infrastructure, not a user-facing signal event.
.docs/decisions/0014-mvp-scope.md:24:- Core loop is: scheduler triggers briefing → user reads dashboard → archive cleans up at midnight
.docs/decisions/0035-incremental-prep-generation.md:8:When the calendar poller (ADR-0032) detects a new meeting that wasn't in the morning briefing, the meeting appears on the dashboard with `overlay_status: new` and no prep. Currently the only way to get prep for this meeting is to re-run the entire briefing workflow (all three phases).
.docs/decisions/0035-incremental-prep-generation.md:10:This is wasteful. The existing three-phase pattern (prepare → enrich → deliver) operates on the full day's schedule. A single new meeting shouldn't require re-fetching all emails, re-enriching all preps, and re-delivering the entire briefing.
.docs/decisions/0035-incremental-prep-generation.md:20:The existing full-briefing workflow remains unchanged. This is an additive capability.
.docs/decisions/0035-incremental-prep-generation.md:28:- Risk: partial briefing state if the incremental prep fails mid-way — mitigated by treating the prep file as atomic (write or don't)
.docs/decisions/0107-source-taxonomy-alignment.md:8:**Required by:** [ADR-0105](0105-provenance-as-first-class-output.md) §4, [ADR-0108](0108-provenance-rendering-and-privacy.md)
.docs/decisions/0107-source-taxonomy-alignment.md:16:- [ADR-0105](0105-provenance-as-first-class-output.md) §4 references `DataSource` in provenance attribution, including REDACTED and others not in ADR-0098's enum.
.docs/decisions/0107-source-taxonomy-alignment.md:18:The field naming is also inconsistent: [ADR-0080](0080-signal-intelligence-architecture.md) uses `source`; [ADR-0098](0098-data-governance-source-aware-lifecycle.md) and [ADR-0105](0105-provenance-as-first-class-output.md) use `data_source`. Lifecycle behavior per source (purge vs. flag-for-re-enrichment) is also not consistent: [ADR-0098](0098-data-governance-source-aware-lifecycle.md) says AI-generated intelligence is flagged for re-enrichment rather than purged, but downstream consumers (provenance, maintenance audit) had been assumed to purge.
.docs/decisions/0107-source-taxonomy-alignment.md:20:This ADR defines the authoritative `DataSource` enum, the `SourceIdentifier` extension pattern, the canonical field name, and the per-source lifecycle rules that every provenance-consuming surface honors.
.docs/decisions/0107-source-taxonomy-alignment.md:72:`SourceIdentifier` from [ADR-0105](0105-provenance-as-first-class-output.md) §4 gains variants to match the expanded `DataSource` enum:
.docs/decisions/0107-source-taxonomy-alignment.md:131:The `scoring_class` field on `SourceAttribution` ([ADR-0105](0105-provenance-as-first-class-output.md) §4) is derived from this table, not author-set. Consumers that aggregate sources for scoring filter by class to avoid conflating Slack chatter with REDACTED ARR.
.docs/decisions/0107-source-taxonomy-alignment.md:166:**Resolution of the ADR-0098/ADR-0105 conflict.** ADR-0098 is right: AI-synthesized output is flagged, not purged, because the underlying source revocation removes the input data but the synthesis itself may remain useful as an annotation awaiting re-generation. [ADR-0105](0105-provenance-as-first-class-output.md) §8 is amended: "source revocation triggers a masking pass" applies only to sources whose `LifecycleBehavior` is `Mask`; for `Purge`, the records are deleted; for `FlagForReEnrichment`, the records remain with a flag. [ADR-0108](0108-provenance-rendering-and-privacy.md) specifies the masking and flag-rendering shapes surfaces see.
.docs/decisions/0107-source-taxonomy-alignment.md:170:Every stored field that is itself LLM-synthesized carries a `SynthesisMarker` (per [ADR-0105](0105-provenance-as-first-class-output.md) §4) so the trust model's `contains_stored_synthesis` check works. This ADR specifies the marker storage:
.docs/decisions/0107-source-taxonomy-alignment.md:188:7. **Stored AI output is identifiable.** Synthesis markers close the trust-laundering hole from [ADR-0105](0105-provenance-as-first-class-output.md) §3.
.docs/decisions/0107-source-taxonomy-alignment.md:209:- [ADR-0105: Provenance as First-Class Output](0105-provenance-as-first-class-output.md) — §4 `SourceAttribution` uses this ADR's `DataSource` and `SourceIdentifier`.
.docs/decisions/0107-source-taxonomy-alignment.md:210:- [ADR-0108: Provenance Rendering and Privacy](0108-provenance-rendering-and-privacy.md) — Renders masking behavior and flag-for-re-enrichment status.
.docs/decisions/0107-source-taxonomy-alignment.md:219:Codex round 2 finding 13 surfaced that the v1.4.0 spine plan (DOS-299) needed to preserve item-level confidence semantics for legacy items without `ItemSource` (the current `HasSource::effective_confidence` defaults missing source to `0.5` per `intelligence/io.rs:1185-1189`). My earlier writeup proposed registering this in the `CLAIM_TYPE_REGISTRY` (per ADR-0125). That was wrong — it's a `DataSource` taxonomy concern, not a claim type. ADR-0107 owns `DataSource`; this amendment adds the variant.
.docs/decisions/0107-source-taxonomy-alignment.md:297:- **Default `confidence`:** not pinned by this amendment — `abilities-runtime` computes per-claim confidence from freshness + corroboration.
.docs/decisions/0024-email-ai-triage-not-client.md:12:The app shows AI-curated summaries and suggested actions, not raw emails. Morning briefing auto-archives low-priority with a reviewable manifest. The app surfaces intelligence; the user's email client handles the actual replies.
.docs/decisions/0030-weekly-prep-with-daily-refresh.md:16:- A new meeting detected by calendar polling can't get prep without re-running the entire briefing
.docs/decisions/0030-weekly-prep-with-daily-refresh.md:35:| `calendar:poll` | Detect new/changed/cancelled meetings against briefing baseline | Timer (already isolated) |
.docs/decisions/0030-weekly-prep-with-daily-refresh.md:88:- New meetings get prep within minutes via calendar polling, not hours via next briefing
.docs/decisions/0091-intelligence-provider-abstraction.md:14:Supporting other LLM backends requires abstracting away the PTY-specific transport at `intel_queue.rs`. The rest of the pipeline (processor/enrich.rs, prepare/email_enrich.rs, executor.rs, deliver.rs, risk_briefing.rs) remains PTY-based for now. Those are workflow-level operations that are Claude Code-specific in character and are not on the post-1.0 path.
.docs/decisions/0111-surface-independent-ability-invocation.md:7:**Extends:** [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0027](0027-mcp-dual-mode.md)  
.docs/decisions/0111-surface-independent-ability-invocation.md:8:**Depends on:** [ADR-0104](0104-execution-mode-and-mode-aware-services.md), [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0108](0108-provenance-rendering-and-privacy.md)
.docs/decisions/0111-surface-independent-ability-invocation.md:12:[ADR-0102](0102-abilities-as-runtime-contract.md) §7 specifies the ability registry and declares that surfaces invoke abilities through a single path. §10 Phase 2 requires migrating app Tauri commands and MCP sidecar handlers to wrap ability invocations. The existing MCP sidecar ([ADR-0027](0027-mcp-dual-mode.md)) has its own query logic and data access path; migrating it requires concrete binding rules.
.docs/decisions/0111-surface-independent-ability-invocation.md:14:This ADR specifies the exact binding pattern for Tauri commands and MCP tools, how abilities are generated into both surfaces from the registry, and how surfaces construct `AbilityContext` with mode, actor, and provenance renderer wiring.
.docs/decisions/0111-surface-independent-ability-invocation.md:25:| MCP server | `Agent` | `Live` (never escalated) | `McpAbilityBridge` |
.docs/decisions/0111-surface-independent-ability-invocation.md:67:6. Renders provenance via [ADR-0108](0108-provenance-rendering-and-privacy.md)'s `render_provenance_for(prov, User, TauriApp)`.
.docs/decisions/0111-surface-independent-ability-invocation.md:68:7. Returns `AbilityResponseJson` containing domain output plus rendered provenance.
.docs/decisions/0111-surface-independent-ability-invocation.md:74:The MCP server iterates the registry at startup and registers one MCP tool per ability whose `AbilityPolicy.allowed_actors` includes `Agent`:
.docs/decisions/0111-surface-independent-ability-invocation.md:77:for descriptor in AbilityRegistry::iter().filter(|d| d.policy.allowed_actors.contains(&Actor::Agent)) {
.docs/decisions/0111-surface-independent-ability-invocation.md:95:7. Renders provenance via [ADR-0108](0108-provenance-rendering-and-privacy.md)'s `render_provenance_for(prov, Agent, McpTool)`.
.docs/decisions/0111-surface-independent-ability-invocation.md:96:8. Returns the MCP response with domain output and filtered provenance.
.docs/decisions/0111-surface-independent-ability-invocation.md:173:The original three actor classes (`User`, `Agent`, `System`) were sufficient when DailyOS's surfaces were exclusively first-party: Tauri app for `User`, MCP for `Agent`, background workers / eval / tests for `System`. The decision in [ADR-0129](0129-composable-surfaces-wordpress-studio-as-primary-surface.md) to support third-party local surfaces — initially WordPress, generalized to any local CMS, editor, extension, or paired app that mediates user interactions with the substrate — introduces an actor that does not fit those three.
.docs/decisions/0111-surface-independent-ability-invocation.md:175:A WordPress plugin running locally is not the user — it is *code mediating* the user. It is not an agent in the LLM/autonomous sense — it has no reasoning surface; it dispatches user-bounded invocations. It is not a system process — it is third-party code with bounded trust and per-instance identity. Encoding "WordPress" as a vendor-specific class would couple the substrate to one vendor; generalizing to `SurfaceClient` lets WordPress, Obsidian, browser extensions, future PWA, and future mobile all inherit the same trust shape.
.docs/decisions/0111-surface-independent-ability-invocation.md:179:`SurfaceClient` is a third-party local client that mediates user interactions with the substrate on behalf of a paired user. Distinct from:
.docs/decisions/0111-surface-independent-ability-invocation.md:183:- `System` — an internal substrate process (cron, scheduled worker, eval harness, integration tests).
.docs/decisions/0111-surface-independent-ability-invocation.md:189:- **Instance identity.** Per-site / per-install / per-device identity, distinct from the paired user's identity. Each instance is its own entity to the substrate. Identity is logged with every substrate operation for audit attribution.
.docs/decisions/0111-surface-independent-ability-invocation.md:191:- **Scope grants.** An explicit enumerable set of scopes granted at pairing time (e.g., `read.account_overview`, `read.briefing`, `submit.feedback`, `manage.pairing`). Scope grants are *not* equivalent to host-OS user privileges or host-CMS site-wide capabilities. The host's own permission model (WP capabilities, OS user role, etc.) is one layer; DailyOS scopes are a separate, independent layer; both must pass for invocation.
.docs/decisions/0111-surface-independent-ability-invocation.md:192:- **Token model.** Short-lived capability tokens (not long-lived bearer tokens), refreshable, revocable, scoped to the instance's grants. Compromise of host state must not equal compromise of substrate state.
.docs/decisions/0111-surface-independent-ability-invocation.md:194:- **Write authority.** Feedback-only per [ADR-0128](0128-headless-dailyos-mcp-as-product-surface.md) §5. No direct claim creation, no source-reliability mutation outside feedback events, no tombstoning of others' claims. Raw editor-side content diffs (e.g., a Gutenberg block save diff) never become substrate writes.
.docs/decisions/0111-surface-independent-ability-invocation.md:195:- **Audit attribution.** Every substrate operation (read OR write) logs the `SurfaceClient` instance identity. Forensic traceability is a requirement, not an optional feature.
.docs/decisions/0111-surface-independent-ability-invocation.md:216:7. Renders provenance via [ADR-0108](0108-provenance-rendering-and-privacy.md)'s `render_provenance_for(prov, SurfaceClient, <instance>)`.
.docs/decisions/0111-surface-independent-ability-invocation.md:222:Registry enumeration for `SurfaceClient` filters at two levels: the actor class (is `SurfaceClient` in the ability's `allowed_actors`?) AND the per-instance scope grants (does this specific instance have the scope required for this ability?). Both must pass. Abilities NOT permitted for `SurfaceClient` invocation never appear in any SurfaceClient's tool surface, even if they appear for `User`. Abilities with `mcp_exposure: McpExposure::None` (per [ADR-0102](0102-abilities-as-runtime-contract.md) §7.1 + §7.6) are not enumerated for SurfaceClient-mediated MCP servers regardless of permission state; abilities with `mcp_exposure: McpExposure::MetadataOnly` are enumerated with name + description only (no input/output schema); no invocation handler is registered. Full schema is reserved for `McpExposure::Invocable`. The orthogonal `client_side_executable: bool` field on `AbilityPolicy` separately gates the WordPress 7.0 client-side `executeAbility()` JS path; an ability fully `Invocable` via MCP may still be `client_side_executable: false`, and vice versa, per Phase 0 artifact 05 lines 383-412.
.docs/decisions/0111-surface-independent-ability-invocation.md:226:WordPress is the immediate concrete instance and the one DOS-546 validates. But the substrate must remain neutral. A SurfaceClient class lets us:
.docs/decisions/0111-surface-independent-ability-invocation.md:230:3. Compose with [ADR-0130](0130-surface-independent-composition-contract.md): all SurfaceClient instances consume the same substrate-authored `Composition` model, rendered through their surface-specific renderer.
.docs/decisions/0111-surface-independent-ability-invocation.md:233:The substrate is the product; the heads are surfaces over it. `SurfaceClient` is the canonical actor class for *any* head that is not the first-party app, an autonomous agent, or an internal system process.
.docs/decisions/0111-surface-independent-ability-invocation.md:244:6. **Provenance rendering integrated into bridges.** Every response carries appropriately-rendered provenance per [ADR-0108](0108-provenance-rendering-and-privacy.md).
.docs/decisions/0111-surface-independent-ability-invocation.md:257:4. **Action latency growth.** Generic dispatch + schema validation + provenance rendering adds ~1–5ms per invocation. Mitigation: negligible for synthesis-bound abilities; measurable only for trivial reads.
.docs/decisions/0111-surface-independent-ability-invocation.md:262:- [ADR-0102: Abilities as the Runtime Contract](0102-abilities-as-runtime-contract.md) — Registry and §7 binding rules.
.docs/decisions/0111-surface-independent-ability-invocation.md:265:- [ADR-0108: Provenance Rendering and Privacy](0108-provenance-rendering-and-privacy.md) — Bridges call the renderer on return.

exec
/bin/zsh -lc "sed -n '536,555p' .docs/plans/v1.4.7-waves.md" in /private/tmp/dailyos-v147-w1a
exec
/bin/zsh -lc "sed -n '1,130p' .docs/decisions/0102-abilities-as-runtime-contract.md" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:

---

## Wave 2 — Ability-Backed Reads

Three agents. DOS-175 (briefing replacement — Bug priority Urgent) merges first; DOS-172 + DOS-173 run in parallel after.

### Agent W2-A — DOS-175: Replace stale briefing MCP reads with ability-backed current context

- **Spec:** [DOS-175](https://linear.app/a8c/issue/DOS-175).
- **Goal:** The current MCP briefing path reads stale generated snapshots. Replace with ability-backed current intelligence so MCP is a co-equal head, not an export layer.
- **Files owned (exclusive):** `src-tauri/src/services/mcp_v2/handlers/tool_briefing.rs` (W1-A pre-created placeholder; W2-A fills); legacy briefing snapshot read paths to deprecate (enumerated at L0 plan via grep).
- **Don't touch:** Gateway / contracts / actor policy (W1 owners); other handlers (other lane owners).
- **Depends on:** W1 merged.
- **Done when:** `dailyos.read.daily_briefing` and `dailyos.read.meeting_briefing` tools (canonical naming per cycle 2 #8) dispatch to ability-backed current-context queries; legacy snapshot read code paths removed (or wrapped with deprecation warnings); response shape includes provenance + freshness per ADR-0108; `cargo test` + `cargo clippy -D warnings` clean.

### Agent W2-B — DOS-172: Pagination + ranked discovery

- **Spec:** [DOS-172](https://linear.app/a8c/issue/DOS-172).
- **Goal:** Entity discovery tools (account list, person list, project list) get cursor-based pagination + ranked discovery so host workflows stay fast on large portfolios. **Cycle 1 amendment (preempt v1.4.5 cycle 8 lesson):** pagination cursor format frozen at L0 plan as opaque base64 (same convention as v1.4.5 W3-C `WorkspaceGraphResponse`). Page size capped at 200 default, 500 max.

 succeeded in 0ms:
# ADR-0102: Abilities as the Runtime Contract

**Status:** Accepted  
**Date:** 2026-04-18  
**Amended:** 2026-05-10 — extended `AbilityPolicy.allowed_actors` to include `SurfaceClient` (defined in [ADR-0111](0111-surface-independent-ability-invocation.md) §8); added §7.6 specifying SurfaceClient policy and introspection semantics. Per the formal L0 panel on [DOS-546](https://linear.app/a8c/issue/DOS-546).
**Amended (later same day):** 2026-05-10 — extended `AbilityPolicy` schema with `required_scopes: Vec<SurfaceClientScope>` and `mcp_exposure: bool` as first-class fields per the formal L0 Cycle 2 unanimous finding. Two-level SurfaceClient enforcement is now substrate-enforceable from the canonical policy, not just prose in §7.6. Default policy expanded: `required_scopes = vec![]`, `mcp_exposure = false`.
**Amended (later same day):** 2026-05-10 — promoted `mcp_exposure` from `bool` to tri-state `McpExposure { None | MetadataOnly | Invocable }` enum per Phase 0 artifact 05 lines 383-412; retained `client_side_executable: bool` as an orthogonal separate field for the WordPress 7.0 client-side `executeAbility()` JS path. The two fields govern different trust boundaries — `mcp_exposure` controls the network-facing MCP tool surface (discovery + invocation), `client_side_executable` controls whether a trusted in-product SurfaceClient may invoke the ability after WordPress capability + runtime scope + actor checks pass. The fields are independent; either may be true while the other is false. Default policy is `mcp_exposure = McpExposure::None` + `client_side_executable = false`, opt-in per ability. This cycle-3 reversal of the cycle-2 collapse was made after re-reading artifact 05's explicit warning against the symmetric `client_side_exposure` framing.  
**Amended:** 2026-05-19 — added `Actor::McpClient` actor variant + `ActorKind::McpClient` discriminator for MCP-originated calls; pinned MCP client authentication + session contract; froze the v1.4.7 scope namespace convention. See §"Amendment 2026-05-19 — `Actor::McpClient` + MCP client trust boundary + scope namespace" appended at end of doc. Companion ADR-0128 amendment of the same date documents the headless-head consequences (tool description as product copy, cross-conversation continuity, displacement-use-case framing applied to v1.4.7 W5 eval).
**Target:** v1.4.0  
**Extends:** [ADR-0101](0101-service-boundary-enforcement.md), [ADR-0091](0091-intelligence-provider-abstraction.md), [ADR-0082](0082-entity-generic-prep-pipeline.md)  
**Related:** [ADR-0080](0080-signal-intelligence-architecture.md), [ADR-0097](0097-account-health-scoring-architecture.md), [ADR-0098](0098-data-governance-source-aware-lifecycle.md), [ADR-0100](0100-glean-first-intelligence-architecture.md), [ADR-0111](0111-surface-independent-ability-invocation.md), [ADR-0129](0129-composable-surfaces-wordpress-studio-as-primary-surface.md), [ADR-0130](0130-surface-independent-composition-contract.md) §2 (block-level `ProvenanceRef` preserves the ADR-0105 §8 lives-once invariant and avoids payload-cap blowups when ability outputs are composed into surface-rendered blocks)

## Context

DailyOS delivers depth-of-context before every interaction. A user walking into a meeting receives not a summary of their calendar but a briefing shaped by the full state of the relationships, accounts, commitments, and signals relevant to that moment. This is the product's differentiation and its moat: depth compounds over time as more signals flow in, more context accumulates, and more relationships are understood.

Depth of this kind cannot be assembled on demand, feature by feature. It has to be maintained continuously and composed consistently. Today, every feature in the product assembles context in its own command handler: meeting prep queries one set of tables, daily readiness queries another, the MCP sidecar queries a third. Context builders (`build_intelligence_context`, `gather_account_context`) are shared helpers but not the single invocation path. The result:

- New signal sources require touching every feature that should benefit from them.
- Consumers receive subtly different context depending on how they entered the system (app vs. MCP vs. background worker).
- Capability quality cannot be measured at the capability level, because capabilities are not first-class units.
- Prompts, synthesis logic, and provenance are distributed across the codebase rather than owned by a capability.

The substrate ADRs — services ([ADR-0101](0101-service-boundary-enforcement.md)), signals ([ADR-0080](0080-signal-intelligence-architecture.md)), entities ([ADR-0082](0082-entity-generic-prep-pipeline.md)), providers ([ADR-0091](0091-intelligence-provider-abstraction.md)), data governance ([ADR-0098](0098-data-governance-source-aware-lifecycle.md)), Glean-first retrieval ([ADR-0100](0100-glean-first-intelligence-architecture.md)) — establish how state is mutated, how signals flow, how entities are modeled, and how intelligence is retrieved. None of them establishes how a **product capability** is shaped, named, versioned, or exposed. That layer is missing.

This ADR introduces it.

## Decision

Every product capability in DailyOS is implemented as an **ability**: a named, typed, versioned function that synthesizes state and signals into a structured output. All consumers — the desktop app, the MCP server, background workers, and any future surface — invoke capabilities exclusively through the abilities layer. The abilities layer is the runtime contract of the product.

### 1. What an Ability Is

An ability is a single function that:

- Has a stable, public name in `verb_subject` snake_case form (`prepare_meeting`, `get_entity_context`, `detect_risk_shift`).
- Takes a typed input struct and an `AbilityContext`.
- Returns a typed output wrapped in `AbilityOutput<T>`, which carries the domain output alongside a mandatory provenance envelope. Provenance shape is specified in [ADR-0105](0105-provenance-as-first-class-output.md). The domain output itself does not re-embed provenance.
- Corresponds to a concrete product capability visible to a user or an agent. Abilities are not computational primitives; they are the things a chief of staff does.

**Boundary scope.** The abilities layer governs synthesized user-facing and agent-facing outputs — briefings, narratives, entity context, risk detection, curated publications. Purely mechanical operations (list entity names for a picker, format a timestamp, read a single field for a toolbar badge) remain in their owning service or command and are not abilities. A good heuristic: if an output is structured, composed from multiple sources, or involves synthesis, it is an ability. If it is a single read or a trivial projection, it is not.

An ability is not:

- A bare CRUD function (those belong in `services/`, per [ADR-0101](0101-service-boundary-enforcement.md)).
- A prompt template (those are internal to the ability that owns them).
- A context builder (those are composed by abilities but are not themselves abilities).

### 2. Ability Location and Module Structure

Abilities live in `src-tauri/src/abilities/`:

```
src-tauri/src/abilities/
├── mod.rs                      # Registry + AbilityContext
├── context.rs                  # AbilityContext definition
├── provenance.rs               # Provenance types (detail in ADR-0105)
├── result.rs                   # AbilityResult, AbilityError
├── read/
│   ├── get_entity_context.rs
│   ├── get_daily_readiness.rs
│   ├── list_open_loops.rs
│   └── explain_salience.rs
├── transform/
│   ├── prepare_meeting.rs
│   ├── detect_risk_shift.rs
│   ├── generate_weekly_narrative.rs
│   └── summarize_recent_changes.rs
├── publish/
│   ├── publish_to_p2.rs
│   └── export_report.rs
└── maintenance/
    ├── refresh_entity_state.rs
    ├── reconcile_signals.rs
    └── repair_graph_links.rs
```

Each ability is a single module — typically one file, but a directory is permitted for abilities large enough to warrant submodules (e.g., `prepare_meeting/` containing `mod.rs`, `prompts.rs`, `synthesis.rs`, `tests.rs`). The module has one public entry point (the ability function) and opaque internals. Eval fixtures live under `src-tauri/tests/abilities/{ability_name}/` (detail in [ADR-0107](0107-evaluation-harness-for-abilities.md)).

### 3. Ability Categories

Abilities belong to one of four categories. **Category is determined by the ability's call-graph effect on services, not by author preference.** A static check (runtime during Phase 3, compile-time during Phase 4) enforces the classification: an ability whose transitive service calls include mutations is a Publish or Maintenance ability; one whose calls are read-only is a Read or Transform ability.

| Category | Call-graph effect | Consumes LLM? | Examples |
|----------|------------------|---------------|----------|
| **Read** | No service mutation anywhere in the call graph. May emit ephemeral logs and telemetry but never writes domain state or emits propagating signals. | Usually no | `get_entity_context`, `list_open_loops`, `get_daily_readiness` |
| **Transform** | No service mutation anywhere in the call graph. May invoke the intelligence provider. Results must not be used to authorize mutation without an additional trust signal (see §12). | Yes | `prepare_meeting`, `detect_risk_shift`, `generate_weekly_narrative` |
| **Publish** | Writes externally (third-party APIs, file exports). Requires explicit user or policy confirmation. | Sometimes | `publish_to_p2`, `export_report` |
| **Maintenance** | Mutates internal state through services. Subject to [ADR-0103](0103-maintenance-ability-safety-constraints.md). | Sometimes | `refresh_entity_state`, `reconcile_signals` |

Category consequences:

- **Read** abilities are idempotent and safe to cache by a composite key that includes `ctx.user`, `ctx.services.clock.now()` (bucketed), provider settings, Glean connection state, and DB watermarks. Caching keyed solely on input is forbidden — outputs depend on ambient context.
- **Transform** abilities are eval-gated on output quality (see [ADR-0107](0107-evaluation-harness-for-abilities.md)). Their outputs are marked untrusted by default (§10); they cannot authorize mutation on their own, but they can participate in mutation-authorizing composition when paired with a trust signal (user confirmation, policy pre-authorization, or schema-constrained value).
- **Publish** abilities require a `ConfirmationToken` from the invoking surface. A maintenance ability cannot call a publish ability unless its registry policy explicitly grants `may_publish = true`, which requires an ADR amendment.
- **Maintenance** abilities are bound by [ADR-0103](0103-maintenance-ability-safety-constraints.md). Contributors writing a maintenance ability MUST read ADR-0103; the constraints are non-optional.

**Leakage is enforced, not trusted.** If a Read or Transform ability accidentally introduces a mutation (through an imported service function or a composed ability), the call-graph check flags it at registration time and the registry refuses to bind the ability.

### 4. Ability Signature

Every ability has the same shape:

```rust
pub async fn prepare_meeting(
    ctx: &AbilityContext,
    input: PrepareMeetingInput,
) -> AbilityResult<MeetingBrief> {
    // 1. Gather: compose read abilities + signal queries
    // 2. Synthesize: invoke IntelligenceProvider where needed
    // 3. Attribute: attach provenance to each output field
    // 4. Return: typed output + provenance
}
```

`AbilityResult<T>` is `Result<AbilityOutput<T>, AbilityError>`, where `AbilityOutput<T>` wraps the domain output with required provenance, version metadata, and diagnostics.

### 5. AbilityContext

`AbilityContext` is the single dependency entry point for abilities, analogous to `ServiceContext` ([ADR-0101](0101-service-boundary-enforcement.md)) but operating one layer higher. `ServiceContext` itself is amended to carry `ExecutionMode` in [ADR-0104](0104-execution-mode-and-mode-aware-services.md), which is a prerequisite to this ADR.

```rust
pub struct AbilityContext<'a> {
    pub services: &'a ServiceContext<'a>,      // Carries DB, signals, queue, mode, clock, RNG — see ADR-0104
    pub intelligence: &'a dyn IntelligenceProvider,  // ADR-0091; scope expansion noted in §12
    pub user: &'a UserEntity,                   // ADR-0089
    pub tracer: &'a AbilityTracer,              // Observability and audit capture
    pub actor: Actor,                           // Who invoked this ability (User | Agent | System)
    pub confirmation: Option<ConfirmationToken>, // Present when user/policy confirmed; required for Publish
}

exec
/bin/zsh -lc "sed -n '1,115p' .docs/decisions/0111-surface-independent-ability-invocation.md" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
# ADR-0111: Surface-Independent Ability Invocation

**Status:** Proposed  
**Date:** 2026-04-18  
**Amended:** 2026-05-10 — added §8 introducing `SurfaceClient` as a fourth canonical actor class for third-party local surfaces (WordPress, Obsidian, browser extensions, future PWA / mobile). Per the formal L0 panel on [DOS-546](https://linear.app/a8c/issue/DOS-546) (Codex Pass 3 STRIDE review + Pass 1 finding on canonical actor mapping). Resolves: `SurfaceClient` is generalized, not platform-coupled; WordPress is one concrete instance.  
**Target:** v1.4.0  
**Extends:** [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0027](0027-mcp-dual-mode.md)  
**Depends on:** [ADR-0104](0104-execution-mode-and-mode-aware-services.md), [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0108](0108-provenance-rendering-and-privacy.md)

## Context

[ADR-0102](0102-abilities-as-runtime-contract.md) §7 specifies the ability registry and declares that surfaces invoke abilities through a single path. §10 Phase 2 requires migrating app Tauri commands and MCP sidecar handlers to wrap ability invocations. The existing MCP sidecar ([ADR-0027](0027-mcp-dual-mode.md)) has its own query logic and data access path; migrating it requires concrete binding rules.

This ADR specifies the exact binding pattern for Tauri commands and MCP tools, how abilities are generated into both surfaces from the registry, and how surfaces construct `AbilityContext` with mode, actor, and provenance renderer wiring.

## Decision

### 1. Surface Construction Rule

Each surface holds exclusive responsibility for constructing `AbilityContext` for its invocations. Abilities never construct their own context; composed abilities inherit their caller's context. The surfaces are:

| Surface | Actor | Default Mode | Context constructed by |
|---------|-------|--------------|------------------------|
| Tauri app | `User` | `Live` (or `Live + dry_run` for maintenance) | `TauriAbilityBridge` |
| MCP server | `Agent` | `Live` (never escalated) | `McpAbilityBridge` |
| Background worker | `System` | `Live` | `WorkerAbilityBridge` |
| Evaluation harness | `System` | `Evaluate` | `EvalAbilityBridge` per [ADR-0110](0110-evaluation-harness-for-abilities.md) |
| Integration tests | declared | `Simulate` or `Live` (test DB) | `TestAbilityBridge` per test |
| `SurfaceClient` instance (e.g. WordPress plugin, Obsidian plugin, browser extension) | `SurfaceClient` | `Live` (never escalated) | `SurfaceClientBridge` per instance, parameterized by instance identity + scope grants (per §8) |

Each bridge owns construction logic but invokes abilities through the shared registry. There is one invocation entry point per bridge; no ability-specific code in the bridge.

### 2. Tauri Command Binding

v1.4.0 ships a single generic Tauri command, `invoke_ability`, that dispatches by name:

```rust
#[tauri::command]
pub async fn invoke_ability(
    state: tauri::State<'_, AppState>,
    ability_name: String,
    input_json: serde_json::Value,
    dry_run: Option<bool>,
    confirmation: Option<ConfirmationToken>,
) -> Result<AbilityResponseJson, String> {
    let bridge = state.tauri_bridge();
    bridge.invoke(
        &ability_name,
        input_json,
        InvocationContext {
            actor: Actor::User,
            mode: ExecutionMode::Live,
            dry_run: dry_run.unwrap_or(false),
            confirmation,
        },
    ).await
}
```

The bridge:

1. Looks up the ability descriptor by name in the registry.
2. Validates `input_json` against the ability's input JSON schema.
3. Checks actor policy (`User` is allowed).
4. Constructs `AbilityContext` with Live mode, User actor, and the app's production services.
5. Invokes the ability through the registry's erased wrapper.
6. Renders provenance via [ADR-0108](0108-provenance-rendering-and-privacy.md)'s `render_provenance_for(prov, User, TauriApp)`.
7. Returns `AbilityResponseJson` containing domain output plus rendered provenance.

**Per-ability typed Tauri commands** are not generated in v1.4.0. The single `invoke_ability` command is ergonomic enough for the app's internal call sites (which can define thin TypeScript wrappers) and avoids generating ~20 commands per ability migration. Per-ability commands may be added later if type ergonomics become a pain point.

### 3. MCP Tool Registration

The MCP server iterates the registry at startup and registers one MCP tool per ability whose `AbilityPolicy.allowed_actors` includes `Agent`:

```rust
for descriptor in AbilityRegistry::iter().filter(|d| d.policy.allowed_actors.contains(&Actor::Agent)) {
    mcp_server.register_tool(McpTool {
        name: descriptor.name,
        description: descriptor.description,
        input_schema: descriptor.input_schema.clone(),
        handler: Box::new(McpAbilityHandler::new(descriptor.clone())),
    });
}
```

Each `McpAbilityHandler`:

1. Receives the agent's invocation with JSON inputs.
2. Validates inputs against the ability's schema.
3. Checks actor policy (`Agent` is allowed).
4. Refuses invocation if `allowed_modes` does not include `Live` with no additional surface (agents do not escalate to `Evaluate` or `Simulate`).
5. Constructs `AbilityContext` with Live mode, Agent actor, production services.
6. Invokes the ability through the registry.
7. Renders provenance via [ADR-0108](0108-provenance-rendering-and-privacy.md)'s `render_provenance_for(prov, Agent, McpTool)`.
8. Returns the MCP response with domain output and filtered provenance.

**No hand-written MCP tool handlers.** Every MCP tool is an ability. Pre-existing MCP tools from [ADR-0027](0027-mcp-dual-mode.md) that do not correspond to an ability are either migrated to an ability or removed during Phase 3 cutover.

### 4. Actor-Filtered Discovery

MCP tool discovery (the `list_tools` endpoint) is filtered by actor. An agent enumerating the server's tools sees only abilities whose policy permits `Agent`. Maintenance abilities, admin-only abilities, and abilities that specifically exclude agents never appear in the list.

The Tauri bridge has a complementary `list_abilities(actor: Actor)` endpoint for the app's UI when it needs to enumerate capabilities (e.g., the Maintenance settings panel lists maintenance abilities; a developer panel lists all).

### 5. Background Worker Binding

The scheduled maintenance worker uses its own bridge:

```rust
impl WorkerAbilityBridge {
    pub async fn run_scheduled(&self, schedule: &MaintenanceSchedule) {
        for entry in schedule.due_now() {
            let descriptor = self.registry.get(entry.ability_name).expect("scheduled ability must exist");
            let ctx = AbilityContext::new(

exec
/bin/zsh -lc "sed -n '1,120p' .docs/decisions/0108-provenance-rendering-and-privacy.md && sed -n '1,90p' .docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md" in /private/tmp/dailyos-v147-w1a
 succeeded in 0ms:
# ADR-0108: Provenance Rendering and Privacy

**Status:** Proposed  
**Date:** 2026-04-18  
**Target:** v1.4.0  
**Extends:** [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0107](0107-source-taxonomy-alignment.md)  
**Related:** [ADR-0094](0094-audit-log-and-enterprise-observability.md), [ADR-0093](0093-prompt-injection-hardening.md), [ADR-0111](0111-surface-independent-ability-invocation.md), [ADR-0130](0130-surface-independent-composition-contract.md) §2 (block-level `ProvenanceRef` references this ADR's actor-filtered envelope; the ref shape avoids exceeding the rendering-side payload caps this ADR governs)

## Context

[ADR-0105](0105-provenance-as-first-class-output.md) defines the `Provenance` envelope and requires surfaces to render it. It defers three concerns to this ADR:

1. **Rendering rules per surface** — what the Tauri app, MCP, P2 publications, and logs show.
2. **Privacy and safety** — what gets masked, redacted, or filtered when rendered to different actors or published externally.
3. **The `ProvenanceMasked` shape** referenced by [ADR-0105](0105-provenance-as-first-class-output.md) §8 and used when sources are revoked.

[ADR-0094](0094-audit-log-and-enterprise-observability.md) governs the append-only JSONL security audit with strict PII hygiene. [ADR-0093](0093-prompt-injection-hardening.md) established that LLM outputs can contain adversarial content. This ADR integrates both: provenance must be renderable to users and agents without leaking internal graph structure or PII, and sanitization of LLM-generated `explanation` text must prevent re-injection.

## Decision

### 1. Per-Surface Rendering Rules

**Tauri app (first-party).**

- Every user-facing ability output shows an "About this" affordance (previously described as "Why?"). The affordance is conditional: display it when the output is a composed brief, a narrative, a risk assessment, or any LLM-synthesized content. Do not display it on compact badges, tooltip strings, or single-field projections.
- Expanding the affordance shows: ability name, produced_at timestamp (relative + absolute), number of sources, trust assessment (Trusted / Untrusted with reason), composition depth summary.
- A secondary "Details" expansion shows field-level attributions: for each significant output field, the sources that contributed, their `data_source` (using human-readable names via [ADR-0107](0107-source-taxonomy-alignment.md)'s `DataSource::display_name()`), and any `confidence` score.
- A third-level "Full provenance" expansion shows the JSON envelope for power users and debugging.
- Sources whose `SynthesisMarker.producer_ability` is set display a visible "AI-generated" badge.

**MCP server (agent-facing).**

- Every tool response includes provenance by default. The `Provenance` envelope serializes as a structured field alongside the domain output.
- Actor-filtered contents: the MCP wrapper strips content from provenance that is not permitted for `Actor::Agent`:
  - Source identifiers that reference internal graph IDs (signal_id, entity_id, document chunk IDs) are replaced with `SourceIdRedacted { data_source, scoring_class }`.
  - Child provenance trees are collapsed to `ChildElided { ability_name, data_sources_summary }` beyond depth 2.
  - Prompt fingerprints include `provider`, `model`, `prompt_template_id`, `prompt_template_version`, but NOT `canonical_prompt_hash` (internal) or `seed`.
  - `FieldAttribution.explanation` text is returned as-is but is sanitized per §3.
- An MCP tool `get_provenance(invocation_id)` provides a fuller view for authorized agents if needed; default response is summary-sized.

**P2 publications.**

- Publications carry a provenance footnote listing the top-level sources by `data_source` class (e.g., "Based on REDACTED, recent meetings, and team activity") — **no source identifiers, no entity names, no attribution text**.
- Expandable "Details" block is available in the publication renderer but only if the publishing user confirms at publish time that the detail level is appropriate for the P2 audience. This is an explicit confirmation in the publish dialog, not a default.
- Synthesis markers render as "AI-generated summary" prefix on the publication body.
- Trust classification does not render to external readers; it is internal operational metadata.

**Logs and structured telemetry.**

- The `maintenance_audit` table stores the full `ProvenanceOrMasked` per invocation.
- Application logs reference `invocation_id` and do not include provenance content; to retrieve a run's provenance, query the audit table.
- [ADR-0094](0094-audit-log-and-enterprise-observability.md)'s append-only JSONL security audit remains unchanged — it captures security events (auth, permissions, source revocations, etc.) with its existing hygiene rules. Provenance lives in the operational `maintenance_audit` SQLite table, separate from the security audit.

### 2. Actor-Filtered Rendering

A universal renderer transforms `Provenance` into an actor-appropriate view:

```rust
pub fn render_provenance_for(prov: &Provenance, actor: Actor, surface: Surface) -> RenderedProvenance { ... }

pub enum Surface {
    TauriApp,         // Full detail, three-level expansion
    McpTool,          // Actor-filtered, default-summary
    McpToolDetail,    // Actor-filtered, full-detail (invoked via get_provenance)
    P2Publication,    // External audience, heavy redaction
    LogStructured,    // Operator view, no PII
}
```

The renderer consults `actor` and `surface` to determine what to include. A user rendering provenance in the app sees the full graph; an agent accessing the same ability's output via MCP sees a filtered version; a P2 publication sees source classes only.

**Agents never see:** internal signal IDs, entity IDs (unless explicitly part of the response schema), raw `prompt_template` text, internal `canonical_prompt_hash`, internal entity version watermarks. These are operational internals; surfacing them to agents gives attackers a map of the user's graph.

### 3. Sanitization of `FieldAttribution.explanation`

LLM-generated `explanation` text is untrusted per [ADR-0093](0093-prompt-injection-hardening.md). It can contain prompt injection attempts, PII from other sources, or links to attacker-controlled content. Every explanation is passed through a sanitizer before rendering:

1. **HTML entities encoded.** Any `<`, `>`, `&`, `"`, `'` is entity-escaped. No raw HTML or Markdown links rendered from explanations.
2. **Length bounded.** Explanations over 500 characters are truncated with an ellipsis. Long explanations are a smell.
3. **URL stripping.** Anything resembling a URL (regex-based) is removed and replaced with `[url removed]`. If the explanation wants to cite a URL, it must come through a typed `SourceAttribution` with a URL field, not through free text.
4. **Banned token list.** Explanations containing instruction-like phrases common in prompt injection attempts (e.g., "ignore previous", "you are now", "system:") are filtered: the whole explanation is replaced with `[explanation removed by sanitizer]` and a `ProvenanceWarning::ExplanationFiltered` is added.
5. **No executable content.** Regardless of surface, explanations are never passed to a markdown renderer, a JS `eval` context, or any other dynamic evaluator.

The sanitizer is shared across all surfaces; the rendering layer calls it once per `FieldAttribution` before composing the rendered output.

### 4. The `ProvenanceMasked` Shape

When a source is revoked and its associated provenance records are masked per [ADR-0107](0107-source-taxonomy-alignment.md) §5:

```rust
pub struct ProvenanceMasked {
    pub original_invocation_id: InvocationId,
    pub original_ability_name: &'static str,
    pub original_produced_at: DateTime<Utc>,
    pub masked_at: DateTime<Utc>,
    pub mask_reason: MaskReason,
    pub sources_masked: Vec<DataSource>,     // Which sources triggered the masking
}

pub enum MaskReason {
    SourceRevoked { data_source: DataSource },
    GleanDisconnected,
    UserDeletedEntry,
    RetentionExpired,
}
```

The `ProvenanceOrMasked` enum in [ADR-0103](0103-maintenance-ability-safety-constraints.md) §8's `MaintenanceAuditRecord` is either the full envelope or this masked placeholder. Consumers handle both.

**Masking is irreversible.** Once a provenance is masked, the original content is discarded. Re-running the ability after the source is re-connected produces a new invocation with its own provenance. This is the cost of the mask — diagnosis of past runs becomes unavailable for revoked sources, but user privacy is protected.

### 5. Size Budgets for Rendering

[ADR-0105](0105-provenance-as-first-class-output.md) §9 specifies storage size budgets. Rendering has its own:

- **Tauri app initial render:** top-level summary ≤ 2KB. Expansion fetches additional detail on demand.
- **MCP default response:** provenance payload ≤ 10KB. Full detail available via `get_provenance(invocation_id)`.
- **P2 publication footnote:** ≤ 500 characters of human-readable text summarizing sources.

When rendering exceeds a budget, the renderer truncates deepest-first (children elided first, sources last) and surfaces a `ProvenanceWarning::TruncatedForRender { surface, budget_bytes, original_bytes }`.
# ADR-0125: Claim Anatomy v2 — Temporal Scope, Sensitivity, Claim Type Registry

**Status:** Accepted (substrate primitives for v1.4.0 spine; downstream enforcement in v1.4.1+)
**Date:** 2026-04-24
**Amended:** 2026-05-19 — added §5 `RecommendationClaim` variant for v1.4.6 Salience & Recommendations. Registers `ClaimType::Recommendation` in `CLAIM_TYPE_REGISTRY` (canonical persisted name `"recommendation"`) with `State` temporal scope, `Internal` sensitivity, `Medium` freshness decay, `Replace` commit policy, agent-only authorship, attaches to `Account` / `Project` / `Person` subjects. Salience factors, user-feedback state (`Accepted` / `Dismissed` / `NotUseful` / `TooNoisy` / `Converted`), conversion state, and supersession + dismissal lifecycle are declared at the substrate level; concrete DTOs land in v1.4.6 W1-A `services/recommendations/contracts.rs`. Per the v1.4.6 W0 hardening cycle in [`.docs/plans/v1.4.6-waves.md`](../plans/v1.4.6-waves.md).
**Target:** v1.4.0 (schema + enums + registry); v1.4.1 (DOS-10 freshness + DOS-214 render policy); v1.4.2/3 (per-surface enforcement); v1.4.6 (Recommendation variant)
**Extends:** [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md), [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0114](0114-scoring-unification.md)
**Pattern parallel:** [ADR-0115](0115-signal-granularity-audit.md) Signal Policy Registry

## Context

The 2026-04-24 Claim Anatomy Review (`.docs/plans/claim-anatomy-review-2026-04-24.md`) walked every dimension a claim should express in the v1.4.x substrate. Three dimensions were under-scrutinized at the substrate level and warrant a substrate primitive in v1.4.0 spine:

1. **Temporal scope** (Review §11) — distinguishes PointInTime / State / Trend claims; affects freshness, supersession, and contradiction semantics.
2. **Sensitivity** (Review §12) — claim-level sensitivity tier; structural safeguard against accidental surface leakage.
3. **Claim type taxonomy** (Review §15) — currently free-form strings; should be a closed-but-extensible registry following the ADR-0115 Signal Policy Registry pattern.

This ADR is the substrate allowance: schema fields + types + registry mechanism. Downstream enforcement (freshness scoring, render gates) is named per-dimension and lands in v1.4.1+.

## Decision

### 1. Temporal scope

```rust
pub enum TemporalScope {
    /// Claim is about an event that happened at a moment in time.
    /// Example: "Bob said X in 4/23 meeting." Doesn't go stale — the event occurred.
    /// Freshness: never decays.
    /// Supersession: cannot be superseded (different events are different facts).
    /// Contradiction: usually doesn't contradict another PointInTime; may contradict a State.
    PointInTime { occurred_at: DateTime<Utc> },

    /// Claim is about a persistent condition until contradicted.
    /// Example: "Bob is the champion." Has a half-life.
    /// Freshness: decays per DOS-10 table.
    /// Supersession: a newer State at same field_path supersedes.
    /// Contradiction: State + State with different values is a contradiction.
    State,

    /// Claim is about evolution over time.
    /// Example: "Engagement has been declining over the last 30 days."
    /// Freshness: based on recency of latest underlying data point.
    /// Supersession: recomputed; new trend on fresher data supersedes.
    /// Contradiction: another Trend with opposite direction at same field_path.
    Trend { window_start: DateTime<Utc>, window_end: DateTime<Utc> },
}
```

Schema allowance:

```sql
ALTER TABLE intelligence_claims ADD COLUMN temporal_scope TEXT NOT NULL DEFAULT 'state';
```

v1.4.0 spine ships the column + enum + serde impls. Default `State` for behavioral parity with current implicit assumption. v1.4.1 DOS-10 freshness factor consults `temporal_scope` to apply the right decay rule.

### 2. Sensitivity

```rust
pub enum ClaimSensitivity {
    /// Fine on customer-facing surfaces.
    /// Example: public press release content; product feature mentions.
    Public,

    /// Fine on user's own surfaces, briefings, MCP. Never customer-facing or shared/published.
    /// Example: CRM notes, internal Slack discussions.
    /// **Default for claims sourced from internal systems.**
    Internal,

    /// Fine on the user's own surfaces only. Not on MCP, not on Publish, not on briefings shared with others.
    /// Example: personal notes about a stakeholder's challenges.
    Confidential,

    /// Only the user themselves; not even trusted human analysts in multi-user (future).
    /// Example: legal-confidential, HR-sensitive content.
    UserOnly,
}
```

Schema allowance:

```sql
ALTER TABLE intelligence_claims ADD COLUMN sensitivity TEXT NOT NULL DEFAULT 'internal';
```

Default `Internal` is the conservative choice — most claims sourced from internal systems should default Internal. `Public` requires explicit author choice or source-class inheritance. `Confidential` / `UserOnly` require explicit author choice.

v1.4.0 spine ships the column + enum. Render-time gates land per-surface in v1.4.1+ (see §4).

### 3. Claim type registry

codex
VERDICT: NEEDS-CHANGES

Findings:
- HIGH — §4 deferral of `dailyos.read.meeting_briefing` is not internally consistent with DOS-624. DOS-624 says W2-A ships both `daily_briefing` and `meeting_briefing`, and says DOS-175 MUST enumerate §3 ACs. Wave plan W2-A also says done means both briefing tools dispatch.
- HIGH — Packet adds `account_status`, but wave W2-A owns `tool_briefing.rs` and is scoped to replacing stale briefing reads. This matches the user outcome, but contradicts the wave plan.
- MED — AC/test mismatch: AC-8 says no cross-tool rejection test; test plan says unauthorized `daily_briefing` is tested.
- K-in done: `docs/solutions/` + `.docs/decisions/` grep confirms substrate exists (`ADR-0102`, `ADR-0108`, `ADR-0111`, `ADR-0125`). No reinvention concern.

Recommended scope tightening:
Ship only `dailyos.read.account_status` for the stated Claude Desktop → Acme overview outcome, and explicitly amend/split DOS-175/DOS-624 wave scope. If DOS-175 remains the briefing-replacement lane, include `meeting_briefing` or don’t approve.
tokens used
86,422
VERDICT: NEEDS-CHANGES

Findings:
- HIGH — §4 deferral of `dailyos.read.meeting_briefing` is not internally consistent with DOS-624. DOS-624 says W2-A ships both `daily_briefing` and `meeting_briefing`, and says DOS-175 MUST enumerate §3 ACs. Wave plan W2-A also says done means both briefing tools dispatch.
- HIGH — Packet adds `account_status`, but wave W2-A owns `tool_briefing.rs` and is scoped to replacing stale briefing reads. This matches the user outcome, but contradicts the wave plan.
- MED — AC/test mismatch: AC-8 says no cross-tool rejection test; test plan says unauthorized `daily_briefing` is tested.
- K-in done: `docs/solutions/` + `.docs/decisions/` grep confirms substrate exists (`ADR-0102`, `ADR-0108`, `ADR-0111`, `ADR-0125`). No reinvention concern.

Recommended scope tightening:
Ship only `dailyos.read.account_status` for the stated Claude Desktop → Acme overview outcome, and explicitly amend/split DOS-175/DOS-624 wave scope. If DOS-175 remains the briefing-replacement lane, include `meeting_briefing` or don’t approve.
