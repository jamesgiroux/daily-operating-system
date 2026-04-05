# Design System Violations

Known deviations from the design system documented for tracking and future remediation.

**Last audited:** 2026-03-12

## Active Violations

(none — all tracked violations resolved)

## Resolved

### status-badge.tsx — Hardcoded colors
**Resolved:** 2026-03-12
**Resolution:** Component removed from codebase.

### Report pages — Inline style duplication
**Resolved:** 2026-03-12
**Resolution:** Created `report-shell.module.css`, `report-page.module.css`, and `report-slides.module.css`. ReportShell, ReportPage, AccountHealthPage, EbrQbrPage, SwotPage, and WeeklyImpactPage all use CSS modules. Remaining inline styles are CSS variable passthroughs for per-page accent colors (2 per page).

### ActionDetailPage.tsx — No CSS module
**Resolved:** 2026-03-12
**Resolution:** Created `ActionDetailPage.module.css`. Extracted local EditableInline, EditableTextarea, EditableDate to shared `src/components/ui/` components with co-located CSS modules. Inline styles reduced from 59 to 4 (dynamic state-dependent only).

### HistoryPage.tsx — No CSS module, non-standard hero
**Resolved:** 2026-03-12
**Resolution:** Created `HistoryPage.module.css`. All 17 inline styles replaced with CSS module classes. Zero inline styles remaining.

### MonthlyWrappedPage.tsx — Monolith page
**Resolved:** 2026-03-12
**Resolution:** Decomposed ~1550-line monolith into `src/pages/monthly-wrapped/` directory with 10 per-slide components, shared CSS module, extracted types/constants/hooks. Main orchestrator reduced to ~290 lines. Inline styles reduced from ~70 to 3 (dynamic gradient/animation styles).
