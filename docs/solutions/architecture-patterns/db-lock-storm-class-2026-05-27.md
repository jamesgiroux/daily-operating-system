---
title: DB lock-storm class — seven layers of write discipline, zero of throughput shaping
problem_type: architecture_pattern
track: knowledge
module: src-tauri/src/db_service.rs (PooledConnection, NUM_READERS), src-tauri/src/db/core.rs (WRITE_TRANSACTION_GATE, with_transaction), src-tauri/src/intel_queue.rs (persist_enrichment_write_results_via_db_service), src-tauri/src/executor.rs (background worker DB-connection anti-pattern), src-tauri/src/meeting_prep_queue.rs (split-lock fresh-connection pattern), src-tauri/src/context_provider/glean.rs (Tokio-worker-blocking fresh-connection pattern)
tags: [db, sqlite, sqlcipher, wal, write-discipline, lock-storm, throughput, atomicity, recurring-class, k-out, w0]
date: 2026-05-27
related_adr: ADR-0067, ADR-0092, ADR-0101, ADR-0104, ADR-0120, ADR-0126, ADR-0133, ADR-0134
related_plans: .docs/plans/db-throughput-architecture.html, .docs/plans/db-write-queue-consolidation.html (superseded)
---

## Context

Over fourteen weeks the DailyOS backend has shipped seven distinct interventions on the database write path. Each one was supposed to be the last. The eighth instance of the same shape — foreground beachball at twenty entities, 8.8 s `get_account_detail` latency on a routine click — surfaced 2026-05-27. The pattern is a recurring class, not a parade of bugs.

This document captures the class so future L0 packets that propose anything touching the write path can grep `db-lock-storm-class` and inherit the diagnosis before re-deriving it from scratch. It is the K-out deliverable named in §6 of the throughput-architecture plan and required by AC9 of the W0 acceptance criteria.

## The class

Eight instances of one shape over fourteen weeks. Each layer answers "who is allowed to write, and in what order." None has ever shaped throughput — the rate at which background work consumes the writer, foreground priority over background, removal of the persistent store from the synchronous read path.

| Date | Layer | Shape | Class |
|------|-------|-------|-------|
| 2026-02-12 | ADR-0067 staged split-lock helpers | `with_db_try_read` / `with_db_read` / `with_db_write` convention | Lock semantics |
| 2026-04-20 | Pool unification (commit `72e7b036`) | 218 `ActionDb::open` sites → pool | Serialization (reverted) |
| 2026-04-20 | Pool revert (commit `8bfdfdd8`) | `parking_lot` mutex held across 240 s Glean `.await` → 31 threads parked, beachball worse than before | Same class, worse failure mode |
| 2026-05-08 | Ability-runtime crate split (ADR-0102) | Compile-time boundary on DB access | Lock semantics |
| 2026-05-10 | `with_db` merge | Helper consolidation | Lock semantics |
| 2026-05-22 | Foreground-DB-contention L0 PR1 | Foreground `get_*` routes read-pure; calendar batch closure; Phase 2 (read-models) deferred behind measurement gate | Lock semantics on foreground side |
| 2026-05-27 | PR #407 `WRITE_TRANSACTION_GATE` | Process-wide `parking_lot::Mutex` on every `with_transaction` | Serialization gate |
| 2026-05-27 | PTY `ai_usage` queue-routing | Single auto-commit bypass routed through writer | Lock semantics — same shape as the 2026-04-20 fresh-open class, fifth instance |

Every intervention above is, structurally, about *who is allowed to write and in what order*. None constrains the *rate* of writes. None prioritizes foreground over background. None removes contention by moving the foreground reader off the embedded database entirely.

The L0 packet on foreground DB contention came closest. It explicitly named "materialized read models / cross-render cache" as the Phase 2 answer, and explicitly deferred it pending measurements that prove Phase 1 insufficient. Today is that measurement. Twenty entities, 387 MB DB, 8.8 s foreground latency on a routine click. Phase 1 was necessary. It is not sufficient.

## The inverted-model anti-pattern

A consequence of seven layers of lock discipline without throughput shaping is a recurring anti-pattern in the codebase: long-running background workers open their *own* DB connection on the explicit reasoning that doing so will "avoid starving foreground IPC commands." The comment travels with the code. Six sites carry it:

- `src-tauri/src/executor.rs:630` — `// Own DB connection to avoid starving foreground IPC commands`
- `src-tauri/src/executor.rs:1051` — same comment, second instance
- `src-tauri/src/meeting_prep_queue.rs:254` — `/// Opens own DB connection (split-lock) to load meeting data`
- `src-tauri/src/meeting_prep_queue.rs:518` — `/// Uses own DB connection (split-lock pattern) to avoid blocking UI.`
- `src-tauri/src/intel_queue.rs:1301` — `/// Phase 1: Open own DB connection to gather all context needed for enrichment.`
- `src-tauri/src/context_provider/glean.rs:1026` — `// own DB connection to avoid blocking Tokio worker threads.`
- `src-tauri/src/capture.rs:287` — `// Open own DB connection to avoid holding state.db Mutex`

The reasoning inverts the SQLite contract:

1. **SQLite WAL permits exactly one file-level writer at a time.** A second mutating connection does not give you a second writer; it gives you a second queue position behind the same file-level lock.
2. **`WRITE_TRANSACTION_GATE` serializes every `BEGIN IMMEDIATE` process-wide.** A worker that opens its own connection still has to acquire the gate before any mutating SQL runs. The gate does not care which connection issued the `BEGIN`.
3. **The pool already exists.** As of `PooledConnection` (`db_service.rs:216`) and the gate (`db/core.rs:29`), "starving" is not a thing connection multiplication can fix. Starving was a symptom of pool absence in early 2026. The fix shipped. The comments did not catch up.
4. **Connection multiplication bypasses instrumentation.** Per ADR-0120 and W0-B's per-call queue-wait + execution-time telemetry, the latency picture only assembles if writes route through the pool. Fresh-open connections write invisibly; the rollups under-report; the next debugging session re-derives the same wrong story.

What the anti-pattern actually does is *trade visibility for the illusion of independence*. The worker writes invisibly, the gate serializes it anyway, and the next reader still has to walk the WAL frame index that the fresh-open writer just extended. None of the original problem is solved. All of the observability is.

ADR-0133 names this anti-pattern explicitly. AC8 of the W0 plan requires the `executor.rs:630/1051` comments specifically be removed; the W1-C bypass closure migrates the worker sites to `state.db_write`; the W1-C CI gate makes the regression impossible.

## The 2026-04-20 unify-pool revert — load-bearing context

The previous attempt to unify on a single pool (commit `72e7b036`) was reverted the same day (commit `8bfdfdd8`) because it held a `parking_lot::Mutex` guard across a 240-second Glean `.await` call. Thirty-one threads parked on the held mutex. macOS beachballed worse than before the unification attempt. The user-visible symptom was indistinguishable from the contention the unification was meant to fix.

The team's reasonable conclusion at the time was "pool unification is dangerous," and every subsequent fix has been incremental gate-tightening rather than topological change. That conclusion is half the lesson. The other half is the one worth memorializing:

**Synchronous locks cannot span async suspension points. Ever.** This is not a "be careful" lesson; it is a permanent forbidden pattern. ADR-0133 §6 codifies it:

- No `parking_lot::Mutex` or `std::sync::Mutex` guard may live across an `.await` in the same function body.
- No `tokio::sync::Mutex` guard either, unless the held interval is sub-millisecond and documented.
- No `Arc<AtomicBool>` "foreground-active" flag may be swapped to `true` before an `.await` whose path can lead to another writer claim of that flag.
- No `Arc<WriterPermit>` may be held across an LLM, network, file-system, or any external `.await`.

The writer queue's mpsc handoff is the boundary: the caller does the awaiting; the writer thread executes the closure synchronously on a `std::thread`. Closures may not themselves invoke `.await`.

Without this rule the revert recurs. With it, pool topology — including the W2 partitioned writer queue if W1's hard gate earns it — becomes a safe shape to ship. The revert is not evidence that the topology cannot be improved; it is evidence that the topology must respect this invariant.

## The proposal — production hygiene as the cheap first step, A* and B earned by measurement

The throughput-architecture plan stages the response to the eighth instance as four named alternatives, ranked by "cheapest that closes the user-defined issues":

**A — Production hygiene, staged.** Four sub-decisions, each gated on measurement that names whether the next is needed.

- W0-A: WAL pragmas (`wal_autocheckpoint = 200`, `mmap_size = 256MB`, `cache_size = -65536`, `journal_size_limit = 64MB`) plus a dedicated checkpoint thread running `PRAGMA wal_checkpoint(PASSIVE)` every 30 s on the writer's encrypted connection.
- W0-B: `NUM_READERS` 2 → 4 (no tier ownership yet; that is W1-D conditional); per-command queue-wait + execution-time + writer-side vs reader-side time + gate-wait at 50 ms granularity + tier-of-origin labels. Durable telemetry (persistent rollups, not 256-sample in-memory windows).
- W0-C: ADR-0133, ADR-0134, this K-out doc, and a Linear ticket for re-measure at 50 / 100 / 200 entities.
- W1 (conditional): per-claim-type and side-effect-class TX decomposition + row-chunked durable cursor (W1-A); presence-aware admission (W1-B); bypass closure + CI gate (W1-C, mechanical, unconditional); `ReaderTier` enum + 197-site migration (W1-D, conditional).

A is overdue production hygiene for SQLite under background-write load. It is **not** architectural completion. Treating it as such is the primary failure mode the wave plan exists to mitigate; the Linear re-measure ticket is the observable claim that "we did A, no need for the rest" cannot become silent inertia.

**A* — Subject-ref-hash partitioned writer queue.** Earned by W1's hard gate if writer `execution_ms` p95 remains dominant after W0+W1 *and* multiple subject domains contend concurrently. A* partitions the writer queue into N threads keyed by `hash(subject_ref) % N`. **It does not enable concurrent WAL writes** — WAL permits one file-level writer; `WRITE_TRANSACTION_GATE` remains process-wide; A* writers interleave at `BEGIN`/`COMMIT` granularity at the gate. What A* does is break application-level mpsc head-of-line blocking so account-A's enrichment does not queue meeting-B's prep rebuild on the same channel. The win is fairness across subject domains, not throughput.

**B — In-memory claim graph.** Earned only if reader-CPU on SQLCipher page decryption is the residual after A and A*. SQLCipher's per-page AES cost (387 MB with `cipher_page_size = 4096` → ~100K pages on a full scan) is real and unfixable by pool sizing. The Linear / Notion / Obsidian pattern of moving the read path off the encrypted store entirely is what closes reader-CPU starvation. B opens its own L0 packet (WX in the wave plan) when earned; it must answer all five Intelligence Loop integration questions (ADR-0126 lineage), not just the signal-invalidation leg.

**C — Separate cache service process.** Deferred until multi-surface need joins (Tauri renderer + MCP service + WordPress surface all needing live claim access). Latency alone does not justify the IPC overhead.

The wave plan makes which alternative gets earned an observable claim, not a feeling. Each gate's measurement names the next wave by diagnosing the residual: multi-subject writer-hold → W2 (A*); reader-CPU starvation → WX (B); both → W2 first, then WX re-evaluates.

## Why this matters

- **Class-pattern recurrence is the signal, not the bug.** Memory `feedback_zoom_out_for_class_pattern_in_l2_loop`: same shape twice = class-wide structural fix, not a third patch. Eight instances over fourteen weeks is well past that threshold. The sweep — production hygiene plus the conditional substrate moves — IS the work.
- **False completion is the primary failure mode.** Each prior layer was treated as architectural completion. Each prior layer left the class open. The W0 measurement gate and the Linear re-measure ticket are the observable claim that this attempt is not making the same error.
- **Honesty about what each layer does and does not do.** The pool serializes; it does not parallelize WAL writes. The gate ensures one `BEGIN IMMEDIATE` at a time; it does not throttle. A* partitions queues; it does not give concurrent file-level writers. B moves reads off SQLite; it does not change writer-side semantics. Each layer's responsibility is locked down (ADR-0133 for the queue, ADR-0134 for the pool, ADR-0104 amendment for atomicity if W1-A ships, separate ADR if B ships).
- **The inverted-model anti-pattern is durable.** Without ADR-0133 and W1-C's CI gate, the next worker added will copy-paste "// Own DB connection to avoid starving foreground IPC commands" and the codebase will accrete a ninth instance. The ADR + the lint together close it.

## When to apply

- **Authoring or reviewing any L0 packet that touches the DB write path.** Grep this doc; cite the recurring-class table; verify the proposal addresses *throughput shaping* and not just another layer of write discipline. If the proposal is "add another lock / gate / helper / mutex," ask which instance of the class it closes and which it leaves open.
- **Reviewing any new background worker that proposes its own DB connection.** Reject. Route through `state.db_write`. Cite ADR-0133 §1 and §2.
- **Reviewing any change that introduces a `parking_lot::Mutex` near async code.** Verify the 2026-04-20 lesson is honored. The `await_holding_lock` clippy lint catches the obvious cases; the L2 reviewer (`ce-reliability-reviewer`) catches the structural cases.
- **Reviewing any proposal that frames "more readers" or "more writers" as the fix.** Sizing is structural (ADR-0134 N + 1 rule). Bumping numbers without naming a new tier or a new failure mode is the eighth instance shape.
- **Reviewing any proposal to move the foreground read path off SQLite.** That is B's territory. It opens its own L0 packet when reader-CPU residual earns it. It must answer all five Intelligence Loop integration questions per ADR-0126. It is not a side-trip from a writer-side fix.

## Examples

**The 2026-04-20 pool unification (commits `72e7b036` / `8bfdfdd8`)** — the second instance of the class. The pool was the right move; the held mutex across `.await` was the failure mode. The revert taught the team "synchronous lock cannot span `.await`," not "pools are dangerous." ADR-0133 §6 codifies the former; the throughput plan acts on the latter being false.

**PR #407 (2026-05-27) — `WRITE_TRANSACTION_GATE`** — the seventh instance, and the one that resolved the *lock-class storm* in the strict sense (zero contention logs). The current 8.8 s foreground latency is not the lock-class storm; it is WAL bloat starving the reader pool while it walks the WAL frame index, amplified by an undersized pool and a transaction shape that combines unrelated claim domains. Same recurring class; different sub-symptom.

**`persist_enrichment_write_results_via_db_service` (`intel_queue.rs:2711`)** — the W1-A target. ~150 risks + 162 wins + 9 stakeholders + products + assessment + signal emissions in **one transaction**. Most rows tie to the entity's own `subject_ref`; stakeholder insights can switch to person subject_refs. The decomposition unit is *claim-type and side-effect class*, not strict subject-domain. Splitting reduces critical-section length and is independent of row-batching — both reduce writer-hold but address different structural problems. The new ADR amending ADR-0104's atomicity contract names the relaxation.

**The eight bypass sites carrying "// Own DB connection to avoid starving..."** — the inverted-model anti-pattern. ADR-0133 §"Anti-patterns rejected" rejects the shape; AC8 of the W0 plan requires the specific `executor.rs:630/1051` comments be removed; W1-C migrates the sites; the CI gate makes the regression impossible.

## Forward link

This proposal is `.docs/plans/db-throughput-architecture.html`. W0 lands as the `feat/db-throughput-w0` branch:

- W0-A — WAL pragmas + checkpoint thread (`src-tauri/src/db_service.rs` only).
- W0-B — reader pool 2 → 4 + per-command instrumentation (`src-tauri/src/db_service.rs`, `src-tauri/src/latency.rs`, ~12 call sites).
- W0-C — ADR-0133 (writer queue responsibility), ADR-0134 (reader pool sizing), this K-out doc, plus the Linear re-measure ticket at 50 / 100 / 200 entities. AC7 of W0 makes the Linear ticket a hard gate: W0 is not done until the ticket exists and is assigned.

W1 starts only if W0's measurement gate (p95 `get_account_detail` < 200 ms under 10–15 min scripted peak load, no main-thread interaction stall > 100 ms, zero gate-waits > 250 ms, WAL < 5 MB, growth-slope predicts no breach at 100 entities) misses. W2 and WX are conditional on W1's hard gate diagnosing which residual bottleneck dominates.

The next L0 packet that touches this surface starts by grepping `db-lock-storm-class` and reading this doc — that is the K-Channel's job. Reinventing what is already documented gets BLOCKED at L0.

## Related

- ADR-0133 — writer queue responsibility (the queue's invariants and the rejected anti-patterns).
- ADR-0134 — reader pool sizing (the N + 1 rule and W0-B's 2 → 4 bump).
- ADR-0067 — staged split-lock helpers; this work is the "earn it" outcome it named.
- ADR-0092 — SQLCipher PRAGMA-key-first ordering, unchanged.
- ADR-0101 — service-boundary-enforcement; W1-C is the runway to Phase 3.
- ADR-0104 — execution-mode-and-mode-aware-services; owns atomicity contract; W1-A amendment if shipped.
- ADR-0120 — observability contract; W0-B telemetry conforms.
- ADR-0126 — memory substrate invariants; claim correctness preserved across W0+W1+W2; B (if earned) must satisfy all five Intelligence Loop questions.
- Memory: `feedback_zoom_out_for_class_pattern_in_l2_loop` — same shape twice = class-wide sweep.
- Memory: `feedback_systemic_look_for_recurring_issue_classes` — recurrence is the signal.
- Memory: `feedback_dont_swing_past_center_when_correcting` — trim the inverted-model comments and the held-mutex pattern; preserve the substrate.
- `.docs/plans/db-throughput-architecture.html` — the wave plan this doc summarizes.
- `.docs/plans/db-write-queue-consolidation.html` (v2, superseded) — the prior proposal; its bypass-closure work becomes W1-C with the bypass list expanded from 12 to ~190 sites.
- `.docs/plans/foreground-db-contention/L0-packet.md` (V0.3, 2026-05-22) — Phase 1 shipped; Phase 2 (materialized read models) deferred behind the measurement gate this work executes.
- `tasks/lessons.md` (2026-05-25) — "Do not edit Rust backend files while `pnpm tauri dev` is writing to the production SQLCipher database." Directly relevant to W0-A and W1-A landing risk; flagged in throughput-plan §Risks.
