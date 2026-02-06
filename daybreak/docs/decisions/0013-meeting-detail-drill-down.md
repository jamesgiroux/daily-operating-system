# ADR-0013: Meeting detail is a drill-down, not a nav item

**Date:** 2026-02
**Status:** Accepted

## Context

Meeting prep detail needs a home. Could be a sidebar page or a drill-down from the dashboard.

## Decision

Accessed by clicking meeting cards on the dashboard. Back button returns to dashboard. No sidebar entry needed. Route: `/meeting/$prepFile`.

## Consequences

- Keeps sidebar clean (ADR-0010)
- Meeting detail is contextual — you arrive at it from a specific meeting card
- Requires meeting cards to actually link to the detail page (see backlog I14)
