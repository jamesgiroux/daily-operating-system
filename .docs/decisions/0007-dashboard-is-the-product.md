# ADR-0007: Dashboard is the product

**Date:** 2026-02
**Status:** Accepted

## Context

The app has multiple pages (dashboard, actions, inbox, settings, week, focus, emails). Need to decide the information hierarchy.

## Decision

The dashboard is the primary surface — 80% of user time is spent here. Meetings, actions, emails, and focus all render on the dashboard. Other pages are drill-downs or supporting views, not peers.

## Consequences

- Dashboard gets the most design and performance attention
- Other pages are secondary — they exist to expand what the dashboard summarizes
- Risk: dashboard becomes overloaded. Mitigated by density-aware design (see ADR-0034).
- Rejected: Equal-weight pages (spreads attention, loses focus)
