# I457 — Background Task Throttling

**Status:** Done
**Priority:** P1
**Version:** 0.14.2
**Area:** Backend / Performance

## Summary

DailyOS spawns 15 independent background loops at startup. Three poll every 5 seconds (intel queue, prep queue, embeddings) even when queues are empty — 36 wakeups/minute combined. PTY subprocess and embedding inference can run simultaneously with no coordination, saturating CPU. No awareness of user activity or system load. Result: UI lag, system-wide slowdowns, other apps affected.

**Fix:** Three-layer throttling system — ActivityMonitor (user presence awareness), HeavyWorkSemaphore (one expensive op at a time), adaptive polling (activity-aware intervals with Notify wake signals).

## Acceptance Criteria

### Layer 1: ActivityMonitor

1. **ActivityMonitor tracks three states**: `Active` (window focused + interaction in last 2 min), `Idle` (focused, no interaction 2+ min), `Background` (window not focused). Verify: `get_background_status` command returns correct `activityLevel`.
2. **Frontend signals activity**: window focus/blur events and debounced click/keypress (1 call per 5s max) call `signal_window_focus` and `signal_user_activity` Tauri commands.
3. **No polling in the frontend**: activity signaling is event-driven, not interval-based.

### Layer 2: HeavyWorkSemaphore

4. **PTY and embeddings never run simultaneously**: `heavy_work_semaphore` (permits=1) is acquired before PTY calls in `intel_queue.rs` and embedding inference in `embeddings.rs`. Verify: enqueue both an intel enrichment and embedding request — only one runs at a time.
5. **Hygiene skips when busy**: `hygiene.rs` uses `try_acquire()` — if semaphore is held, hygiene defers to next cycle rather than waiting.
6. **Manual enrichment bypasses semaphore**: `enrich_account` and `enrich_person` commands go through `services::intelligence::enrich_entity()` which doesn't acquire the semaphore. User clicks are never blocked.

### Layer 3: Adaptive Polling

7. **Queue processors back off during active use**: intel/prep/embedding processors poll every 30s when user is Active and queues are empty (was 5s). Full speed (5s) when app is in Background. 2s when work is queued.
8. **Instant wake on enqueue**: all enqueue call sites fire `Notify::notify_one()` so processors wake immediately when work arrives, regardless of current poll interval.
9. **Calendar poller throttled**: 120s when Active, 60s when Idle, 30s when Background (was fixed 30s).
10. **Email poller throttled**: same adaptive intervals, preserving existing `tokio::select!` wake pattern.
11. **`cargo test` passes**: no regressions. All 1004 tests pass.

### Observability

12. **`get_background_status` command** returns: `activityLevel`, `intelQueueDepth`, `prepQueueDepth`, `heavyWorkPermits`. Visible in dev tools.

## Files Modified

### New
- `src-tauri/src/activity.rs` — ActivityMonitor, ActivityLevel, adaptive interval helpers
- `src/hooks/useActivitySignal.ts` — frontend activity signaling hook

### Modified (backend)
- `src-tauri/src/state.rs` — activity, heavy_work_semaphore, 3 queue wake Notify signals
- `src-tauri/src/lib.rs` — mod activity, command registration
- `src-tauri/src/commands.rs` — signal_user_activity, signal_window_focus, get_background_status
- `src-tauri/src/intel_queue.rs` — semaphore + adaptive polling + Notify wake
- `src-tauri/src/meeting_prep_queue.rs` — adaptive polling + Notify wake
- `src-tauri/src/processor/embeddings.rs` — semaphore + adaptive polling + Notify wake
- `src-tauri/src/hygiene.rs` — try_acquire semaphore
- `src-tauri/src/google.rs` — adaptive network intervals for calendar + email pollers
- `src-tauri/src/services/accounts.rs` — intel_queue_wake on enqueue
- `src-tauri/src/services/meetings.rs` — intel_queue_wake + prep_queue_wake on enqueue
- `src-tauri/src/watcher.rs` — embedding_queue_wake + intel_queue_wake on enqueue
- `src-tauri/src/intelligence/lifecycle.rs` — prep_queue_wake on enqueue

### Modified (frontend)
- `src/router.tsx` — mount useActivitySignal in RootLayout

## Expected Impact

| Metric | Before | After (active use) | Reduction |
|--------|--------|---------------------|-----------|
| Queue processor wakeups/min | 36 | 6 | 83% |
| Calendar + email wakeups/min | 4 | 1 | 75% |
| Max concurrent heavy ops | 2 | 1 | 50% CPU peak |
