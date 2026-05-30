# DOS-304 — Runtime capability boundary + single registry source contract

**Status:** verified satisfied at v1.4.0 wave tip (`658dbd07`).
**Acceptance walk last refreshed:** 2026-05-07.

## Contract

`ServiceContext` capability handles are the enforcement boundary. Abilities receive scoped capabilities appropriate for category × execution mode, not raw app state or raw DB handles. The `#[ability]` macro is registry metadata + lint/trybuild coverage, **not** the hard mutation boundary. There is one ability registry (`AbilityRegistry::global_checked`) and one operations source.

## Acceptance criteria — verification

### Ability code cannot obtain unauthorized write handles by construction

`ServiceContext` (`src-tauri/src/services/context.rs:784`) is the only handle abilities receive. Three mode constructors:
- `new_live(clock, rng, external)` (`:927`)
- `new_simulate(clock, rng, external)` (`:948`)
- `new_evaluate(clock, rng, external)` (`:975`) + `new_evaluate_default(clock, rng)` (`:1000`)

Each constructor wires `Clock`, `SeededRng`, and `ExternalClients` (replay-only in Evaluate per ADR-0104). Abilities call `services::*` APIs that route writes through `services/claims.rs::commit_claim`, never raw `ActionDb`.

Lint enforcement: `src-tauri/scripts/check_no_db_state_imports_in_abilities.sh` rejects raw `ActionDb` / `AppState` imports inside `src-tauri/src/abilities/**`. CI green at wave tip.

### Evaluate / Simulate / Live behavior enforced through capability handles, not just check_mutation_allowed convention

`ExecutionMode` is part of the `ServiceContext` mode constructors (above). Mutation gating: `services::claims::commit_claim` calls `check_mutation_allowed()` (DOS-209), but the *handle delivery* — which `ExternalClients` you receive, whether you get a Live PTY provider or `ReplayProvider` — is bound at construction time. A Replay `ServiceContext` cannot ever resolve a live external client because the handle was never instantiated.

`ReplayProvider` returns `ReplayFixtureMissing` instead of falling through to live (`src-tauri/src/intelligence/provider.rs:560`+ ADR-0104 §207-209). Verified at L2 cycle-15 APPROVE.

### Raw ActionDb / direct SQL / file-write unavailable to ability implementations

Lint scripts active in `src-tauri/scripts/`:
- `check_no_db_state_imports_in_abilities.sh` — rejects raw DB handles in abilities
- `check_no_direct_clock_rng_in_abilities.sh` — rejects `Utc::now()` / `thread_rng()` in abilities
- `check_claim_writer_allowlist.sh` — confines `intelligence_claims` writes to `services/claims.rs`
- `check_intelligence_claims_no_delete.sh` — append-only / supersession-only

CI green at wave tip; rejects on regression.

### Proc macro scope is documented as registry metadata + lint/trybuild, not complete mutation proof

ADR-0102 §`Abilities as the Runtime Contract` documents the macro's role:
> "The `#[ability]` macro registers the ability in the global registry and runs trybuild compile-fail tests on category/mode invariants. It is NOT the mutation boundary — that's `ServiceContext`."

`AbilityRegistry::global_checked` (`src-tauri/src/abilities/registry.rs:213`) is the single source of truth; `RegistryViolation` enumerates trybuild-checked invariants (duplicate names, invalid actor combinations, missing schemas).

### One registry/operations source of truth

`AbilityRegistry::global_checked` is the single source. `DOS-217` consolidated bridges (`Tauri`, `Mcp`, `Worker`, `Eval`) to dispatch through `AbilityRegistry::invoke_by_name_json`. ADR-0102 + ADR-0111 frozen the registry contract; bridges are generic over registry entries.

Resolution of DOS-217 vs DOS-264 — abilities are operations; the operations registry is the same as the abilities registry. No second registry.

### IntelligenceProvider split between low-level completion/replay and orchestration services

DOS-259 (W2) introduced the `IntelligenceProvider` trait at `src-tauri/src/intelligence/provider.rs:247-263`. The trait covers low-level completion + replay. Orchestration logic (subject resolution, claim composition, prompt assembly) lives in `services/` and abilities, not in providers.

`PtyClaudeCode` (Live), `ReplayProvider` (Evaluate), `MockProvider` (Simulate) all implement the trait via the W2-B AppState bridge (`set_context_mode_atomic`, `ContextProviderBundle`, `state.rs:1116-1168`).

### Pilot Tauri/MCP bridge path is explicit

DOS-217 defined the four bridges in `src-tauri/src/bridges/`:
- `TauriAbilityBridge` — generic `invoke_ability` Tauri command at `commands/abilities.rs`
- `McpAbilityBridge` — auto-registers MCP tools from registry at startup
- `WorkerAbilityBridge` — scheduled maintenance with `Actor::System`
- `EvalAbilityBridge` — fixture-driven invocation through `ServiceContext::new_evaluate`

The Tauri pilot is `commands/abilities.rs::invoke_ability` (deleted hand-written capability-level commands in W4-W6 cleanup). MCP pilot is registry-derived; no hand-written MCP tools added since W4. CI lint blocks regressions.

### Startup / background workers have explicit execution-mode behavior so Evaluate cannot accidentally start live pollers

`AppState::set_context_mode_atomic` (`src-tauri/src/state.rs:1116-1168`) atomically swaps `ContextProviderBundle` (context + provider + glean-provider Arcs). When mode flips to Evaluate, the bundle replaces live providers with `ReplayProvider` + replay externals. Workers consume the bundle via `context_snapshot()` so the next dequeue picks up the new mode (per ADR-0091 "switch mid-queue takes effect on next dequeue" — verified at W2-B L2).

Background queues / pollers are constructed with the current bundle's provider; on mode change they drain the previous mode's in-flight work and resume against the new bundle.

## Outstanding

None. Contract is fully satisfied at v1.4.0 wave tip.

## References

- ADR-0102 — Abilities as the Runtime Contract
- ADR-0104 — ExecutionMode and Mode-Aware Services
- ADR-0091 — AppState bridge swap semantics
- ADR-0111 — Surface-Independent Ability Invocation
- W2 proof bundle — DOS-259 IntelligenceProvider trait + AppState-Arc bridge
- W4 proof bundle — DOS-217 bridge consolidation (commit `25bd3196`)
- W6 proof bundle — boundary cycle-15 APPROVE (commit `17afb9e7`)
