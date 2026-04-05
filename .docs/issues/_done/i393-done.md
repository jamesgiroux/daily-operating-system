# I393 — Parent Account Detail Page — Portfolio Surface (Hotspots, Cross-BU Patterns, Portfolio Narrative)

**Status:** Open (0.13.3)
**Priority:** P1
**Version:** 0.13.3
**Area:** Frontend / UX

## Summary

Opening a parent account detail page (any account with child accounts) should show a qualitatively different page from a leaf-node account — a portfolio surface rather than a single-entity intelligence view. This issue adds a **Portfolio** chapter to parent account detail pages that renders from the `intelligence.json.portfolio` field produced by I384. The chapter shows: a portfolio-level health narrative, hotspot child accounts with one-line reasons sorted by urgency, cross-BU patterns (if present), and a condensed list of child accounts with name, type badge, and health indicator.

Leaf-node account pages are unchanged — the Portfolio chapter only appears when the account has children.

## Acceptance Criteria

From the v0.13.3 brief, verified with real data in the running app:

1. Opening a parent account detail page (any account with children) renders a distinct **Portfolio** chapter as the primary content section. This chapter does not appear on leaf-node account pages.
2. The Portfolio chapter contains:
   - **Health summary** — portfolio-level narrative from `intelligence.json.portfolio.portfolio_narrative`
   - **Hotspots** — list of child accounts with active risk or opportunity signals, with a one-line reason per child. Sorted by signal recency/urgency. Each hotspot links to that child's detail page.
   - **Cross-BU patterns** — signal types or topics appearing in 2+ children, surfaced as a distinct callout. If no cross-BU patterns exist, this section is hidden (not an empty state).
3. Below the Portfolio chapter, the existing account intelligence sections render (the parent's own signals, meetings, actions) — the same content a leaf-node account would show for its own entity.
4. The child account list renders as a condensed view within the Portfolio chapter — each child shown with its name, type badge, and health indicator. Clicking a child navigates to that child's detail page.
5. Meetings tagged directly to the parent appear in the parent's meeting section. Meetings tagged to child accounts do NOT appear in the parent's meeting list. Verify: the parent's meeting list contains only meetings where the parent's `id` is in `meeting_entities`, not meetings linked to any child.
6. The page uses the existing design system and editorial layout patterns. The Portfolio chapter uses the existing chapter/section rule treatment. Hotspots use the existing callout/signal prose component. No new design patterns are invented.

## Dependencies

- Blocked by I384 (portfolio intelligence) — the page renders from `intelligence.json.portfolio`; that section doesn't exist until I384 ships.
- Related to I385 (bidirectional propagation) — more meaningful hotspot data after child signals accumulate at parent.
- See ADR-0087 decisions 3 and 5.

## Notes / Rationale

The portfolio surface is what transforms DailyOS from "a better way to track accounts" to "executive intelligence for complex account hierarchies." A CS leader managing Crestview Media Enterprises shouldn't need to visit 10 BU pages to understand the portfolio — they should open the Crestview Media Enterprises page and see the portfolio view immediately: which BUs are healthy, which need attention, what pattern is emerging across the book. That's the surface this issue delivers.
