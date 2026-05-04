# I406 — Entity quality scoring

**Status:** Open
**Priority:** P1
**Version:** 0.13.7
**Area:** Backend / Intelligence

## Summary

Implement a Beta distribution quality model per entity, replacing the binary has/missing classification. Quality scores measure confidence in AI-generated intelligence: entities start at 0.5 (maximum uncertainty), increase when enrichment succeeds, and decrease when users correct errors. The `entity_quality` table also hosts circuit breaker state for coherence retries (I410). This establishes the foundational mechanism for all downstream quality awareness and feedback closure.

## Acceptance Criteria

1. An `entity_quality` table exists in the DB with columns: `entity_id` (TEXT PK), `entity_type` (TEXT), `quality_alpha` (REAL DEFAULT 1.0), `quality_beta` (REAL DEFAULT 1.0), `quality_score` (REAL, computed: alpha/(alpha+beta)), `last_enrichment_at` (TEXT), `correction_count` (INTEGER DEFAULT 0), `coherence_retry_count` (INTEGER DEFAULT 0), `coherence_window_start` (TEXT), `coherence_blocked` (INTEGER DEFAULT 0), `created_at` (TEXT), `updated_at` (TEXT). Migration exists.

2. Quality score starts at 0.5 (`Beta(1,1)`) for newly-created entities. Verify: create a new account; `SELECT quality_score FROM entity_quality WHERE entity_id = '<id>'` returns 0.5.

3. After a successful enrichment completes (IntelligenceService writes new `entity_intel`): `quality_alpha` increments by 1.0. `quality_score` increases. Verify with a known entity that has not been corrected.

4. After a user edits an AI-generated intelligence field (via IntelligenceService.update_intelligence_field): `quality_beta` increments by 1.0. `quality_score` decreases. Verify by editing a field on a real account and checking the table.

5. `get_entities_without_intelligence()` and `get_stale_entity_intelligence(14)` are replaced (or augmented) by `get_entities_below_quality_threshold(threshold: f64)`. The hygiene scan uses the quality score to prioritize enrichment — entities with quality < 0.45 are enqueued before entities that are merely stale.

6. The hygiene report (`HygieneReport`) includes a `low_quality_entities` count: entities with `quality_score < 0.45`. The UI hygiene panel surfaces this as a new gap type.

7. Entities with no `entity_quality` row (pre-existing entities not yet scored) default to 0.5 in all quality-aware code paths. No null pointer / missing-row errors. `cargo test` passes.

8. The `entity_quality` table is entity-type-agnostic: accounts, projects, and people all use it. The schema accommodates the user entity coming in v0.14.0 without migration changes.

## Dependencies

- Foundational — required by I408 (trigger function uses quality_score as input)
- Required by I409 (feedback closure writes to quality_alpha/beta)
- Required by I410 (event-driven triggers update quality score + circuit breaker columns)

## Notes / Rationale

From the research document: Beta distribution quality scores are Thompson Sampling applied to quality rather than source reliability — architecturally identical to what `sampling.rs` already implements for signal sources. A new entity starts at `Beta(1,1)` — maximum uncertainty, mean 0.5. After 10 enrichments with no corrections: `Beta(11,1)` — quality score 0.92. After 3 corrections out of 5 enrichments: `Beta(3,4)` — quality score 0.43, flag for re-enrichment. This replaces the binary "has intelligence or doesn't" classification with a continuous measure of confidence in the intelligence content.

Circuit breaker columns (`coherence_retry_count`, `coherence_window_start`, `coherence_blocked`) live on this table because circuit breaker state is health metadata — same domain as quality scores. Keeping it here avoids splitting health state across `entity_quality` and `entity_intel`.
