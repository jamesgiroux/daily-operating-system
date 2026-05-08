# DOS-306 - Signal durability, policy registry, and claim-event load contract

**Status:** frozen W1 stage-1 contract. Implementation verification is deferred to `.docs/plans/wave-W1/DOS-306-verification.md` after W1-B/C/D merge.
**Acceptance walk last refreshed:** 2026-05-08.

## Contract

Signals notify and invalidate. Jobs schedule, retry, coalesce, lease, and dead-letter work. Claims own truth, provenance, trust, contradiction state, tombstones, and user corrections. Read models and briefing/prep/email artifacts are rebuildable projections.

The signal registry is the only place that decides signal policy. The durable invalidation queue is the only durable path from a propagating signal to claim/read-model recompute. Claim recompute consumes queue jobs idempotently and re-reads canonical claims/source rows; it must not treat signal payloads as authoritative facts.

For any Live-mode signal whose policy requires downstream work, `emit_signal` commits the signal event and the required invalidation job rows in the same SQLite transaction. Either the signal and every required job commit together, or the caller receives an error and neither is durable. No code path may commit a claim event and then best-effort enqueue recompute work afterward.

## Boundary roles

| Boundary | Owns | Does not own |
|---|---|---|
| Signal registry | `SignalType` inventory, propagation policy, durability class, coalescing key/window, target resolver, channel eligibility | Claim truth, retry state, worker leases, direct read-model writes |
| Signal emission | Append `signal_events`, resolve policy, create signal-originated job rows in the same transaction, return queue receipts | Running long recompute work inline except bounded post-commit wait |
| Durable invalidation queue | Job state, idempotency key, chain id, ancestry, leases, retries, dead-letter/cycle states, stale markers | Policy selection by call site, source-of-truth claim decisions |
| Claim recompute | Re-read canonical claims/source rows, recompute trust/read models, apply projections, mark jobs terminal | Trusting signal payloads as facts, bypassing claim commit/projection services |
| Surfaces | Present active/proposed/stale states and dispatch explicit user intent | Running LLM enrichment or mutating enrichment state directly |

## Signal bus durability semantics

### Durability classes

| Class | Examples | Event log | Job delivery | Loss/coalescing allowed | Caller semantics |
|---|---|---|---|---|---|
| Durable propagation | `ClaimAsserted`, `ClaimSuperseded`, `ClaimRetracted`, `ClaimContradiction`, user correction/tombstone signals, source lifecycle signals, non-coalescable ability-output invalidations | Committed in Live mode | Required job rows committed atomically with the event | No job loss. Retries may duplicate execution, so jobs must be idempotent | `Ok` means event and job receipt are durable; failure rolls back the emission unit |
| Coalesced durable propagation | `ClaimTrustChanged` on trust-band boundary crossings, high-volume `AbilityOutputChanged`, broad `EntityUpdated`/read-model invalidations | Every raw signal event is committed in Live mode | At least one pending/running job per coalescing key survives | Individual job payloads may collapse; the newest metadata wins, earlier raw events remain auditable | `Ok` means the raw event is durable and covered by a durable job |
| Durable local/audit | Session, audit, or local-only observations with `Local` policy | Committed if emitted through the bus | No invalidation job required | Downstream delivery is not promised because no downstream durable work exists | `Ok` means the event is durable only |
| Ephemeral/non-bus | UI progress ticks, debug telemetry, metrics samples, Evaluate/Simulate ring-buffer observations | Not written to Live `signal_events` unless explicitly promoted to a signal | None | Fully lossy | Must not be consumed for claim correctness |

### Guaranteed-delivered signals

A signal is guaranteed-delivered when its registry policy is `PropagateSync`, `PropagateAsync`, or `PropagateAndHeal` in Live mode. The guarantee is at-least-once durable work delivery:

- A committed signal has a committed `signal_events` row.
- A committed propagating signal has all required `invalidation_jobs` rows or is covered by a coalesced pending/running job with the same registry-declared coalescing key.
- Pending/running jobs survive process restart.
- Each affected output is eventually recomputed, marked stale, dead-lettered, or cycle-detected. Silent stale output is not allowed.
- Duplicate execution after retry is allowed; duplicated durable jobs for the same target/idempotency key are not.

The guarantee does not mean recompute finishes before `emit_signal` returns, except for registry entries explicitly marked `PropagateSync { await_completion: true }`. Even then, the wait is post-commit, bounded, and returns timeout rather than silently rendering stale state.

### Lossy or coalescable signals

Loss is allowed only where the registry says so:

- Coalescable policies may collapse multiple raw signals into one job per coalescing key/window. The job may keep only newest payload metadata, but raw signal audit rows remain.
- Evaluate and Simulate modes record signals in an in-memory fixture/ring-buffer path and do not persist DB rows or queue jobs.
- UI progress, telemetry, and debug channels are not correctness signals. They must not drive claim recompute, tombstone behavior, or read-model freshness.

Claim state transitions, user corrections, tombstones, source withdrawal/restriction, and contradiction signals are never lossy in Live mode.

## Policy registry semantics

### Exhaustiveness check

`SignalType` is the closed Rust inventory for production signals. Because some variants may carry data, policy lookup is a function, not a const slice:

```rust
pub fn policy_for(signal: &SignalType) -> SignalPolicy {
    match signal {
        // every variant listed here
    }
}
```

The match must live in non-test code so adding a `SignalType` variant without a policy fails normal builds. Tests may add coverage, but test-only exhaustiveness is not sufficient.

Each registry entry must declare:

- durability class: local, durable propagation, coalesced durable propagation, or propagate-and-heal
- execution-mode behavior: Live persistence vs Evaluate/Simulate in-memory capture
- coalescing key and default window, if any
- target resolver: how signal metadata maps to affected claim/read-model outputs
- retry/dead-letter class and stale-marker behavior
- whether `await_completion` is allowed, and the timeout if it is
- payload privacy classification; signal payloads must not carry PII or claim facts needed for recompute

### Channel inventory

W1-B must write `.docs/plans/wave-W1/W1-B-channel-inventory.md` before refactoring emission call sites. The inventory is part of this contract, not an optional audit note.

Minimum required channel families:

| Channel family | Contract |
|---|---|
| `services/` service calls | Route through the canonical service facade and then `emit_signal`; no raw bus variants after consolidation |
| `abilities/` and ability runtime | Emit only through ServiceContext-approved capability handles; mode behavior comes from the registry |
| Tauri, MCP, Worker, Eval bridges | Bridge-originated signals route through the registry or are explicitly excluded as ephemeral/test-only |
| Trigger-derived state and derived-state subscribers | Cannot insert `signal_events` directly; subscribers consume emitted signals and write derived state through owned services |
| Connector/background processors | Gmail, Glean, Clay, Gravatar, Linear, transcript/file processors, schedulers, and hygiene workers route through the same facade |
| Replay/evaluation binaries | In Evaluate mode, capture in memory and require explicit fixture flush helpers for invalidation assertions |
| `src-tauri/src/bin/` binaries | Every binary added after W2-I is inventoried; production-like binaries route through the registry, one-shot migrations document any override |
| Telemetry/progress sinks | Explicitly classified as non-bus or Local; cannot enqueue invalidation jobs |

The inventory must include the current entrypoint, file glob, expected policy path, allowed exclusions, and the lint/test that keeps the channel from drifting.

### Single-writer guarantee

Production writes to `signal_events` are allowed only inside the canonical `emit_signal` implementation. Production creation of signal-originated invalidation jobs is allowed only through the queue enqueue path called by `emit_signal` inside the same transaction.

Allowed exceptions are narrow:

- Schema migrations and test fixtures may seed tables directly.
- Backfill/migration binaries may use an explicit override only when inventoried, idempotent, and paired with a static reason string.
- Queue workers may update job status/lease/attempt fields and may create child jobs only through the queue module with inherited `origin_signal_id`, `chain_id`, and ancestry. They do not choose signal policy.

CI must reject:

- direct SQL `INSERT`/`UPDATE` against `signal_events` outside the allowlist
- direct creation of signal-originated `invalidation_jobs` outside the emit/queue module allowlist
- call-site-selected propagation policy outside `policy_registry.rs`
- new signal emission channels missing from the channel inventory

## Claim-event load envelope

The load gate is a minimum production safety envelope for a local-first app with bursty background jobs. Implementations may exceed it, but cannot lower it without amending this contract.

| Envelope item | Required bound |
|---|---|
| Coalescable claim-event burst | 5,000 raw `ClaimTrustChanged`-class events in 60 seconds across at least 100 subjects; raw events remain auditable and runnable jobs are <= 10% of raw count when keys coalesce |
| Hot-subject burst | A single subject may emit up to 50 claim-level events/minute before adaptive per-subject coalescing/throttling starts |
| Sustained mixed load | 1,000 `ClaimTrustChanged` events/minute plus 100 `AbilityOutputChanged` events/minute for 10 minutes, matching ADR-0115's ship gate |
| Non-coalescable claim transitions | `ClaimAsserted`, `ClaimSuperseded`, `ClaimRetracted`, and `ClaimContradiction` do not collapse below one durable event/job coverage per transition; producers must back off when queue high-water is reached |
| Queue hard bound | 10,000 pending/running durable invalidation jobs by default |
| Queue high-water | 8,000 pending/running jobs triggers aggressive coalescing and producer throttling before hard failure |
| Queue low-water | Normal coalescing resumes once depth falls below 5,000 |
| Queue-full behavior | At the hard bound, non-local propagating emissions fail closed; the signal transaction rolls back and the caller receives a queue-full error |
| Dead-letter budget | Under the sustained load gate, dead-letter rate must remain below 0.1% excluding deliberately injected permanent failures |
| Silent-stale budget | Zero. Every affected output is recomputed or marked stale/dead-lettered/cycle-detected |

Bulk backfills and shadow-mode trust recomputes must use chunking/backoff against the same queue-depth signals. They may not bypass the signal registry or write stale read models directly to avoid the load gate.

## Transactional seam

### Source mutation to signal to job

For claim-originated signals, the commit transaction must include:

1. The source claim mutation and any synchronous claim-version/projection writes that are part of the claim commit.
2. The `signal_events` row for the emitted signal.
3. The `invalidation_jobs` insert or coalescing update that covers each required recompute target.
4. Any idempotency/chain metadata needed for retry and cycle detection.

If any step fails, the whole transaction rolls back. This includes queue bound failures. Claim commits that require guaranteed signal delivery must not commit the claim and leave recompute as a post-commit best-effort action.

### Coalescing inside the transaction

Coalescing is a transaction-local enqueue decision:

- The raw signal row is still written.
- The queue either inserts a new pending job or updates an existing pending/running coalesced job for the registry-declared key.
- The job records enough metadata to prove the raw signal is covered: coalescing key, latest signal id, count, and timestamp range.
- A coalescing update failure is an enqueue failure and rolls back the emit transaction.

### Await completion

`PropagateSync { await_completion: true }` is a rare registry policy for user-perceived correctness paths. It does not run recompute against uncommitted claim state. The source mutation, signal, and job first commit; then the caller waits on the job receipt for the bounded timeout. Timeout leaves the durable job in place and returns a typed timeout to the caller.

### Evaluate and Simulate

Evaluate and Simulate do not write DB signal rows or durable jobs. Fixtures that assert invalidation behavior must explicitly call the in-memory flush helper. This keeps non-Live modes side-effect-free while still letting tests inspect intended signal/job behavior.

## Claim recompute seam

Claim recompute jobs consume queue metadata and canonical storage, not signal payload facts.

Required recompute behavior:

- Load the signal by `origin_signal_id` and resolve affected outputs from registry-declared target metadata.
- Re-read canonical claims, source rows, trust inputs, tombstones, and source lifecycle state.
- Write claims only through the claim service/commit path and write read-model projections through their owned projection services.
- Use idempotency keys so retrying the same job cannot duplicate claims, repair jobs, or read-model writes.
- Preserve `chain_id` and bounded ancestry for fan-out; cycles become terminal `CycleDetected` jobs with stale markers, not dropped work.
- Emit secondary signals only through the registry, with chain ancestry inherited.
- Dead-lettered jobs mark affected outputs stale and expose enough state for the later W1 verification doc to cite file:line evidence.

## Acceptance criteria - verification

### Contract artifact exists and gates W1-B/C/D

This document exists at `.docs/plans/wave-W1/DOS-306-contract.md`. W1-B, W1-C, and W1-D L0 plans cite it directly and either accept this contract or record an explicit amendment before implementation starts.

### Signal durability classes are implemented by registry policy

Verification must show a registry entry for every production `SignalType` with durability class, coalescing behavior, target resolver, execution-mode behavior, and stale/dead-letter policy. Claim state transitions, user corrections/tombstones, source lifecycle signals, and contradictions are classified as guaranteed-delivery in Live mode.

### Guaranteed-delivery transaction test exists

A test injects failure after signal event creation but before invalidation job creation and proves no orphan `signal_events` row remains. A second test injects queue-full failure during a claim commit and proves the claim mutation, signal row, and job row all roll back together.

### Restart and terminal-state tests exist

A pending claim recompute job survives process restart and completes. Retry exhaustion transitions to `DeadLettered` and marks affected outputs stale. Cycle ancestry detection transitions to `CycleDetected` and marks affected outputs stale. No test may pass by silently dropping the affected output.

### Coalesced signals remain auditable

A burst of coalescable claim events writes all raw signal audit rows but creates or updates bounded job rows according to the registry coalescing key. The test asserts job coverage metadata links the coalesced job back to the raw signal range.

### Policy registry exhaustiveness fails normal builds

Adding a new production `SignalType` without updating `policy_for` fails a normal Rust build, not only `cargo test`. Variants carrying data are covered by pattern matches that preserve exhaustiveness.

### Channel inventory is complete and enforced

`.docs/plans/wave-W1/W1-B-channel-inventory.md` lists every signal-emission channel family in this contract, including all current `src-tauri/src/bin/` binaries. CI rejects direct production writes to `signal_events`, direct signal-originated job creation outside the allowlist, and new channel files that are absent from the inventory.

### Single-writer guarantee is enforced

Production code has one signal-event writer and one signal-originated durable-job creation seam. Queue workers may update job state and create child jobs only through the queue module with inherited origin/chain metadata. Call sites cannot choose propagation policy.

### Load gate holds

The W1-D load harness passes the burst and sustained envelopes above:

- 5,000 coalescable claim events in 60 seconds produce durable audit rows and runnable jobs <= 10% of raw count when keys coalesce.
- 1,000 `ClaimTrustChanged`/minute plus 100 `AbilityOutputChanged`/minute for 10 minutes keeps queue depth below 10,000.
- High-water at 8,000 triggers aggressive coalescing/throttling.
- Hard-bound queue-full fails closed with rollback.
- Dead-letter rate is below 0.1% excluding injected permanent failures.
- Silent-stale count is zero.

### Claim recompute consumes canonical state

Verification cites recompute code proving jobs re-read canonical claims/source rows and do not use signal payloads as facts. Retry tests prove idempotency. Suite S verifies signal payloads do not leak PII or cross-tenant claim content.

### Evaluate and Simulate are side-effect-free

Evaluate/Simulate tests prove signal capture stays in memory, no DB `signal_events` or durable queue rows are written, and invalidation assertions require an explicit in-memory flush helper.

## Outstanding

Implementation evidence is intentionally outstanding for W1-E. Per the v1.4.1 wave plan, the post-implementation verification document is separate and belongs at `.docs/plans/wave-W1/DOS-306-verification.md` after W1-B/C/D merge.

## References

- ADR-0115 - Signal granularity, policy registry, and durable invalidation, including R1 transactionality/coalescing fixes and R2 `await_completion`.
- ADR-0123 - Typed claim feedback semantics and repair-job budgets.
- ADR-0113 - Human and agent analysis as first-class claim sources.
- ADR-0114 - Scoring unification.
- ADR-0125 - Temporal scope, sensitivity, and claim type registry.
- DOS-241 Enrichment Refactor Design, latest historical artifact at commit `b2a24dc1` (`.docs/research/enrichment-refactor-design.md` is not present in the current checkout).
- `.docs/plans/v1.4.1-waves.md` Wave 1 stage-1/stage-2 ordering and W1-B/C/D done-when clauses.
- `.docs/plans/v1.4.0-contracts/DOS-302-projection-manifest.md`, `DOS-303-trust-feedback-tombstone.md`, `DOS-304-capability-boundary.md`, `DOS-307-system-owned-verification.md`.
