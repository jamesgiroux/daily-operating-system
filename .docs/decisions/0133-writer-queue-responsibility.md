# ADR-0133 — Writer Queue Responsibility

**Status:** Proposed
**Date:** 2026-05-27
**Supersedes:** N/A
**Superseded by:** N/A
**Related:** ADR-0067 (resume-latency-and-db-concurrency-guardrails), ADR-0092 (data-security-at-rest-and-operational-hardening), ADR-0101 (service-boundary-enforcement), ADR-0104 (execution-mode-and-mode-aware-services), ADR-0120 (observability-contract), ADR-0126 (memory-substrate-invariants)

## Context

Over fourteen weeks the DailyOS backend has shipped seven distinct write-discipline layers — split-lock helpers (ADR-0067), a pool unification attempt (`72e7b036`, 2026-04-20) that was reverted the same day (`8bfdfdd8`) because a `parking_lot` mutex was held across a 240-second Glean `.await`, an abilities-runtime crate split (ADR-0102), a `with_db` helper merge, a foreground-DB-contention L0 deliverable, PR #407's process-wide `WRITE_TRANSACTION_GATE`, and a PTY `ai_usage` queue-routing fix. Each one constrains *who is allowed to write and in what order*. None shapes throughput.

A consequence of that history is a recurring anti-pattern in the codebase: long-running background workers open *their own* DB connection on the explicit reasoning that doing so will "avoid starving foreground IPC commands." `src-tauri/src/executor.rs:630` and `:1051` carry that comment verbatim. `src-tauri/src/meeting_prep_queue.rs:254/518`, `src-tauri/src/intel_queue.rs:1301`, `src-tauri/src/context_provider/glean.rs:1026`, and `src-tauri/src/capture.rs:287` use the same shape. The reasoning inverts the actual SQLite contract: WAL allows exactly one file-level writer in a process, the `WRITE_TRANSACTION_GATE` already serializes every `BEGIN IMMEDIATE` process-wide, and opening another connection does not buy concurrency — it adds another serialized writer that the gate must coordinate, and it bypasses the pool's instrumentation, queue-depth telemetry, and forbidden-pattern lints. "Starving" was a symptom of pool absence in early 2026; once `PooledConnection` (`db_service.rs:216`) and the gate (`db/core.rs:29`) existed, the coping mechanism was no longer doing what its comment claimed.

The same history has produced a second class of confusion: the pool, the gate, and `with_transaction` are sometimes described as if they *also* govern transaction shape — what subjects a single transaction may span, how many claim mutations may live inside one `BEGIN`/`COMMIT`, when a commit may yield. They do not. Transaction shape (atomicity) is ADR-0104's domain; the writer queue executes whatever closure it is handed. Without this separation written down, every new ticket that touches the write path has to re-derive which layer owns what.

This ADR locks down the writer queue's responsibility and its non-responsibilities. It closes the inverted-model anti-pattern, names the forbidden patterns the 2026-04-20 revert taught us, and clarifies the boundary against ADR-0104's atomicity contract.

## Decision

The writer queue is governed by the following invariants. Each is enforced in code (CI lint or test) where possible. Each future feature either honors these invariants or proposes an amendment to this ADR.

### 1. Single writer connection per process

Exactly one mutating SQLite connection exists in-process. It is owned by the writer slot of `DbService` (`src-tauri/src/db_service.rs:429`) and accessed through `PooledConnection::call_*` and `state.db_write`. No code path is permitted to open a second mutating handle to the encrypted database.

The historic justification "open a fresh `ActionDb` per worker to avoid starving the foreground" is rejected. SQLite WAL permits exactly one file-level writer at a time; adding a second connection adds a queue slot, not parallelism. `WRITE_TRANSACTION_GATE` (`src-tauri/src/db/core.rs:29`) serializes every `BEGIN IMMEDIATE` regardless of which connection issues it. The pool, not the connection count, owns prioritization.

Enforcement: W1-C lands a CI gate (`startup_background_db_access_lint_test.rs` extension) that fails on `conn_ref().execute*` outside an explicit `with_transaction` or named-allowlist file. The current ~190 bypass sites are migrated to `state.db_write` in the same wave; the remaining tail becomes forbidden-by-default with allowlist entries requiring review.

### 2. All mutations route through `state.db_write`

Every foreground command, background worker, signal-emission path, maintenance job, and ability mutation that writes to the DB enqueues its closure on `state.db_write` (which dispatches to the writer's mpsc loop and reaches the gate-serialized `with_transaction`). Direct calls to `conn_ref().execute*`, `conn_ref().prepare`, or any other connection method outside an explicit transaction wrapper are forbidden.

Two narrow exceptions exist and are named: (a) the SQLCipher pragma sequence in `apply_pragmas` (which by ADR-0092's PRAGMA-key-first ordering cannot route through `with_transaction`); (b) the dedicated checkpoint thread W0-A introduces, which runs `PRAGMA wal_checkpoint(PASSIVE)` on the writer's own encrypted connection. Both are documented allowlist entries in the W1-C CI gate.

Enforcement: forbidden-pattern lint (W1-C); existing service-boundary test (`startup_background_db_access_lint_test.rs`) extended to cover the bypass shape.

### 3. The gate serializes; it does not throttle

`WRITE_TRANSACTION_GATE` is a process-wide `parking_lot::Mutex<()>` (`src-tauri/src/db/core.rs:29`). Its sole job is to ensure no two `BEGIN IMMEDIATE` calls overlap in the same process — covering the rare non-pool fresh-open caller (the existing SQLCipher pragma path; nothing else) so SQLite WAL never sees two concurrent in-process writers.

The gate does not implement priority, fairness, backpressure, or rate limiting. It records gate-wait latency (W0-B instruments waits above 50ms; existing log fires only above 5s) so observability can distinguish "contention because the writer is busy" from "contention because work is queued unfairly," but the gate itself is policy-free.

Enforcement: code review on any change to `db/core.rs:200-260`; no new gate flags or priority parameters without an amendment to this ADR.

### 4. The queue does not own transaction shape

The writer queue executes closures. It does not decide what subject domains a transaction may span, how many claim mutations may live inside one `BEGIN`/`COMMIT`, or when a commit may yield. Atomicity is ADR-0104's domain. The queue is correct as long as it serializes; whether the *boundary* of a transaction is right is a separate, contractual question owned by the caller.

This separation matters because the W1-A decomposition of `persist_enrichment_write_results_via_db_service` (`intel_queue.rs:2711`) — splitting one large transaction by claim-type and side-effect class — is a *caller-side* change that does not require any modification to the queue. W1-A's row-chunked durable cursor (caller-driven re-enqueue from a persisted offset) is also a caller-side primitive; the queue sees it as N small transactions instead of one large one. Conversely, no queue-side change can fix a transaction whose shape combines unrelated subject domains.

Enforcement: any PR that proposes adding policy ("this kind of mutation can't go in the same transaction as that kind") to the queue is BLOCKED at L2 and routed to an ADR-0104 amendment.

### 5. WAL writes are file-level serialized; the queue cannot make them concurrent

SQLite's WAL journal allows exactly one writer at the file level. A partitioned writer queue (W2 / Alternative A* in the throughput plan) does *not* enable concurrent WAL writes. What it does is prevent application-level mpsc head-of-line blocking — so that account-A's long enrichment does not queue meeting-B's prep rebuild on the same channel — and let those writers interleave at `BEGIN`/`COMMIT` granularity at the SQLite gate.

This invariant exists because it is repeatedly forgotten. "Partition the writer queue" sounds like "now writes happen in parallel"; they do not. The win at the SQLite layer is fairness, not throughput. The throughput win, if any, lives at the application layer (mpsc, scheduler, presence-aware admission).

Enforcement: documentation in the W2 ADR (if W1's hard gate earns it) explicitly cites this invariant.

### 6. Forbidden pattern: holding any DB guard, writer permit, or foreground flag across `.await`

The 2026-04-20 unify-pool revert (`8bfdfdd8`) was caused by holding a `parking_lot::Mutex` guard across a 240-second Glean `.await` call. Thirty-one threads parked. macOS beachballed worse than before the unification attempt. The conclusion is not "pool unification is dangerous" — it is "synchronous locks cannot span async suspension points, ever."

Concretely:

- No `parking_lot::Mutex` or `std::sync::Mutex` guard may live across an `.await` in the same function body.
- No `tokio::sync::Mutex` guard either, unless the held interval is bounded by a measurable budget (sub-millisecond) and documented in a comment.
- No `Arc<AtomicBool>` "foreground-active" flag may be `swap(true, ...)` before an `.await` whose path can lead to another writer claim of that flag.
- No `Arc<WriterPermit>` may be held across an LLM, network, file-system, or any external `.await`.

The writer queue's mpsc handoff is the boundary: the caller does the awaiting; the writer thread executes the closure synchronously. Closures may not themselves invoke `.await`; the writer thread is `std::thread`-based, not async.

Enforcement: clippy lint `await_holding_lock` is enabled on the workspace; any new `parking_lot::Mutex` introduction in async context requires a comment justifying the bounded hold; L2 reviewer (`ce-reliability-reviewer`) is named on every PR that touches `db_service.rs`, `db/core.rs`, or any writer-path file.

### 7. The queue is the single place where writer telemetry lives

Per ADR-0120's observability contract, the writer path emits:

- Per-call queue-wait duration (time between mpsc enqueue and writer thread dequeue).
- Per-call execution time (time inside `with_transaction`, excluding gate-wait).
- Per-call gate-wait duration (sampled at 50ms granularity per W0-B; long-tail warning fires at 250ms).
- A stable, PII-free label per call site (used for distributions in `get_latency_rollups`).
- Per-call writer-thread vs reader-side time separation, so the W1 hard gate can route between W2 (writer-dominant residual) and WX (reader-CPU residual).

The reader pool emits the analogous metrics for read paths (per-tier queue depth, per-tier-of-origin label so W1-D can distinguish tier confusion from pool saturation; see ADR-0134). No write-path consumer adds its own latency counter outside this contract; doing so produces drift across surfaces and re-creates the "every ADR made its own choice" problem ADR-0120 was written to close.

Enforcement: ADR-0120's observability contract; W0-B is the implementation slice.

### 8. The bypass-closure work is not optional

ADR-0101 §"Rule 1" requires that all domain mutations go through `services/`. The compile-time door-close — making `ActionDb::open` `pub(in crate::db)` (ADR-0101 Phase 3) — is the structural enforcement and remains the long-term goal.

The runway between today and Phase 3 is the W1-C bypass closure plus a forbidden-pattern CI gate. The two are not interchangeable:

- The CI gate fails the build on new `conn_ref().execute*` calls outside an allowlist; it makes the rule durable.
- The bypass closure migrates the existing sites (~190 found by the W0-C feasibility re-audit; the architecture map's 16+ was the high-frequency hot subset). The hot subset migrates in W1-C; the lower-frequency tail is gated by the CI lint and migrates as touched.

Without the CI gate, the migration regresses; without the migration, the CI gate would need an unbounded allowlist on day one.

Enforcement: W1-C CI gate; ADR-0101 Phase 3 remains the eventual compile-time replacement.

## Anti-patterns explicitly rejected

Each of these has been proposed or attempted; listing them here so future tickets do not re-propose:

- **"Open a fresh DB connection in a background worker to avoid starving foreground commands."** Multiplies writers without multiplying writer slots; bypasses pool instrumentation; collides with the gate. The five existing sites (`executor.rs:630/1051`, `meeting_prep_queue.rs:254/518`, `intel_queue.rs:1301`, `glean.rs:1026`, `capture.rs:287`) get migrated to `state.db_write` and their comments removed. AC8 of the W0 plan requires the `executor.rs:630/1051` comments specifically be deleted.
- **Holding a `parking_lot::Mutex` across `.await`.** The 2026-04-20 revert's lesson. Permanent forbidden pattern.
- **Adding a priority field to `WRITE_TRANSACTION_GATE`.** The gate is policy-free by invariant #3. Priority belongs in the pool's mpsc routing (A*) or in admission control (W1-B presence-aware), not in the gate.
- **A "fast path" that bypasses `with_transaction` for single-statement writes.** Single-statement writes still need the gate (so the WAL frame ordering stays predictable), still need the instrumentation, and still need the bypass-closure lint to be enforceable. There is no fast path.
- **Per-worker writer connections rationalized as "isolation."** Workers do not need DB isolation — they need execution-mode awareness (ADR-0104). Isolation at the connection layer is conflation.
- **Holding `Arc<AtomicBool>` "writer-active" flags across `.await`.** Same failure mode as mutex-across-await; the flag's hold interval becomes unbounded.
- **Letting the queue decide transaction shape.** Belongs to ADR-0104. The queue executes; the caller composes.

## Consequences

**What this makes easier:**

- Future tickets know which layer owns each concern (queue: serialization + telemetry; caller: shape + atomicity; ADR-0120: observability cross-cut).
- L2 reviewers have a concrete list to check against: "does this PR introduce a writer connection? hold a lock across `.await`? add policy to the gate?"
- The CI gate (W1-C) and the bypass-closure migration are framed as the same work — not "lint plus migration" but "the lint requires the migration to be tractable, and the migration requires the lint to stay closed."
- The "own a DB connection to avoid starving" anti-pattern stops returning. New worker code that proposes a fresh connection now has a written rule to bounce against.

**What this makes harder:**

- Some legacy bypass sites have non-obvious migrations (e.g., `capture.rs:287`'s `state.db` mutex avoidance). W1-C is paced accordingly: hot subset first, tail gated by the CI lint and migrated as touched.
- ADR-0104's atomicity contract amendment for W1-A's row-chunked cursor is now explicitly separate from this ADR. Two ADRs touch the write path; the boundary is named.

**Who owns this ADR:**

- Queue substrate and pool sizing: `db_service.rs` owners.
- Gate behavior and serialization: `db/core.rs` owners.
- Bypass closure + CI gate: W1-C lane owner.
- Atomicity contract amendments: ADR-0104 owners (separate from this ADR).

Amendments to this ADR happen when a feature ticket proposes a principled exception (e.g., the future SQLCipher key-rotation path, which currently lives outside `with_transaction` and may need explicit accommodation). Amendments are numbered and dated, not silently merged.

## Enforcement summary

| Invariant | Mechanism | Location |
|-----------|-----------|----------|
| Single writer connection | CI lint on `ActionDb::open` outside `db/` + `services/` (existing) | `startup_background_db_access_lint_test.rs` |
| Mutations route through `state.db_write` | CI lint on `conn_ref().execute*` outside `with_transaction` (W1-C, new) | `startup_background_db_access_lint_test.rs` extension |
| No locks across `.await` | clippy `await_holding_lock` + L2 reviewer | workspace `clippy.toml`, `ce-reliability-reviewer` |
| Queue executes; caller composes | L2 review on any policy added to gate or queue | `ce-architecture-strategist` |
| WAL is file-level serialized | Documentation invariant; cited in W2 ADR if earned | this ADR §5 |
| Writer telemetry is centralized | ADR-0120 observability contract | W0-B implementation |
| Bypass closure is mandatory | W1-C migration + CI gate together | W1-C lane |

## References

- `src-tauri/src/db_service.rs:40` — `NUM_READERS` constant (see ADR-0134).
- `src-tauri/src/db_service.rs:216-301` — `PooledConnection` substrate.
- `src-tauri/src/db_service.rs:429` — `DbService` writer/readers fields.
- `src-tauri/src/db/core.rs:29` — `WRITE_TRANSACTION_GATE` declaration.
- `src-tauri/src/db/core.rs:200-260` — `with_transaction` gate-acquisition path.
- `src-tauri/src/executor.rs:630` — "own DB connection to avoid starving" anti-pattern site (instance 1).
- `src-tauri/src/executor.rs:1051` — "own DB connection to avoid starving" anti-pattern site (instance 2).
- `src-tauri/src/meeting_prep_queue.rs:254`, `:518` — same anti-pattern, split-lock-pattern framing.
- `src-tauri/src/intel_queue.rs:1301` — same anti-pattern, "open own DB connection to gather context."
- `src-tauri/src/context_provider/glean.rs:1026` — same anti-pattern under Tokio-worker-blocking framing.
- `src-tauri/src/capture.rs:287` — same anti-pattern under `state.db` mutex framing.
- Commit `72e7b036` (2026-04-20) — pool unification attempt.
- Commit `8bfdfdd8` (2026-04-20) — pool unification revert, the load-bearing lesson on mutex-across-await.
- PR #407 — process-wide `WRITE_TRANSACTION_GATE` introduction.
- ADR-0067 — staged split-lock helpers; this ADR is the "earn it" outcome ADR-0067 named.
- ADR-0092 — SQLCipher PRAGMA-key-first ordering, unchanged by this ADR.
- ADR-0101 — service-boundary-enforcement; this ADR is the runway to its Phase 3.
- ADR-0104 — execution-mode-and-mode-aware-services; owns atomicity contract; sibling to this ADR.
- ADR-0120 — observability contract; W0-B telemetry conforms.
- ADR-0126 — memory substrate invariants; claim correctness preserved.
- `.docs/plans/db-throughput-architecture.html` — wave plan motivating this ADR.
- `docs/solutions/architecture-patterns/db-lock-storm-class-2026-05-27.md` — K-out doc capturing the recurring class.

## Amendments

None yet.
