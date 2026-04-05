# ADR-0055: Schedule-First Dashboard Layout

**Date:** 2026-02-08
**Status:** Accepted
**Supersedes:** Layout portions of [ADR-0053](0053-dashboard-ux-redesign.md) (component decisions in 0053 remain accepted)

## Context

ADR-0053 shipped the readiness strip, sidebar flip, greeting removal, and maxVisible improvements. But the daily briefing page still buries the meeting schedule behind 440-560px of preamble (date heading, focus callout, summary card, readiness strip, intelligence card). On a 13" MacBook, the schedule can be completely below the fold on a loaded day.

The schedule is the star content — it's what users check first every morning. Pushing it below the fold forces scrolling before value delivery, violating Principle 7 (Consumption Over Production) and Principle 2 (Prepared, Not Empty).

## Decision

Restructure the daily briefing page from a sequential top-to-bottom flow to a two-column layout:

**Left column (5fr):** Date heading + MeetingTimeline immediately. The schedule starts at pixel one after the heading — no preamble.

**Right column (2fr):** Context sidebar stacking Focus, IntelligenceCard, ActionList, EmailList.

Key changes:
- **Summary card eliminated.** Every fact it states (meeting count, action count) is visible elsewhere in the meeting cards and action list. It consumed 120px narrating the interface.
- **ReadinessStrip eliminated.** All readiness information is already co-located with its source: prep coverage shows on MeetingCard badges, overdue actions show in the ActionList, next meeting is the top of the timeline. The strip was redundant aggregation.
- **Cancelable badge moved to MeetingCard.** Internal/team_sync meetings without prep get a "Cancelable" badge directly on the card, removing the need for the IntelligenceCard's "Cancel / Protect" section. Cancelable signals excluded from IntelligenceCard's totalSignals count so the card hides correctly when cancelable was the only signal type.
- **Overview component deleted.** Date heading and focus callout move directly into Dashboard.tsx. No intermediate wrapper.
- **Focus callout expanded.** No longer truncated — wraps naturally with `leading-relaxed`. Remains clickable (links to /focus page — I109 tracks building out the stub).
- **Responsive:** Below `lg` breakpoint, collapses to single column — schedule first, sidebar content below.

## Consequences

**Easier:**
- Schedule visible immediately on any screen size (above the fold on 13")
- Context elements are adjacent but secondary — visible without scrolling past them
- Sidebar is clean: focus + intelligence + actions + emails — no redundant aggregation
- Cancelable signal is co-located with the meeting it applies to

**Harder:**
- Sidebar width constrains component designs — ActionList, EmailList, IntelligenceCard must work in narrow widths (they already do from the previous 2fr sidebar)
- Summary narrative is gone — users who valued the AI-written overview sentence lose it

**Trade-offs:**
- More horizontal layout means less vertical breathing room between sections in the sidebar
- Below `lg`, the single-column fallback places schedule above sidebar content, which is correct priority-wise
