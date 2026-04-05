# I410 — Hygiene event-driven triggers

**Status:** Open
**Priority:** P1
**Version:** 0.13.7
**Area:** Backend / Hygiene

## Summary

Wire quality re-evaluation and coherence checking into the signal event flow so that enrichment and validation respond to real-time context changes rather than only at the 4-hour scan boundary. When a signal arrives for an entity, the system immediately evaluates whether enrichment is needed and checks the intelligence for coherence violations. This makes the hygiene system reactive while preserving the 4-hour sweep as a catch-all.

## Acceptance Criteria

1. When a signal arrives for entity X (any signal via `bus::emit_signal_and_propagate`), the signal handler calls `self_healing::scheduler::evaluate_on_signal(entity_id, entity_type, db)`. This function: (a) updates the entity's trigger score via `compute_enrichment_trigger_score`, (b) if the trigger score exceeds 0.7, enqueues the entity via IntelligenceService at `ContentChange` priority without waiting for the next 4-hour scan.

2. The 4-hour hygiene scan (`run_hygiene_loop`) is unchanged and serves as the catch-all sweep for entities that haven't received signals recently. The event-driven path handles freshly-active entities.

3. The coherence check (I407) fires on intel_queue completion events — when entity intelligence is freshly written, `self_healing::scheduler::on_enrichment_complete(entity_id)` runs the coherence check on the new content. If it fails, re-enqueue via IntelligenceService at `ContentChange`. This is the ONLY path coherence checks run (not in prep assembly, not in the 4-hour scan).

4. **Circuit breaker** prevents thrashing. State tracked on `entity_quality` (not `entity_intel`):
   - `coherence_retry_count` — incremented on each coherence-triggered re-enrichment
   - `coherence_window_start` — ISO8601 timestamp of first retry in current 24h window
   - `coherence_blocked` — set to 1 when count >= 3 within the window
   - **Window logic:** On coherence failure: if `coherence_window_start` is within last 24h, increment `coherence_retry_count`; if not, reset count to 1 and set new window start. If count >= 3, set `coherence_blocked = 1` and do NOT re-enqueue.
   - **Auto-expiry:** When `coherence_window_start` is >72h ago (24h window + 48h cooldown), reset `coherence_blocked = 0` and `coherence_retry_count = 0` on next evaluation. The entity gets one more try with whatever new context has accumulated.
   - **User override:** If the user manually triggers "Refresh Intelligence" (Manual priority), the circuit breaker is bypassed. User intent overrides the safety valve. The coherence retry count resets on manual trigger.
   - Blocked entities surface in the hygiene report as needing review. Verify: `SELECT entity_id, coherence_blocked, coherence_retry_count FROM entity_quality WHERE coherence_blocked = 1`.

5. The event-driven path does not hold the DB Mutex longer than the 4-hour scan path. Signal emission is already O(1) and the quality evaluation is a single DB read + write. Verify via latency logs: signal emission latency does not measurably increase after this change.

## Dependencies

- addBlockedBy I406 — needs quality scoring table (including circuit breaker columns)
- addBlockedBy I407 — coherence check is the main event-driven action
- addBlockedBy I408 — trigger score drives the enqueue decision

## Notes / Rationale

From the research document: the current 4-hour hygiene scan approximates the self-healing pattern but runs on a timer rather than responding to events. Real enrichment pipelines (dbt, Apache Flink) are event-driven: tests run after every pipeline output, not on a schedule. For DailyOS, the signal bus is architecturally a Change Data Capture (CDC) system — the hygiene loop should respond to signal events, not run on a fixed 4-hour boundary. This issue adds event-driven triggers on top of (not replacing) the 4-hour sweep, creating a hybrid approach: freshly-active entities get immediate attention, dormant entities are still covered by the scheduled scan.

**Design decisions (2026-02-22):**
- Circuit breaker state lives on `entity_quality` table (not `entity_intel`) — health metadata belongs with health metadata.
- 72h total cooldown (24h window + 48h rest) before auto-retry. Long enough to avoid thrashing, short enough that new context (meetings, emails, signals) may have improved the entity's enrichment prospects.
- Manual "Refresh Intelligence" always bypasses the breaker and resets retry state. The circuit breaker protects AI budget from autonomous thrashing, not from deliberate user action.
- Coherence checks run ONLY on enrichment completion (post-write), not in prep assembly or the 4-hour scan. This is the single path — simple, event-driven, no hot-path cost.
