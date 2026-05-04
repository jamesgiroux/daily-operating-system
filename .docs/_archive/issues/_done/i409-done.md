# I409 — Feedback closure

**Status:** Open
**Priority:** P1
**Version:** 0.13.7
**Area:** Backend / Signals

## Summary

Wire user corrections back to source reliability tracking and entity quality scores. When users edit AI-generated or enrichment-source fields, the system updates the Thompson Sampling weights for that source and increments the entity's quality_beta. This closes the feedback loop: the system learns which sources are reliable for which entity types, and quality scores reflect observed correction patterns. Over time, the system gets better at knowing what to trust.

## Acceptance Criteria

1. When IntelligenceService.update_intelligence_field is called (user editing an AI-generated field), in addition to emitting a `user_correction` signal, it calls `self_healing::feedback::record_enrichment_correction(entity_id, entity_type, "intel_queue")`. This: (a) increments `quality_beta` in `entity_quality`, (b) calls `signals::sampling::update_source_reliability("intel_queue", entity_type, false)` to decrement the source's `learned_beta`.

2. When `update_account_field`, `update_person_field`, or `update_project_field` is called (user editing a non-intelligence field that may have been populated by Clay enrichment), and the field's known source is `clay`, call `self_healing::feedback::record_enrichment_correction(entity_id, entity_type, "clay")`. The Clay source's reliability for that entity type decrements.

3. Verify the feedback loop closes end-to-end: edit an intelligence field on a known account. Check: (a) `signal_events` has a new `user_correction` signal; (b) `entity_quality` has `quality_beta` incremented for this entity; (c) the source reliability store has an updated `learned_beta` for `intel_queue`. All three should change in one edit action.

4. A successful enrichment that is NOT subsequently corrected (no user edits within 7 days) results in a `quality_alpha` increment. Verify: identify an entity whose last enrichment was 10+ days ago with no corrections; check `quality_alpha` increased after enrichment vs. before.

5. Verify the system gets better over time: run the app for a week with normal usage. `SELECT entity_id, quality_alpha, quality_beta, quality_score FROM entity_quality ORDER BY quality_score ASC LIMIT 10` — entities the user has corrected most should have lower quality scores, not equal quality scores. The scores must be differentiated, not uniformly reset.

## Dependencies

- addBlockedBy I406 — feedback writes to quality_alpha/beta
- Independent of I407/I408 — can be built in parallel with other issues

## Notes / Rationale

From the research document: user corrections are currently recorded as signals (`user_correction` signal type in `signal_events`) but are not plumbed back to update source reliability in the Thompson Sampling store. Result: the system never learns that a specific enrichment source (e.g., Clay enrichment for a particular company type) produces unreliable data. The sampling weights don't change regardless of how many corrections accumulate. This issue closes that gap by integrating corrections into both the quality scoring system (entity-level) and the source reliability system (source-level). Architecturally, this uses the existing Thompson Sampling infrastructure in `signals::sampling::update_source_reliability`, treating corrections as negative feedback signals that decrement a source's `learned_beta`.
