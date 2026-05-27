# ADR-0134 — Reader Pool Sizing

**Status:** Proposed
**Date:** 2026-05-27
**Supersedes:** N/A
**Superseded by:** N/A
**Related:** ADR-0067 (resume-latency-and-db-concurrency-guardrails), ADR-0092 (data-security-at-rest-and-operational-hardening), ADR-0101 (service-boundary-enforcement), ADR-0120 (observability-contract), ADR-0133 (writer-queue-responsibility)

## Context

`NUM_READERS = 2` has lived in `src-tauri/src/db_service.rs:40` as a bare constant since the reader-pool substrate landed. There is no ADR justifying the value. Code review of any change that hits the readers (the 197 `state.db_read` call sites surveyed by the W0-C feasibility audit) has nothing to point at; the constant has been propagated by inertia.

Today, at twenty entities and a 387 MB encrypted database, the constant is wrong. Four concurrent foreground commands fire on a single navigation: `get_account_detail` + `get_linear_status` + `get_audit_log` + the context-mode command. With two reader slots, two execute and two queue on the readers' mpsc channels. The synchronized "all four release within a 32 ms window" signature in the latency rollups (W0-B will surface this as durable telemetry) is the smoking gun.

Two compounding facts make the symptom worse than naive queueing math predicts:

1. **SQLCipher per-page AES decryption cost.** Every page touched by every read decrypts via AES. At 387 MB with `cipher_page_size = 4096`, a full-page-walk hits ~100K pages through AES; even partial scans accumulate CPU. The reader thread that is "busy" is not blocked on I/O — it is busy on encryption work. Adding a third concurrent reader to compete for the same two slots does not just queue; it queues *behind* CPU-bound work.

2. **No tier separation today.** The two reader slots are used round-robin across foreground UI, foreground sync commands, background workers, and maintenance. A long-running background read can occupy one slot while a foreground UI read queues behind it on the other. The observable symptom — foreground latency that correlates with background-worker activity — is not a writer-side contention story; it is head-of-line blocking on undersized readers without ownership.

Resizing alone does not introduce ownership. W0-B does the resize (2 → 4) without ownership; ownership comes in W1-D and only if the W0-B tier-of-origin telemetry shows wrong-tier routing as a residual bottleneck. This ADR separates the two decisions: the *sizing rule* now, the *ownership policy* in a future amendment if W1-D earns its way in.

## Decision

The reader pool is governed by the following invariants. Each is enforced in code (CI lint or test) where possible.

### 1. Reader count ≥ N + 1, where N is the number of read-path latency tiers

DailyOS today has four read-path latency tiers, each with a distinct service-level expectation and a distinct interruptibility profile:

- **ForegroundUi.** Tauri IPC commands invoked by user interaction. Target p95 < 200 ms. Interruptible by definition (the user can navigate away); no batching.
- **ForegroundSync.** Tauri IPC commands invoked by polling, refresh actions, or scheduled foreground UI refreshes. Target p95 < 500 ms. Tolerant of small bursts.
- **Background.** Intel-queue workers, hygiene loop, embeddings, enrichment processor, signal propagation consumers. Target throughput, not latency. Tolerant of multi-second waits; not tolerant of starvation.
- **Maintenance.** Checkpoint thread, schema migrations, key rotation, claim invalidation jobs, evidence backfill. Runs on its own cadence; tolerant of any latency; must not block foreground.

Four tiers; the rule sets the floor at five. The current `NUM_READERS = 2` does not satisfy the floor under any tier mapping. W0-B raises the constant to **4** (one short of the floor; see invariant #3).

### 2. The N + 1 rule is a floor, not a target

The +1 buffer exists so that a temporarily-stalled tier (a maintenance read that hits a contended page; a background scan that walks a wide index) cannot starve every other tier. It is not a performance margin; it is a structural guarantee that "one tier blocked" does not collapse to "all tiers blocked."

For tiers above 4, the +1 may need to grow — e.g., if a tier is split (foreground UI subdivided into navigation vs incremental refresh), or if a new surface (MCP service, WordPress backend) joins the read path with distinct latency expectations. Each split is its own amendment to this ADR.

For tiers below 4, the +1 may shrink — but only if a tier is *merged* via an ADR that names the merge. Removing a tier silently produces drift between the ADR and the code.

### 3. W0-B sizes the pool to 4, not 5, deliberately

The floor is 5 (N + 1 = 4 + 1). W0-B raises `NUM_READERS` to **4**, one short, because:

- **Tier ownership does not exist yet.** Without `ReaderTier` enum, `DbService::reader_for(tier)`, and the 197-site migration, the +1 buffer cannot be reserved for "the next-blocked tier." A fifth slot today would just be another round-robin slot — additional capacity without the structural guarantee the +1 is meant to provide.
- **W1-D earns its way in by measurement.** W0-B's per-tier-of-origin telemetry distinguishes "wrong-tier routing" (foreground command consistently using a slot also serving background, even when another slot is idle) from "pool saturation" (every slot busy). Without the telemetry, the saturation signal alone is ambiguous between "need more readers" (which W0-B's bump to 4 addresses) and "need tier separation" (which W1-D would address). Bundling W1-D into W0 is the path the W3-D scope-creep memory warns against.
- **Four reads concurrently is the observed worst case at 20 entities.** The four-command-per-navigation signature names 4 as the immediate need. Five becomes correct once tier ownership lands.

W1-D, if W0-B's telemetry earns it, introduces the `ReaderTier` enum (`ForegroundUi`, `ForegroundSync`, `Background`, `Maintenance`), `DbService::reader_for(tier)`, and resizes to 5 with strict ownership. This ADR pre-commits to that shape so the W1-D L0 packet does not have to re-derive it.

### 4. Reader pool sizing is independent of SQLCipher decryption cost

Pool sizing prevents head-of-line blocking from queuing on too few connections. It does not eliminate SQLCipher's per-page AES decryption cost. A 387 MB database with `cipher_page_size = 4096` and a full-table-scan query still hits ~100K pages through AES; that work is real CPU regardless of how many reader slots exist.

If the reader pool is correctly sized and reader-CPU still dominates as residual latency, the answer is not "more readers" — it is moving the foreground read path off SQLite entirely (Alternative B in the throughput plan; opens its own L0 packet as WX if earned). This invariant exists to prevent the eighth instance of the recurring lock-storm class from being "we added more readers and called it done."

### 5. The pool sizing constant is named, not magic

`NUM_READERS` stays a single workspace constant. It is not parameterized at runtime; it is not read from config; it is not adjusted per platform. Changes require an amendment to this ADR.

The reasoning: the constant participates in the structural N + 1 invariant. A runtime-tunable value cannot carry a structural guarantee — operators changing it from 4 to 3 would silently violate the invariant on a deployment-by-deployment basis. ADR-level amendment is the right cadence for changes that affect substrate guarantees.

### 6. Reader-pool telemetry conforms to ADR-0120

Per W0-B and ADR-0120's observability contract, the reader pool emits:

- Per-tier queue depth (once tier ownership lands; until then, a single aggregate queue-depth metric).
- Per-tier-of-origin label on each reader call **once W1-D earns its way in**. W0-B ships the API (`PooledConnection::with_tier`, `DbService::reader_for_tier`) but does not migrate the 197 `state.db_read` call sites — that migration IS W1-D. Until W1-D, the tier-of-origin rollup namespace exists but is sparsely populated, by design. The plain writer/reader split (also added in W0-B) is sufficient for the W0 close gate.
- Per-call queue-wait + execution-time, contributing to `get_latency_rollups`.
- WAL-frame walk cost is *not* attributed to the reader pool; it is a SQLCipher-layer cost and surfaces in execution-time, not queue-wait.

Drift between reader telemetry and writer telemetry (ADR-0133 §7) produces the cross-cutting observability gap ADR-0120 was written to close. Same contract; same NDJSON schema; same invocation-id correlation.

## Anti-patterns explicitly rejected

- **`NUM_READERS` as a runtime config parameter.** Defeats the structural guarantee of invariant #1.
- **A reader pool sized "to match CPU cores" or "to match average concurrency."** Conflates throughput tuning with the structural N + 1 floor. A four-core machine with two tiers needs three readers, not four. An eight-core machine with five tiers needs six, not eight.
- **A "fast path" reader that bypasses the pool for "lightweight" queries.** Same shape as the writer-side fast-path rejection in ADR-0133 §"Anti-patterns rejected." Lightweight queries still need pool telemetry; bypassing the pool defeats W1-D's earn-signal collection.
- **Round-robin assignment without tier-of-origin labels.** Defensible as W0-B's interim state (resize without ownership). Defensible long-term only if W1-D's measurement says tier confusion is not the residual bottleneck. Defaulting to permanent round-robin would silently swallow the very signal that determines whether W1-D is earned.
- **Resizing the pool reactively in response to a queue-depth alarm.** Pool sizing is structural; runtime resizing is admission control's job (W1-B presence-aware admission, or the W1-D tier policy).

## Open questions, resolved at W1-D

The following questions are explicitly *not* resolved in this ADR. They are resolved in a W1-D amendment if W0-B's measurement names W1-D as earned:

- Whether tier confusion (a foreground command using a slot also serving background) is a measurable residual bottleneck at 20 entities. W0-B's per-tier-of-origin telemetry names this.
- Whether `DbService::reader_for(tier)` is the right API shape (vs. a tier parameter on every `state.db_read` call). The 197-site migration in W1-D forces the call-site ergonomics question.
- Whether the default-to-`ForegroundUi` policy for Tauri commands is correct (per-site judgment vs. mechanical default). The L0 packet for W1-D owns this.
- Whether the +1 slot is reserved (one connection always idle) or floating (any tier can use it when its own is busy, but its own tier reclaims it first). This is the substantive ownership decision W1-D makes.

This ADR commits to the *sizing* (4 now, 5 with ownership). It does *not* commit to the ownership *policy*.

## Consequences

**What this makes easier:**

- Future scaling decisions have a written rule (N + 1) to apply, rather than picking a number by feel.
- W0-B's bump to 4 is justified by a concrete tier inventory, not by "the four-foreground-commands signature implies four readers."
- The W1-D L0 packet inherits the shape (5 slots with tier ownership) without re-deriving it.
- New surfaces joining the read path (MCP service, WordPress) trigger a written amendment, not a quiet constant bump.

**What this makes harder:**

- Bumping `NUM_READERS` past 4 in response to a future symptom requires either an amendment naming a new tier or a W1-D successor decision. Inertia gets harder; principled growth gets easier.
- The four-tier mapping is now load-bearing. If someone proposes merging `ForegroundSync` and `ForegroundUi` (or splitting `Background` into "intel queue" and "hygiene loop"), the amendment cost is real. This is the right cost; tier proliferation without an ADR is exactly the drift this ADR exists to prevent.

**Who owns this ADR:**

- Reader-pool substrate and `NUM_READERS` constant: `db_service.rs` owners.
- Tier ownership policy (deferred): W1-D lane owner; will produce a W1-D amendment to this ADR.
- Telemetry conformance: ADR-0120 owners; W0-B is the implementation slice.

## Enforcement summary

| Invariant | Mechanism | Location |
|-----------|-----------|----------|
| N + 1 floor | This ADR + reviewer check on any `NUM_READERS` change | `db_service.rs:40` |
| Sizing is structural, not runtime-tunable | Code review on any attempt to parameterize `NUM_READERS` | this ADR §5 |
| Telemetry conforms to ADR-0120 | ADR-0120 observability contract | W0-B implementation |
| Tier ownership deferred to W1-D | Per-tier-of-origin telemetry in W0-B as earn signal | `latency.rs` extension |
| SQLCipher cost is not a sizing problem | This ADR §4; reader-CPU residual routes to WX (B) | W1 hard-gate diagnosis |

## References

- `src-tauri/src/db_service.rs:40` — `NUM_READERS` constant declaration.
- `src-tauri/src/db_service.rs:216-301` — `PooledConnection` substrate (shared with writer pool per ADR-0133).
- `src-tauri/src/db_service.rs:430` — `DbService::readers` field.
- `src-tauri/src/db_service.rs:719` — `DbService::reader()` round-robin assignment.
- ADR-0067 — staged split-lock helpers and the latency-rollups substrate W0-B extends.
- ADR-0092 — SQLCipher; per-page AES decryption cost cited in §4.
- ADR-0101 — service-boundary-enforcement; reader-side bypass closure tracked alongside writer-side (ADR-0133 §8).
- ADR-0120 — observability contract; reader pool emits per its NDJSON + invocation-id schema.
- ADR-0133 — writer queue responsibility; sibling ADR; same `PooledConnection` substrate.
- `.docs/plans/db-throughput-architecture.html` — wave plan; W0-B is the implementation slice for this ADR's sizing; W1-D is the conditional successor.

## Amendments

None yet.
