# I375 — Refresh Button Audit — Design Continuity with Add Buttons + Action/Surface Alignment

**Status:** Open (0.13.1)
**Priority:** P2
**Version:** 0.13.1
**Area:** UX / Code Quality

## Summary

Refresh buttons across the app have accumulated inconsistent visual treatment — different sizing, icon weight, hover behavior, and disabled-state treatment compared to other interactive controls on the same surface. Additionally, some refresh buttons trigger backend enrichment for sections that have been removed from the layout (orphaned calls). This issue is a design continuity and correctness audit of every refresh button in the app.

## Acceptance Criteria

From the v0.13.1 brief, verified in the running app:

1. Enumerate every refresh button (or icon trigger that re-fetches/re-enriches) across all pages: Daily Briefing, Week Forecast, Actions, Meeting Detail, Account Detail, Person Detail, Project Detail, Email page, Settings. List is confirmed complete before work begins.
2. Every refresh button is visually consistent with its page's add/action buttons — same sizing, icon weight, disabled-state treatment, and hover behavior. No page has a refresh button that uses a different design language than other interactive controls on the same surface.
3. Each refresh action targets only the data currently rendered on its page. A refresh on the Meeting Detail page does not re-enrich sections that have been removed from the layout. Verify by checking the invoked command or handler against the current page component — no orphaned enrichment calls.
4. Refresh buttons that trigger background work (async enrichment, AI pipeline) show a loading state for the duration. Clicking again while loading has no effect (button is disabled or re-entrant clicks are ignored).
5. No refresh button silently does nothing — if the action cannot be taken (e.g., already in progress, no data to refresh), it either shows a toast explaining why or the button is visibly disabled with a tooltip.

## Dependencies

- Independent of other v0.13.1 issues. Can be worked in parallel.
- Related to the "Refresh as Last Resort" design principle in ADR-0086 — refresh buttons are escape hatches, not the normal path.

## Notes / Rationale

Orphaned refresh calls are a correctness issue: if the Meeting Detail refresh button still triggers enrichment for sections that were removed in v0.13.0 (I342), it wastes AI budget on sections the user never sees. Visual inconsistency is a polish issue. Both are addressable in a single pass.
