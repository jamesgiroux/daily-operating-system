# ADR-0104: ExecutionMode and Mode-Aware Services

**Status:** Proposed  
**Date:** 2026-04-18  
**Target:** v1.4.0  
**Extends:** [ADR-0101](0101-service-boundary-enforcement.md)  
**Prerequisite for:** [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0103](0103-maintenance-ability-safety-constraints.md)

## Context

The abilities layer introduced by [ADR-0102](0102-abilities-as-runtime-contract.md) and the maintenance safety envelope in [ADR-0103](0103-maintenance-ability-safety-constraints.md) both depend on execution mode being a first-class, enforced property of the runtime. Three requirements motivate this ADR:

1. **Offline evaluation.** The evaluation harness ([ADR-0107](0107-evaluation-harness-for-abilities.md)) runs abilities against fixture data and captures their output and planned mutations for scoring. This is only safe if mutations are structurally impossible during the evaluation run. A mode flag that abilities check voluntarily is insufficient — any forgotten check leaks test writes into real data.
2. **Maintenance safety.** [ADR-0103](0103-maintenance-ability-safety-constraints.md) §3 requires that dry-runs and simulations produce a `MaintenanceReport` without any side effect on state, signals, or external calls. [ADR-0103](0103-maintenance-ability-safety-constraints.md) §8 distinguishes audit records written to `maintenance_audit` (Live only) from tracer captures (Simulate/Evaluate). Both rules rely on the service layer knowing which mode it is operating in.
3. **Async transactional composition.** Maintenance abilities compose async service calls inside transactions ([ADR-0103](0103-maintenance-ability-safety-constraints.md) §2). ADR-0101's `with_transaction` is synchronous. Without an async transactional primitive that preserves SQLite's write-serialization guarantee, maintenance abilities cannot honor their atomicity contract.

Today, `ServiceContext` ([ADR-0101](0101-service-boundary-enforcement.md)) carries `db`, `signals`, and `intel_queue`. It has no notion of mode and no async transaction support. Tests that need write-free runs construct their own in-memory DB; there is no consistent way to run an ability against production-anonymized fixtures in a guaranteed read-only posture.

This ADR amends `ServiceContext` and the service mutation contract to make `ExecutionMode` load-bearing, and introduces `with_transaction_async`.

## Decision

### 1. The `ExecutionMode` Type

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExecutionMode {
    /// Normal operation. Mutations, signals, external calls all apply.
    Live,
    
    /// Developer-facing "try this without side effects" mode.
    /// Mutations rejected at the service layer. Signal emission captured to
    /// tracer only. External calls stubbed or replayed from fixture.
    /// Intelligence provider calls stubbed or replayed.
    Simulate,
    
    /// Structured evaluation mode used by the evaluation harness.
    /// Everything that Simulate blocks is also blocked here.
    /// Additionally, tracer output is normalized for snapshot-diff scoring.
    /// Clocks and random seeds are deterministic.
    Evaluate,
}
```

The three modes have non-overlapping operational semantics summarized in [ADR-0103](0103-maintenance-ability-safety-constraints.md) §3. This ADR specifies how the service layer enforces them.

Throughout this ADR and its peers, `Evaluate` is the enum variant name. Prose references to "the evaluation harness" or "an evaluation run" refer to code operating under `ExecutionMode::Evaluate`.

### 2. `ServiceContext` Amendment

`ServiceContext` gains a mode field. Construction is explicit — callers must choose a mode; there is no default:

```rust
pub struct ServiceContext<'a> {
    pub db: &'a ActionDb,
    pub signals: &'a PropagationEngine,
    pub intel_queue: &'a IntelligenceQueue,
    pub mode: ExecutionMode,                // NEW
    pub clock: &'a dyn Clock,               // NEW — injected for determinism
    pub rng: &'a dyn SeededRng,             // NEW — injected for determinism
    pub external: ExternalClients<'a>,      // NEW — mode-aware external wrappers
    pub(crate) tx: Option<TxHandle<'a>>,   // NEW — active transaction handle, private
}

pub struct ExternalClients<'a> {
    pub glean: GleanWrapper<'a>,         // routes to real client in Live, replay in Simulate/Evaluate
    pub slack: SlackWrapper<'a>,
    pub gmail: GmailWrapper<'a>,
    pub salesforce: SalesforceWrapper<'a>,
    // ...
}
```

Each `*Wrapper` respects `ExecutionMode`: in `Live`, it calls the real client; in `Simulate` or `Evaluate`, it consults the replay fixture supplied during construction (per `ServiceContext::new_simulate`/`new_evaluate`). Abilities access external services exclusively through `ctx.services.external.glean.*`, `ctx.services.external.slack.*`, etc. The `AbilityContext` does not hold any external client reference directly — that would bypass mode-aware routing.

Construction:

```rust
impl<'a> ServiceContext<'a> {
    pub fn new_live(
        db: &'a ActionDb,
        signals: &'a PropagationEngine,
        intel_queue: &'a IntelligenceQueue,
        external: ExternalClients<'a>,  // live external clients
    ) -> Self { ... }
    
    pub fn new_simulate(
        db: &'a ActionDb,
        signals: &'a PropagationEngine,
        intel_queue: &'a IntelligenceQueue,
        external: ExternalClients<'a>,  // fixture-backed replay wrappers
    ) -> Self { ... }
    
    pub fn new_evaluate(
        db: &'a ActionDb,
        signals: &'a PropagationEngine,
        intel_queue: &'a IntelligenceQueue,
        fixture: &'a EvalFixture,        // supplies clock, RNG, and replay wrappers
    ) -> Self { ... }
}
```

- `new_live` is the production constructor used by the app and the MCP server; callers supply real external clients.
- `new_simulate` takes fixture-backed external wrappers and a tracer-only signal sink.
- `new_evaluate` internally constructs `ExternalClients` from the `EvalFixture`'s replay data; the caller does not provide them separately. The fixture also supplies the deterministic clock and seeded RNG.

There is no `ServiceContext::new()`. The mode must be chosen deliberately.

### 3. Plan/Apply Split and Mutation-Mode Enforcement

Maintenance abilities and any ability that produces a `MaintenanceReport` execute in two explicit phases: **plan** (compute what would change) and **apply** (execute the changes). The phases exist regardless of mode; what differs is whether `apply` runs.

#### 3.1 The Plan Phase

In the plan phase, the ability reads current state, computes synthesis (real LLM in `Live`, stubs in `Simulate`/`Evaluate`), and produces a `PlannedMutationSet`:

```rust
pub struct PlannedMutationSet {
    pub operations: Vec<PlannedMutation>,
}

pub struct PlannedMutation {
    pub service: &'static str,            // e.g. "entities"
    pub operation: &'static str,          // e.g. "refresh_state"
    pub affected_entity_ids: Vec<EntityId>,
    pub affected_rows_estimate: u32,
    pub emitted_signals_estimate: u32,
    pub external_calls: Vec<ExternalCallSpec>,
    pub inputs: serde_json::Value,        // Fully-materialized call arguments
    pub provenance_ref: ProvenanceRef,    // Reference into ability's provenance envelope per ADR-0105 §8
}

// Lightweight reference rather than a duplicated envelope
pub struct ProvenanceRef {
    pub invocation_id: InvocationId,
    pub field_path: FieldPath,            // Which part of the ability's output this mutation derives from
}
```

Services expose **planner functions** for each mutating operation, named with a `plan_` prefix:

```rust
// Mutating function — Live only
pub async fn refresh_entity_state(ctx: &ServiceContext<'_>, id: EntityId) -> Result<RefreshReport, ServiceError>;

// Planner function — safe in any mode; returns what the mutating function would do
pub async fn plan_refresh_entity_state(ctx: &ServiceContext<'_>, id: EntityId) -> Result<PlannedMutation, ServiceError>;
```

Planner functions read current state and compute the intended change without applying it. They are pure (no mutation, no signal emission) and are safe to call under any `ExecutionMode`.

#### 3.2 The Apply Phase

In the apply phase, the ability iterates its planned mutations and invokes the corresponding mutating service functions. **The apply phase only runs in `Live` mode with `dry_run=false`.** In all other cases (`Simulate`, `Evaluate`, or `Live + dry_run=true`), the ability short-circuits after the plan phase and does not invoke the mutating service functions at all. The service layer's `check_mutation_allowed` (§3.3) is defense in depth against a bug that bypasses this discipline, not an expected code path.

```rust
pub async fn reconcile_signals(
    ctx: &AbilityContext<'_>,
    input: ReconcileSignalsInput,
) -> AbilityResult<ReconcileReport> {
    // Phase 1: Plan (always runs)
    let plan = plan_reconcile(ctx, &input).await?;
    
    // If non-Live OR dry_run: return plan as the output
    if ctx.services.mode != ExecutionMode::Live || input.dry_run {
        return Ok(AbilityOutput::from(ReconcileReport::from_plan(plan)));
    }
    
    // Phase 2: Apply (Live only, non-dry-run)
    let applied = apply_reconcile(ctx, &plan).await?;
    Ok(AbilityOutput::from(applied))
}
```

**Audit records are written by the registry's maintenance-ability wrapper, not by the ability body.** Per [ADR-0103](0103-maintenance-ability-safety-constraints.md) §8, the wrapper observes the plan after Phase 1 and writes the Intent record. After Phase 2 (or after the short-circuit in non-Live / dry_run), the wrapper writes the Outcome record. The ability does not call `ctx.tracer.record_maintenance_*` directly; that is the wrapper's responsibility. This keeps the audit contract uniform across all maintenance abilities regardless of their internal structure.

**Even in non-Live modes, both records are produced.** In `Live + dry_run=true`, both records persist to `maintenance_audit` — the Outcome captures `mutations_applied: 0` with a `skipped_reason: DryRun` field. In `Simulate` and `Evaluate`, both records go to the tracer. "Two records per invocation" is always honored.

The planner functions produce rich metadata; the apply phase is mechanical. This structure resolves the "check at boundary can't produce `planned_mutations`" problem (Codex finding): the plan exists independent of whether mutations apply.

#### 3.3 Mutation Functions Honor the Mode

Every mutating function in `services/` calls a shared mode check before touching the DB or emitting a signal. This is a defense in depth: a correctly-written ability uses planner functions and only calls mutators in `Live`, but if it skips that discipline (or has a bug), the service layer still refuses to write in non-`Live` modes:

```rust
impl<'a> ServiceContext<'a> {
    pub(crate) fn check_mutation_allowed(&self) -> Result<(), ServiceError> {
        match self.mode {
            ExecutionMode::Live => Ok(()),
            ExecutionMode::Simulate | ExecutionMode::Evaluate => {
                Err(ServiceError::WriteBlockedByMode(self.mode))
            }
        }
    }
}

pub async fn refresh_entity_state(ctx: &ServiceContext<'_>, id: EntityId) -> Result<RefreshReport, ServiceError> {
    ctx.check_mutation_allowed()?;
    let report = ctx.db.refresh_entity(id).await?;
    ctx.signals.emit(ctx.mode, SignalEvent::entity_refreshed(id))?;
    Ok(report)
}
```

**Signal emission is mode-aware.** `PropagationEngine::emit(mode, event)` accepts the mode and routes: in `Live`, it writes to the signal bus and triggers propagation; in `Simulate`/`Evaluate`, it records the event to the tracer and returns without side effect. The engine is not mutated in non-Live modes.

**External calls are mode-aware.** Services that call external APIs (Glean, Slack, Gmail, Salesforce) check mode at the call boundary and route to either the live client (`Live`) or a replay fixture supplied by the `EvalFixture` (`Simulate`/`Evaluate`). The replay fixture API is specified in [ADR-0107](0107-evaluation-harness-for-abilities.md).

**Intelligence provider is mode-aware via injection.** In `Live`, `AbilityContext.intelligence` is the production provider. In `Simulate`/`Evaluate`, it is a replay provider constructed from the `EvalFixture` and matched by prompt hash. The provider interface ([ADR-0091](0091-intelligence-provider-abstraction.md)) is the same; only the implementation differs per mode.

### 4. `with_transaction_async` Primitive

Maintenance abilities ([ADR-0103](0103-maintenance-ability-safety-constraints.md) §2) compose async service mutations that must succeed or fail atomically. `with_transaction_async` adds the async counterpart to ADR-0101's synchronous primitive:

```rust
impl<'a> ServiceContext<'a> {
    pub async fn with_transaction_async<F, Fut, T>(&self, body: F) -> Result<T, ServiceError>
    where
        F: FnOnce(TxCtx<'_>) -> Fut,
        Fut: Future<Output = Result<T, ServiceError>>,
    { ... }
}
```

Implementation behavior:

- Acquires SQLite's write lock for the duration of the transaction (SQLite serializes writers, which bounds concurrent risk).
- Provides a `TxCtx` handle that exposes the same service interface as `ServiceContext` but routes all DB writes through a single transaction.
- On closure success, commits. On closure failure or panic, rolls back.
- **Bans LLM provider calls and external API calls inside the transaction.** The `TxCtx` does not expose `intelligence` or external clients. An ability that needs synthesis before writing performs the synthesis first, then opens the transaction with the computed inputs.
- Honors `ExecutionMode`. In Simulate/Evaluate, opening a transaction is a no-op wrapper (since abilities operating correctly in non-Live modes run only their plan phase and never reach the apply phase that would open the transaction). If an ability incorrectly attempts mutations in non-Live, the service-layer check rejects the write and the transaction aborts cleanly.

**Deadlock avoidance.** Only one write transaction can hold SQLite's write lock at a time. Abilities invoked concurrently under write contention serialize on the lock. This is acceptable for maintenance workloads (which are not latency-sensitive) and prevents the async deadlock patterns that can appear with multi-lock transactions.

**Nesting vs. composition.** `with_transaction_async` is not reentrant. Attempting to open a *new* transaction inside an active one returns `Error::NestedTransactionsForbidden`.

However, ability composition is transaction-propagating, not transaction-nesting (per [ADR-0102](0102-abilities-as-runtime-contract.md) §11.2). A parent maintenance ability that holds a transaction may invoke a child ability; the child inherits the parent's `TxCtx` via `AbilityContext.services` and its service calls participate in the same transaction. This is the inheritance pattern, not a new transaction. The child does not call `with_transaction_async` itself — it uses the parent's open scope. If a child ability attempts its own `with_transaction_async` while a parent transaction is active, it fails with `NestedTransactionsForbidden`.

Abilities that need genuinely independent transactional writes perform them sequentially outside any parent transaction, not nested.

### 5. Who Sets the Mode

`ExecutionMode` is set by the **surface** that constructs `ServiceContext` and propagates through `AbilityContext` to any ability invoked in that request. Abilities cannot change the mode. Surfaces:

| Surface | Default mode | When set otherwise |
|---------|-------------|--------------------|
| Tauri app | `Live` | User-initiated "maintenance dry-run" from Settings uses `Live` with `dry_run=true` on the ability input — not `Simulate`. Dry-run preserves real provider synthesis; Simulate does not. |
| MCP server | `Live` | Never (agents have no permission to escalate or de-escalate mode) |
| Background worker (scheduler) | `Live` | Never |
| Evaluation harness ([ADR-0107](0107-evaluation-harness-for-abilities.md)) | `Evaluate` | Never |
| Developer simulation REPL / dev tooling | `Simulate` | Never — `Simulate` is a developer-mode affordance (stubbed providers, replay fixtures) not intended for user-facing flows |
| Integration test suite | `Simulate` or `Live` (against test DB) | Per-test declaration |

Inside a running invocation, mode is immutable. An ability that invokes another ability passes the same `AbilityContext`; the child ability sees the same mode as the parent.

### 6. Determinism Contract for Evaluate Mode

Evaluate mode requires deterministic execution to make snapshot-diff scoring stable across runs. The following are injected via `ServiceContext`:

- **Clock.** `ctx.clock.now()` returns fixture-supplied timestamps, not the system clock. All abilities and services read time via the injected clock, never via `chrono::Utc::now()` directly.
- **RNG.** `ctx.rng.gen()` returns deterministic values from a fixture-supplied seed. Abilities that sample (e.g., stochastic health scoring in [ADR-0097](0097-account-health-scoring-architecture.md)) use the injected RNG.
- **Intelligence provider.** Replaced with a replay provider that returns fixture-supplied completions keyed by prompt hash.
- **External API stubs.** Replaced with replay fixtures keyed by request signature.

Abilities that bypass these injections (e.g., call `Utc::now()` directly) are non-conforming. CI includes a grep-based lint that flags direct clock and RNG usage in ability code.

In Live mode, the injected clock and RNG are thin wrappers over real ones; the cost is a single indirection per call.

### 7. Error Handling

A mutation attempt blocked by mode returns a distinct error:

```rust
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    // ... existing variants ...
    
    #[error("write blocked by execution mode: {0:?}")]
    WriteBlockedByMode(ExecutionMode),
    
    #[error("nested transactions forbidden")]
    NestedTransactionsForbidden,
}
```

Abilities that call services inside their own dry-run or evaluation paths should never see `WriteBlockedByMode` — they should short-circuit before the service call. If an ability does see it, that is a bug: the ability attempted a mutation in a non-Live mode. The registry's test harness asserts that no maintenance ability in dry-run or evaluation mode reaches a mutation call site.

### 8. Phases

**Phase 1 (v1.4.0 planning window): Foundations.**
- `ExecutionMode` enum and `ServiceContext` amendment land.
- `check_mutation_allowed` added. No services call it yet. Existing mutation functions are untouched.
- `with_transaction_async` implementation lands alongside the existing sync `with_transaction`.
- Clock and RNG injection scaffolding lands; abilities and services gradually migrate to injected variants.

**Phase 2 (v1.4.0 refactor): Service migration.**
- Every mutation function in `services/` adds `ctx.check_mutation_allowed()?` as its first line.
- `PropagationEngine::emit` becomes mode-aware.
- External service wrappers (Glean, Slack, etc.) gain mode routing.
- CI lint for direct clock/RNG use in ability code activates.

**Phase 3 (v1.4.0 cutover): Evaluation harness integration.**
- `ServiceContext::new_evaluate` and fixture wiring enable the evaluation harness ([ADR-0107](0107-evaluation-harness-for-abilities.md)).
- Snapshot-diff evaluation runs against maintenance abilities.
- Regression gate activates in CI.

**Phase 4 (post-v1.4.0): Hardening.**
- `ServiceContext::new` (default, no-mode) removed if it still exists.
- Migration to `with_transaction_async` completes; sync `with_transaction` retained only for non-async code paths.

## Consequences

### Positive

1. **Mode becomes structural, not advisory.** Evaluate and Simulate cannot silently leak writes. The service layer rejects the mutation at the boundary regardless of ability behavior.
2. **Evaluation harness is safe by construction.** An ability invoked under `Evaluate` mode cannot corrupt real state even if the ability has a bug.
3. **Maintenance dry-runs are reliable.** The semantics in [ADR-0103](0103-maintenance-ability-safety-constraints.md) §3 become enforceable rather than aspirational.
4. **Async transactional composition is possible.** Maintenance abilities that compose async service calls can honor atomicity without breaking SQLite's write model.
5. **Determinism enables quality metrics.** Evaluate-mode determinism produces stable snapshot-diffs. Regression scoring becomes meaningful rather than noisy.
6. **Test discipline improves.** Integration tests that need mutations run in Live against a test DB. Tests that do not need mutations run in Simulate. The distinction is typed, not cultural.
7. **Tauri app gains a "what-if" capability.** A user can run a maintenance dry-run from Settings and see exactly what would change without any risk. Same code path as Live, different mode.

### Negative

1. **Every mutation function gains a check call.** Mechanical but touches a lot of files. Roughly ~60 mutation functions identified in [ADR-0101](0101-service-boundary-enforcement.md)'s audit, each needs one line added.
2. **Services cannot use the system clock directly.** Migration of `Utc::now()` call sites to `ctx.clock.now()` across services is a non-trivial sweep.
3. **RNG migration is similar.** Services that sample (health scoring Bayesian updates, reliability Thompson sampling per [ADR-0080](0080-signal-intelligence-architecture.md)) must take RNG through `ctx`.
4. **External-call routing adds a mode branch to every integration.** Glean, Slack, Gmail, Salesforce wrappers each gain a mode check and a fixture replay path.
5. **No default `ServiceContext` constructor.** Callers must choose explicitly. This is correct but adds friction to ad-hoc code.

### Risks

1. **Missed `check_mutation_allowed` calls.** A mutation function added after the sweep forgets to call the mode check, creating a silent mode-bypass. Mitigation: pre-commit hook verifies every `db.insert/update/upsert/delete` has a preceding `check_mutation_allowed` in the same function. Phase 4 adds compile-time enforcement via a macro that wraps all DB writes.
2. **Clock or RNG used directly.** An ability or service calls `Utc::now()` or `rand::thread_rng()` instead of injected versions. Mitigation: CI lint. Offending lines fail the build.
3. **Async transaction contention.** A long-running maintenance transaction blocks user writes. Mitigation: maintenance runs outside user-active hours when possible; budget limits (per [ADR-0103](0103-maintenance-ability-safety-constraints.md) §4) bound transaction duration; blast-radius declaration surfaces expected latency.
4. **Fixture drift.** Evaluation fixtures become stale relative to production data, reducing evaluation value. Mitigation: [ADR-0107](0107-evaluation-harness-for-abilities.md) specifies fixture refresh cadence and anonymization.
5. **Mode confusion in integration tests.** A test runs Simulate when it needs Live, or vice versa, and silently passes without actually testing the mutation path. Mitigation: test-mode declaration is explicit; integration tests that need Live mutations declare so in their fixture setup.
6. **Performance regression from determinism injections.** Thin-wrapper clock and RNG add one indirection per call. Mitigation: Rust inlining handles this at zero runtime cost in Live mode; measured impact is expected to be negligible.

## Known Limitations and Implementation Open Questions

The design locks in the right contract, but several implementation details are deliberately left open. These are acknowledged here so they are not surprises during build:

1. **Async transaction signature needs real Rust work.** The `with_transaction_async<F, Fut, T>` signature in §4 is illustrative. Producing a concrete signature that allows the closure to borrow `TxCtx` while returning a `Future` requires explicit higher-ranked trait bounds (HRTBs) or boxed futures, and the SQLite transaction handle's lifetime needs careful design. Expected implementation path: use `async-trait`-style erasure or a concrete `BoxedFuture<'a>` return type. This is an implementation question for v1.4.0, not a design question. Marker: see `codex-adr-pass2` finding Part 3 #6.
2. **Pre-commit lint for `check_mutation_allowed` is heuristic.** A grep-based pre-commit hook that flags DB-write call sites missing the mode check will miss raw SQL, helper wrappers, aliased service methods, and dynamic dispatch. Phase 4 adds compile-time enforcement via a `#[mutates]` attribute macro that wraps the function body and inserts the check at macro expansion time. Phase 1–3 accept the heuristic gap and backstop with the service layer's runtime check; a missed hook produces a runtime `WriteBlockedByMode` error in `Simulate`/`Evaluate`, not a silent bypass.
3. **Determinism for DB-level defaults and triggers.** SQLite `DEFAULT CURRENT_TIMESTAMP` columns and triggers that compute timestamps bypass the injected clock. `Evaluate` mode cannot fully deterministically reproduce runs if the schema relies on DB-side clocks. Mitigation: the schema audit performed in Phase 2 migrates any DB-side clock usage to service-layer clock injection. Migration ticket tracked separately; until complete, fixtures may see timestamp drift on re-run, which the snapshot-diff scorer tolerates via a small field-level ignore list.
4. **External-call replay interface is load-bearing but specified elsewhere.** Services that call Glean, Slack, Gmail, Salesforce must route through a replay interface in `Simulate`/`Evaluate`. The interface specification lives in [ADR-0107](0107-evaluation-harness-for-abilities.md). Until 0107 lands, the external-call routing in §3.3 is contract-only; tests cannot actually run in `Evaluate` mode against external-dependent abilities.
5. **Provider and external-client wiring.** `ServiceContext` holds mode-aware external clients via `ExternalClients` (§2). `IntelligenceProvider` is held on `AbilityContext` rather than `ServiceContext` because multiple providers may co-exist (different abilities can use different providers), whereas external API clients are scoped to the service layer and share replay fixtures. `ServiceContext::new_simulate` and `new_evaluate` wire the mode-appropriate `ExternalClients` wrappers that route to replay fixtures. The surface (app, MCP, worker, harness) wires the mode-appropriate `IntelligenceProvider` into the `AbilityContext` it constructs. This split is intentional: externals are mode-routed by the service layer; provider selection is per-ability concern owned by the surface.
6. **Panic semantics in async transactions.** "Rollback on panic" is the intended behavior, but panic-safety in async Rust requires `FutureExt::catch_unwind` or a `Drop`-based transaction guard. The implementation follows one of these patterns; which one is an implementation detail to be validated against the `rusqlite`/`sqlx` version in use.

## References

- [ADR-0101: Service Boundary Enforcement](0101-service-boundary-enforcement.md) — This ADR amends `ServiceContext` from that document. All ADR-0101 rules (services own mutations, every mutation emits a signal, multi-write transactionality, error propagation, reads don't mutate) remain in force; this ADR adds mode-awareness as a prerequisite for [ADR-0102](0102-abilities-as-runtime-contract.md) and [ADR-0103](0103-maintenance-ability-safety-constraints.md).
- [ADR-0102: Abilities as the Runtime Contract](0102-abilities-as-runtime-contract.md) — Consumes `AbilityContext` which wraps `ServiceContext` with its mode. §5 and §11 of that ADR require the contract established here.
- [ADR-0103: Maintenance Ability Safety Constraints](0103-maintenance-ability-safety-constraints.md) — §3 (dry-run semantics) and §8 (audit vs. tracer) depend on this ADR. §2 (transactional boundaries) consumes `with_transaction_async`.
- [ADR-0080: Signal Intelligence Architecture](0080-signal-intelligence-architecture.md) — `PropagationEngine::emit` becomes mode-aware per §3.
- [ADR-0091: IntelligenceProvider Abstraction](0091-intelligence-provider-abstraction.md) — Evaluate mode requires a replay variant of `IntelligenceProvider` supplied by the fixture.
- **ADR-0107 (forthcoming): Evaluation Harness for Abilities** — Consumes `ExecutionMode::Evaluate` and specifies fixture structure, replay semantics, and snapshot-diff scoring.

---

## Amendment — 2026-04-20 — Phase 0 acknowledgment: `ServiceContext` does not yet exist

Code-reality check (2026-04-20) confirmed that `ServiceContext` does not exist in the codebase. Services currently receive raw `ActionDb` + individual context objects. The ADR originally read as if `ServiceContext` was a minor amendment to an existing type; in fact the struct must be introduced from scratch.

This amendment makes Phase 0 explicit: landing the struct and wiring the happy path is a prerequisite before any mutation guard, any mode-aware external client, any `with_transaction_async`, or any ability entry point can function.

**Phase 0 deliverable (prerequisite to everything else in this ADR and every downstream ADR):**

- `pub struct ServiceContext<'a>` defined at `src-tauri/src/services/context.rs` (or equivalent module).
- Fields: `mode: ExecutionMode`, `clock: &'a dyn Clock`, `rng: &'a dyn SeededRng`, `external: &'a ExternalClients`, `tx: Option<TxHandle<'a>>`, `db: &'a ActionDb`.
- `Clock` trait with `fn now(&self) -> DateTime<Utc>` + `SeededRng` trait with the RNG surface services use today.
- Explicit constructors `ServiceContext::new_live(...)`, `::new_simulate(...)`, `::new_evaluate(...)`. No `Default` or zero-arg.
- `check_mutation_allowed(&self) -> Result<(), WriteBlockedByMode>` method returning error outside `Live`.
- `with_transaction_async` primitive lands in this same Phase 0 (may require HRTB design iteration; spike signature early).

**What changes in Phase 1 (per original §8):**

- Every mutation function in `services/` (~60 call sites) migrates to take `&ServiceContext` and call `ctx.check_mutation_allowed()?` as first line.
- Direct `Utc::now()` and `rand::thread_rng()` calls replaced with `ctx.clock.now()` and `ctx.rng.*` throughout services and abilities.
- CI lint against direct clock/RNG use in ability code.

**Tracking:** [DOS-209](https://linear.app/a8c/issue/DOS-209) (Substrate: ExecutionMode and Mode-Aware Services) has been amended 2026-04-20 to name `ServiceContext` explicitly as the Phase 0 deliverable. Everything downstream (ADR-0102, 0103, 0105, 0110, 0113, 0115, 0116, 0117, 0119) waits on DOS-209 Phase 0.

### Brownfield-as-greenfield note

Per founder guidance (2026-04-20), backward compatibility with existing zero-arg `ActionDb::open()` callers is not required. Every call site is migrated to construct a `ServiceContext::new_live(...)` explicitly. A temporary `ServiceContext::test_live()` helper under `#[cfg(test)]` exists during migration only; it is guarded by a `compile_error!` check if referenced from non-test code.
