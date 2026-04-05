# I550 — Account Detail Editorial Redesign: Margin Label Layout + Visual Storytelling

## Problem

The account detail page presents all intelligence as visually identical lists. There is no typographic hierarchy beyond the chapter heading, no visual differentiation between sections, and no sense of narrative progression as you scroll. The vitals strip sits too far from the account name. Spacing between sections is tight enough that the page reads as a dense dashboard rather than an editorial magazine document. Enrichment provenance (Clay, Gravatar) and feedback controls (thumbs up/down) exist in the component layer but don't have adequate visual presence.

The page should feel like scrolling through a New York Times special report — progressive reveal, breathing room, and the sense that a story is unfolding — not scanning a dashboard.

## Solution

Frontend-only redesign of `AccountDetailEditorial.tsx` and related components. Three pillars:

### 1. Margin Label Column

Every chapter section (State of Play through Reports) uses a CSS grid layout with a 110px left gutter containing a sticky mono section label. The hero stays full-width. The label acts as a persistent wayfinding anchor while content flows in the right column.

```
| 110px      | 1fr                              |
| State of   | ─── chapter rule ───             |
| Play       | State of Play (serif 28px)       |
|            | [content]                        |
```

- `entity-detail.module.css`: new `.marginLabelSection`, `.marginLabel`, `.marginContent` classes
- `.marginLabel` is `position: sticky; top: 72px` (below folio bar)
- Collapses to single column below 860px

### 2. Visual Storytelling: Section Differentiation

Each section gets a distinct visual shape instead of uniform lists:

| Section | Visual Treatment |
|---------|-----------------|
| **Hero** | 76px serif name + vitals strip immediately below + 21px serif italic lede (executive assessment as editorial opening paragraph) |
| **State of Play** | Two-column layout: Working / Struggling side by side. Items in serif 17px with sans 13px evidence beneath. 2px colored top border per column. |
| **Pull Quote** | Centered 38px serif breathing moment between State of Play and Health. Extracted from executive assessment. 100px vertical padding. |
| **Health** | The score (120px serif number) IS the section hero, paired with a narrative column. Dimension bars are secondary, in a 2-column grid below. |
| **The Room** | Stakeholder cards with layered typography: mono title line, italic serif role label, sans assessment, serif italic bio from enrichment. Enrichment provenance tags (Clay/Gravatar). Avatar with larkspur ring for linked persons. |
| **Expansion Billboard** | Two large serif numbers (expansion potential, time-to-value) as a visual break between Watch List and The Work. 100px vertical padding. |
| **Watch List** | Serif 16px item text (not sans). Urgency badges on risks, impact labels on wins. Color-accented section cards. |
| **The Work** | Serif 16px action text. Owner attribution, status badges, due dates. |
| **The Record** | Grid timeline: date | dot | body. Source attribution per entry. |
| **Reference** | Two-column editorial grid with mono labels + serif 17px subheads. |

### 3. Scroll-Driven Progressive Reveal

IntersectionObserver triggers staggered animations as sections enter the viewport:

- `.reveal`: translateY(24px) + fade, 0.7s ease-out
- `.reveal-slow`: opacity-only fade, 1.2s (for large typography: hero narrative, pull quote, billboard numbers)
- `.reveal-stagger`: children cascade in with 80ms delays (stakeholder cards, dimension bars, timeline entries, state items)

Threshold: 0.08 with -60px bottom root margin (triggers slightly before fully in view).

### 4. Vitals Strip Repositioned

Move vitals strip from below the executive assessment to immediately below the account name. Compact mono text with dot separators. Color highlights: turmeric for ARR, sage for health status, larkspur for meeting cadence.

### 5. Spacing

140px `padding-top` on `.marginLabelSection` (up from current `var(--space-5xl)` which is ~64px). Pull quotes and billboards get 100px vertical padding. The page breathes.

## Files

| File | Changes |
|------|---------|
| `src/styles/entity-detail.module.css` | New `.marginLabelSection`, `.marginLabel`, `.marginContent` classes. Responsive collapse at 860px. |
| `src/pages/AccountDetailEditorial.tsx` | Wrap sections 2-9 in margin label grid. Add pull quote after State of Play. Add expansion billboard after Watch List. Reposition vitals strip. Apply reveal classes. |
| `src/pages/AccountDetailEditorial.module.css` | New styles for pull quote, health hero, expansion billboard, state columns, serif item text treatments. |
| `src/components/entity/StateOfPlay.tsx` | Two-column layout with serif item text + evidence subtext. |
| `src/components/entity/WatchList.tsx` | Serif 16px item text. Urgency/impact badge rendering. |
| `src/components/shared/DimensionBar.tsx` | 2-column grid layout. Secondary to health hero score. |
| `src/components/account/AccountHero.tsx` | Vitals strip immediately after name. 21px serif italic lede for executive assessment. |
| `src/components/entity/StakeholderGallery.tsx` | Layered typography in cards. Enrichment tags (Clay/Gravatar) visible on cards. Serif italic bio from person enrichment. |
| `src/hooks/useRevealObserver.ts` | IntersectionObserver hook for scroll-driven reveal (may already exist — extend if needed). |

## Mockup

Reference mockup: `.docs/mockups/account-detail-margin-label-v3.html`

Open in browser to see the full treatment with scroll animations, typography scale, breathing room, and all visual patterns.

## Dependencies

- I545 (entity detail style migration) should land first — avoids conflicting with inline style cleanup
- I529 (intelligence feedback) already landed — thumbs up/down controls exist

## Out of Scope

- Backend / data model changes
- Person detail or project detail pages (same patterns could apply later but not in this issue)
- New intelligence fields or enrichment sources
- Dark mode / theming

## Implementation Status

**Pass 1 (complete):** Layout restructuring — margin label grid, hero repositioning, pull quote, health hero, spacing, graceful degradation. The existing components were moved into the new layout structure without creating new visual components.

**Pass 2 (suspended):** Editorial refinements deferred. Phase 4b (I554-I558) will change schema, capture types, and intelligence fields significantly — new data will drive different component needs on the account detail page. Remaining ACs should be revisited after Phase 4b lands, applying I550's editorial spirit (serif typography, visual storytelling, section differentiation) to whatever new surfaces Phase 4b produces.

## Acceptance Criteria

### Pass 1 — Layout Restructuring (done)

1. ✅ Every chapter section (State of Play through Reports) renders in a `110px | 1fr` grid with a sticky mono margin label in the left gutter.
2. ✅ Margin labels collapse to inline on viewports below 860px.
3. ✅ Hero shows: 76px serif name → vitals strip (immediately below) → 21px serif italic executive assessment. No large health bar, no duplicate tags.
5. ✅ A pull quote (centered, 38px serif, 100px vertical padding) appears between State of Play and Relationship Health.
6. ✅ Health section leads with a large serif score number (100px+) paired with a narrative column. Dimension bars are a secondary 2-column grid below.
9. ✅ Spacing between chapter sections is 140px (minimum). Pull quote has 100px vertical padding.
14. ✅ All user-facing text follows ADR-0083 vocabulary.
15. ✅ Page renders correctly with empty/partial intelligence (graceful degradation, no blank sections).

### Pass 2 — Editorial Refinements (remaining)

4. State of Play renders with serif 17px item text and sans 13px evidence subtext. (Currently: sans 15px, no evidence subtext.)
7. Stakeholder cards show serif italic bio when available. (Enrichment tags and larkspur ring are done; bio is missing.)
8. An expansion billboard (two large serif numbers with labels) appears between Watch List and The Work.
9b. Billboard has 100px vertical padding. (Pull quote padding done; billboard pending.)
10. Scroll-driven reveal: rootMargin should be -60px (currently -40px). Stagger should support >8 children (currently caps at 8).
11. Watch list items use serif 16px text. Wins show impact labels. (Urgency badges done; font is sans 14px, impact labels missing.)
12. Timeline entries show source attribution (e.g., "Source: meeting note · Sarah Chen sync").
13. Zero new inline styles — migrate StateBlock, WatchList, AccountHero to CSS modules.
