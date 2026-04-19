# ADR-0115: Signal Granularity, Policy Registry, and Durable Invalidation

**Status:** Proposed
**Date:** 2026-04-19
**Target:** v1.4.0 (policy registry + consolidated emission, durable job model, coalescing + back-pressure)
**Extends:** [ADR-0080](0080-signal-intelligence-architecture.md)
**Related:** [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0104](0104-execution-mode-and-mode-aware-services.md), [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0110](0110-evaluation-harness-for-abilities.md), [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md), [ADR-0114](0114-scoring-unification.md)
**Consumed by:** [DOS-235](https://linear.app/a8c/issue/DOS-235) Policy Registry, [DOS-236](https://linear.app/a8c/issue/DOS-236) Durable Invalidation Job Model, [DOS-237](https://linear.app/a8c/issue/DOS-237) Coalescing + Queue Bounds

## Context

[ADR-0080](0080-signal-intelligence-architecture.md) established the signal bus as the backbone of DailyOS intelligence: events fire, listeners react, Bayesian weights update, briefings recompute. It was designed when signals operated at entity granularity — "this account changed," "this person's title moved." Three in-flight changes break that granularity model:

1. **Claim-level events.** [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md) introduces per-claim identity and a claim state machine. Every claim insert, supersede, retract, and trust recompute is an event someone wants to react to. That's claim granularity, orders of magnitude finer than entity granularity.
2. **Ability-output events.** [ADR-0102](0102-abilities-as-runtime-contract.md) makes every capability a named, versioned ability producing an `AbilityOutput<T>`. Each output is cache-keyed and invalidation-tracked. "This output changed" is a new event class operating at ability-output granularity.
3. **Trust recomputes as a high-volume source.** [ADR-0114](0114-scoring-unification.md) + [DOS-5](https://linear.app/a8c/issue/DOS-5) mean every claim gets a trust score that may shift on any of six factor inputs. Backfills and shadow-mode runs can produce millions of `ClaimTrustChanged` events in hours.

Three existing problems also need naming so the fix is unambiguous:

- **Policy distributed across call sites.** Today the decision "emit this signal synchronously" vs "fire-and-forget" lives at each call site. Three overlapping functions (`emit`, `emit_signal`, `emit_signal_and_propagate`) coexist with inconsistent semantics. A maintainer who doesn't know the signal's invariants can silently pick the wrong variant, and the signal bus has no compile-time way to catch this.
- **Depth-limited drops.** The original invalidation design capped propagation depth and dropped signals past the cap to avoid runaway cascades. That is correctness-unsafe — a dropped invalidation leaves briefings, scores, and claims silently stale. The adversarial review flagged this.
- **Back-pressure missing.** High-volume sources (trust recompute, shadow-mode runs, background enrichment) can flood the signal bus. Without coalescing, rate limits, and a bounded queue, the system degrades from "eventual consistency" to "eventual collapse."

This ADR unifies granularity, moves propagation policy from call sites into a compile-time-checked registry, replaces depth-limited drops with durable invalidation jobs, and adds the back-pressure mechanisms needed for claim-level and ability-level events to be safe in production.

## Decision

### 1. Signal granularity levels

Signals operate at one of four granularity levels. Every `SignalType` variant declares its level in the policy registry (§3).

| Level | Example signal types | Typical volume |
|---|---|---|
| Entity | `EntityUpdated`, `AccountHealthRecomputed`, `PersonRoleChanged` | 10² / day |
| Ability output | `AbilityOutputChanged { ability_name, output_id }`, `AbilityOutputInvalidated` | 10³ / day |
| Claim | `ClaimAsserted`, `ClaimSuperseded`, `ClaimRetracted`, `ClaimTrustChanged`, `ClaimContradiction` | 10⁴ / day |
| Session | `UserSessionStarted`, `FeedbackSubmitted` | 10² / day |

Granularity is a property of the signal type, not the emitter. The signal catalog in `src-tauri/src/signals/types.rs` groups types by level for readability.

### 2. Propagation policies

Every `SignalType` maps to exactly one `PropagationPolicy`:

```rust
pub enum PropagationPolicy {
    Local,
    PropagateSync,
    PropagateAsync { coalesce: bool },
    PropagateAndHeal,
}
```

- **Local** — emitted to the event log and subscribers in-process; does not invalidate downstream outputs. Example: `UserSessionStarted`.
- **PropagateSync** — emitted, subscribers react synchronously in the same transaction, and invalidation jobs for affected outputs are written inline. Used when downstream correctness must hold before the current transaction commits. Example: `ClaimRetracted`, `UserCorrectionSubmitted`.
- **PropagateAsync { coalesce }** — emitted to the event log, an invalidation job is enqueued for each affected output. If `coalesce = true`, jobs for the same `(SignalType, EntityId)` collapse within a configurable window. Example: `EntityUpdated { coalesce: true }`, `ClaimTrustChanged { coalesce: true }`, `AbilityOutputChanged { coalesce: true }`.
- **PropagateAndHeal** — emitted, async invalidation enqueued, **and** a healing job is triggered to re-derive the authoritative state from upstream sources (used when a signal implies the current state may be systemically wrong). Rate-limited per entity per signal type (§6). Example: `SourceRevoked`, `ClaimContradiction`.

Policies are declared, not chosen per call site.

### 3. Policy registry — compile-time checked

`src-tauri/src/signals/policy_registry.rs` contains a `const` slice mapping every `SignalType` variant to its `PropagationPolicy`. A Rust exhaustiveness check (via an unreachable arm on a `match` over the enum) fails the build if a variant is missing.

```rust
pub const SIGNAL_POLICY_REGISTRY: &[(SignalType, PropagationPolicy)] = &[
    (SignalType::EntityUpdated,         PropagationPolicy::PropagateAsync { coalesce: true }),
    (SignalType::ClaimAsserted,         PropagationPolicy::PropagateAsync { coalesce: false }),
    (SignalType::ClaimSuperseded,       PropagationPolicy::PropagateAsync { coalesce: false }),
    (SignalType::ClaimRetracted,        PropagationPolicy::PropagateSync),
    (SignalType::ClaimTrustChanged,     PropagationPolicy::PropagateAsync { coalesce: true }),
    (SignalType::ClaimContradiction,    PropagationPolicy::PropagateAndHeal),
    (SignalType::AbilityOutputChanged,  PropagationPolicy::PropagateAsync { coalesce: true }),
    (SignalType::SourceRevoked,         PropagationPolicy::PropagateAndHeal),
    (SignalType::UserCorrectionSubmitted, PropagationPolicy::PropagateSync),
    // ...
];

// Exhaustiveness check (compile-time)
#[cfg(test)]
const _: () = {
    fn assert_all_variants(s: SignalType) {
        match s {
            SignalType::EntityUpdated => (),
            SignalType::ClaimAsserted => (),
            // ... must list every variant; missing one fails compile
        }
    }
};
```

Policy changes require editing the registry, which appears in diffs. Distributed call-site policy decisions are no longer possible.

### 4. Consolidated emission function

One canonical emission function replaces the three existing variants:

```rust
pub fn emit_signal(signal: Signal) -> Result<()>;

pub fn emit_signal_with_override(
    signal: Signal,
    override_policy: PropagationPolicy,
    exception_reason: &'static str,
) -> Result<()>;
```

- `emit_signal` looks up the policy from the registry and applies it. Callers never choose a policy.
- `emit_signal_with_override` exists only for documented exceptions (tests, migrations, one-shot backfills). The `exception_reason` parameter must be a `&'static str` — the reason is visible in code and log, not in runtime values. Every override is a maintainable exception, not a silent deviation.

Existing call sites migrate to `emit_signal(signal)`. The legacy `emit`, `emit_signal_and_propagate` functions are deleted after migration. No deprecation period — call-site audit ([DOS-235](https://linear.app/a8c/issue/DOS-235)) replaces them exhaustively.

### 5. Durable invalidation jobs

The prior depth-limited-drop design is replaced with a durable job model. Invalidation is a database-backed work queue, not in-memory propagation.

```rust
pub struct InvalidationJob {
    pub id: JobId,
    pub chain_id: ChainId,                      // Shared across all jobs in a causal chain
    pub origin_signal_id: SignalId,
    pub depth: u32,                             // Chain depth, monotonic
    pub affected_output_ids: Vec<OutputId>,
    pub enqueued_at: DateTime<Utc>,
    pub status: JobStatus,                      // Pending / Running / Completed / Failed / DeadLettered / CycleDetected
    pub attempt_count: u32,
    pub last_error: Option<String>,
}
```

**Schema:** `invalidation_jobs` table with indexes on `status`, `chain_id`, and `enqueued_at`.

**Worker pool:** bounded, default size 4, configurable. Workers pull `Pending` jobs in FIFO order within chain, exponential backoff on retry.

**Idempotent recompute:** each affected output addressable by `(OutputId, content_version)`. Duplicate jobs on the same tuple collapse at enqueue time — the queue never holds two pending jobs for the same invalidation target.

**Cycle detection:** a job's chain contains its `OutputId` or depth exceeds the configured cap (default 16) → `status = CycleDetected`, affected outputs marked with `last_known_good_as_of: T`. **Outputs are never silently dropped.** The stale marker is visible to downstream surfaces, which render it explicitly.

**Dead-letter queue:** retry-exhausted jobs (default 5 attempts with 1s, 4s, 16s, 64s, 256s backoff) transition to `DeadLettered`. Affected outputs marked stale. An admin surface shows the dead-letter queue (v1.5.0; the data model and telemetry land in v1.4.0).

**Healing jobs:** signals with `PropagateAndHeal` policy enqueue a healing job alongside the invalidation job. Healing jobs re-derive state from upstream sources; rate-limited per §6.

### 6. Coalescing, queue bounds, and back-pressure

High-volume claim-level and ability-output-level events require back-pressure to keep the bus safe.

**Coalescing window (default 500ms).** Signals with `PropagationPolicy::PropagateAsync { coalesce: true }` and matching `(SignalType, EntityId)` within the window collapse to a single invalidation job. Coalescing is configurable per `SignalType` — `ClaimTrustChanged` may use a wider window (e.g., 2s) than `EntityUpdated` (500ms). Coalescing preserves the newest signal's metadata; earlier ones are dropped from the job but remain in the event log for audit.

**Queue bounds (default 10,000 pending jobs).** When the queue reaches the bound:

- **Aggressive coalescing mode** activates — coalescing windows widen to 5s for coalescable types, collapsing more aggressively.
- If the queue is still full after aggressive coalescing for 30s, subsequent non-coalescable emissions return an error. Callers log and signal the operator; **jobs are never silently dropped.** Emission errors propagate to the caller; the caller decides whether to retry or surface to the user.
- The event log itself (for audit) never blocks — only the invalidation queue does.

**Per-entity rate limits.** Claim-level events for a single entity are capped at 50/minute. Exceeding triggers adaptive coalescing for that entity. Rationale: a single hot account should not starve the global queue.

**Trust-score micro-drift suppression.** `ClaimTrustChanged` fires only on **band-boundary crossings** (`likely_current` ↔ `use_with_caution` ↔ `needs_verification`). A trust score drifting from 0.72 to 0.73 does not emit; 0.71 to 0.69 (crossing the `likely_current → use_with_caution` boundary) does. The band is computed in the Trust Compiler ([DOS-5](https://linear.app/a8c/issue/DOS-5)) and passed into the emit check.

**Healing rate limits.** Healing jobs per `(entity_id, signal_type)` capped at 3/hour. Enforced by the healing scheduler, not the emitter. Prevents a flapping signal from burning upstream API budget.

**Load-test gate.** Before any claim-level `PropagateAsync` or `PropagateAndHeal` ships to production ([DOS-237](https://linear.app/a8c/issue/DOS-237)):

- Synthetic workload: 1,000 `ClaimTrustChanged` per minute + 100 `AbilityOutputChanged` per minute sustained for 10 minutes.
- Verify coalescing effective (effective emit rate ≤ 10% of raw).
- Queue never exceeds bound.
- Dead-letter rate under 0.1%.
- Every affected output recomputed or marked stale — no silent-stale-without-marker.

Failing the gate blocks the claim-level release.

### 7. Event log separation

The event log (audit trail of every signal ever emitted) is separate from the invalidation queue (work to be done). The log is append-only, retains all signals including coalesced ones, and is used for replay, telemetry, and provenance. The queue is transient — jobs complete, fail, or dead-letter, and are purged after a configurable retention period (default 90 days).

This separation is important: the log never drops, the queue applies back-pressure. Audit integrity and operational stability are independent concerns.

### 8. Interaction with execution modes

Under `ExecutionMode::Evaluate` ([ADR-0104](0104-execution-mode-and-mode-aware-services.md)):

- Signals are emitted to an in-memory ring buffer, not persisted.
- Invalidation jobs are executed inline rather than enqueued.
- Coalescing window is 0 (immediate); back-pressure disabled.
- Workers do not run; the test or fixture asserts expected job completions.

This keeps `Evaluate` deterministic and fast, without changing the semantics that `Live` mode uses.

### 9. Observability

- Every emission logs `(signal_id, signal_type, policy_applied, chain_id, coalesced_with)`.
- Queue depth, throughput, coalescing rate, dead-letter rate, and cycle-detected count exposed as metrics.
- Dead-letter surface shows job payload, attempt history, and last error. Admin-visible in v1.5.0.

## Consequences

### Positive

- **Policy is compile-time checked.** Adding a new `SignalType` without a policy fails the build. No silent semantic choices per call site.
- **One emission function.** `emit_signal(signal)` is the canonical path; reviewers see one call and know the policy is correct by construction. Exceptions are explicit and rare.
- **Durable invalidation.** No dropped signals. Cycle and depth-cap cases mark stale rather than silently losing work.
- **Back-pressure real.** Coalescing, queue bounds, and rate limits make claim-level events safe at high volume. The load-test gate ensures we don't find the limits in production.
- **Event log and queue decoupled.** Audit and operation are independent; one does not starve the other.
- **Healing rate-limited.** Flapping signals cannot melt upstream API budgets.
- **Band-boundary trust emission.** 90%+ reduction in `ClaimTrustChanged` volume without losing any actionable state change.

### Negative / risks

- **Durable queue adds write volume.** Every async invalidation is now a DB insert. Mitigated by coalescing and bounded workers; measured in the load test.
- **`emit_signal_with_override` is a documented escape hatch.** It must not become a normal path. PR review convention: any new use of `_with_override` requires an ADR amendment or a linked issue explaining the exception.
- **Registry edits show up in diffs (intentional).** A contributor adding a signal type must learn the policy model. This is a feature, not a tax — but it raises the bar for "just add a signal."
- **Cycle detection may hide genuine bugs.** An output marked stale due to cycle is correct but may signal a design problem in the ability graph. Dead-letter and cycle counts must be monitored; chronic cycles deserve refactor, not config.
- **Healing rate limits may delay recovery from a revoked source.** Default 3/hour per `(entity, signal_type)` is a conservative floor; tuning required in v1.4.1 shadow.

### Neutral

- No user-visible behavior change in v1.4.0 beyond surfaces consuming stale markers rendering them explicitly.
- [ADR-0080](0080-signal-intelligence-architecture.md) is not replaced; it is the substrate this ADR refines. The Bayesian weight-update semantics remain as specified there.
- Load-test gate is a ship gate, not a CI gate (too slow for PR). Runs weekly in a dedicated environment.

---

## Revision R1 — 2026-04-19 — Reality Check

Adversarial review + reference pass found the ADR built on typed machinery that does not exist. Today's signal bus is stringly-typed, has four emission functions (not three), and lacks the enum/registry infrastructure this ADR assumes. Revision below.

### R1.1 `SignalType` enum is a prerequisite, not a feature of this ADR

Ground truth: there is no `SignalType` enum. Signal types are strings across 41 files. The original §3 compile-time-checked policy registry is infeasible without first introducing the enum. This blocks everything downstream.

**Revised sequencing — Phase 0 precedes §3–§6:**

- **Phase 0 (prerequisite work, lands before any policy registry):** Introduce `pub enum SignalType` in `src-tauri/src/signals/types.rs`. Initial variants mirror the existing string catalog found in signal-emitting code (`stakeholder_change`, `title_change`, `champion_risk`, `renewal_risk_escalation`, `engagement_warning`, `intelligence_confirmed`, `meeting_frequency_drop`, etc.). Plus the new claim-layer variants from [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md) (`ClaimAsserted`, `ClaimSuperseded`, `ClaimRetracted`, `ClaimTrustChanged`, `ClaimContradiction`). The enum is `#[non_exhaustive]` for forward compatibility.
- `signal_events.signal_type` column stays `TEXT` at the DB level; the Rust layer serializes the enum. Migration is not required.
- Phase 0 lands as a separate issue that [DOS-235](https://linear.app/a8c/issue/DOS-235) depends on.

### R1.2 Actual emission function count is four, not three

Reference pass identified four existing emission functions:

- `bus::emit()` at `src-tauri/src/signals/bus.rs:134` — modern structured builder, 54 call sites.
- `bus::emit_signal()` at `:172` — legacy positional args, 27 sites.
- `bus::emit_signal_and_propagate()` at `:215` — deprecated in favor of evaluate variant, 35 sites.
- `bus::emit_signal_propagate_and_evaluate()` at `:266` — current modern variant with self-healing evaluation hook (I410), 2 sites.

Plus 195 additional call sites via `services::signals::*` wrappers.

**Revised §4:** the consolidation target is `emit_signal(signal: Signal) -> Result<()>` as originally specified. All four existing functions + the service wrappers migrate to it. Migration scope: 318 call sites total. Acknowledged as a significant audit, not a local refactor. [DOS-235](https://linear.app/a8c/issue/DOS-235) sizing revised upward.

### R1.3 Compile-time exhaustiveness cannot rely on `#[cfg(test)]`

Codex flagged: `#[cfg(test)]` exhaustiveness only fails test builds, not normal builds. A contributor adding a `SignalType` variant can ship without adding a registry entry and the production build will compile.

**Revised:** exhaustiveness enforced via a non-test `const` fn that performs an exhaustive match:

```rust
const _: () = {
    const fn assert_registry_complete(s: SignalType) -> PropagationPolicy {
        match s {
            SignalType::EntityUpdated => PropagationPolicy::PropagateAsync { coalesce: true },
            SignalType::ClaimAsserted => PropagationPolicy::PropagateAsync { coalesce: false },
            // ... every variant must be listed
        }
    }
};
```

Missing a variant fails every build, not just tests.

**Also:** `SignalType` variants that carry data (e.g., `AbilityOutputChanged { ability_name, output_id }`) do not fit the `&[(SignalType, PropagationPolicy)]` slice shape. Use a function form instead: `fn policy_for(signal: &SignalType) -> PropagationPolicy` with an exhaustive match. The "const slice" in the original §3 is replaced by a function.

### R1.4 Transactionality fix — event log + invalidation enqueue must share the transaction

Codex flagged: the original §5 says the event log "never blocks" but the invalidation queue applies back-pressure. If an emit commits to the log but the enqueue fails, outputs go stale while the audit says no signal was dropped.

**Revised:** `emit_signal` writes to the event log and enqueues invalidation jobs **in the same DB transaction**. Either both succeed or both roll back. If the invalidation queue is at its bound, the transaction fails at enqueue time and the emission errors up to the caller. The "aggressive coalescing mode" (§6) activates **before** transaction failure to widen the queue's effective capacity first; only if coalescing cannot clear the pressure does the transaction fail.

The "event log never blocks" claim in §6 is revised: under normal conditions emissions commit quickly because coalescing absorbs bursts; under sustained pressure past coalescing, the log does block the emitter, because the emitter needs to know its signal was not silently dropped.

### R1.5 Coalescing key — add granularity

Codex flagged: coalescing by `(SignalType, EntityId)` collapses claim-level or field-level changes into one job. Stale outputs either skip needed recomputation or over-recompute.

**Revised:** coalescing key is per-signal-type-specific, declared alongside the policy:

```rust
pub enum PropagationPolicy {
    // ...
    PropagateAsync { coalesce: Option<CoalesceKey> },
}

pub enum CoalesceKey {
    EntityId,                              // Coarsest — per-entity
    EntityAndClaimType,                    // Per-claim-type within entity
    EntityAndField { field_path: bool },   // Per-field within claim type
    OutputId,                              // Per-output-id for ability-output changes
}
```

`ClaimTrustChanged` uses `EntityAndField`. `AbilityOutputChanged` uses `OutputId`. `EntityUpdated` uses `EntityId` (current behavior). Per-type default in the registry, overridable by config.

### R1.6 Cycle detection needs ancestry — expand the job shape

Codex flagged: the proposed `InvalidationJob` shape does not carry enough history to detect `A → B → A` cycles.

**Revised:** add `chain_ancestry: Vec<OutputId>` to the job shape. Each job inherits its parent's ancestry and appends its origin output. Cycle detection: if a proposed job's `affected_output_ids` intersect `chain_ancestry` within the current chain, the job transitions to `CycleDetected` immediately without enqueue. Depth cap is retained as a belt-and-suspenders second line.

Storage: `chain_ancestry` is a JSON array of OutputIds; bounded by the depth cap (default 16). Not a separate table.

### R1.7 Evaluate-mode mutation conflict with ADR-0103/0104

Codex flagged: [ADR-0103](0103-maintenance-ability-safety-constraints.md) and [ADR-0104](0104-execution-mode-and-mode-aware-services.md) say non-Live modes must not mutate. The original §8 says `Evaluate` mode runs inline invalidation jobs — which can mutate caches or touch queues.

**Revised:** under `Evaluate`, `emit_signal` writes to the in-memory ring buffer (no DB, no queue) and **does not run invalidation jobs**. Consumers that need to assert on invalidation behavior in tests use a separate `run_pending_invalidations_in_memory()` helper that a fixture explicitly invokes. This is analogous to how mutation-aware testing uses explicit flushes rather than implicit cascade. Matches [ADR-0104](0104-execution-mode-and-mode-aware-services.md)'s no-side-effect contract.

### R1.8 `OutputId` model dependency

Codex flagged: the ADR consumes an `OutputId` concept that doesn't exist in code yet ([ADR-0102](0102-abilities-as-runtime-contract.md)/[ADR-0105](0105-provenance-as-first-class-output.md) are also doc-only).

**Revised:** `OutputId` is defined in this ADR as a dependency placeholder and implemented in the abilities runtime work ([ADR-0102](0102-abilities-as-runtime-contract.md) implementation). Shape:

```rust
pub struct OutputId {
    pub ability_name: &'static str,
    pub ability_version: AbilityVersion,
    pub input_key: Hash,  // Canonical hash of ability inputs + composing provenance
}
```

This ADR's Phase 0 is blocked on the `AbilityOutput` + `Provenance` types landing from [ADR-0102](0102-abilities-as-runtime-contract.md) / [ADR-0105](0105-provenance-as-first-class-output.md) implementation. Document that dependency explicitly in [DOS-235](https://linear.app/a8c/issue/DOS-235)+.

### R1.9 Scope for v1.4.0 — revised

- Phase 0: `SignalType` enum, migration of existing string-typed signals to enum-serialized strings in `signal_events`, audit of 318 call sites.
- Phase 1: Policy registry as a function (not const slice), consolidated `emit_signal`.
- Phase 2: Durable invalidation jobs with transactional enqueue, cycle detection via chain ancestry, coalescing with granular keys.
- Phase 3: Load test gate.

Phase 0 is a hard prerequisite for the others. Acknowledged as larger scope than original.
