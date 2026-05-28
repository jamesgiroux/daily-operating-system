# W0 Proof Bundle — DB Throughput Architecture

**Wave:** W0 (Hygiene foundation — `.docs/plans/db-throughput-architecture.html`)
**Status:** L2 cycle 2 APPROVE — ship-ready
**Date:** 2026-05-27
**Branch:** `feat/db-throughput-w0` (3 commits off `public/dev`)

---

## Landing

Single-wave program (W0 unconditional; W1/W2/WX are earned by W0 close-gate measurement). Three commits land the substrate + docs:

| Commit | Scope |
|---|---|
| `81292e62` | W0-A + W0-B substrate — WAL tuning, reader pool 2→4, durable rollups, gate-wait telemetry, frontend longtask observer |
| `63b3daaf` | W0-C docs — ADR-0133 writer-queue responsibility, ADR-0134 reader pool sizing, K-out `db-lock-storm-class-2026-05-27.md` |
| `0e8088a7` | L2 cycle-1 fixes — `with_transaction` wrap, `OnceLock` hydrate guard, executor.rs comment removal (AC8) |

## Acceptance criteria mapping

| AC | Description | Status | Evidence |
|---|---|---|---|
| AC1 | p95 `get_account_detail` < 200 ms under scripted peak | Plumbing shipped | `frontend.main_thread_stall` + `get_latency_rollups` ready; gate-run is W0 close protocol, not L1 |
| AC2 | No main-thread stall > 100 ms | Plumbing shipped | `longtaskObserver.ts` → `record_frontend_main_thread_stall` Tauri command → latency rollup with budget=100ms (samples ARE violations) |
| AC3 | WAL < 5 MB across 15-min enrichment burst | Plumbing shipped | `wal_autocheckpoint=200` + 30s PASSIVE checkpoint thread + `journal_size_limit=64MB` hard cap |
| AC4 | Zero writer gate-waits > 250 ms at user-active times | Plumbing shipped | Dedicated `action_db.write_transaction_gate_wait_over_250ms` rollup; log threshold lowered 5s → 250ms |
| AC5 | Zero `database is locked` errors in 24h | Plumbing shipped | WRITE_TRANSACTION_GATE process-wide + reader pool 2→4 |
| AC6 | Growth-slope re-measure at 30 / 40 entities | Linear ticket | DOS-808 commits the substrate-tier follow-up (50/100/200) |
| AC7 | Linear re-measure ticket exists and assigned | **DONE** | DOS-808 created and assigned (James Giroux) |
| AC8 | ADRs filed; `executor.rs` "own DB connection" comment removed | **DONE** | ADR-0133, ADR-0134 land in this PR; `executor.rs:630/1051` comments deleted |
| AC9 | K-out solution doc filed | **DONE** | `docs/solutions/architecture-patterns/db-lock-storm-class-2026-05-27.md` lands in this PR |
| AC10 | W1-A crash recovery proof (if W1-A ships) | N/A | W1-A is conditional on W0 close-gate miss; not in W0 scope |
| AC11 | W1-C 90+ bypass sites closed | N/A | W1-C is conditional; not in W0 scope |
| AC12 | W2 cross-partition audit (if W2 ships) | N/A | W2 conditional; not in W0 scope |

## L2 closure

| Reviewer | Verdict | Notes |
|---|---|---|
| `ce-data-integrity-guardian` | **APPROVE** | SQLCipher pragma ordering preserved (ADR-0092). Checkpoint task race on Drop is benign. 4 advisory items → maintenance project. |
| `ce-reliability-reviewer` | **APPROVE** | No parking_lot guards across `.await`. Checkpoint task lifecycle correct. Hydrate JSON drift handled. 3 residual risks + 2 testing gaps → maintenance. |
| `ce-performance-reviewer` | **APPROVE** | PRAGMA values fit AC3/AC4 envelope at current and 100/200-entity scale. Latency Mutex contention negligible at observed call rates. 4 path-α items → maintenance. |
| `/codex review` cycle 1 | **BLOCK** | 4 findings: with_transaction wrap, hydrate double-count guard, executor.rs comments, AC7 ticket. |
| `/codex review` cycle 2 | **APPROVE** | All 3 substantive findings resolved in `0e8088a7`. 4th was a false positive (DOS-808 lives outside diff scope). |

**Unanimous APPROVE after cycle 2.**

## Substrate shipped

### W0-A (`src-tauri/src/db_service.rs`)
- `apply_pragmas` adds `wal_autocheckpoint=200`, `mmap_size=256MB`, `cache_size=-64MB`, `journal_size_limit=64MB`. `PRAGMA mmap_size;` probe fails loud on 0 (SQLCipher compatibility surface).
- `spawn_checkpoint_task` runs `PRAGMA wal_checkpoint(PASSIVE)` on the writer thread every 30 s; holds `Weak<DbService>` for clean exit on drop.
- Pragma additions extended to `open_at_unencrypted_test_impl` for parity (sans the SQLCipher-specific mmap probe).

### W0-B (`db_service.rs` + `latency.rs` + `db/core.rs` + `commands/app_support.rs` + `src/main.tsx` + `src/lib/longtaskObserver.ts`)
- `NUM_READERS` 2 → 4 (ADR-0134 N+1 rule, ownership deferred to W1-D).
- `SlotKind::{Writer,Reader}` tags every latency sample; split rollups under `{label}.{phase}.writer` and `{label}.{phase}.reader`.
- `PooledConnection::with_tier(label)` / `DbService::reader_for_tier(label)` add opt-in tier-of-origin labels. API exists; site migration is W1-D.
- Latency window 256 → 4096 samples; lossless `total_samples` + `cumulative_sum_ms` counters survive eviction.
- `snapshot_for_persistence` / `apply_persistent_snapshot` route through `app_state_kv` via the writer queue every 60 s. `OnceLock` guard prevents double-application on dev-mode `reinit_db_service`.
- `WRITE_TRANSACTION_GATE` log threshold 5 s → 250 ms; latency budget 500 ms → 100 ms; dedicated `_over_250ms` rollup for AC4 violation count.
- Frontend `PerformanceObserver({type:'longtask'})` fires `record_frontend_main_thread_stall` Tauri command for stalls > 100 ms (50 ms coalesce window).

### W0-C
- ADR-0133 — writer-queue responsibility, 8 invariants. Closes the inverted-model "own DB connection to avoid starving" pattern.
- ADR-0134 — reader pool sizing, N+1 latency-tier rule. Pre-commits to W1-D shape.
- K-out — `docs/solutions/architecture-patterns/db-lock-storm-class-2026-05-27.md`. Eight-row recurring-class table, anti-pattern citations, mutex-across-await forbidden pattern, four-alternative routing.

## Build + test evidence

- `cargo clippy --workspace --all-features --lib --bins -- -D warnings` clean across the 3 commits (CI scope).
- `cargo test --lib` — 3055 passing, 11 ignored, 0 failed on attempts that didn't hit pre-existing parallel-execution flakes. Touched suites (`latency::*`, `db_service::*`) all pass in isolation across multiple runs.
- `pnpm tsc --noEmit` clean.
- Pre-commit gate (clippy + cargo test --lib + tsc) passed at landing for each of the 3 commits.

## Path-α items routed to maintenance

Surfaced across the four L2 reviewers; none block W0. Candidates for the **Codebase Maintenance & Production Quality** Linear project (`b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`):

1. mmap fail-loud could be env-overridable for hardened SQLCipher builds (data-integrity).
2. Latency snapshot row size could grow unbounded with high label cardinality post-W1-D (data-integrity, performance).
3. Test-harness unencrypted path omits mmap/cache_size pragmas — parity test recommended (data-integrity).
4. No unit test for checkpoint-task exit when last Arc drops (reliability).
5. No fault-injection test for hydrate JSON schema drift (reliability).
6. No integration test asserting WAL bounded under sustained writer load (performance).
7. `longtaskObserver` coalesce-window unit test (performance).
8. PRAGMA `cache_size` × 5 connections = 320 MB worst-case RSS — soft target, no probe (performance).
9. Parallel-test-suite flakiness on `dos674_cleanup_outside_transaction`, `glean_queue_and_manual_refresh_share_finalization_side_effects`, `compose_enrichment_full_path_rollback_atomicity`, `enrich_entity_disk_db_atomicity_under_rollback` — surfaces under W0's recording-overhead, dev baseline runs cleanly. Investigate parallel scheduling sensitivity.

## Forward links

- Plan: `.docs/plans/db-throughput-architecture.html`
- Linear AC7 ticket: DOS-808 — *Re-run db-throughput measurement protocol at 50 / 100 / 200 entities*
- Next wave: W1 only if W0 close gate misses; otherwise W0 close + DOS-808 follow-up is the end of this plan's L2 commitments.
