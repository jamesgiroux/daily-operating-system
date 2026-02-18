# ADR-0031: Actions: SQLite as working store, markdown as archive

**Date:** 2026-02
**Status:** Accepted

## Context

Actions exist in multiple places: SQLite (`actions` table), daily JSON snapshot (`actions.json`), and source markdown files. No single source of truth — completing an action in the app updates SQLite but not the source markdown.

## Decision

| Store | Role | Lifetime |
|-------|------|----------|
| SQLite | Working store. App reads/writes. Fast queries. | Persistent across days, disposable (ADR-0018) |
| actions.json | Daily snapshot for dashboard. Read-only. | Ephemeral — regenerated each briefing |
| Markdown files | Historical record. Source of new action extraction. | Persistent — user-owned |

No single `master-task-list.md`. Actions are scoped to their source (meeting, email, manual). The Actions page aggregates across sources.

Action completion flow: user clicks checkbox → SQLite `complete_action()` sets status + timestamp → `upsert_action_if_not_completed()` prevents re-briefing from overwriting user completions.

Post-meeting captured actions go directly to SQLite with `source_type = "post_meeting"`.

Post-enrichment hooks should write completion markers (`[x]`) back to source markdown when `source_label` points to a specific file. This writeback is not yet implemented.

## Consequences

- Fast action management in the app (SQLite queries, optimistic UI)
- Actions persist across briefing cycles — completion status survives re-briefing
- Within-day deduplication in `deliver_today.py` via seen-ID set
- If SQLite is deleted, in-progress status is lost — markdown writeback would mitigate this
- Open gaps: markdown writeback hooks, cross-briefing deduplication in `prepare_today.py`, manual action creation from UI
