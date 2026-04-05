# I546 — Design Documentation: Interaction, Data Presentation, Navigation

**Priority:** P2
**Area:** Documentation / Design System
**Version:** v1.0.0 (Phase 3d, Wave 4)
**Depends on:** I543 (GA Design Documentation — covers PAGE-ARCHITECTURE, COMPONENT-INVENTORY, STATE-PATTERNS)

## Problem

I543 documents pages, components, and state patterns. But three categories of design knowledge remain undocumented — knowledge that a new developer needs before they can confidently build or modify features:

1. **Interaction patterns** — How does inline editing work? What's the slide-deck navigation model? When do we use expansion vs. navigation vs. modal? These patterns are implemented but never codified.
2. **Data presentation** — When do we use a list vs. a card vs. a row vs. a table? What's the decision tree for choosing between them? The app has all four but no documented rationale.
3. **Navigation architecture** — How does FloatingNavIsland work? What's the page connection map? What routes exist and why? The router has 20+ routes with no documented map.

Without these docs, developers reverse-engineer patterns from existing pages — and sometimes get it wrong, creating inconsistency.

## Scope

### INTERACTION-PATTERNS.md

Document the interaction vocabulary used across the app:

- **Inline editing** — EditableText, EditableList, EditableTagList. When to use each. Click-to-edit lifecycle. Save/cancel/escape behavior. Validation. Optimistic update pattern.
- **Slide-deck navigation** — Used in MeetingDetailPage and reports. Keyboard nav (← →). Progress indicator. Section model.
- **Expansion/disclosure** — When to use inline expansion vs. navigate to detail. Disclosure groups in Settings. Expand-to-reveal vs. click-to-navigate decision tree.
- **Entity linking** — How entity chips work. Click behavior (navigate). Hover behavior (preview, if any). Linking UI in meeting detail.
- **Drag and drop** — If used anywhere, document. If not, state "not in vocabulary."
- **Selection** — Multi-select patterns (if any). Single-select (radio-style) in Settings role presets.
- **Feedback** — Toast patterns. Inline success/error. Thumbs up/down (I529).

### DATA-PRESENTATION-GUIDELINES.md

Decision tree for data presentation patterns:

- **Lists** — When: sequential items with uniform structure. Examples: actions list, email list. Pattern: vertical stack, optional grouping via ChapterHeading.
- **Cards** — When: featured content that deserves visual weight. Examples: BriefingMeetingCard, reports. Rule: cards are for featured items only (ADR-0073).
- **Rows** — When: tabular data with multiple attributes per item. Examples: audit log, stakeholder table, connector grid.
- **Sections** — When: editorial narrative with mixed content. Examples: entity detail chapters, meeting briefing sections.
- **Key-value pairs** — When: metadata display. Horizontal label: value or stacked label/value.
- **Metrics/stats** — When: numeric KPIs. Pattern: stat line (number + label + optional trend).
- **Empty states** — Copy voice guide. Which EditorialEmpty variant. When to show illustration vs. text-only.

### NAVIGATION-ARCHITECTURE.md

Map of the app's navigation model:

- **FloatingNavIsland** — All nav items, icons, active states, order. Which pages are in the island vs. reachable only through links.
- **Route map** — Every route in `src/router.tsx`, its page component, and how users reach it.
- **Page connections** — Which pages link to which. Entity detail → account list. Meeting card → meeting detail. Etc.
- **Deep linking** — URL patterns, parameter extraction, direct navigation support.
- **Back navigation** — Browser back behavior. Any custom back patterns.
- **Shell configuration** — Which pages use which atmosphere, which show FloatingNavIsland, which are full-bleed.

## Acceptance Criteria

1. INTERACTION-PATTERNS.md exists in `.docs/design/` with documented patterns for inline editing, slide-deck nav, expansion, entity linking, selection, and feedback.
2. DATA-PRESENTATION-GUIDELINES.md exists in `.docs/design/` with decision tree for lists vs. cards vs. rows vs. sections.
3. NAVIGATION-ARCHITECTURE.md exists in `.docs/design/` with FloatingNavIsland map, route map, and page connections.
4. All three documents reference existing components by name (not abstract descriptions).
5. No dead links in any design document after these are created.

## Out of Scope

- ACCESSIBILITY.md (post-GA)
- Design token changes (I447)
- Implementation changes — these are documentation-only
