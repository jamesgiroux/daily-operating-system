# ADR-0067: Resume latency rollups + staged split-lock DB strategy

**Date:** 2026-02-12
**Status:** accepted

## Context

I197 identified two remaining runtime risks after initial startup/auth hardening:

1. Latency regressions were only visible as individual logs, with no p95 rollups.
2. DB concurrency behavior was inconsistent across commands due to direct ad-hoc
   `state.db.lock()`/`try_lock()` usage.

We needed to improve focus-resume and cold-start responsiveness without a large
storage-layer rewrite in the same issue.

## Decision

1. Introduce in-memory latency rollups for command diagnostics.
- Record bounded per-command samples and expose `p50`/`p95`/max, budget violations,
  and degraded-result counters.
- Surface rollups through backend command `get_latency_rollups` and devtools.
- Keep production UI unchanged.

2. Adopt staged split-lock enforcement as the DB concurrency strategy.
- Keep current single `ActionDb` connection for now.
- Standardize DB access through `AppState` helpers:
  - `with_db_try_read` for hot, non-blocking read paths
  - `with_db_read` for non-hot reads
  - `with_db_write` for writes
- Migrate highest-contention read paths first (`get_dashboard_data`, `get_focus_data`)
  and continue incremental migration in follow-up passes.

3. Defer deeper architecture options.
- Do not introduce a connection pool or worker-queue DB model in I197.
- Re-evaluate pool/queue only if helper-based staged migration fails latency targets.

## Consequences

### Positive
- p95 visibility is now immediate for hot commands without extra infra.
- Hot reads can degrade quickly under DB contention instead of blocking UI.
- Commands have one explicit lock-scope pattern, reducing accidental long-held locks.

### Trade-offs
- Rollups are process-local and reset on app restart.
- Mixed old/new DB call sites remain until follow-up migration passes complete.
- Single-connection SQLite model still bounds peak concurrent throughput.
