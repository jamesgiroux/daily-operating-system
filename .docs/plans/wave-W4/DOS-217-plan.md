# Implementation Plan: DOS-217

## Revision history
- v1 (2026-05-01) — initial L0 draft.

## 1. Contract restated

DOS-217 adds the bridge layer between the W3 ability registry and every runtime surface: Tauri app, MCP server, scheduled worker, and evaluation harness. Load-bearing Linear lines: "Implement `TauriAbilityBridge`, `McpAbilityBridge`, `WorkerAbilityBridge`, `EvalAbilityBridge`"; "`TauriAbilityBridge`: schema-validates input JSON, constructs `AbilityContext` with `Actor::User` + `Mode::Live`, invokes via registry, renders provenance for `Surface::TauriApp`, returns `AbilityResponseJson`"; "Generic `invoke_ability(ability_name, input_json, dry_run, confirmation)` Tauri command"; "`McpAbilityBridge`: iterates registry at startup; registers one MCP tool per ability with `Agent` in `allowed_actors`"; "`list_tools` MCP endpoint returns actor-filtered list; never exposes maintenance or admin-only"; and "`WorkerAbilityBridge`: scheduled maintenance invocation with `Actor::System`; only path for `global` blast radius scheduled runs."

The 2026-04-18 amendment applies: "**Test: Maintenance-category abilities invoked from `Actor::McpAgent` are rejected with `AbilityError::RequiresUserActor`.**" DOS-304's 2026-04-24 contract also applies: "`ServiceContext` capability handles are the enforcement boundary"; "Proc-macro AST inspection cannot be the hard safety boundary"; and "Do not build two registries and reconcile later." The resolution is locked: abilities are operations; MCP tools are registry-derived.

ADR pins: ADR-0111 fixes the four surfaces, actors, and modes (`.docs/decisions/0111-surface-independent-ability-invocation.md:21-29`), the generic Tauri command (`:33-66`), MCP startup registration and handler steps (`:70-96`), actor-filtered discovery (`:98-102`), worker/eval bindings (`:104-147`), and Phase 2/3 coexistence/cutover (`:150-162`). ADR-0102 says erased invocation is schema-checked at the registry boundary (`.docs/decisions/0102-abilities-as-runtime-contract.md:232-239`) and that registry enumeration must not leak unauthorized names/schemas (`:250-258`). ADR-0108 defines MCP/Tauri provenance rendering and redaction (`.docs/decisions/0108-provenance-rendering-and-privacy.md:31-72`).

Current repo reality: `src-tauri/src/bridges/` and `src-tauri/src/abilities/` do not exist today (`.docs/plans/wave-W3/DOS-211-plan.md:15`). Tauri command exports are centralized in `src-tauri/src/commands.rs:31-51`, with registration in `src-tauri/src/lib.rs:510-987`. Existing command shape uses `State<'_, Arc<AppState>>` and `state.live_service_context()` (`src-tauri/src/commands/core.rs:184-191`). The MCP binary is still a static, read-only, macro-generated sidecar: `#[tool(tool_box)]` starts at `src-tauri/src/mcp/main.rs:164`, `ServerHandler` delegates through that tool box at `:497-517`, and startup opens `ActionDb::open_readonly()` then serves stdio at `:640-666`. W2 frozen seams stay read-only for this plan: `ServiceContext` owns mode/clock/rng/external but no provider (`src-tauri/src/services/context.rs:271-277`), and `IntelligenceProvider::complete()` is the provider API (`src-tauri/src/intelligence/provider.rs:196-202`).

## 2. Approach

Create `src-tauri/src/bridges/` with `mod.rs`, `types.rs`, `tauri.rs`, `mcp.rs`, `worker.rs`, and `eval.rs`. `types.rs` owns `InvocationContext`, `AbilityResponseJson`, `AbilityInvokeError`, `BridgeActor`, `BridgeSurface`, and the bounded `InvocationProvenanceCache` for ADR-0108 `get_provenance(invocation_id)`. Add `pub mod bridges;` near `src-tauri/src/lib.rs:66-68`.

Create `src-tauri/src/commands/abilities.rs` for the single `#[tauri::command] invoke_ability`; add `mod abilities; pub use abilities::*;` in `src-tauri/src/commands.rs:31-51`; add `commands::invoke_ability` to the `generate_handler!` list near `src-tauri/src/lib.rs:510`. The command does no ability-specific branching: it rejects locked app state using the existing lock (`src-tauri/src/state.rs:153-158`, `src-tauri/src/commands/workspace.rs:813-817`), builds `InvocationContext { actor: User, mode: Live, surface: TauriApp, dry_run, confirmation }`, and delegates to `TauriAbilityBridge`.

Bridge algorithm for all surfaces:

1. Resolve descriptor from W3-A `AbilityRegistry` by name.
2. Reject if descriptor is experimental, actor-disallowed, mode-disallowed, category-disallowed for surface, or confirmation requirements are unmet.
3. Validate `input_json` against `AbilityDescriptor.input_schema` before dispatch; call W3-A's registry validation helper if exposed, otherwise share one bridge-local validator over the registry schema. The registry erased wrapper remains the second validation layer.
4. Construct `AbilityContext` using W3-A's constructor, W2 `ServiceContext` (`state.live_service_context()` for Tauri/worker, `ServiceContext::new_evaluate` for eval), and W2 `IntelligenceProvider` snapshot (`src-tauri/src/state.rs:246-281`, `:850-856`).
5. Invoke only `AbilityRegistry::invoke_by_name_json` / typed category entry points from W3-A (`.docs/plans/wave-W3/DOS-210-plan.md:24-30`); bridges contain no ability implementation.
6. Convert W3-B `AbilityOutput<T>` (`.docs/plans/wave-W3/DOS-211-plan.md:19-45`) into `AbilityResponseJson { invocation_id, ability_name, ability_version, schema_version, data, rendered_provenance, diagnostics }`.
7. Render provenance through the single ADR-0108 renderer: `Surface::TauriApp` for Tauri, `Surface::McpTool` for MCP default, `Surface::McpToolDetail` for `get_provenance`.

Modify `src-tauri/src/mcp/main.rs` from pure static tool-box use to a hybrid Phase 2 server: keep the current mechanical read tools temporarily, but override `ServerHandler::list_tools` and `call_tool` manually. The local rmcp 0.1.5 trait explicitly supports these dynamic methods (`~/.cargo/registry/src/.../rmcp-0.1.5/src/handler/server.rs:184-197`), and `Tool` is just `{ name, description, input_schema }` (`.../rmcp-0.1.5/src/model/tool.rs:10-20`). `McpAbilityBridge::tool_descriptors(Actor::Agent)` appends registry-derived tools, and `call_tool` first routes ability names through `McpAbilityBridge`; existing static tools remain only on an explicit Phase 2 allowlist.

Add a bridge-owned MCP `get_provenance` tool as the ADR-0108 exception, not a capability-level hand-written tool. It is authorized by `Actor::Agent`, looks up only invocation IDs previously returned to that MCP session or present in an authorized maintenance audit row, and renders through `Surface::McpToolDetail`. Default tool responses include summary provenance by value; detail fetch is bounded and audited by invocation id only.

Add a CI drift guard script, likely `scripts/check_ability_surface_drift.sh`, and wire it beside existing boundary checks in `.github/workflows/test.yml:62-79`. It blocks new hand-written capability-level Tauri commands under `src-tauri/src/commands/` and new hand-written MCP `#[tool]` handlers outside the Phase 2 allowlist. Mechanical reads and `get_provenance` stay allowlisted until Phase 3.

End-state alignment: this makes the W3 registry the only operation catalog across app, MCP, worker, and eval. It forecloses a second MCP/Tauri operations registry, actor-selected capability handles, and surface code that bypasses schema/provenance policy.

## 3. Key decisions

Input validation: schema comes from W3-A `AbilityDescriptor.input_schema`, generated from ability input types (`.docs/plans/wave-W3/DOS-210-plan.md:24-30`). There is no per-surface or per-ability ad hoc validator. Tauri may inject the current `schema_version` only if W3-A exposes that defaulting API, per ADR-0102's internal-caller allowance (`.docs/decisions/0102-abilities-as-runtime-contract.md:174-179`); MCP rejects missing `schema_version` because it is external.

Actor identity: caller-supplied JSON never chooses actor. The bridge chooses actor from the surface: Tauri command = `Actor::User`, MCP stdio server = `Actor::Agent`, worker schedule = `Actor::System`, eval harness = `Actor::System` in `Evaluate` mode, matching ADR-0111 (`.docs/decisions/0111-surface-independent-ability-invocation.md:21-27`). If W3-A adopts ADR-0113's richer agent identity, MCP uses a server-derived canonical label such as `agent:dailyos-mcp:<CARGO_PKG_VERSION>`, not a request header.

Capability-handle leakage: MCP discovery calls `registry.iter_for(Actor::Agent)` and caches only already-filtered descriptors; `call_tool` re-fetches and re-checks actor policy by name before invocation. Unauthorized abilities return the same non-enumerating error shape as unknown names. No descriptor, schema, blast radius, confirmation token, raw `ServiceContext`, or provenance handle is exposed to MCP.

Return type: success is typed `AbilityResponseJson`, not raw JSON. MCP serializes that envelope as JSON content inside `CallToolResult::success`, whose rmcp shape is `content + is_error` (`~/.cargo/registry/src/.../rmcp-0.1.5/src/model.rs:775-793`). Errors are typed internally, then redacted at the surface boundary; no prompt text, source snippets, or provider messages leave the bridge.

MCP registration: prefer manual `ServerHandler::list_tools/call_tool` over trying to generate rmcp macro methods from runtime inventory. The existing macro is static (`src-tauri/src/mcp/main.rs:164-177`); registry-derived tools require runtime enumeration.

Provider dispatch: bridges do not call `IntelligenceProvider::complete()` themselves except to pass the provider into `AbilityContext`. Ability code owns intelligence calls through W2-B's trait (`src-tauri/src/intelligence/provider.rs:196-202`). This keeps W2-B's seam frozen and avoids a bridge-level orchestration fork.

## 4. Security

Primary attack surface is actor-filtered discovery and invocation. Suite S must prove an MCP agent cannot enumerate or invoke Maintenance/admin/human-only abilities, even by guessing names. ADR-0103 default-denies Maintenance to agents (`.docs/decisions/0103-maintenance-ability-safety-constraints.md:197-221`), and global maintenance is never agent-invokable (`:225-245`). The synthetic Maintenance test from the Linear amendment is mandatory.

Tauri auth/authz: the command rejects while app lock is active (`src-tauri/src/commands/workspace.rs:813-817`), then relies on registry actor policy and confirmation tokens for privileged operations. This is a single-user local app; there is no separate admin identity, so user-confirmed global maintenance must use the typed `ConfirmationToken` path from ADR-0103 (`.docs/decisions/0103-maintenance-ability-safety-constraints.md:232-243`).

MCP auth/authz: current MCP binds to local stdio per ADR-0027 and opens read-only state today (`src-tauri/src/mcp/main.rs:1-6`, `:645-646`). W4-C must not treat local stdio as a human actor. MCP mode is always Live and never escalates to Simulate/Evaluate (`.docs/decisions/0104-execution-mode-and-mode-aware-services.md:243-254`).

Input JSON is schema-validated before dispatch and never deserialized by hand in the bridge. Confirmation is accepted only on Tauri/User paths; MCP rejects a `confirmation` field even if supplied. Provenance rendering follows ADR-0108: MCP strips internal IDs, prompt hashes, seeds, and deep child trees (`.docs/decisions/0108-provenance-rendering-and-privacy.md:31-40`, `:54-72`).

No secrets/PII in logs. Bridge logs include invocation id, ability name, actor, surface, mode, error kind, and duration only, aligning with ADR-0120 redaction (`.docs/decisions/0120-observability-contract.md:126-135`). `AbilityInvokeError` variants must not embed input JSON, output JSON, prompt/completion text, OAuth tokens, or raw provider errors.

## 5. Performance

Startup: MCP tool derivation is O(N) over filtered registry descriptors and should cache `Vec<Tool>` by actor. Registry validation itself is W3-A's O(N + E) startup work (`.docs/plans/wave-W3/DOS-210-plan.md:64-68`); W4-C must not duplicate graph validation.

Per invocation: expected overhead is one registry lookup, one schema validation, actor/mode/category checks, one erased JSON boundary, one provenance render, and optional recent-provenance cache insert. ADR-0111 budgets generic dispatch + schema validation + rendering at roughly 1-5ms for trivial calls (`.docs/decisions/0111-surface-independent-ability-invocation.md:190-192`), which is below synthesis-bound ability latency.

Cache/locks: use `AppState::context_snapshot()` for coherent provider/context reads (`src-tauri/src/state.rs:841-856`), not separate provider getters. Provenance detail cache is bounded by count and serialized-byte cap; MCP default provenance render must stay <=10KB per ADR-0108 (`.docs/decisions/0108-provenance-rendering-and-privacy.md:111-119`), while W3-B enforces the envelope cap.

No new SQL migration is planned. The bridge may read existing maintenance audit records for `get_provenance`; non-maintenance recent detail can live in the bounded in-memory cache unless W3-C supplies a durable provenance table first.

## 6. Coding standards

Services-only mutations: bridges do not mutate DB, files, signals, or external systems directly. They construct context and invoke the registry; ability implementations call services, and service mutators remain behind `ServiceContext::check_mutation_allowed()` (`src-tauri/src/services/context.rs:412-422`). Do not edit `src-tauri/src/services/context.rs` or `src-tauri/src/intelligence/provider.rs`.

Intelligence Loop 5-question check: no new schema, signal type, health-scoring rule, briefing surface, or feedback hook is introduced by the bridge. It is a routing/rendering layer over W3 ability output. Any UI TypeScript wrapper around `invoke_ability` is out of this W4-C Rust-owned plan unless a later pilot needs it.

No direct `Utc::now()` or `thread_rng()` in `src-tauri/src/bridges/` or `src-tauri/src/commands/abilities.rs`; bridge timestamps come from `ctx.services.clock.now()` or existing tracer records. Existing lint coverage is provider-only (`scripts/check_no_direct_clock_rng_in_provider_modules.sh:1-9`); W4-C should extend the W3-A abilities lint or add bridge coverage.

Fixtures and tests use synthetic ability names and generic entities only. Clippy budget remains the existing gate: `cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-features --lib --bins -- -D warnings` (`.github/workflows/test.yml:74-75`).

## 7. Integration with parallel wave-mates

W3-A/DOS-210 owns `AbilityRegistry`, descriptors, `Actor`, `AbilityContext`, schema metadata, `invoke_by_name_json`, and `iter_for(Actor::Agent)` (`.docs/plans/wave-W3/DOS-210-plan.md:24-36`). W4-C consumes those. If W4-C implementation starts before W3-A lands, define a narrow `AbilityInvoker` trait in `src-tauri/src/bridges/types.rs` and replace it with W3-A imports during integration.

W3-B/DOS-211 owns `AbilityOutput<T>`, `Provenance`, and `RenderedProvenance` inputs (`.docs/plans/wave-W3/DOS-211-plan.md:19-45`). W4-C consumes the envelope and renderer only; it does not define provenance internals.

W4-B/DOS-216 consumes `EvalAbilityBridge`. Its API must accept injected fixture services/provider/tracer and run without a Tauri runtime. It should call the same registry erased invocation path as Tauri/MCP so fixture evidence covers the real bridge.

W2-B/DOS-259 is already merged. W4-C reads `AppState::context_snapshot()` and passes the selected provider into `AbilityContext`; it does not change the provider trait or selection seam (`src-tauri/src/intelligence/provider.rs:14-22`, `:309-326`).

Phase 2 collision points: `src-tauri/src/lib.rs` command registration, `src-tauri/src/commands.rs` module exports, and `src-tauri/src/mcp/main.rs` hybrid handler. Phase 3 cleanup of hand-written capability commands/tools is not this PR unless the migrated abilities already exist.

## 8. Failure modes + rollback

If registry startup validation fails, bridges fail closed: `invoke_ability` returns a typed unavailable error and MCP derived list is empty, while the Phase 2 static mechanical allowlist can still serve existing read tools. If schema validation fails, ability code is never executed. If provenance rendering fails, return a hard bridge error; do not return domain data without provenance.

If MCP tool derivation fails midway, the server must not partially expose unfiltered descriptors. Build the filtered `Vec<Tool>` in a local value, then atomically swap/cache it. If `call_tool` sees a tool name that was listed earlier but is no longer authorized after registry reload, invocation loses and returns an authorization error.

Rollback is mechanical and migration-free: remove `invoke_ability` from `generate_handler!`, stop constructing bridges, and let current hand-written commands/static MCP tools continue during Phase 2. W1-B universal write fence is honored because W4-C adds no direct write path; all writes remain inside abilities/services and the existing fence (`scripts/check_write_fence_usage.sh:1-22`).

Global maintenance rollback safety: if `WorkerAbilityBridge` cannot prove it is the scheduled-worker path, it rejects `global` rather than falling back to Tauri or MCP. Eval rollback safety: missing replay fixture returns a typed fixture error and never falls through to Live, matching W2-B replay behavior (`src-tauri/src/intelligence/provider.rs:227-295`).

## 9. Test evidence to be produced

Bridge/unit tests: `invoke_ability_rejects_locked_app`, `invoke_ability_rejects_unknown_ability_without_enumeration`, `invoke_ability_schema_invalid_input_fails_before_dispatch`, `invoke_ability_rejects_actor_override_in_input_json`, `invoke_ability_user_actor_requires_confirmation_when_policy_requires`, `invoke_ability_returns_ability_response_json_with_tauri_provenance`, and `bridge_errors_do_not_include_input_or_prompt_content`.

MCP tests under `--features mcp`: `mcp_list_tools_derives_from_registry`, `mcp_list_tools_filters_agent_actor`, `mcp_list_tools_hides_maintenance_admin_and_experimental`, `mcp_call_tool_rechecks_actor_policy_for_guessed_name`, `mcp_maintenance_synthetic_actor_rejected_requires_user_actor`, `mcp_rejects_confirmation_field`, `mcp_tool_descriptor_uses_registry_input_schema`, `mcp_response_includes_actor_filtered_provenance`, and `mcp_get_provenance_redacts_internal_ids_for_agent`.

Worker/eval tests: `worker_bridge_invokes_maintenance_as_system_live`, `worker_bridge_rejects_global_without_scheduled_worker_marker`, `eval_bridge_constructs_evaluate_context`, `eval_bridge_uses_replay_provider_and_never_live_provider`, and `eval_bridge_runs_without_tauri_runtime`.

CI/lint evidence: `surface_drift_lint_blocks_new_capability_tauri_command`, `surface_drift_lint_blocks_new_handwritten_mcp_tool`, and the workflow line added beside `.github/workflows/test.yml:62-79`. Gate artifact for this PR: `cargo test bridge`, `cargo test --features mcp mcp_ability_bridge`, `cargo test eval_bridge`, `./scripts/check_ability_surface_drift.sh`, and the standard clippy/test gate.

Suite contributions: Suite S owns auth/authz on `invoke_ability`, MCP actor-filter discovery, schema-validation on input JSON, and capability-handle leakage. Suite E gets an end-to-end no-op ability through `EvalAbilityBridge` and MCP/Tauri bridge smoke coverage. Suite P records startup tool-derivation count and p95/p99 bridge overhead for a no-op Read ability.

## 10. Open questions

1. W3-A constructor shape: confirm `AbilityContext` exposes a bridge-safe constructor from `ServiceContext`, provider, user, tracer, actor, and confirmation without giving W4-C raw `ActionDb`/`AppState` handles.

2. `get_provenance(invocation_id)` storage: approve bounded in-memory cache for non-maintenance ability invocations, or require W3-C/DOS-7 durable provenance rows before exposing the MCP detail tool.

3. Actor detail shape: if ADR-0113's richer `agent:<name>:<version>` identity lands in W3-A, confirm the exact MCP server identity string; otherwise W4-C will use simple `Actor::Agent` for policy and keep the detailed label as metadata only.

4. Phase 2 static MCP allowlist: confirm the current five MCP tools in `src-tauri/src/mcp/main.rs:174-438` are mechanical/read enough to keep until Phase 3, or require immediate migration/removal if any are considered capability-level.
