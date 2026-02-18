# ADR-0062: Briefing Artifacts vs. Live Queries — Data Paradigm Boundary

**Date:** 2026-02-10
**Status:** Accepted
**Deciders:** James, Claude

## Context

DailyOS has two data paradigms coexisting without an explicit boundary:

1. **Briefing artifacts** — `_today/data/*.json` (schedule.json, actions.json, emails.json, preps/*.json, week-overview.json, manifest.json). Produced during morning workflow as point-in-time snapshots. AI-enriched. Consumed as rendered documents by the frontend.

2. **Live state** — SQLite (meetings_history, actions, people, entities, person_entities, meeting_entities) and `AppState::calendar_events` (updated every 5 minutes by the Google Calendar poller). Reflects current reality.

The two paradigms overlap at the dashboard: `get_dashboard_data()` merges schedule.json (briefing enrichment) with live calendar events (current truth) via `calendar_merge.rs`. This works because the merge is explicit and the consumer (dashboard) expects both signals.

The problem surfaces in I178/I179/I180: the focus page needs "available time today" — a **live query** that depends on the current schedule, not the morning snapshot. I180 proposed putting computed fields (`gaps`, `availableMinutes`, `deepWorkBlocks`, `meetingLoad`) into schedule.json. This is wrong for three reasons:

1. **Staleness.** schedule.json is a morning snapshot. A meeting added at 11am would not update the computed fields until the next briefing run. Users would see stale capacity data.
2. **Paradigm violation.** Briefing artifacts are rendered documents — enriched by AI, consumed as-is, archived at end of day. Adding live-computed fields turns a document into a database that needs continuous maintenance.
3. **Rewrite cost.** Keeping computed fields fresh would require rewriting schedule.json on every calendar poll. This introduces race conditions (concurrent reads during write), invalidation complexity, and breaks the "produce once, consume as-is" contract.

The same confusion will recur for any feature that needs current-state awareness: action feasibility ("can I finish this given my remaining time?"), proactive suggestions ("you have a 90-minute gap — use it for X"), or capacity forecasting.

## Decision

**Briefing artifacts are rendered documents, not live databases. Features needing current state compute from the live layer via query functions.**

### The Boundary

| Need | Source | Pattern |
|------|--------|---------|
| AI-enriched content (narrative, talking points, summaries, prep context) | Briefing artifacts (`_today/data/*.json`) | Read file |
| Current meeting list, timing, attendance | Live layer (SQLite + `AppState::calendar_events`) | Query function |
| Computed time awareness (available time, meeting load, deep work blocks) | Live layer | Query function |
| Current action state (overdue, due today, completion) | SQLite `actions` table | Query function |
| Entity intelligence (assessment, risks, wins) | Entity files (`intelligence.json`) + SQLite cache | Read file + query |

### Query Module

Time-aware and state-aware computations live in a query module:

```
src-tauri/src/queries/
├── mod.rs
├── schedule.rs    // day_schedule(), available_blocks(), meeting_load()
└── capacity.rs    // action_feasibility(), deep_work_potential()
```

Query functions are **pure** — they take references to live data and return computed structs. No internal state, no caching, no file I/O. Any Tauri command can call them.

```rust
// Example: schedule.rs
pub fn day_schedule(
    live_events: &[CalendarEvent],
    work_hours: (NaiveTime, NaiveTime),
) -> DaySchedule {
    // Compute from live calendar events, not from schedule.json
    DaySchedule {
        meetings: ...,
        total_meeting_minutes: ...,
        available_blocks: ...,    // gaps >= 30 min
        deep_work_blocks: ...,    // gaps >= 60 min
        density: ...,             // light/moderate/busy/packed
        meeting_load_pct: ...,
    }
}
```

### Why Not Cache?

For a single-user desktop app with <50 daily meetings, computing available time from a sorted vec is microseconds. Caching adds invalidation complexity (calendar poll changes, user edits) with zero perceptible latency benefit. If profiling later shows this matters, caching can be added behind the same function signatures without changing consumers.

### Why Not Services?

Formal "services" (ScheduleService, ActionService) with interfaces, lifecycle management, and dependency injection would be abstraction theater for a single-user app. Query functions are simpler, testable, and composable. If the system grows to need service-level concerns (rate limiting, circuit breaking, multi-tenant isolation), the functions can be wrapped — but that day is not today.

### Relationship to Existing Patterns

- **`get_dashboard_data()`** continues merging briefing + live calendar for the dashboard. It's a consumer of both paradigms and that's correct — the dashboard shows enriched content (briefing) overlaid on current timing (live).
- **`calendar_merge.rs`** remains the merge layer. Query functions consume the same `CalendarEvent` data that the merger uses.
- **`deliver_schedule()`** continues producing schedule.json as a briefing document. No changes to the delivery pipeline.
- **Week overview** (`build_day_shapes()` in orchestrate.rs) already computes gap/density data during weekly prep. This is correct — weekly briefing is a document, computed at generation time. The query module handles intra-day freshness for the daily view.

## Consequences

**Easier:**
- I178 (available time) unblocked — implement as `queries::schedule::available_blocks()` consuming live calendar, not schedule.json.
- I179 (action prioritization) can compose `queries::capacity::action_feasibility()` with action data from SQLite.
- Future live features (capacity forecasting, proactive suggestions, time blocking) have a clear pattern.
- Briefing artifacts stay simple, archivable, and AI-enrichable without concern for freshness.
- Testing is straightforward — pure functions with mock inputs.

**Harder:**
- Two code paths for "schedule data": briefing (schedule.json, for AI content) and live (query functions, for current state). Developers must know which to use. The boundary table above is the guide.
- Dashboard already merges both paradigms. Adding a third consumer (focus page) that uses query functions but not briefing artifacts may feel inconsistent — but it's correct because the focus page needs live truth, not morning enrichment.

**Trade-offs:**
- Accepting that briefing content goes stale is a feature, not a bug. "Your morning briefing said X" is a valid statement even if the schedule has since changed. The briefing is a document you received; the live query is the current reality. Both are useful.
- No caching means every focus page render recomputes from live events. For <50 events this is negligible. If it becomes measurable, cache with calendar-poll invalidation.

**Supersedes:** I180's proposal to enrich schedule.json with computed fields. I180 is reframed: the architectural decision is this ADR; the implementation work is I178 (available time query) and I179 (action feasibility query).
