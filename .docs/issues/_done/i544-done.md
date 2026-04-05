# I544 — Component DRY/SRP Reconciliation

**Priority:** P1
**Area:** Frontend / Code Quality
**Version:** v1.0.0 (Phase 3d, Wave 4)
**Depends on:** I521 (structural cleanup — ghost component removal)

## Problem

The codebase has grown organically through beta. Components were built per-page rather than extracted as shared patterns. The result: duplicate implementations, components with multiple responsibilities, and dead code that clouds the inventory. Examples already identified:

- `StatusDot` defined in SettingsPage.tsx, `statusDot()` in styles.ts, and inline divs in SystemStatus.tsx — three implementations of the same concept
- Multiple empty state components across pages instead of using `EditorialEmpty`
- Loading skeleton implementations vary per-page instead of using `EditorialLoading`
- Button styling fragmented between `styles.ts` variants and inline definitions
- Action row presentation duplicated between ActionsPage and MeetingDetailPage

I521 removes ghost components and consolidates duplicate _types_. I544 goes deeper: duplicate _implementations_, SRP violations, and shared extraction opportunities.

## Scope

### Phase 1: Duplicate Detection Audit

Full inventory of duplicate patterns across `src/components/` and `src/pages/`:

1. **Status indicators** — StatusDot, health badges, connection status, any colored-dot pattern
2. **Empty states** — Per-page empty implementations vs. EditorialEmpty usage
3. **Loading states** — Per-page skeletons vs. EditorialLoading usage
4. **Error states** — Per-page error displays vs. EditorialError usage
5. **Row components** — ActionRow, ProposedActionRow, MeetingRow, PersonRow — shared vs. page-specific
6. **Button patterns** — styles.ts variants vs. CSS module classes vs. inline
7. **Section headers** — ChapterHeading vs. ad-hoc heading patterns
8. **Disclosure/expandable** — Multiple expand/collapse implementations

### Phase 2: Shared Component Extraction

For each duplicate cluster identified in Phase 1:
- If a shared component exists (e.g., EditorialEmpty), migrate all pages to use it
- If no shared component exists but 3+ pages use the same pattern, extract one
- Document the shared component in COMPONENT-INVENTORY.md

### Phase 3: SRP Violations

Identify and split components that handle multiple unrelated responsibilities:
- Components rendering AND fetching data (should be hook + presentational split)
- Components with multiple visual "modes" that should be separate components
- Page-level components with embedded business logic that belongs in hooks

### Phase 4: Dead Code Removal

- Delete components confirmed unused by I521 audit + this reconciliation
- Remove orphaned CSS module classes with no JSX references
- Remove orphaned type definitions with no runtime usage

## Acceptance Criteria

1. Zero duplicate implementations of StatusDot — one shared component, used everywhere.
2. Every page uses EditorialEmpty/EditorialLoading/EditorialError for state display (not custom inline implementations).
3. No component file exceeds 400 lines without documented justification.
4. Every extracted shared component is documented in COMPONENT-INVENTORY.md.
5. Dead code identified and removed (files deleted, not commented out).
6. `pnpm tsc --noEmit` clean after all changes.

## Out of Scope

- Architectural refactoring (moving pages, changing routing)
- Hook extraction from pages (unless SRP violation is severe)
- New feature development
- Performance optimization
