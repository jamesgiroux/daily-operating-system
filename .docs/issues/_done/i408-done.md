# I408 — Enrichment trigger function

**Status:** Open
**Priority:** P1
**Version:** 0.13.7
**Area:** Backend / Intelligence

## Summary

Replace the hardcoded 14-day enrichment threshold with a continuous trigger score that accounts for meeting imminence, staleness, entity importance, and recent signal activity. This enables the system to correctly prioritize enrichment budget toward high-value contexts (important meetings in the next 24 hours) and away from dormant entities. The trigger score drives both scheduled hygiene sweeps and event-driven enrichment decisions.

## Acceptance Criteria

1. A `compute_enrichment_trigger_score(entity_id, entity_type, db) -> f64` function exists in `self_healing/remediation.rs`. It returns a continuous score in [0, 1] computed from:
   - `meeting_imminence`: 1.0 if linked meeting within 24h, 0.5 if within 7 days, 0.1 otherwise
   - `staleness`: days_since_enrichment / 14.0 (capped at 1.0 — the 14-day threshold is now the "normal" staleness ceiling, not an absolute gate)
   - `entity_importance`: derived from account tier, meeting frequency, or signal count (normalized to [0,1])
   - `signal_delta`: number of new signals since last enrichment / 10.0 (capped at 1.0)
   - Weighted sum: `(imminence × 0.4) + (staleness × 0.3) + (importance × 0.2) + (signal_delta × 0.1)`

2. `self_healing::evaluate_portfolio()` (called from hygiene Phase 3, replacing `enqueue_ai_enrichments()`) uses `compute_enrichment_trigger_score` to prioritize the queue: entities are sorted by trigger score descending before being enqueued via IntelligenceService. Entities with trigger score < 0.25 are not enqueued even if they exist in `get_entities_without_intelligence()` (they're not important enough to spend AI budget on right now).

3. An entity with a meeting in 12 hours and intelligence last enriched 20 days ago with 5 new signals has a higher trigger score than an entity with no upcoming meetings and intelligence 8 days old. Verify by computing the function on known entities in the DB and confirming the ordering makes sense.

4. The `check_upcoming_meeting_readiness()` function is refactored to use `compute_enrichment_trigger_score` instead of its own `PRE_MEETING_STALE_DAYS` constant. The pre-meeting window becomes a special case of the trigger score: when `meeting_imminence = 1.0`, even modestly stale entities get enqueued.

5. The trigger score is logged at DEBUG level for each entity evaluated, allowing manual inspection of why an entity was or was not enqueued. Verify via logs during a hygiene scan run.

## Dependencies

- addBlockedBy I406 — quality_score is a trigger function input

## Notes / Rationale

From the research document: the decision to enrich is a cost/benefit trade — AI budget spent vs. meeting value at stake. The current binary approach (> 14 days = enrich, <= 14 days = skip) treats a renewal account 24 hours before an important meeting the same as an archived account that hasn't had contact in months. A continuous trigger score makes this priority explicit: `meeting_imminence × 0.4` gives meetings the highest weight (captures the pre-meeting window), `staleness × 0.3` ensures aging entities get re-evaluated, `entity_importance × 0.2` respects the portfolio's actual needs (high-touch accounts), and `signal_delta × 0.1` surfaces entities with fresh context changes. The 14-day threshold becomes the "normal" staleness ceiling — a special case of the continuous function, not a gate.
