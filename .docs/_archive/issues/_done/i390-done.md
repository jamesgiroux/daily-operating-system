# I390 — Person Relationship Graph — `person_relationships` Table, Typed Edges, Confidence Scoring, Context Scoping, Tauri CRUD

**Status:** Open (0.13.5)
**Priority:** P1
**Version:** 0.13.5
**Area:** Backend / Entity

## Summary

People in DailyOS are currently isolated rows — they have individual intelligence but no structural relationships to each other. A buying committee at Acme Corp is a network of individuals with influence flows: the CFO approves but defers to the champion, the champion won't buy without the technical evaluator's sign-off, and the blocker has informal veto power. This issue establishes the data foundation for the people relationship network: a `person_relationships` table (migration 038) with typed, directional edges, confidence scoring with computed-on-read decay, optional context scoping (a person can be a "champion" in one account context and a "peer" in another project context), and Tauri CRUD commands for programmatic and future UI use.

This is the data layer that I391 (network intelligence) and I392 (network view on person detail) build on.

## Acceptance Criteria

1. `person_relationships` table exists in the DB (migration 038) with columns: `id`, `from_person_id`, `to_person_id`, `relationship_type`, `direction`, `confidence`, `context_entity_id`, `context_entity_type`, `source`, `created_at`, `updated_at`, `last_reinforced_at`. Creating an edge via the `upsert_person_relationship` Tauri command succeeds without error.

2. All 10 relationship types are valid enum values: `champion`, `executive_sponsor`, `decision_maker`, `technical_evaluator`, `blocker`, `peer`, `ally`, `detractor`, `collaborator`, `dependency`. Attempting to insert an invalid type is rejected by the Rust type system (enum validation before DB insert).

3. Context scoping works: insert two edges for the same person pair — one scoped to Account X (`context_entity_id` and `context_entity_type` both set), one scoped to Project Y. Query returns both edges. Filtering by `context_entity_id` returns only the relevant edge(s).

4. Confidence decay is computed on read: query an edge created 30 days ago with initial confidence 0.8 and `source = 'inferred'` (90-day half-life). The returned effective confidence is `0.8 * 2^(-30/90) ≈ 0.635`. A `user_confirmed` edge does not decay regardless of age.

5. `upsert_person_relationship` and `delete_person_relationship` Tauri commands exist. Upserting an existing edge (same `from_person_id`, `to_person_id`, `relationship_type`, `context_entity_id`) updates its fields and advances `updated_at`. Deleting an edge removes it from the DB.

6. A DB migration handles the `person_relationships` table creation. `cargo test` passes. All existing person data is unaffected — no person records are deleted or corrupted.

## Implementation Notes

- Next available migration number: **038** (current max: 037_project_hierarchy.sql).
- `entity_people` already links persons to accounts/projects — different concern, no collision.
- `account_team` has role assignments (csm, champion, tam) for account teams — different concern from person-to-person relationships.
- Decay formula reuses `signals/decay.rs` logic: `decayed = base * 2^(-age_days / half_life_days)`. Default half-life: 90 days for `source = 'inferred'`, no decay for `source = 'user_confirmed'`. Computed at query time from `last_reinforced_at` (or `created_at` if never reinforced).
- Edge creation has two paths: enrichment pipeline writes `source = 'inferred'` at ~0.4 confidence (I391); Tauri commands enable programmatic and future UI use (edge suggestion confirmation UI ships in v0.13.6).

## Dependencies

- Foundational for I391 (network intelligence reads edges, writes inferred edges) and I392 (UI renders edges).
- See ADR-0088 — this issue implements decision 1 (typed, directional graph).

## Notes / Rationale

ADR-0088 distinguishes people from the hierarchy model (ADR-0087): people don't form trees, they form relationship graphs. The `person_relationships` table is the structural foundation for making those relationships visible, persistent, and intelligent. Without it, person intelligence is individual-level only — it can tell you what's happening with Jack, but not how Jack relates to Sarah, and what that relationship means for the Acme deal.
