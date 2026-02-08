# ADR-0054: List Page Design Pattern — Signal-First Flat Rows

**Date:** 2026-02-08
**Status:** Accepted

## Context

AccountsPage and PeoplePage both use a card-per-row pattern: each list item is a `Card` component with `p-4` padding, border, shadow, and a `hover:-translate-y-0.5 hover:shadow-md` lift animation. Each row leads with a 40px avatar circle showing a single letter initial.

This creates three problems:

1. **Low information density.** Each row consumes ~70-80px of vertical space. A CSM with 20 accounts or 50+ people must scroll to see their full list. The design system principle P7 (Consumption Over Production) demands that scanning be fast — a user should see their entire book of business in one or two viewports.

2. **Signal hierarchy is inverted.** The largest visual element (40px avatar initial) carries the least information (a single letter). The most actionable signals — health status for accounts, temperature/trend for people — are either small inline badges or completely absent. The eye lands on decoration first, data second.

3. **Wrong interaction metaphor.** Card-with-hover-lift is a consumer/e-commerce pattern (product cards, app grids). It implies each item is a self-contained object to inspect individually. For a daily working list, the metaphor should be a scannable table — closer to macOS Finder list view or Mail.app than a Shopify storefront.

## Decision

List pages (AccountsPage, PeoplePage, and future entity list pages) adopt a **flat row pattern with signal-first hierarchy**:

### Row Pattern

- **No card wrapper.** Rows are `div` elements separated by a `border-b border-border` bottom border.
- **Hover state** is a background color change (`hover:bg-muted/50`), not a transform/shadow.
- **Row height** targets 44-52px (roughly half of the current card height).
- **No avatar initials.** The leading visual element is a signal indicator (health dot, temperature dot) that uses design system colors directly.

### Signal Hierarchy (left to right)

1. **Signal indicator** (leading) — small colored dot or icon encoding the most actionable state (health for accounts, temperature for people). Uses design system colors: sage for healthy/hot, gold for caution/warm, muted for cool, peach for warning/cold/red.
2. **Name** — primary identification, `font-medium`.
3. **Badges/metadata** — inline after name. Minimal — one badge max per row (lifecycle for accounts, relationship for people on "All" tab only).
4. **Secondary context** — org/role, CSM, etc. Smaller, muted.
5. **Numeric data** (right-aligned) — in JetBrains Mono for vertical alignment. ARR for accounts, meeting count for people.
6. **Temporal signal** (right-aligned) — days since last meeting / last seen. Color-coded when stale (peach when >14d).

### Shared Component

A `ListRow` or similar shared primitive encapsulates the flat row pattern (border, hover, padding, leading signal slot) so AccountsPage and PeoplePage stay consistent without duplicating layout code.

## Consequences

### Easier
- Scanning 20+ accounts or 50+ people without excessive scrolling
- Spotting at-risk accounts or cold contacts at a glance (signal is the first thing the eye hits)
- Maintaining visual consistency across list pages (shared row primitive)
- Aligning with macOS native list patterns

### Harder
- Individual rows are less visually "rich" — less obvious as clickable objects (mitigated by cursor and hover state)
- Future list pages must adopt this pattern for consistency (but that's a feature, not a bug)

### Trade-offs
- The card pattern is more visually appealing in screenshots with 3 items. The flat pattern is more useful in daily use with 20+ items. We optimize for daily use.
- Avatar initials provide a small amount of visual variety. Signal dots provide more information per pixel. We trade decoration for data.
