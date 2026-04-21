# Consistency Model

**Purpose:** State, per operation class, what consistency guarantee DailyOS provides. A new contributor should not have to infer from ADRs independently.
**Date:** 2026-04-20 | **Reflects:** ADRs 0101–0120 + R1/R2 amendments
**Audience:** Anyone reasoning about "if I write X, when does Y see it?"

## The short version

DailyOS is a single-user native app over encrypted SQLite. Most reads are strongly consistent. Invalidation of downstream outputs is eventually consistent, bounded by the invalidation queue processing time. Some specific flows (user corrections) use bounded-synchronous propagation so "save and immediately render" feels correct.

No distributed consistency primitives. No eventual-consistency surprises across machines. One DB file, one process.

## Per operation class

### Claim reads

**Within a transaction:** strongly consistent. A `commit_claim` inside a transaction followed by a query in the same transaction returns the committed row. SQLite's default isolation (serializable for writers) provides this.

**Across transactions:** strongly consistent. The claim table is the source of truth; there are no read replicas or caches that serve stale data.

**Default read filter:** `claim_state = 'committed' AND superseded_at IS NULL`. Tombstoned claims are included in this filter (they're authoritative negative assertions). See [ADR-0113 R1.1](../decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md#r11-state-model-fix--tombstones-are-a-distinct-state-not-a-committed-with-null).

**History reads:** drop the default filter. Return all states including superseded and withdrawn.

### Claim writes (`commit_claim`, `propose_claim`)

**Strongly consistent.** SQLite serializable isolation + pessimistic row-lock on `(entity_id, claim_type, field_path)` per [ADR-0113 R2](../decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md#revision-r2--2026-04-20--pessimistic-row-lock-on-commit_claim).

Two concurrent writers against the same field_path: one acquires the lock, writes, releases; the second acquires, reads the (now-updated) latest claim, routes correctly per [ADR-0113 R1.2](../decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md#r12-supersede-vs-contradiction--clarified-by-source-identity) (supersede vs contradiction vs new insert), writes.

**Lock timeout:** 500 ms default. Exceeded → `Err(ClaimError::LockTimeout)`. Caller retries or gives up.

### Signal emission and event log

**Strongly consistent within the emitting transaction.** [ADR-0115 R1.4](../decisions/0115-signal-granularity-audit.md#r14-transactionality-fix--event-log--invalidation-enqueue-must-share-the-transaction) specifies the event log write and the invalidation job enqueue share the same DB transaction. Either both happen or neither does.

**Eventually consistent across invalidation.** Downstream outputs dependent on the signal become consistent when the invalidation job processes. Queue processing time under normal load is seconds; under pressure, bounded by back-pressure mechanisms in [ADR-0115 §6](../decisions/0115-signal-granularity-audit.md).

**Exception: bounded-synchronous.** `PropagateSync { await_completion: true }` ([ADR-0115 R2](../decisions/0115-signal-granularity-audit.md#revision-r2--2026-04-20--propagatesync--await_completion--variant)) waits up to 500 ms for the invalidation job to complete before `emit_signal` returns. Used for user-correction → next-briefing paths.

**Exception: PropagateSync without await.** Event log + invalidation enqueue in the same transaction; the invalidation processes async after return. The caller is guaranteed the signal will propagate, not that it has propagated yet.

### Trust score computation

**Deterministic given inputs at time T.** Pure function of claim corroborations, contradictions, source reliability posterior, freshness decay, user feedback ([ADR-0114](../decisions/0114-scoring-unification.md), [DOS-5](https://linear.app/a8c/issue/DOS-5)). The same inputs and clock produce the same score.

**Re-computable.** Trust history is not persisted; the compiler recomputes on demand. If audit demand ever emerges for historical trust scores, add a `claim_trust_history` table as follow-on ([ADR-0118 Gap C resolution](../decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md)).

**Not hot-reloaded.** Config changes (weights, band thresholds, freshness decay half-lives) apply on next compilation pass, not to cached scores. Restart or explicit recompute.

### Ability invocation

**Read abilities:** no mutation anywhere in call graph ([ADR-0102 §3](../decisions/0102-abilities-as-runtime-contract.md)). Idempotent. Safely cacheable by composite key including `ctx.clock.now()` bucketed + DB watermarks.

**Transform abilities:** no mutation. Results untrusted for mutation authorization ([ADR-0102 §3](../decisions/0102-abilities-as-runtime-contract.md)). Composition tree is synchronous; one Transform composing another sees strongly consistent output.

**Publish abilities:** Pencil phase (draft) is local, transactional. Pen phase writes the outbox entry transactionally and returns a receipt; actual delivery is async ([ADR-0117 R1.2](../decisions/0117-publish-boundary-pencil-and-pen.md#r12-completion-contract-fix--commit_publish-returns-queue-receipt-not-publishedrecord)). At-least-once delivery with idempotency keys enforces exactly-once side effects on destinations.

**Maintenance abilities:** constrained by [ADR-0103](../decisions/0103-maintenance-ability-safety-constraints.md) — idempotent, transactional, mode-aware, budgeted. Multiple passes of the same maintenance ability converge to the same state.

### Evaluator retry

**Within the invocation.** [ADR-0119](../decisions/0119-runtime-evaluator-pass-for-transform-abilities.md) runtime evaluator pass runs after primary Transform output. If composite < threshold, **single retry** with critique in the same invocation boundary. The retry's output replaces the primary in the returned `AbilityOutput<T>`. No recursion — retry fails → ships with score annotated.

Test-time evaluation ([ADR-0110](../decisions/0110-evaluation-harness-for-abilities.md)) is offline; no consistency concern.

### Mode boundaries

Under `ExecutionMode::Evaluate`:

- DB writes fail-hard via `check_mutation_allowed()` ([ADR-0104](../decisions/0104-execution-mode-and-mode-aware-services.md)).
- Signals emit to in-memory ring buffer; no event log write, no invalidation ([ADR-0115 R1.7](../decisions/0115-signal-granularity-audit.md#r17-evaluate-mode-mutation-conflict-with-adr-01030104)).
- Provider calls route to fixture replay; no real LLM or Glean ([ADR-0106](../decisions/0106-prompt-fingerprinting-and-provider-interface.md)).
- Clock + RNG from fixture, not wall clock ([ADR-0104 §6](../decisions/0104-execution-mode-and-mode-aware-services.md)).
- Runtime evaluator ([ADR-0119 §7](../decisions/0119-runtime-evaluator-pass-for-transform-abilities.md#7-mode-awareness)) does not run.

Under `ExecutionMode::Simulate`:

- Writes fail-hard just like Evaluate (per ADR-0104 rules).
- Signals emit with `mode: "Simulate"` flag.
- Real providers may be called but with tracing markers; explicit guard when the ability checks for real external effect.

Under `ExecutionMode::Live`:

- All operations as described in this document.

## What is **not** guaranteed

- **Strong consistency across machines.** Not applicable — single-user, single-device.
- **Strong consistency across process boundaries.** DailyOS is a single process; if future tooling ever runs two processes against the same DB, this assumption breaks and claim-commit locks need to become DB-backed rather than app-level mutexes ([ADR-0113 R2](../decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md#revision-r2--2026-04-20--pessimistic-row-lock-on-commit_claim)).
- **Durable delivery of signal events to non-DB consumers.** Signals are DB-backed. If a future feature wires signals to an external webhook, the delivery semantics there become at-least-once and need their own idempotency design.
- **Content consistency across publish destinations.** A publish goes out; the destination's consistency is the destination's concern. DailyOS guarantees exactly-once delivery per `IdempotencyKey` (per destination), not synchronization across destinations.

## Quick reference

| Operation | Consistency | Timeout/bound |
|---|---|---|
| Claim read (default filter) | Strong | — |
| Claim write | Strong, pessimistic-locked | 500 ms lock |
| Signal emit (event log + invalidation enqueue) | Strong within transaction | — |
| Signal invalidation processing | Eventually consistent | Queue bound 10 K jobs |
| `PropagateSync { await_completion: true }` | Bounded-synchronous | 500 ms |
| Trust score computation | Deterministic; re-computable | — |
| Read ability output | Strong; cacheable with composite key | — |
| Publish delivery | At-least-once with exactly-once idempotency | Per-destination retry |
| Evaluator retry | Single retry within invocation | — |

## When you're not sure

Default: **assume eventual consistency across transactions; strong consistency within.** If your feature breaks under that assumption, you need `PropagateSync { await_completion: true }` explicitly. Anything else is a sign the consistency model needs sharpening, not that this doc is missing something.
