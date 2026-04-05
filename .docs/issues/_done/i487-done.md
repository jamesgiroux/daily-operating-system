# I487 — Glean signal emission

**Priority:** P1
**Area:** Backend / Connectors + Signals
**Version:** v1.0.0 (Phase 2)
**Depends on:** ADR-0098 (purge-on-revocation infrastructure)

> **Revised 2026-02-28:** Removed feedback loop claim (moved to I507). Added signal volume governance, ADR-0098 purge compliance, and I505 delineation. Original spec overstated feedback loop — see I507 for the real gap.

## Problem

Glean data produces zero signals in the signal bus. The signal weight infrastructure already supports Glean — in `signals/bus.rs`, `source_base_weight()` defines:

```rust
"glean" | "glean_search" | "glean_org" => 0.7,
```

And `default_half_life()` defines:

```rust
"glean" | "glean_search" | "glean_org" => 60,  // 60-day half-life
```

These weight and decay definitions exist but are never used because no code path emits signals with source `"glean"`, `"glean_search"`, or `"glean_org"`.

The result: Glean context contributes to intelligence prompts but produces no durable signals. The signal bus — which drives prep invalidation, health scoring (ADR-0097 signal momentum dimension), and propagation rules — has zero visibility into what Glean provides. When Glean surfaces a new document about an account, no signal is emitted, no meeting prep is invalidated, and no health score recalculation is triggered.

## Design

### 1. Emit `glean_document` signals for NEW document results only

In `glean.rs`, after each Glean search query returns results (inside `gather_glean_context()`, ~line 394–419), emit a signal per **new** document result — not every result on every enrichment cycle.

**Why new-only:** Glean can return 30+ document snippets per account. Emitting all of them on every enrichment cycle would flood `signal_events` and dominate the health engine's signal momentum dimension (I499). Signal momentum should reflect actual changes in information flow, not the static size of Glean's document corpus for an account.

**Implementation:** Before emitting, check if a `signal_events` row already exists for this entity + signal_type `"glean_document"` + same URL (stored in `value`). Only emit if no prior signal exists for that URL, or if the Glean result's `updated_at` timestamp is newer than the existing signal's `created_at` (the document was actually updated since we last saw it).

**Why `updated_at` instead of a fixed 30-day window:** A fixed 30-day re-emit creates artificial signal bursts for stable accounts. If Glean returns the same 30 documents every cycle and they haven't changed, re-emitting them monthly inflates signal momentum for accounts with a large Glean document corpus. Using `updated_at` means signals only fire when documents actually change — which is the real information event. If `GleanSearchResult` doesn't expose `updated_at`, fall back to the 30-day window as a conservative default.

```rust
for result in &deduped_results {
    if result.snippet.is_some() {
        let url = result.url.as_deref().unwrap_or("");

        // Skip if document hasn't changed since our last signal
        // Prefer updated_at check; fall back to 30-day window if no timestamp available
        let updated_at = result.updated_at.as_deref();
        if db.has_signal_newer_than(entity_type, entity_id, "glean_document", url, updated_at, 30)? {
            continue;
        }

        let value = format!(
            "{}|{}|{}",
            result.title.as_deref().unwrap_or(""),
            result.doc_type.as_deref().unwrap_or(""),
            url,
        );
        let _ = crate::signals::bus::emit_signal(
            db,
            entity_type,
            entity_id,
            "glean_document",
            "glean_search",
            Some(&value),
            0.7,
        );
    }
}
```

Signal type: `"glean_document"` — a document found in Glean relevant to this entity.
Source: `"glean_search"` — matches the existing weight definition in `bus.rs`.
Value: pipe-delimited title, type, and URL for traceability and dedup.

### 2. Person signal emission: I505 owns, not I487

The original spec proposed `glean_person` signals emitted here. **This is now I505's responsibility.** I505 handles Glean contact discovery, profile enrichment, and entity linkage. Person-related signals should be emitted as part of that discovery/linkage flow, not during raw context gathering.

Rationale: I505 creates people records, links them to entities, and updates profiles — those are the meaningful events worth signaling. Emitting a signal per raw `GleanPersonResult` before any persistence happens produces signals with no durable referent. A `glean_person` signal should mean "we discovered or updated a person from Glean," not "Glean returned a name in a search result."

I487 emits **document signals only**. I505 emits person and relationship signals.

### 3. Use `emit_signal()` not `emit_signal_and_propagate()`

Inside `gather_glean_context()`, we are already within the enrichment pipeline. Using `emit_signal_and_propagate()` here would trigger propagation rules synchronously, potentially causing recursive enrichment. Use the simpler `emit_signal()` which writes to `signal_events` but defers propagation to the post-enrichment step in `intel_queue.rs`.

The signals will still drive prep invalidation and health scoring because:
- `apply_enrichment_results()` in `intel_queue.rs` already calls `mark_reports_stale()` after writing intelligence
- The health engine (I499) reads `signal_events` rows during scoring — new Glean document signals contribute to the signal momentum dimension
- Future enrichment cycles will see these signals in the entity's signal history

### 4. Document signal dedup query

Add a helper to `db/signals.rs` (or wherever signal queries live):

```rust
/// Check if a signal already exists for this document.
/// If `updated_at` is provided, check if our signal is newer than the document's last update.
/// If `updated_at` is None, fall back to max_age_days window.
pub fn has_signal_newer_than(
    &self,
    entity_type: &str,
    entity_id: &str,
    signal_type: &str,
    value_contains: &str,  // URL substring match
    updated_at: Option<&str>,  // document's updated_at timestamp from Glean
    max_age_days: i32,  // fallback if no updated_at
) -> Result<bool, DbError>
```

Logic:
1. If `updated_at` is Some: check if a signal exists with `created_at >= updated_at` (document hasn't changed since our signal)
2. If `updated_at` is None: fall back to checking `created_at >= datetime('now', '-N days')`

This prevents re-emitting signals for unchanged documents while still catching actual document updates.

### 5. ADR-0098 compliance: purge-on-revocation

Per ADR-0098, when Glean auth is revoked, all Glean-sourced data must be purged. This includes signal_events rows.

The `purge_source("glean")` function (defined in ADR-0098, implemented in `data_lifecycle.rs`) must include:

```sql
DELETE FROM signal_events WHERE source IN ('glean', 'glean_search', 'glean_org');
```

This is part of ADR-0098's infrastructure, not I487's code — but I487 creates the data that needs purging. The acceptance criteria below verify that purge works correctly for these signals.

### 6. Health scoring interaction (ADR-0097)

Glean document signals feed into the health engine's **signal momentum** dimension (I499). Signal momentum measures "is new information flowing in about this account?" — Glean documents are one signal source among many (email, calendar, user corrections).

The new-only dedup (§1) prevents Glean from dominating signal momentum. On a stable account where the same 30 Glean documents appear every cycle, zero new signals are emitted. On an account where Glean surfaces a new competitive analysis or QBR deck, one signal fires — appropriately bumping signal momentum.

No special handling needed in the health engine for Glean signal types. `signal_events` rows are consumed generically by I499's momentum calculation.

## Files to Modify

| File | Change |
|---|---|
| `src-tauri/src/context_provider/glean.rs` (~line 394) | After Glean search returns deduped results, check `has_recent_signal()` per URL, emit `"glean_document"` signal for new documents only. Source: `"glean_search"`, confidence: 0.7. |
| `src-tauri/src/context_provider/glean.rs` (imports) | Add `use crate::signals::bus::emit_signal;` |
| `src-tauri/src/db/signals.rs` (or appropriate query module) | Add `has_recent_signal()` helper for dedup check. |

## Acceptance Criteria

1. After enrichment with Glean connected, `SELECT COUNT(*) FROM signal_events WHERE source = 'glean_search'` > 0 for entities with Glean document results
2. Signal type is `"glean_document"` with source `"glean_search"`, confidence 0.7
3. **New-only**: Re-enriching the same account without new Glean documents produces zero new `glean_document` signals (dedup via URL + 30-day window)
4. **New documents surface**: Adding a new document to Glean's index for an account → next enrichment emits a signal for it
5. Signal weight 0.7 with 60-day half-life (matching existing definition in `signals/bus.rs`)
6. `emit_signal()` used (not `emit_signal_and_propagate()`) — verify no recursive enrichment triggers
7. After Glean auth revocation + `purge_source("glean")`, `SELECT COUNT(*) FROM signal_events WHERE source IN ('glean_search', 'glean_org')` = 0 (ADR-0098 compliance)
8. Health engine signal momentum (I499) reflects Glean document signals — account with new Glean docs has higher momentum than stable account

## Out of Scope

- **Person signal emission** — I505 owns `glean_person` / `glean_org` signals as part of contact discovery and entity linkage
- **Feedback loop** — the current feedback system doesn't attribute intelligence corrections to specific sources like Glean. This is I507's scope (source-attributed correction feedback)
- **`emit_signal_and_propagate()`** from inside `gather_entity_context()` — would cause recursive enrichment
- **New propagation rules for Glean signal types** — the existing propagation engine handles new signal types generically
- **Glean change notifications or webhooks** — Glean MCP is on-demand search only, not a push channel
- **Cross-enrichment signal dedup beyond 30-day window** — signal decay handles older signals naturally via half-life
