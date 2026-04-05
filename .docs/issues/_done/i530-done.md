# I530 — Signal Taxonomy: Curation vs Correction

**Priority:** P1
**Area:** Backend / Signals
**Version:** 1.0.0
**Depends on:** I529 (intelligence quality feedback UI — establishes explicit feedback path)

## Problem

The system treats every user deletion as a correction — penalizing the source that produced the content. But deletion has multiple meanings:

1. **"This is wrong"** — the intelligence is factually incorrect. The source should be penalized.
2. **"I don't need this right now"** — the intelligence is accurate but not relevant to the user's current focus. No source penalty.
3. **"This is redundant"** — the intelligence duplicates something else. No source penalty.
4. **"I'm simplifying"** — the user is curating for clarity or space. No source penalty.

Only case 1 should penalize the source. Cases 2-4 are curation — editorial decisions about what to show, not judgments about accuracy. The current behavior (all deletions penalize) means that a user who curates aggressively trains the system to produce less intelligence overall, which is the opposite of what they want.

### Current behavior

```
User deletes a risk item from entity intelligence
  → self_healing/feedback.rs::record_enrichment_correction()
  → "intel_queue" blamed (beta += 1)
  → Future enrichments produce fewer/weaker risk items for this entity type
```

### Desired behavior

```
User deletes a risk item (curation)
  → No source penalty. Item removed from display. System continues producing risk items at same quality level.

User thumbs-down a risk item (correction, via I529)
  → Source penalized (beta += 1). System learns this source is less reliable for risk assessment.

User edits a risk item (implicit correction)
  → Source penalized (beta += 1). System learns the original content was wrong.
```

## Design

### 1. Delete = curation (no signal penalty)

Change the delete behavior across all editable intelligence surfaces:

- **Entity detail page:** Deleting an intelligence item (risk, win, recommendation, etc.) removes it from the display and from the DB/intelligence record. No correction signal emitted. No source penalty.
- **Meeting prep:** Dismissing a prep topic removes it from the briefing. No correction signal.
- **Reports:** Reports are read-only (no deletion), so this doesn't apply.

The delete action should still emit a `user_curation` signal (not `user_correction`) for audit/analytics purposes, but with no source penalty:

```rust
emit_signal(
    "intelligence_curated",  // not "user_correction"
    "user_curation",         // source
    0.5,                     // neutral confidence — informational only
);
// NO call to upsert_signal_weight(). NO call to record_enrichment_correction().
```

### 2. Edit = correction (source penalized)

This behavior stays the same. When a user rewrites content, the original was wrong. The source that produced it should be penalized. `record_enrichment_correction()` continues to fire on edits.

### 3. Thumbs down = correction (source penalized, via I529)

I529's thumbs-down already handles this. The user explicitly says "this is wrong" without needing to edit or delete. This is the primary correction path.

### 4. Signal type taxonomy

| User Action | Signal Type | Source Penalty | Why |
|---|---|---|---|
| Thumbs up (I529) | `intelligence_confirmed` | alpha += 1 (reward) | User says "this is correct" |
| Thumbs down (I529) | `intelligence_rejected` | beta += 1 (penalize) | User says "this is wrong" |
| Edit content | `user_correction` | beta += 1 (penalize) | User rewrites = original was wrong |
| Delete content | `intelligence_curated` | none | User curates = editorial decision, not quality judgment |
| Dismiss prep topic | `prep_topic_dismissed` | none | User says "not relevant right now" |

### 5. Migration path

The change is behavioral, not schema. The `signal_events` table already supports any `signal_type` string. The change is in the code paths:

- `self_healing/feedback.rs::record_enrichment_correction()` — only called on **edits**, not deletes
- Delete handlers in `services/accounts.rs`, `services/intelligence.rs` — emit `intelligence_curated` instead of calling `record_enrichment_correction()`
- Prep dismissal in meeting detail — emit `prep_topic_dismissed` (or continue current behavior if it already doesn't penalize)

## Files to Modify

| File | Change |
|---|---|
| `src-tauri/src/self_healing/feedback.rs` | Audit all callers. Ensure `record_enrichment_correction()` is only called on edits (content changed), never on deletes (content removed). |
| `src-tauri/src/services/accounts.rs` | Delete handlers emit `intelligence_curated` signal instead of correction. |
| `src-tauri/src/services/intelligence.rs` (post-I512) | Intelligence field deletion emits curation signal, not correction. |
| `src-tauri/src/commands.rs` | Any delete command that currently calls `record_enrichment_correction()` stops doing so. |

## Acceptance Criteria

1. Delete an intelligence item (risk, win, recommendation). `signal_events` has a `intelligence_curated` row. `signal_weights` is unchanged — no source penalized.
2. Edit an intelligence item (change the text). `signal_events` has a `user_correction` row. `signal_weights` updated — source penalized (beta += 1).
3. Thumbs-down an intelligence item (I529). `signal_events` has a `intelligence_rejected` row. `signal_weights` updated — source penalized (beta += 1).
4. Thumbs-up an intelligence item (I529). `signal_events` has a `intelligence_confirmed` row. `signal_weights` updated — source rewarded (alpha += 1).
5. Delete 10 intelligence items across 5 accounts. `signal_weights` table unchanged — system does not learn to produce less intelligence.
6. After 5+ thumbs-down on Glean-sourced risks, `get_learned_reliability("glean", "account", "risk")` returns < 0.5.

## Out of Scope

- Undo for deletions (restoring curated-out content)
- "Why did you remove this?" prompt (asking user to categorize their deletion reason)
- Curation analytics (tracking what users curate most to improve relevance)
- Distinguishing "wrong" from "irrelevant" in thumbs-down (both penalize the source equally)

## Relationship to I507 and I529

- **I507** covers source-attributed corrections for person profiles and email disposition. I530 does not change I507's scope — those are edit-based corrections that correctly penalize sources.
- **I529** adds the thumbs up/down UI. I530 defines the signal taxonomy that makes thumbs up/down meaningful alongside edit and delete.
- Together: I507 (backend correction routing) + I529 (explicit feedback UI) + I530 (signal taxonomy) form the complete feedback system.
