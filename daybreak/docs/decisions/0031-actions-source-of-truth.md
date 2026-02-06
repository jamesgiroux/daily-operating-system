# ADR-0031: Actions: SQLite as working store, markdown as archive

**Date:** 2026-02
**Status:** Proposed

## Context

Actions exist in multiple places: SQLite (`actions` table), daily JSON snapshot (`actions.json`), and source markdown files. No single source of truth — completing an action in the app updates SQLite but not the source markdown.

## Decision

| Store | Role | Lifetime |
|-------|------|----------|
| SQLite | Working store. App reads/writes. Fast queries. | Persistent across days, disposable (ADR-0018) |
| actions.json | Daily snapshot for dashboard. Read-only. | Ephemeral — regenerated each briefing |
| Markdown files | Historical record. Source of new action extraction. | Persistent — user-owned |

No single `master-task-list.md`. Actions are scoped to their source (meeting, email, manual). The Actions page aggregates across sources.

Post-enrichment hooks write completion markers (`[x]`) back to source markdown when a `source_label` points to a specific file.

## Consequences

- Fast action management in the app (SQLite)
- User's markdown files stay in sync via writeback hooks
- If SQLite is deleted, in-progress status is lost — markdown writeback mitigates this
- Deduplication needed: `prepare_today.py` must check SQLite before extracting from markdown
- Open: does manual action creation (from app UI) write to SQLite only or also create markdown?
