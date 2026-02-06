# ADR-0033: Meeting entity unification

**Date:** 2026-02
**Status:** Proposed

## Context

Meetings exist in three independent forms with no shared state:
- Daily dashboard card (`schedule.json`) — no state tracking
- Weekly grid cell (`week-overview.json`) — decorative prep badge
- Meeting detail page (`preps/*.json`) — full content, orphaned (no entry point)

Marking prep as "reviewed" on daily view has no effect on weekly view. No shared lifecycle.

## Decision

Near-term: Shared ID mapping (option 3). Keep independent data sources but add a lookup table mapping calendar event IDs across views. A "prep reviewed" flag in SQLite can be queried by any view.

Long-term (Phase 3): Unified Meeting entity in SQLite with stable ID (calendar event ID), state (prep status, notes, outcomes), and lifecycle. All views read from the same source.

## Consequences

- Near-term: Fixes backlog I14 (meeting card links), enables prep tracking, minimal refactoring
- Long-term: Consistent meeting experience everywhere, but significant architectural change
- Depends on ADR-0032 (calendar source of truth determines the ID scheme)
