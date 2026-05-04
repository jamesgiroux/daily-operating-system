# I512 — ServiceLayer: Mandatory Mutation Path + Signal Emission

**Priority:** P0
**Area:** Backend / Architecture
**Version:** v1.0.0 (Phase 1)
**Depends on:** I511 (schema decomposition)
**Blocks:** I513 (workspace read-path elimination), I508 (intelligence schema)
**Absorbs:** I380 (commands.rs service extraction), I402 (IntelligenceService)

## Problem

DailyOS has a partial service-layer pattern but no hard architectural boundary for mutations. Runtime write paths still exist in non-service modules, and several required signal emissions are either missing or best-effort (`let _ = ...`) in places where consistency requires strict handling.

This creates stale-data regressions in prep invalidation, propagation, and downstream refresh behavior.

## Scope

I512 is crate-wide for runtime mutation hotspots, not commands-only.

Covered files:
- `src-tauri/src/commands.rs`
- `src-tauri/src/intel_queue.rs`
- `src-tauri/src/processor/transcript.rs`
- `src-tauri/src/workflow/deliver.rs`
- `src-tauri/src/workflow/reconcile.rs`
- `src-tauri/src/hygiene.rs`

Estimated implementation magnitude: ~70-75 mutation-level changes (service extraction + signal contract + tests/CI guardrails).

## Design

### 1) ServiceLayer is required in I512

Implement `ServiceLayer` now (not optional / not deferred). It is the command-facing mutation boundary.

Important split-lock requirement:
- Background processors may continue opening owned DB handles via `ActionDb::open()`.
- Those processors must call service-owned mutation functions (`services::<domain>::...`) instead of direct `db.upsert/update/...` calls.
- I512 does **not** ban `ActionDb::open()` itself. It bans direct non-service mutations.

### 2) Mutation classes

Every runtime mutation must be classified in audit:
- `DomainRequiredSignal`: domain write that must emit a signal.
- `OperationalNoSignal`: operational/cache/log/sync write that is explicitly exempt.

### 3) Transaction semantics

For `DomainRequiredSignal` writes:
- Domain mutation + required signal insert are in one `ActionDb::with_transaction(...)` block.
- If required signal insert fails, mutation fails and rolls back.
- Error must surface signal failure context clearly.

Propagation/evaluation behavior is documented per mutation row in audit (required vs best-effort).

### 4) Hygiene ownership

Add `src-tauri/src/services/hygiene.rs` and migrate hygiene mutations through service-owned APIs instead of inline direct DB writes in `hygiene.rs`.

### 5) CI enforcement

Add boundary check script:
- Allow `ActionDb::open()` in runtime modules.
- Fail on direct non-service mutation calls outside allowed modules.
- Fail on direct write SQL shortcuts (`conn_ref().execute(...)` writes) outside allowed modules.
- Support explicit escape hatch annotation:
  - `// DIRECT_DB_ALLOWED: <reason>`

## Implementation Waves

1. **Wave 0: Spec hardening + sizing**
   - This spec update.
   - Naming debt carried into migration (`entity_intelligence` / `account_team` API naming modernization while preserving compatibility).

2. **Wave 1: Audit-first deliverable**
   - Create `.docs/audits/signal-emission-audit.md` with mutation inventory and status.

3. **Wave 2: Service foundation**
   - Add `ServiceLayer` in `services/mod.rs`.
   - Register in `lib.rs` managed state.
   - Add `services/hygiene.rs`.

4. **Wave 3: Migration**
   - Move direct mutation logic from six hotspots into service-owned mutation APIs.
   - Remove required-path signal swallowing.

5. **Wave 4: Enforcement + verification**
   - Add CI boundary script and wire into workflow.
   - Run rust tests + clippy + end-to-end mutation→signal→propagation checks.

## Files to Modify

- `src-tauri/src/services/mod.rs` (ServiceLayer + hygiene module export)
- `src-tauri/src/services/hygiene.rs` (new)
- `src-tauri/src/lib.rs` (managed state registration)
- Hotspots listed above (migration)
- `.github/workflows/test.yml` (boundary check step)
- `scripts/check_service_layer_boundary.sh` (new)
- `.docs/audits/signal-emission-audit.md` (new)

## Acceptance Criteria

1. ServiceLayer exists and is active for mutating command paths.
2. Zero direct DB mutation calls remain in the six hotspot files.
3. `ActionDb::open()` usage is permitted only when mutations are performed via service-owned mutation APIs.
4. Every `DomainRequiredSignal` mutation writes signal event transactionally with the domain mutation.
5. No silent signal swallowing in required mutation paths.
6. Audit doc is complete with no unresolved mutation rows.
7. CI blocks new direct non-service mutation patterns and supports justified exceptions via annotation.
8. Person role/relationship mutation path triggers prep invalidation chain.
9. Intelligence enrichment mutation path emits required signal + propagation effects.
10. App works end-to-end for briefing, meeting detail, and entity update flows.
11. `cargo test` passes.
12. `cargo clippy --manifest-path src-tauri/Cargo.toml --workspace --all-features --lib --bins -- -D warnings` passes.

## Out of Scope

- I514 module decomposition (file splitting)
- New signal taxonomy definitions (reuse existing taxonomy)
