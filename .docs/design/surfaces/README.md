# Surfaces

Full screens. The user-visible products built on top of patterns and primitives. DailyBriefing, AccountDetailPage, MeetingDetail, ProjectDetail, PersonDetail, Settings.

## Index

| Canonical name | Current src name | Status | Spec |
|---|---|---|---|
| [`DailyBriefingRedesign`](./DailyBriefingRedesign.md) | `src/pages/DailyBriefingRedesign.tsx` | Canonical routed briefing | Cutover reference (2026-05-07) |
| [`Settings`](./Settings.md) | `src/pages/SettingsPage.tsx` + `src/features/settings-ui/*` | Canonical shipped + roadmap split | ✓ Reconciled (2026-05-05) |
| [`MeetingDetail`](./MeetingDetail.md) | `src/pages/MeetingDetailPage.tsx` | Canonical shipped + extraction targets | ✓ Reconciled (2026-05-05) |
| [`AccountDetailPage`](./AccountDetailPage.md) | `src/pages/AccountDetailPage.tsx` | Canonical | ✓ Parity pass (2026-05-05) |
| [`ActionsPage`](./ActionsPage.md) | `src/pages/ActionsPage.tsx` | Canonical | ✓ Parity pass (2026-05-05) |
| [`MePage`](./MePage.md) | `src/pages/MePage.tsx` | Canonical | ✓ Parity pass (2026-05-05) |
| [`ProjectsPage`](./ProjectsPage.md) | `src/pages/ProjectsPage.tsx` | Canonical | ✓ Parity pass (2026-05-05) |
| [`OnboardingFlow`](./OnboardingFlow.md) | `src/components/onboarding/OnboardingFlow.tsx` | Canonical | ✓ Shipped sequence reconciled (2026-05-05) |
| [`AccountHealthPage`](./AccountHealthPage.md) | `src/pages/AccountHealthPage.tsx` | Canonical | ✓ Parity pass (2026-05-05) |
| [`BookOfBusinessPage`](./BookOfBusinessPage.md) | `src/pages/BookOfBusinessPage.tsx` | Canonical | ✓ Parity pass (2026-05-05) |
| [`EbrQbrPage`](./EbrQbrPage.md) | `src/pages/EbrQbrPage.tsx` | Canonical | ✓ Parity pass (2026-05-05) |
| [`MonthlyWrappedPage`](./MonthlyWrappedPage.md) | `src/pages/monthly-wrapped/MonthlyWrappedPage.tsx` | Canonical | ✓ Parity pass (2026-05-05) |
| [`RiskBriefingPage`](./RiskBriefingPage.md) | `src/pages/RiskBriefingPage.tsx` | Canonical | ✓ Parity pass (2026-05-05) |
| [`SwotPage`](./SwotPage.md) | `src/pages/SwotPage.tsx` | Canonical | ✓ Parity pass (2026-05-05) |
| [`WeeklyImpactPage`](./WeeklyImpactPage.md) | `src/pages/WeeklyImpactPage.tsx` | Canonical | ✓ Parity pass (2026-05-05) |
| [`StartupBriefingScreen`](./StartupBriefingScreen.md) | `src/components/startup/StartupBriefingScreen.tsx` | Canonical | ✓ Parity pass (2026-05-05) |
| `ProjectDetail` | `ProjectDetailEditorial.tsx` | In v1.4.2 scope | _Wave 5 / surface pass_ |
| `PersonDetail` | `PersonDetailEditorial.tsx` | In v1.4.2 scope | _Wave 5 / surface pass_ |

## What a surface spec captures

Each surface gets one `.md` file with:

- **Job** — what the user accomplishes here
- **Canonical name** vs current `src/` name (rename status if mismatched — see `../NAMING.md`)
- **Source files** — every file under `src/` that implements this surface
- **Layout regions** — header, spine, sidebar, dock, etc.
- **Local nav approach** — chapter inventory provided to `FloatingNavIsland` (per D2)
- **Patterns consumed** — in reading order, with links
- **Primitives consumed**
- **Notable interactions**
- **Empty / loading / error states**
- **Naming notes** — rename history, candidate renames, decisions deferred

## Conventions

- **Surface specs are the contract for what the surface is.** Implementation in `src/` should match. If they disagree, the spec wins (or the spec gets updated, deliberately).
- **A surface re-implementing a pattern is a smell.** Either the pattern is missing a variant, or the surface is wrong, or the pattern is wrong. Resolve, don't paper over.
- **Surfaces provide chapters to `FloatingNavIsland`.** Per D2, surfaces do not invent local nav patterns unless a surface spec calls out an explicit proposed exception, as `DailyBriefingRedesign` now does for `DayStrip`.
- **Don't duplicate the figma/mockup here.** Link to it. The spec is the contract; the mockup is a reference.

## Surface-internal components

Components that are genuinely unique to one surface (and have no plausible reuse) live in `src/` next to the surface, not in `primitives/` or `patterns/`. They don't get a markdown spec here. If two surfaces start needing it, *that's* the trigger to promote.
