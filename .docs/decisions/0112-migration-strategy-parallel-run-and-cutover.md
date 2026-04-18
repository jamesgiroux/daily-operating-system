# ADR-0112: Migration Strategy — Parallel Run and Cutover

**Status:** Proposed  
**Date:** 2026-04-18  
**Target:** v1.4.0  
**Extends:** [ADR-0101](0101-service-boundary-enforcement.md), [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0104](0104-execution-mode-and-mode-aware-services.md)  
**Depends on:** [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0107](0107-source-taxonomy-alignment.md), [ADR-0110](0110-evaluation-harness-for-abilities.md), [ADR-0111](0111-surface-independent-ability-invocation.md)

## Context

[ADR-0102](0102-abilities-as-runtime-contract.md) §14 describes per-capability migration but does not specify parallel-run validation, cutover criteria, rollback, or the three specific substrate migrations that ship as part of v1.4.0:

1. `ServiceContext` amendment per [ADR-0104](0104-execution-mode-and-mode-aware-services.md) — mode-aware services, `check_mutation_allowed()` added to ~60 mutation functions, clock/RNG injection.
2. `source` → `data_source` schema rename per [ADR-0107](0107-source-taxonomy-alignment.md).
3. Synthesis-marker backfill per [ADR-0107](0107-source-taxonomy-alignment.md) §6 — tagging existing stored AI output so [ADR-0105](0105-provenance-as-first-class-output.md)'s trust model works retroactively.

This ADR specifies the per-capability migration flow, the three substrate migrations, parallel-run validation, cutover criteria, and rollback procedures.

## Decision

### 1. Per-Capability Migration Flow

Each capability migrates through five stages. The order is important: skipping stages produces unverified cutovers.

**Stage 0 — Substrate readiness.** The three substrate migrations in §2 must be at Phase 2 or later before a capability can begin its own migration.

**Stage 1 — Ability implementation.** A new ability module is added under `src-tauri/src/abilities/` per [ADR-0102](0102-abilities-as-runtime-contract.md) §2. Includes planner function, apply function (for Maintenance), provenance builder usage per [ADR-0105](0105-provenance-as-first-class-output.md), and declared metadata (`mutates`, `composes`, policy).

**Stage 2 — Eval fixture.** At least one evaluation fixture per ability with expected output per [ADR-0110](0110-evaluation-harness-for-abilities.md). CI runs the fixture on every PR touching the ability.

**Stage 3 — Parallel run.** The new ability runs in production alongside the existing hand-written command/tool. Outputs are compared at runtime via a `ParallelRunValidator`:

```rust
pub struct ParallelRunValidator {
    pub compare_fn: Box<dyn Fn(&OldOutput, &NewOutput) -> ComparisonResult + Send + Sync>,
    pub log_divergence: bool,
    pub max_samples: u32,
}
```

Both the old and new paths execute; the user receives the old path's output. The validator logs divergences to the `maintenance_audit` table (tagged as parallel-run comparisons, not real maintenance invocations). The target: ≤1% divergence on a rolling 100-invocation window. Exact-match required for Read abilities; LLM-as-judge acceptable for Transform.

**Stage 4 — Cutover.** Once parallel run meets criteria, surfaces switch to the new path:

- Tauri command becomes a thin wrapper around `invoke_ability(name, input)` per [ADR-0111](0111-surface-independent-ability-invocation.md).
- MCP tool is removed from hand-written registration; the registry-derived tool takes over.
- The old hand-written code remains in place as a fallback for one release cycle (tagged deprecated).

**Stage 5 — Old-path removal.** After one release cycle with no reported issues from the new path, the old hand-written command/tool and the ParallelRunValidator entry are removed.

### 2. The Three Substrate Migrations

#### 2.1 `ServiceContext` and Mode-Awareness ([ADR-0104](0104-execution-mode-and-mode-aware-services.md))

**Phase 1 (v1.4.0 planning window):**

- `ExecutionMode` enum and amended `ServiceContext` land.
- `check_mutation_allowed()` added to `ServiceContext`.
- `with_transaction_async` implementation added.
- Clock and RNG injection scaffolding: `ctx.services.clock.now()` and `ctx.services.rng.gen()` available; existing `Utc::now()` and `thread_rng()` call sites unchanged.
- `ExternalClients` wrapper infrastructure added per [ADR-0104](0104-execution-mode-and-mode-aware-services.md) §2.

**Phase 2 (v1.4.0 refactor):**

- Every mutation function in `services/` adds `ctx.check_mutation_allowed()?` as its first line (~60 call sites per [ADR-0101](0101-service-boundary-enforcement.md) audit).
- `PropagationEngine::emit(mode, event)` becomes mode-aware.
- External service wrappers (Glean, Slack, Gmail, Salesforce) gain mode routing.
- Clock/RNG migration: service and ability code replaces direct `Utc::now()` and `thread_rng()` with injected variants. CI lint activates.

**Phase 3 (v1.4.0 cutover):**

- `ServiceContext::new_evaluate` wired into [ADR-0110](0110-evaluation-harness-for-abilities.md) fixture harness.
- Regression gate active.

**Rollback.** A partial `check_mutation_allowed` rollout is safe: functions without the check still work in Live. The issue would be unsafe `Evaluate` runs writing data; the harness is opt-in and only enabled after Phase 3.

#### 2.2 `source` → `data_source` Schema Rename ([ADR-0107](0107-source-taxonomy-alignment.md))

**Phase 1 (additive):**

- Schema migration adds `data_source` column alongside `source` in `signal_events` and related tables.
- Backfill: `data_source = source` copied row-by-row, typed to the new `DataSource` enum. Glean-downstream sources are inferred from existing metadata where possible, fall back to `Glean { downstream: Unknown }`.
- Read code paths updated to prefer `data_source` when present; fall back to `source`.

**Phase 2 (write dual):**

- Write code paths populate both `source` (for backward compat) and `data_source`.
- All new queries use `data_source`.

**Phase 3 (drop old):**

- `source` column dropped via schema migration after a full release cycle.
- Queries referencing `source` are refactored or fail hard.

**Rollback.** Phase 1 is safely reversible (drop the new column). Phase 2 is reversible with data loss only for newly-emitted signals that used a new `DataSource` variant not representable in the old `source` type. Phase 3 is irreversible; only ship Phase 3 after production confidence on Phase 2.

#### 2.3 Synthesis-Marker Backfill ([ADR-0107](0107-source-taxonomy-alignment.md) §6)

**Phase 1:**

- Schema migration adds `synthesized_fields: JSON` column to entity tables that store LLM output (`entity_assessments`, `meeting_prep`, etc.).
- Backfill: historical rows inspected to identify which fields were LLM-synthesized. Best-effort; where attribution is ambiguous, mark all candidate fields as `["*unknown*"]`. This errs on the side of `Untrusted` classification.
- New writes populate `synthesized_fields` accurately.

**Phase 2:**

- Read paths consult `synthesized_fields` when building `SourceAttribution.synthesis_marker` per [ADR-0105](0105-provenance-as-first-class-output.md) §4.
- Trust model's `contains_stored_synthesis` flag is now populated correctly.

**Rollback.** `synthesized_fields` is additive; rollback is a no-op (column can remain unused).

### 3. Capability Migration Order

Phase 2 (capability migration) proceeds in priority order:

1. `get_entity_context` — highest read volume; validates substrate across entity-generic pipeline.
2. `prepare_meeting` — highest user-visible value; exercises Transform + composition + provenance.
3. `get_daily_readiness` — exercises composition across many child abilities.
4. `list_open_loops` — validates simple Read ability pattern.
5. `detect_risk_shift` — validates Transform + trajectory consumption from [ADR-0109](0109-temporal-primitives-in-the-entity-graph.md).
6. `generate_weekly_narrative` — validates Transform on longer time windows.
7. Remaining Read and Transform abilities.
8. Maintenance abilities (`refresh_entity_state`, `reconcile_signals`, `repair_graph_links`).
9. Publish abilities (`publish_to_p2`, `export_report`).

Within each group, migration is serial: one capability's parallel run completes before the next begins. This keeps the divergence telemetry interpretable and prevents concurrent migration-induced regressions from masking each other.

### 4. Parallel-Run Validation Details

For each capability in Stage 3:

- **Invocation shadowing.** The Tauri command invokes both paths; the user sees only the old output. The new output is logged for comparison.
- **Sampling.** High-frequency capabilities (e.g., `list_open_loops` called on every app foreground) are sampled at 10% to bound cost. Low-frequency capabilities (e.g., `generate_weekly_narrative` called once/week) are shadowed at 100%.
- **Comparison function.** Per-ability comparator: exact equality for structured fields, LLM-as-judge for synthesized text. Threshold per capability.
- **Divergence triage.** Any divergence beyond the threshold opens a review: inspect inputs, compare planner outputs, compare fingerprints, decide whether to update expected output or fix the new ability.
- **Cutover criterion.** ≥100 production invocations with ≤1% unexplained divergence over a rolling 7-day window.

### 5. Rollback Procedure

Per capability:

- **Stage 3 (parallel run) rollback.** Turn off the new path invocation entirely; old path continues. Zero user impact.
- **Stage 4 (cutover) rollback.** Revert the Tauri command wrapper and MCP tool registration to hand-written versions (retained as fallback). Old path runs again. User impact: if the new path was producing materially different output, users see the old output's characteristics return.
- **Stage 5 (old-path removal) rollback.** Revert the commit that removed the hand-written code. Requires a new release.

**Substrate migration rollback.** §2.1 and §2.3 are additive-first, safe to roll back at Phase 1/2. §2.2's Phase 3 is irreversible by design; holds until Phase 2 confidence is high.

### 6. Release Cadence

v1.4.0 ships the planning window (Phase 1 of all three substrate migrations) and begins Phase 2 migrations of top-priority capabilities. Full cutover (Phase 3) of all capabilities is a v1.4.1 or v1.4.2 milestone — not gated on v1.4.0 GA.

**What v1.4.0 ships:**

- All ADRs 0102–0112 accepted.
- Substrate migrations §2.1 and §2.3 at Phase 2.
- Substrate migration §2.2 at Phase 1 (additive column, dual-read; Phase 2 dual-write ships in v1.4.0 patch).
- Evaluation harness operational for at least `get_entity_context` and `prepare_meeting`.
- Parallel run active for those two capabilities.

**What v1.4.1 ships:**

- §2.2 Phase 2 (dual-write) stable.
- More capabilities migrated past Stage 4.
- Maintenance abilities enter migration.

**What v1.4.2 ships:**

- §2.2 Phase 3 (drop `source` column).
- Remaining capabilities migrated.
- Phase 4 compile-time enforcement of registry-only invocation.

## Consequences

### Positive

1. **Staged migration bounds risk.** Every stage has a defined rollback path.
2. **Substrate dependencies explicit.** Capability migration cannot begin until substrate readiness is established.
3. **Parallel run validates in production.** Divergence signals catch real-world gaps that fixtures miss.
4. **Cadence is honest.** v1.4.0 is not gated on full cutover; v1.4.1 and v1.4.2 finish the migration.
5. **Substrate migrations are additive-first.** Rollback is cheap at every intermediate state.

### Negative

1. **Migration is long.** Three releases (v1.4.0 → v1.4.2) to reach Phase 4.
2. **Parallel run doubles cost for migrated abilities during Stage 3.** Provider calls and service reads happen twice for shadowed invocations.
3. **Divergence telemetry requires ongoing review.** Triaging parallel-run divergences is real work.
4. **`source` → `data_source` rename touches many queries.** Every read path must be updated in Phase 2.

### Risks

1. **Parallel-run masking.** A persistent divergence is explained away as "acceptable" rather than addressed. Mitigation: divergence reviews require a documented decision (accept, fix, rebaseline); no silent dismissals.
2. **Substrate migration incomplete at Phase 2.** Capabilities begin migrating before mode-aware services are everywhere, leading to false confidence. Mitigation: capability Stage 0 gate checks substrate readiness explicitly.
3. **Release cadence slip.** v1.4.2 cutover slips because full migration drags on. Mitigation: the ADR framework tolerates hybrid state; hand-written and registry-derived paths coexist indefinitely if needed, at the cost of duplicated maintenance.
4. **Rollback at Stage 5 is expensive.** Removing the old hand-written path after a release cycle and then needing to restore it requires a full release. Mitigation: Stage 5 removal is gated on a full release cycle with no reported issues; the rollback cost is acknowledged and accepted.
5. **Synthesis-marker backfill imprecision.** Historical data tagged `*unknown*` produces conservative `Untrusted` assessments that mislabel actually-trusted Read outputs. Mitigation: acceptable — over-cautious is the right error direction. Users can explicitly mark content as known-trusted via a user-action that produces a new synthesis marker.

## References

- [ADR-0101: Service Boundary Enforcement](0101-service-boundary-enforcement.md) — §2.1 migration completes ADR-0101's Phase 3 goals.
- [ADR-0102: Abilities as the Runtime Contract](0102-abilities-as-runtime-contract.md) — §14 migration-from-current-architecture refined here.
- [ADR-0103: Maintenance Ability Safety Constraints](0103-maintenance-ability-safety-constraints.md) — Maintenance capability migration order in §3.
- [ADR-0104: ExecutionMode and Mode-Aware Services](0104-execution-mode-and-mode-aware-services.md) — §2.1 substrate migration.
- [ADR-0105: Provenance as First-Class Output](0105-provenance-as-first-class-output.md) — §2.3 synthesis-marker backfill enables `contains_stored_synthesis`.
- [ADR-0107: Source Taxonomy Alignment](0107-source-taxonomy-alignment.md) — §2.2 `source` → `data_source` rename; §2.3 synthesis markers.
- [ADR-0110: Evaluation Harness for Abilities](0110-evaluation-harness-for-abilities.md) — Fixtures required before Stage 3.
- [ADR-0111: Surface-Independent Ability Invocation](0111-surface-independent-ability-invocation.md) — Stage 4 cutover uses its binding pattern.
