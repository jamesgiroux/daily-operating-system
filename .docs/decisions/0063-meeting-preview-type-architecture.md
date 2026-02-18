# ADR-0063: Meeting Preview Type Architecture

**Status:** Accepted
**Date:** 2026-02-11
**Relates to:** I72 (Account Dashboards), I181 (Meeting Intelligence Persistence)

## Context

The Account Detail page needs richer meeting history cards that include prep context previews (intelligence summary, agenda items, risk/action counts). The existing `MeetingSummary` type (4 fields: id, title, start_time, meeting_type) is used across 8+ callsites for entity detail pages (accounts, projects, people) and is intentionally lightweight.

Adding `prep_context` to `MeetingSummary` would:
- Fetch 5KB+ JSON per meeting for every entity detail page, even when not displayed
- Change the type contract for all consumers
- Require a wider SQL SELECT for all meeting queries

## Decision

Create a separate `MeetingPreview` type with optional `prep_context` field. Keep `MeetingSummary` minimal.

**New type:**
```rust
pub struct MeetingPreview {
    pub id: String,
    pub title: String,
    pub start_time: String,
    pub meeting_type: String,
    pub prep_context: Option<PrepContext>,
}
```

**New query:** `get_meetings_for_account_with_prep()` — 12-column SELECT including `prep_context_json`, used only for account detail.

**Unchanged:** `MeetingSummary` and all existing callsites (projects, people, pickers).

## Consequences

- **Zero performance impact** on Projects/People detail pages (continue using 11-column SELECT)
- **Clear semantics:** `MeetingSummary` = fast summary, `MeetingPreview` = richer data with cost
- **Backward compatible:** No changes to existing callsites
- **Type safety:** Function signatures signal data cost
- **Query cost:** +1 TEXT column × 10 rows ≈ 50KB additional. Negligible for local SQLite (<1ms)
