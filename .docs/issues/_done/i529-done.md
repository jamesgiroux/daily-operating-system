# I529 — Intelligence Quality Feedback UI (Thumbs Up/Down)

**Priority:** P1
**Area:** Frontend / Backend / Signals
**Version:** 1.0.0
**Depends on:** I507 (source-attributed correction feedback — Bayesian engine wiring), I513 (DB as sole source — all intelligence reads from DB)

## Problem

Users have no way to give quick quality feedback on intelligence without editing or deleting it. When a report shows "Champion is Sarah Chen" and that's correct, the system never learns it got that right. When a risk assessment is wrong but the user doesn't want to rewrite it (they just want to move on), the system never learns it got that wrong either.

The only feedback path today is implicit: user edits a field → `record_enrichment_correction()` penalizes the source. But editing is a heavy action. And deletion conflates two meanings: "this is wrong" vs "I don't need this right now." The system treats all deletions as corrections, penalizing sources even when the content was accurate but unwanted.

### What we need

A lightweight, hover-triggered thumbs up/down on any intelligence item displayed in the app — report sections, briefing content, entity insights, meeting prep topics. One click = one signal. No editing required.

- **Thumbs up** → confirmation signal (alpha += 1 for the source)
- **Thumbs down** → correction signal (beta += 1 for the source)

This is the simplest feedback loop: the user tells the system "good" or "bad" and the Bayesian weights learn.

## Design

### 1. Feedback target model

Every piece of intelligence that can receive feedback needs an identifier. The feedback target is:

```typescript
interface FeedbackTarget {
  entityId: string;
  entityType: 'account' | 'project' | 'person' | 'user';
  field: string;           // e.g., "risks[0]", "strategic_assessment", "champion_status"
  source?: string;         // if known: "glean", "ai_enrichment", "email", etc.
  context?: string;        // the text content being rated (for audit trail)
}
```

### 2. Frontend component

```typescript
// IntelligenceFeedback.tsx
// Appears on hover over any intelligence item
// Two buttons: thumbs up (confirm) / thumbs down (reject)
// After clicking either, shows the selected state (filled icon) and fades to subtle
// Clicking the same button again un-does (removes the feedback)
// Clicking the opposite button switches the vote
```

**Placement rules:**
- Report sections: each major section (strategic assessment, risk items, value narrative) gets a feedback target
- Briefing content: each prep topic / intelligence snippet gets a feedback target
- Entity detail: each intelligence dimension summary gets a feedback target
- Meeting detail: each prep section gets a feedback target

**Interaction:**
- Hidden by default, appears on hover (desktop) or long-press (future mobile)
- Minimal visual footprint — does not disrupt reading flow
- Feedback state persists (user sees their previous votes on return)

### 3. Backend signal emission

```rust
// New command: submit_intelligence_feedback
// Parameters: entity_id, entity_type, field, feedback ("positive" | "negative"), source (optional)

pub fn submit_intelligence_feedback(
    entity_id: &str,
    entity_type: &str,
    field: &str,
    feedback: &str,  // "positive" | "negative"
    source: Option<&str>,
    context: Option<&str>,
) {
    // 1. Record in intelligence_feedback table (audit trail)
    // 2. If source is known:
    //    - positive: upsert_signal_weight(source, entity_type, field_category, alpha+=1, beta+=0)
    //    - negative: upsert_signal_weight(source, entity_type, field_category, alpha+=0, beta+=1)
    // 3. Emit signal: "intelligence_feedback" with confidence based on direction
    //    - positive: 0.8 (user confirms)
    //    - negative: 0.3 (user rejects)
}
```

### 4. Storage

New table for feedback audit trail:

```sql
CREATE TABLE intelligence_feedback (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_id TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    field TEXT NOT NULL,
    feedback TEXT NOT NULL CHECK (feedback IN ('positive', 'negative')),
    source TEXT,              -- which source produced this intelligence (if known)
    context TEXT,             -- the text content being rated
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(entity_id, entity_type, field)  -- one vote per field per entity
);
```

The UNIQUE constraint means changing your vote replaces the previous one (INSERT OR REPLACE).

### 5. Source identification

The challenge: when displaying a risk item on a report, how do we know which source produced it? Options:

- **I508 `source_attribution`** — deferred post-v1.0.0, so not available
- **Coarse attribution** — if the entity was enriched via Glean, attribute Glean-dependent fields to `"glean"`. If enriched via PTY only, attribute to `"ai_enrichment"`. This is imprecise but better than nothing.
- **No attribution** — record the feedback without a source. The feedback still has value as a quality metric, even if it can't penalize a specific source. When I507's source attribution ships post-v1.0.0, historical feedback can be retroactively attributed.

**Recommendation:** Ship with coarse attribution (option 2) for v1.0.0. The `enrichment_sources` JSON on entity records already tracks which sources contributed. Use the most recent enrichment source for the entity as the attributed source. Record `source = null` when attribution is ambiguous.

## Files to Modify

| File | Change |
|---|---|
| `src/components/ui/IntelligenceFeedback.tsx` | **New file.** Hover-triggered thumbs up/down component. Takes `FeedbackTarget` props. Calls `submit_intelligence_feedback` command. |
| `src-tauri/src/commands.rs` (or `services/signals.rs` post-I512) | New `submit_intelligence_feedback` command. Records feedback, emits signal, updates weights. |
| `src-tauri/src/migrations/` | New migration: `intelligence_feedback` table. |
| `src-tauri/src/db/` | New module or extension for intelligence feedback CRUD. |
| Report renderer components | Add `IntelligenceFeedback` wrapper to each report section. |
| Meeting prep components | Add `IntelligenceFeedback` wrapper to prep topics/snippets. |
| Entity detail components | Add `IntelligenceFeedback` wrapper to intelligence dimension summaries. |

## Acceptance Criteria

1. Hover over any intelligence item (report section, briefing topic, entity insight) shows thumbs up/down buttons
2. Clicking thumbs up records a positive feedback signal. Clicking thumbs down records a negative signal. Button shows selected state.
3. Changing vote (up → down or vice versa) replaces previous feedback — only one vote per field per entity
4. `intelligence_feedback` table has rows after user provides feedback
5. When source is identifiable (from `enrichment_sources`), `signal_weights` table updated: positive → alpha += 1, negative → beta += 1
6. Feedback does not disrupt reading flow — buttons appear on hover, minimal visual footprint
7. User's previous votes visible on return (persisted state)
8. No internal DailyOS terminology in the UI (no "signal", "source weight", "Bayesian")

## Out of Scope

- Feedback analytics dashboard (showing aggregated quality scores per source)
- Bulk feedback (rate an entire report as good/bad)
- Feedback on meeting prep relevance (dismissed topics → source penalization — listed in I507 out of scope)
- Explanation of why something was rated (free-text feedback)
- Mobile/touch interaction (long-press) — desktop hover only for v1.0.0

## Relationship to I507

I507 covers the backend wiring: making corrections blame the right source via provenance. I529 adds the **user-facing feedback surface** that I507 lacks. They share the same Bayesian engine (`signal_weights`, `upsert_signal_weight`). I507 handles implicit feedback (user edits a field → source penalized). I529 handles explicit feedback (user clicks thumbs up/down → source rewarded/penalized).

I507's "Positive feedback" out-of-scope item is absorbed by I529.
