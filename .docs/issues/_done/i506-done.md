# I506 — Co-Attendance Relationship Inference

**Priority:** P2
**Area:** Backend / Intelligence + Signals
**Version:** 1.1.0
**Depends on:** None (parallelizable — uses existing meeting_attendees data)
**Supersedes:** I485 (co-attendance part)

## Problem

People who repeatedly attend the same meetings have an implicit working relationship — they are collaborators, peers, or at minimum professionally connected. DailyOS tracks meeting attendees via `meeting_attendees` table (person_id + meeting_id) but never analyzes co-attendance patterns to infer `person_relationships` edges.

This is a purely algorithmic signal source — no LLM needed, no external API. The data is already in the local DB from calendar sync. Two people who've been in 5+ meetings together in 90 days almost certainly work together, and that relationship should be surfaced in the Network chapter, inform the Stakeholder Coverage health dimension (I499), and feed into the Stakeholder Map report (I496).

Currently, person_relationships edges only come from:
- User-confirmed (manual entry, confidence 1.0)
- AI-inferred (I504, confidence 0.6, once implemented)
- Glean structural (I505, confidence 0.8, once implemented)

Co-attendance would add a fourth source at moderate confidence, reinforced by frequency.

## Design

### 1. Co-Attendance Analysis Function

New function `compute_co_attendance_relationships()` in `src-tauri/src/intelligence/health.rs` (or a new `src-tauri/src/intelligence/relationships.rs` module):

```rust
pub struct CoAttendancePair {
    pub person_a_id: String,
    pub person_b_id: String,
    pub meeting_count: u32,        // meetings both attended in window
    pub most_recent: String,       // ISO timestamp of last shared meeting
    pub entity_id: Option<String>, // account/project context if all meetings share one
}

pub fn compute_co_attendance(
    db: &ActionDb,
    entity_id: &str,
    window_days: u32,  // 90 days default
    min_meetings: u32, // 3 meetings minimum
) -> Result<Vec<CoAttendancePair>, DbError>
```

### 2. SQL Query

```sql
SELECT
    a1.person_id AS person_a,
    a2.person_id AS person_b,
    COUNT(DISTINCT a1.meeting_id) AS meeting_count,
    MAX(m.start_time) AS most_recent
FROM meeting_attendees a1
JOIN meeting_attendees a2
    ON a1.meeting_id = a2.meeting_id
    AND a1.person_id < a2.person_id  -- avoid duplicates and self-pairs
JOIN meetings m ON m.id = a1.meeting_id
JOIN meeting_entities me ON me.meeting_id = m.id AND me.entity_id = ?1
WHERE m.start_time >= datetime('now', '-' || ?2 || ' days')
GROUP BY a1.person_id, a2.person_id
HAVING COUNT(DISTINCT a1.meeting_id) >= ?3
ORDER BY meeting_count DESC
```

The `a1.person_id < a2.person_id` constraint ensures each pair is counted once (not A→B and B→A).

### 3. Confidence Scaling

Confidence scales with meeting frequency — more co-attendance = higher confidence that the relationship is real:

| Meetings Together (90d) | Confidence | Relationship Type |
|--------------------------|-----------|-------------------|
| 3 | 0.4 | `collaborator` |
| 4-5 | 0.5 | `collaborator` |
| 6-8 | 0.6 | `peer` |
| 9+ | 0.7 | `peer` |

Why `collaborator` vs `peer`: at lower frequency, people may be in the same meetings without directly working together (large team meetings). At higher frequency, they're more likely to be peers who regularly coordinate.

### 4. Persistence

For each `CoAttendancePair` above threshold:

```rust
let upsert = UpsertRelationship {
    id: &format!("pr-coatt-{}-{}", pair.person_a_id, pair.person_b_id),
    from_person_id: &pair.person_a_id,
    to_person_id: &pair.person_b_id,
    relationship_type: if pair.meeting_count >= 6 { "peer" } else { "collaborator" },
    direction: "symmetric",
    confidence: compute_coatt_confidence(pair.meeting_count),
    context_entity_id: pair.entity_id.as_deref(),
    context_entity_type: Some("account"),
    source: "co_attendance",
};
db.upsert_person_relationship(&upsert)?;
```

Deterministic ID format `pr-coatt-{a}-{b}` ensures re-computation reinforces existing edges (updates `last_reinforced_at`) rather than creating duplicates.

### 5. Guard Against Overwriting

Same pattern as I504:
- Don't create co-attendance edges where user-confirmed relationships exist (user's judgment takes precedence)
- Don't create co-attendance edges where AI-inferred edges with higher confidence exist (the AI may have identified a more specific relationship type like `manager`)
- DO create co-attendance edges alongside Glean-sourced edges (complementary evidence from different sources)

### 6. When to Run

Two options:

**Option A: During entity enrichment** — run `compute_co_attendance()` in `intel_queue.rs` before the LLM prompt, alongside the health scoring computation (I499). The co-attendance data feeds into the Stakeholder Coverage dimension.

**Option B: Background task** — run periodically (daily, after calendar sync) across all entities. This avoids adding latency to the enrichment pipeline.

Recommend **Option A** for v1.1.0 — keeps it simple and ensures co-attendance is fresh before health scoring runs. The SQL query is fast (indexed meeting_attendees, bounded window).

### 7. Decay Behavior

Co-attendance relationships naturally decay via the existing 90-day half-life on `effective_confidence()` in person_relationships. If two people stop meeting together, their co-attendance edge fades without any explicit removal logic.

Re-computation during enrichment resets `last_reinforced_at` for pairs that are still co-attending, keeping active relationships fresh.

## Files to Modify

| File | Change |
|---|---|
| `src-tauri/src/intelligence/relationships.rs` (NEW) | `compute_co_attendance()` function with SQL query, confidence scaling, pair generation. **Module decision:** Use a new `relationships.rs` module, not `health.rs`. Health scoring and relationship inference are different concerns. I504's AI-inferred relationships could also live here in the future. Add `pub mod relationships;` to `intelligence/mod.rs`. |
| `src-tauri/src/intel_queue.rs` | Call `compute_co_attendance()` during enrichment, before health scoring. Persist results to person_relationships. |
| `src-tauri/src/db/person_relationships.rs` | Ensure `upsert_person_relationship()` handles `source = "co_attendance"` correctly. |

## Acceptance Criteria

1. Two people who've attended 3+ meetings together in 90 days for the same account have a `person_relationships` row with source `"co_attendance"`.
2. Confidence scales: 3 meetings = 0.4, 6 meetings = 0.6, 9+ meetings = 0.7.
3. Relationship type: `"collaborator"` for 3-5 meetings, `"peer"` for 6+.
4. Direction is `"symmetric"` — co-attendance is bidirectional.
5. Re-computation during enrichment updates `last_reinforced_at` for active pairs — no duplicate rows.
6. User-confirmed relationships are NOT overwritten (same guard as I504).
7. Co-attendance edges appear in person detail Network chapter with "inferred from meetings" label.
8. A pair that stops meeting together naturally decays via 90-day half-life — no explicit cleanup needed.
9. SQL query runs in < 100ms for accounts with up to 50 stakeholders and 200 meetings.

## Out of Scope

- AI-inferred relationships (I504 — LLM-based, different confidence model)
- Glean structural relationships (I505 — org chart data, different source)
- Cross-entity co-attendance (two people meeting across multiple accounts) — scoped to single entity context for v1.1.0
- **Weighted co-attendance / meeting size filtering** — a 50-person all-hands generates 1,225 pairs, most of which are noise. For v1.1.0, the simple count is sufficient because the `min_meetings` threshold (3+) naturally filters out most noise: two people attending the same all-hands 3 times is weak evidence. Future enhancement: weight by meeting size (1:1 = 3x, <5 people = 2x, >10 people = 0.5x) or exclude meetings above a size threshold entirely.
- Internal-only meeting filtering — co-attendance counts all meetings regardless of internal/external mix
