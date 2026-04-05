# I515 — Pipeline Reliability

**Priority:** P1
**Area:** Backend / Architecture
**Version:** 1.0.0
**Depends on:** I512 (ServiceLayer — mutations go through services before reliability wrapping makes sense)

## Problem

DailyOS has three async pipelines that run in the background: intelligence enrichment (`intel_queue.rs`), meeting prep (`meeting_prep_queue.rs`), and email enrichment (`prepare/orchestrate.rs`). All three have the same class of reliability gaps:

1. **No retry on transient failures** — PTY timeouts, DB lock contention, and network errors drop work silently. A timeout in `intel_queue.rs` logs a warning and moves on. A meeting prep failure is never re-enqueued. A scheduled task that fails waits 24 hours for the next opportunity.

2. **No partial result preservation** — If enrichment gathers context successfully but the PTY call fails, the gathered context is discarded. If a batch of 3 entities fails on entity #2, entity #1's successful result may be lost depending on where the failure occurs.

3. **Silent failures in automated processes** — The scheduler (`scheduler.rs`) catches errors with `.ok()` or `log::warn!()` and continues. The user never knows that their weekly impact report failed to generate or that 5 entities were dropped from the enrichment queue.

4. **No circuit breaker** — If the PTY subprocess is down, all enrichment and prep calls fail in sequence with no backoff. Each burns a queue slot and logs a warning.

5. **Lock poisoning hidden** — Multiple `.unwrap_or(0)` and `.ok()` calls on Mutex locks hide poisoned state, making the system appear healthy when it's degraded.

## Design

### 1. Retry with Exponential Backoff

Add a `RetryPolicy` to `intel_queue.rs` and `meeting_prep_queue.rs`:

```rust
struct RetryPolicy {
    max_attempts: u32,     // 3 for enrichment, 2 for prep
    backoff_base_ms: u64,  // 1000 (1s)
    backoff_max_ms: u64,   // 900_000 (15min)
}

struct QueueItem {
    // ... existing fields
    attempt: u32,
    retry_after: Option<DateTime<Utc>>,
    last_error: Option<String>,
}
```

Backoff schedule: 1s → 30s → 15min. Queue poller skips items where `retry_after > now()`.

**What gets retried:**
- PTY timeouts → retry (transient)
- PTY parse failures → retry once, then skip (may be prompt issue)
- DB lock contention → retry (transient)
- Gather context failures → do NOT retry (likely data issue, not transient)

**What does NOT get retried:**
- Validation failures after 2 attempts (existing `MAX_VALIDATION_RETRIES`)
- Items explicitly cancelled by the user

### 2. Partial Result Preservation

In `intel_queue.rs`, when a batch PTY call succeeds but individual entity parsing fails:

- Write successfully parsed entities immediately (don't wait for full batch)
- Re-enqueue only the failed entities for individual retry
- Log which entities succeeded vs failed

In `meeting_prep_queue.rs`, when context gathering succeeds but DB write fails:

- Cache the gathered context in memory for the retry attempt
- Don't re-gather context on retry (expensive, wasteful)

### 3. Circuit Breaker for PTY

Add a circuit breaker that trips after N consecutive PTY failures:

```rust
struct PtyCircuitBreaker {
    consecutive_failures: AtomicU32,
    trip_threshold: u32,        // 5 consecutive failures
    cooldown_ms: u64,           // 300_000 (5min)
    tripped_at: Option<Instant>,
}
```

When tripped:
- `intel_queue` stops dequeuing new items (logs "PTY circuit breaker open — waiting {cooldown}")
- `meeting_prep_queue` stops dequeuing new items
- After cooldown, one probe attempt. If it succeeds, circuit closes. If it fails, cooldown resets.

The circuit breaker is shared across both queues via `AppState`.

### 4. Failure Visibility

Add a `pipeline_failures` table:

```sql
CREATE TABLE pipeline_failures (
    id TEXT PRIMARY KEY,
    pipeline TEXT NOT NULL,       -- 'enrichment' | 'meeting_prep' | 'email_enrichment' | 'scheduler'
    entity_id TEXT,
    entity_type TEXT,
    error_type TEXT NOT NULL,     -- 'pty_timeout' | 'pty_parse' | 'db_lock' | 'db_write' | 'context_gather'
    error_message TEXT,
    attempt INTEGER DEFAULT 1,
    resolved INTEGER DEFAULT 0,  -- 1 when retry succeeds or manually dismissed
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    resolved_at TEXT
);
```

This powers:
- I428's system status panel (count of unresolved pipeline failures)
- I431's usage tracking (failed calls don't count toward cost)
- Developer debugging (what's actually failing and how often)

**Frontend exposure:** I428 (offline/degraded mode) shows pipeline health in Settings → System. This issue creates the backend table and writes to it. I428 reads from it.

### 5. Scheduler Retry

For scheduled tasks (weekly impact, monthly wrapped, day-change sweep):

- On failure, schedule a retry in 1 hour (not 24 hours)
- Max 3 retries per scheduled task per day
- Log to `pipeline_failures` table

## Files to Modify

| File | Change |
|---|---|
| `src-tauri/src/intel_queue.rs` | Add `RetryPolicy`, `retry_after` field on queue items, backoff logic in poller, partial batch result preservation, circuit breaker check before PTY calls. |
| `src-tauri/src/meeting_prep_queue.rs` | Add retry with backoff for failed preps. Cache gathered context for retry. Circuit breaker check. |
| `src-tauri/src/scheduler.rs` | Add retry-in-1-hour for failed scheduled tasks. Max 3 retries/day. Log to `pipeline_failures`. |
| `src-tauri/src/migrations/` | New migration: `pipeline_failures` table. |
| `src-tauri/src/migrations.rs` | Register migration. |
| `src-tauri/src/db/pipeline.rs` | New module: `insert_pipeline_failure()`, `resolve_pipeline_failure()`, `count_unresolved_failures()`. |
| `src-tauri/src/db/mod.rs` | Add `pub mod pipeline;`. |

## Acceptance Criteria

1. A PTY timeout during enrichment re-enqueues the entity with exponential backoff (1s → 30s → 15min). After 3 attempts, the entity is logged to `pipeline_failures` and skipped.
2. A meeting prep failure re-enqueues the meeting for retry. User-initiated manual refresh failures are retried, not silently dropped.
3. After 5 consecutive PTY failures, the circuit breaker trips. No new PTY calls are attempted for 5 minutes. After cooldown, one probe attempt determines whether to close the circuit.
4. In a batch of 3 entities, if entity #2 fails to parse but #1 and #3 succeed, #1 and #3 are written to DB. Only #2 is re-enqueued.
5. `pipeline_failures` table has rows for each unrecoverable failure with `pipeline`, `error_type`, and `attempt` count.
6. A scheduled task (e.g., weekly impact) that fails retries in 1 hour, not 24 hours. Max 3 retries/day.
7. No `.unwrap_or(0)` or `.ok()` on Mutex lock operations — lock poisoning is logged as an error and surfaced in `pipeline_failures`.
8. `cargo test` — pipeline retry logic has unit tests for backoff timing, circuit breaker state transitions, and partial batch preservation.

## Out of Scope

- Frontend pipeline health UI (that's I428's system status panel — reads from `pipeline_failures`)
- Dead letter queue with manual retry UI (future enhancement)
- Distributed tracing or OpenTelemetry integration
- Pipeline metrics/dashboards beyond the `pipeline_failures` table
- Changing the queue architecture (e.g., replacing Mutex queues with channels)
