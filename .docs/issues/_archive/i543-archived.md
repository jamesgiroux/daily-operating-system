# I543 — GA Design Documentation

**Priority:** P1
**Area:** Documentation / Design System
**Version:** v1.0.0 (Phase 3d, Wave 4)
**Depends on:** None

## Problem

11 of 23 pages have zero design documentation. 114 of 177 components aren't in the component inventory. No state pattern documentation exists (empty, loading, error, partial, first-run states). PAGE-ARCHITECTURE.md was last audited 2026-02-20 — major releases have shipped since. A new developer joining for v1.0.0 stabilization would spend 5-10 days rediscovering patterns that should be documented.

## Scope

### PAGE-ARCHITECTURE Updates

Document all undocumented pages using existing template (JTBD, intended structure, current state, compliance, remaining issues):

| Page | File | JTBD |
|------|------|------|
| Account Health | AccountHealthPage.tsx | Pre-renewal health assessment |
| Action Detail | ActionDetailPage.tsx | Execute single committed action |
| Inbox | InboxPage.tsx | Route and classify uploaded files |
| Me / User Profile | MePage.tsx | Understand self in professional context |
| History | HistoryPage.tsx | Review file processing log |
| EBR/QBR Report | EbrQbrPage.tsx | Prepare quarterly business review |
| SWOT Report | SwotPage.tsx | Strategic assessment of account |
| Weekly Impact | WeeklyImpactPage.tsx | Reflect on week's progress |
| Monthly Wrapped | MonthlyWrappedPage.tsx | Celebrate monthly progress |
| Generic Report | ReportPage.tsx | Render any report entity/type combo |

Update compliance grades for existing pages (Meeting Detail, Actions, Settings especially).

### COMPONENT-INVENTORY Updates

Add missing components:
- Report suites: Account Health (5), EBR/QBR (7), SWOT (2), Weekly Impact (5), infrastructure (3) = 22 components
- Settings/Connectors: 11 connector components + ActivityLogSection, DatabaseRecoveryCard, ContextSourceSection
- Dashboard: BriefingMeetingCard, DashboardEmpty
- Entity: IntelligenceQualityBadge, ContextEntryList
- Shared: ActionRow, ProposedActionRow, MeetingRow
- Person: PersonHero

Mark dead code and stale entries.

### New Documents

**STATE-PATTERNS.md** — Per-page state matrices:
- Default/loaded, empty, loading, error, partial, first-run states
- Which editorial component to use for each (EditorialEmpty, EditorialLoading, EditorialError)
- Empty state copy voice guidelines
- Per-page state inventory table

**Developer checklists** (append to README.md or DESIGN-SYSTEM.md):
- New page checklist (atmosphere, shell config, FinisMarker, JTBD, state docs, tokens, CSS approach)
- New component checklist (family fit, shared check, compliance, accessibility)

### Freshness Fixes

- Update COMPONENT-INVENTORY.md audit date
- Update PAGE-ARCHITECTURE.md audit date
- Resolve VIOLATIONS.md reference (create or remove dead link)
- Mark DESIGN-SYSTEM.md TODO (opacity tokens) as tracked by I447

## Acceptance Criteria

1. Every page in `src/pages/*.tsx` has a corresponding entry in PAGE-ARCHITECTURE.md.
2. Every shared component in `src/components/` is in COMPONENT-INVENTORY.md (or explicitly marked dead code).
3. STATE-PATTERNS.md exists with per-page state matrices for all 10+ documented pages.
4. Developer checklist for new pages/components documented.
5. All audit dates updated to current.
6. No dead links to missing documents.

## Out of Scope

- INTERACTION-PATTERNS.md — covered by I546
- DATA-PRESENTATION-GUIDELINES.md — covered by I546
- NAVIGATION-ARCHITECTURE.md — covered by I546
- ACCESSIBILITY.md (post-GA)
