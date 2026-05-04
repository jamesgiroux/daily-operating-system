# I504 — AI-Inferred Relationship Extraction from Enrichment

**Priority:** P1
**Area:** Backend / Intelligence
**Version:** 1.1.0
**Depends on:** None (Phase 1, parallelizable)
**ADR:** None (uses existing person_relationships schema from ADR-0088)
**Supersedes:** I485 (AI inference part only — Glean relationships moved to I505, co-attendance to I506)

## Problem

Two broken pieces in the AI relationship inference pipeline:

**1. The enrichment prompt never asks for `inferredRelationships`.** The JSON output schema in `intelligence/prompts.rs` (lines ~1477-1603) defines the expected response structure. It includes `keyRelationships` (AI-assessed key contacts) but does NOT include an `inferredRelationships` field. The LLM is never told to produce person-to-person relationship edges.

**2. The extraction function is never called.** `extract_inferred_relationships()` (prompts.rs line 1929) exists, parses the `inferredRelationships` JSON array, validates against `RelationshipType` enum, filters empty IDs — it works. But `intel_queue.rs` line 804 has an explicit deferral comment citing a name→ID resolution problem that is already solved by the `canonical_contacts` context (which provides person IDs to the LLM).

The result: on every enrichment cycle, the AI has the context to infer person-to-person relationships (canonical contacts with IDs, meeting history, email threads showing who works with whom) but is never asked to produce them, and the extraction function that would persist them is never called.

The `person_relationships` table (migration 038) is fully designed with 7 relationship types, confidence scoring, time-decay, context scoping, and source tracking. `upsert_person_relationship()` works. The table is empty except for user-confirmed edges.

## Design

### 1. Add `inferredRelationships` to the enrichment prompt JSON schema

In `intelligence/prompts.rs`, add to the JSON output schema (after `keyRelationships`):

```json
"inferredRelationships": [
  {
    "fromPersonId": "person ID from canonical contacts",
    "toPersonId": "person ID from canonical contacts",
    "relationshipType": "peer|manager|mentor|collaborator|ally|partner|introduced_by",
    "rationale": "1 sentence explaining why this relationship is inferred"
  }
]
```

Add behavioral instruction:
```
For inferredRelationships: Analyze meeting co-attendance patterns, email threads, and content context to infer person-to-person relationships. Only use person IDs from the canonical contacts list. Only infer relationships with clear evidence — do not guess. Prefer 'collaborator' for people who work together on projects, 'manager' when reporting lines are evident, 'peer' when at similar organizational level. Return empty array if no relationships can be confidently inferred.
```

### 2. Call `extract_inferred_relationships()` after enrichment response parsing

In `intel_queue.rs`, replace the deferral comment at line 804 with:

```rust
// Extract and persist inferred relationships (I504)
let inferred = crate::intelligence::prompts::extract_inferred_relationships(&raw_response);
for rel in &inferred {
    // Guard against overwriting user-confirmed relationships
    let existing = db.get_relationships_between(&rel.from_person_id, &rel.to_person_id)?;
    if existing.iter().any(|r| r.source == "user_confirmed" && r.confidence >= 0.8) {
        continue;
    }

    let upsert = UpsertRelationship {
        id: &format!("pr-ai-{}-{}", rel.from_person_id, rel.to_person_id),
        from_person_id: &rel.from_person_id,
        to_person_id: &rel.to_person_id,
        relationship_type: &rel.relationship_type,
        direction: "symmetric", // AI-inferred edges default symmetric; manager edges use "directed"
        confidence: 0.6,
        context_entity_id: Some(&entity_id),
        context_entity_type: Some(&entity_type),
        source: "ai_enrichment",
        // Store the AI's rationale for this inference — useful for user review and debugging
        rationale: rel.rationale.as_deref(),
    };
    db.upsert_person_relationship(&upsert)?;

    let _ = emit_signal_and_propagate(
        &db, &propagation_engine,
        &entity_type, &entity_id,
        "relationship_inferred", "ai_enrichment",
        Some(&format!("{} → {}: {}", rel.from_person_id, rel.to_person_id, rel.relationship_type)),
        0.6,
    );
}
```

### Rationale field preservation

The `extract_inferred_relationships()` parser currently discards the `rationale` field from the LLM response. The `InferredRelationship` struct should include `rationale: Option<String>` and the parser should extract it. The rationale is persisted via `UpsertRelationship.rationale` (requires adding a `rationale TEXT` column to `person_relationships` if not already present — check migration 038). The rationale is valuable for:
- User review: "AI thinks Alice reports to Bob because they were always in the same meeting and Alice deferred to Bob's decisions"
- Debugging: understanding why the AI inferred a particular relationship type
- Frontend: showing "suggested" relationships with explanatory text

### 3. Direction handling for manager relationships

When `relationship_type` is `"manager"`, set `direction` to `"directed"` (from reports-to, to manager). For all other types, use `"symmetric"`.

### 4. Deterministic IDs for upsert behavior

ID format `pr-ai-{from}-{to}` ensures re-enrichment reinforces (updates `last_reinforced_at`) rather than duplicates. The 90-day half-life decay in `effective_confidence()` means AI-inferred edges that stop being reinforced naturally fade.

### 5. Batch enrichment path

The batch enrichment path (`run_batch_enrichment()`) processes multiple entities per PTY call. Add relationship extraction in `apply_enrichment_results()` after `write_intelligence_json`, iterating each entity's raw response section.

### 6. Add `get_relationships_between()` query

New function in `db/person_relationships.rs`:

```rust
pub fn get_relationships_between(
    &self,
    from_id: &str,
    to_id: &str,
) -> Result<Vec<PersonRelationship>, DbError> {
    // Query both directions since symmetric relationships may be stored either way
    let sql = "SELECT * FROM person_relationships
               WHERE (from_person_id = ?1 AND to_person_id = ?2)
                  OR (from_person_id = ?2 AND to_person_id = ?1)";
    // ...
}
```

## Files to Modify

| File | Change |
|---|---|
| `src-tauri/src/intelligence/prompts.rs` (~line 1560) | Add `inferredRelationships` to JSON output schema with relationship types and rationale field. Add behavioral instruction for when/how to infer. |
| `src-tauri/src/intel_queue.rs` (~line 804) | Replace deferral comment with extraction call. Guard against user-confirmed. Upsert to person_relationships. Emit signals. Handle both single and batch enrichment paths. |
| `src-tauri/src/db/person_relationships.rs` | Add `get_relationships_between(from_id, to_id)` query. Add `rationale TEXT` column to `person_relationships` if not in migration 038. |
| `src/pages/PersonDetailPage.tsx` (or equivalent) | Render AI-inferred relationships in Network chapter with "suggested" label and rationale tooltip. (AC #7 requires frontend changes.) |
| `src/types/index.ts` | Add `rationale?: string` to `PersonRelationship` type if not already present. |

## Acceptance Criteria

1. Enrich account with 3+ stakeholders in canonical_contacts. `SELECT * FROM person_relationships WHERE source = 'ai_enrichment'` has rows.
2. Inferred relationships have confidence 0.6, source `"ai_enrichment"`, and context_entity_id set to the enriched account.
3. Manager relationships have direction `"directed"`. Peer/collaborator relationships have direction `"symmetric"`.
4. Re-enrichment of the same account updates `last_reinforced_at` on existing AI-inferred edges — does not create duplicates.
5. User-confirmed relationships (source `"user_confirmed"`, confidence >= 0.8) are NOT overwritten by AI-inferred ones.
6. Signal `"relationship_inferred"` emitted per new relationship via `emit_signal_and_propagate()`.
7. Person detail page shows AI-inferred relationships in Network chapter with "suggested" label at 0.6 confidence.
8. Enrichment prompt's JSON schema includes `inferredRelationships` field — verified by reading the prompt output.

## Out of Scope

- Glean-sourced relationships (I505)
- Co-attendance relationship inference (I506)
- UI for confirming/rejecting inferred relationships — person detail page feature
- Relationship type expansion — use existing 7 types from migration 038
- New propagation rules triggered by relationship changes — future enhancement
