# I407 — Semantic coherence validation

**Status:** Open
**Priority:** P1
**Version:** 0.13.7
**Area:** Backend / Intelligence + Signals

## Summary

Implement embedding-based coherence validation that detects when AI-generated intelligence has drifted from the entity's actual context. The canonical failure case is the Jefferson Advisors meeting mentioning "Adobe Fonts" when the entity is Agentforce — coherence checking catches this by comparing the intelligence embedding against the entity's linked meeting corpus, flags coherence failures, and triggers re-enrichment. This is the flagship feature that prevents the system from confidently serving wrong intelligence.

## Acceptance Criteria

1. A `coherence_check` function exists in `self_healing/detector.rs` that takes an entity's `executive_assessment` text and the entity's linked meeting corpus (last 90 days of meeting titles + summaries), embeds both using the existing `EmbeddingModel`, and returns a cosine similarity score in [0, 1].

2. The coherence check runs as a **post-enrichment validation step** — triggered when intel_queue completes writing new entity intelligence (wired via I410's `on_enrichment_complete`), NOT in the meeting prep assembly hot path. If cosine similarity < 0.30 (configurable threshold), the intelligence is marked `coherence_flagged = true` in `entity_intel` and the entity is re-enqueued via IntelligenceService at `ContentChange` priority.

3. **The Jefferson Advisors/Adobe Fonts test:** Open the running app. For the Jefferson Advisors meeting (entity: Agentforce project), the coherence check should detect that the intelligence text mentioning "Adobe Fonts" is semantically distant from meeting titles like "Agentforce", "Salesforce", "VIP". Verify: `SELECT coherence_score FROM entity_intel WHERE entity_id = '<agentforce_id>'` returns a score below threshold. After re-enrichment runs, the intelligence no longer mentions Adobe Fonts.

4. Coherence check is skipped for entities with < 2 linked meetings (insufficient reference corpus — not a coherence failure, just sparse data). Verify: an entity with 1 linked meeting does not get `coherence_flagged`.

5. Coherence scores are stored in `entity_intel.coherence_score` (new column). Verify: after running the check for accounts with real meeting data, `SELECT entity_id, coherence_score FROM entity_intel WHERE coherence_score IS NOT NULL` returns rows with scores between 0 and 1.

6. A coherence failure emits a signal (`entity_coherence_flagged` signal type) via `bus::emit_signal_and_propagate()` so downstream consumers (callouts, invalidation) can react. Verify: `SELECT signal_type, entity_id FROM signal_events WHERE signal_type = 'entity_coherence_flagged' ORDER BY created_at DESC LIMIT 5` returns rows after the check runs.

7. The embedding model being unavailable or not yet loaded causes the coherence check to skip gracefully (log a warning, proceed without flagging). No crashes if the model is loading.

## Dependencies

- Depends on embedding model availability (background task #1)
- addBlockedBy I406 — coherence failures should lower quality_score; the quality table must exist first
- Independent of I404/I405 (can be built before or after)

## Notes / Rationale

From the research document: the Jefferson Advisors meeting intelligence mentions "Adobe Fonts" yet the entity is linked to the Agentforce project. "Adobe Fonts" appears nowhere in the meeting history, emails, or signals. This is a topic coherence failure — the AI generated content unrelated to the entity's actual context, likely because an earlier meeting had ambiguous entity resolution and contaminated the context. The fix uses embedding-based text anomaly detection comparing a document (intelligence) against a reference corpus from the same domain (entity's own meeting history). From TAD-Bench (2025), this approach works best when the reference corpus is the entity's own data rather than a generic database. Uses `nomic-embed-text-v1.5` which already runs in background task #1, supporting 8,192 tokens and requiring no new models or API calls.

**Design decision (2026-02-22):** Coherence runs post-enrichment (via I410 `on_enrichment_complete`), not in the prep assembly hot path. This avoids adding embedding computation to every meeting prep load and keeps the check event-driven. The prep path just reads the stored `coherence_flagged` state.
