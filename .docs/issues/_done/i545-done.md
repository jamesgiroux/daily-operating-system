# I545 — Entity Detail Pages Style Migration

**Priority:** P1
**Area:** Frontend / Entity Detail / UX
**Version:** v1.0.0 (Phase 3d, Wave 4)
**Depends on:** I447 (design token audit), I521 (structural cleanup)

## Problem

The three entity detail pages have 105 inline `style={{}}` props combined — the second-worst cluster in the app after Settings. Account detail alone has 51 (matching MeetingDetailPage). All three pages use hardcoded `rgba()` values for borders and backgrounds instead of design tokens.

These are high-traffic pages — every account click, project click, or person click lands here. The craft gap between these pages and RiskBriefingPage (A+) is visible.

| Page | File | Inline Styles | Hardcoded Colors |
|------|------|---------------|------------------|
| Account Detail | AccountDetailEditorial.tsx | 51 | 4 rgba values |
| Project Detail | ProjectDetailEditorial.tsx | 39 | 3 rgba values |
| Person Detail | PersonDetailEditorial.tsx | 15 | 0 |
| **Total** | | **105** | **7** |

## Scope

### Style Migration

Migrate all `style={{}}` usages to CSS modules for each page:

**AccountDetailEditorial.tsx** (51 inline styles):
- Create or extend `account-detail.module.css`
- Layout containers (flex, gap, padding, grid)
- Typography overrides (font-family, font-size, color, line-height)
- Decorative elements (borders, backgrounds, opacity, box-shadow)
- Section spacing and dividers

**ProjectDetailEditorial.tsx** (39 inline styles):
- Create or extend `project-detail.module.css`
- Same categories as Account — shared patterns should use same class names where possible

**PersonDetailEditorial.tsx** (15 inline styles):
- Create or extend `person-detail.module.css`
- Lightest migration — mostly layout containers

### Token Compliance

Replace hardcoded rgba values:
- `rgba(30,37,48,0.06)` (borders) → design token or CSS custom property
- `rgba(30,37,48,0.04)` (backgrounds) → design token or CSS custom property
- `rgba(0,0,0,0.12)` (box-shadow) → design token or CSS custom property
- Audit for any other hardcoded hex/rgba values

### Shared Pattern Extraction

Where Account, Project, and Person detail pages use identical layout patterns:
- Extract shared CSS classes to a common `entity-detail.module.css`
- Per-page modules import or compose with shared module
- Follows I450's "shared CSS module for entity detail" direction

## Acceptance Criteria

1. Zero `style={{}}` in AccountDetailEditorial.tsx.
2. Zero `style={{}}` in ProjectDetailEditorial.tsx.
3. Zero `style={{}}` in PersonDetailEditorial.tsx.
4. Zero hardcoded hex or rgba values in any entity detail CSS module.
5. Visual parity: all three pages look identical before and after migration.
6. Shared patterns extracted to common module (DRY with I450).
7. `pnpm tsc --noEmit` clean.

## Relationship to I450

I450 (Portfolio chapter extraction) covers editorial structure — conclusion-before-evidence order, ChapterHeadings, content organization. I545 covers the mechanical style migration. They complement each other: I545 moves styles to modules, I450 restructures the content within those modules.

## Out of Scope

- Content/logic changes to entity detail pages
- New entity detail features
- Drawer elimination (I343)
- Editorial restructuring (I450)
