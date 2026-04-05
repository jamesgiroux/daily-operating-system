# Frontend Gap Analysis

Cross-cutting synthesis of four audit documents: Components, Hooks, Types, Styles.
Generated: 2026-03-02.

---

## 1. Cross-Cutting P0 Issues (bugs affecting users today)

### P0-1: Broken CSS Token References (3 rendering bugs)

Three CSS files reference tokens that do not exist in `design-tokens.css`. These resolve to `initial` (transparent/inherit), causing visual breakage.

| File | Line | Broken Token | Effect | Fix |
|------|------|-------------|--------|-----|
| `src/styles/WeekPage.module.css` | 99 | `var(--color-surface-linen)` | Background renders transparent instead of linen | Change to `var(--color-paper-linen)` |
| `src/styles/meeting-intel.module.css` | 1102 | `var(--color-turmeric, #d4a853)` | Falls back to off-palette `#d4a853` instead of `--color-spice-turmeric` | Change to `var(--color-spice-turmeric)` |
| `src/styles/meeting-intel.module.css` | 1103 | `var(--color-cream-wash, ...)` | Falls back to off-palette rgba | Use `var(--color-paper-cream)` with opacity or define token |

**Source:** FRONTEND-STYLES.md Section 9 (C1-C3).

### P0-2: Cross-Page Action Staleness

`complete_action` and `reopen_action` are called from 4 locations (`useActions`, `DailyBriefing`, `MeetingDetailPage`, `ActionDetailPage`), but only `useActions` maintains the action list state. When a user completes an action on `MeetingDetailPage`, the `ActionsPage` list is stale until remount. Same for proposed action accept/reject.

**Root cause:** No shared action mutation layer or backend event (`action-updated`) to notify all consumers.

**Source:** FRONTEND-HOOKS.md Sections "State Synchronization Issues" (item 1) and "Missing Hook Candidates" (item 3).

### P0-3: `DbMeeting.accountId` Phantom Field

The TypeScript `DbMeeting` type declares `accountId?: string`, but the Rust `db::types::DbMeeting` struct has no `account_id` field. Any frontend code reading `meeting.accountId` from a backend response silently gets `undefined`.

**Source:** FRONTEND-TYPES.md Section 2 ("Extra Fields").

### P0-4: `WeekOverview` Missing Backend Fields

The TypeScript `WeekOverview` type is missing `weekNarrative` and `topPriority` fields that the Rust backend can populate. If the backend sends these, the frontend silently discards them. These are AI-generated content that users should see on WeekPage.

**Source:** FRONTEND-TYPES.md Section 2 ("Missing Fields").

---

## 2. Cross-Cutting P1 Issues (structural problems blocking new features)

### P1-1: Business Logic Embedded in Presentation (10 pages)

Ten page/component files contain substantial business logic (time parsing, grouping, classification, normalization) mixed into render code. This blocks testability and reuse.

**Highest-impact files:**

| File | Lines of Logic | Recommended Extraction |
|------|---------------|----------------------|
| `src/pages/MeetingDetailPage.tsx` | ~250 lines (L48-300) | `useMeetingDetail` hook + `lib/meeting-utils.ts` |
| `src/pages/ActionsPage.tsx` | ~110 lines (L30-140) | `lib/action-grouping.ts` or `useActionGroups` hook |
| `src/pages/InboxPage.tsx` | ~70 lines (L54-120) | `lib/inbox-classify.ts` |
| `src/pages/MonthlyWrappedPage.tsx` | ~40 lines (L73-110) | `lib/monthly-wrapped-normalize.ts` |
| `src/components/dashboard/DailyBriefing.tsx` | ~90 lines (L48-140) | `lib/briefing-utils.ts` |
| `src/components/settings/SystemStatus.tsx` | ~50 lines (L22-69) | `lib/schedule-utils.ts` |
| `src/components/settings/DiagnosticsSection.tsx` | ~30 lines (L40-69) | `lib/schedule-utils.ts` (shared with SystemStatus) |

**Source:** FRONTEND-COMPONENTS.md Section 3.

### P1-2: Missing Hook Wrappers for Repeated invoke() Patterns

Six invoke() patterns are repeated across 3+ components with no hook abstraction. This means inconsistent error handling, no shared loading states, and no single place to add caching or events.

| Pattern | Call Sites | Suggested Hook |
|---------|-----------|---------------|
| `invoke("update_entity_metadata")` | 3 detail pages (6 calls) | `useEntityMetadata(entityType, entityId)` |
| `invoke("save_report")` | 5 report pages | `useReportSave(entityId, reportType)` |
| `invoke("complete_action"/"reopen_action")` | 4 locations | `useActionMutation` or emit `action-updated` event |
| Enrichment + timer pattern | 3 entity detail hooks | `useEnrichment(entityType, entityId)` |
| File indexing pattern | 3 entity detail hooks | `useEntityFiles(entityType, entityId)` |
| `invoke("set_wizard_step")` | 3 onboarding components (5 calls) | `useOnboardingWizard()` |

**Source:** FRONTEND-HOOKS.md Section "Missing Hook Candidates".

### P1-3: Duplicate Logic Across Files

Four distinct logic patterns are implemented independently in multiple files:

| Logic | Duplicated In | Single Location |
|-------|--------------|----------------|
| `cronToHumanTime()` | `SystemStatus.tsx`, `DiagnosticsSection.tsx` | `lib/schedule-utils.ts` |
| Time parsing (AM/PM to ms) | `DailyBriefing.tsx`, `BriefingMeetingCard.tsx` | `lib/time-utils.ts` |
| `formatTime(iso)` helper | `SystemStatus.tsx`, `ActivityLogSection.tsx` | `lib/format-utils.ts` |
| `toArr<T>(v: unknown): T[]` | 5 report pages (AccountHealth, EbrQbr, MonthlyWrapped, Swot, WeeklyImpact) | `lib/report-utils.ts` |

**Source:** FRONTEND-COMPONENTS.md Section 3 ("Duplicate Logic"), FRONTEND-TYPES.md Section 5.

### P1-4: `IntelligenceQuality` Inline Type Triplication

The `IntelligenceQuality` type (8 fields: `score`, `confidence`, `signalCount`, `freshness`, `sourceCount`, `lastEnriched`, `grade`, `gradeLabel`) is defined inline in three locations:

1. `src/types/index.ts` on `Meeting` interface (~line 103)
2. `src/types/index.ts` on `MeetingIntelligence` interface (~line 745)
3. `src/types/index.ts` on `TimelineMeeting` interface (~line 1748)

Any field added to one location must be manually propagated to the other two. This has already caused the `MeetingDetailPage.tsx` to define its own inline type for intelligence quality as a workaround.

**Source:** FRONTEND-TYPES.md Section 4.

### P1-5: `EmailSignal` Type Conflation

The TypeScript `EmailSignal` type has 12 fields, merging two distinct Rust types: `types::EmailSignal` (7 fields) and `db::types::DbEmailSignal` (12 fields). Fields like `emailId`, `senderEmail`, `personId`, `entityId`, `entityType` are `undefined` when the source is `types::EmailSignal` (used in FullMeetingPrep context), leading to silent data gaps.

**Source:** FRONTEND-TYPES.md Section 2.

### P1-6: Entity Detail Hook Decomposition Blocked

`usePersonDetail` (420 lines, 10 responsibilities) and `useProjectDetail` (287 lines, 8 responsibilities) are monolithic hooks. `useAccountDetail` was partially decomposed (extracted `useAccountFields` + `useTeamManagement`), but `usePersonDetail` and `useProjectDetail` did not follow suit. Adding features to entity detail pages (required for v1.1.0 intelligence schema redesign) means touching these oversized hooks.

**Source:** FRONTEND-HOOKS.md Section "Hook Composition Issues".

### P1-7: Dead `useActivePreset.ts` Creates Import Ambiguity

Two files export the same hook name:
- `src/hooks/useActivePreset.ts` (23 lines) -- standalone, no event listening, **dead code**
- `src/hooks/useActivePreset.tsx` (53 lines) -- context provider, listens to `preset-changed`

TypeScript resolves `.tsx` over `.ts`, so the dead file is never imported. But it creates confusion and could cause bugs if someone explicitly imports the `.ts` version.

**Source:** FRONTEND-HOOKS.md Section "State Synchronization Issues" (item 2).

---

## 3. Cross-Cutting P2 Issues (technical debt)

### P2-1: Ghost Components (23 files, ~3,440 lines)

23 component files are exported but never imported. These fall into three categories:

**Category A: Unwired onboarding chapters (6 files, 2,801 lines)**

Built but never imported by `OnboardingFlow.tsx`. These represent the largest single block of dead code.

| File | Lines |
|------|------:|
| `src/components/onboarding/chapters/InternalTeamSetup.tsx` | 431 |
| `src/components/onboarding/chapters/InboxTraining.tsx` | 427 |
| `src/components/onboarding/chapters/DashboardTour.tsx` | 313 |
| `src/components/onboarding/chapters/PopulateWorkspace.tsx` | 237 |
| `src/components/onboarding/chapters/PrimeBriefing.tsx` | 207 |
| `src/components/onboarding/chapters/MeetingDeepDive.tsx` | 186 |

Each has associated styles (CSS modules) and some have types. Wire these into the onboarding flow or delete as a single cleanup task.

**Category B: Superseded report components (3 files, 460 lines)**

| File | Lines | Reason |
|------|------:|--------|
| `src/components/reports/MonthlyWrappedReport.tsx` | 277 | `MonthlyWrappedPage` uses custom slides instead |
| `src/components/reports/WeeklyImpactReport.tsx` | 162 | `WeeklyImpactPage` uses custom slides instead |
| `src/components/reports/ReportExportButton.tsx` | 21 | Stub, never wired |

**Category C: Dead UI primitives (10 files, ~590 lines)**

| File | Lines | Reason |
|------|------:|--------|
| `src/components/ui/CyclingPill.tsx` | 72 | Never imported |
| `src/components/ui/copy-button.tsx` | 48 | MeetingDetailPage has its own copy logic |
| `src/components/ui/email-signal-list.tsx` | 68 | EmailsPage implements its own |
| `src/components/ui/list-row.tsx` | 76 | Superseded by EntityRow/ActionRow |
| `src/components/ui/search-input.tsx` | 29 | Pages implement their own search |
| `src/components/ui/status-badge.tsx` | 62 | Exports style maps, nothing imports them |
| `src/components/ui/tab-filter.tsx` | 56 | Pages implement their own tabs |
| `src/components/editorial/BriefingCallouts.tsx` | 155 | DailyBriefing has its own callouts |
| `src/components/editorial/EditorialEmpty.tsx` | 34 | Superseded by `editorial/EmptyState.tsx` |
| `src/components/ProfileSelector.tsx` | 137 | Profile switching UI, not connected |

Note: `dropdown-menu.tsx`, `scroll-area.tsx`, `collapsible.tsx`, `label.tsx` are shadcn/ui primitives and should be kept.

**Cross-reference with Types audit:** `EmailSummaryData` and `EmailStats` types (FRONTEND-TYPES.md, dead types) were used by the now-dead email summary display. Delete types alongside `email-signal-list.tsx`.

**Cross-reference with Styles audit:** `status-badge.tsx` (ghost component) references style tokens that are themselves unused elsewhere. The sidebar variables in `index.css` (lines 50-58) correspond to the dead `sidebar.tsx` component. Clean up as a single task.

**Source:** FRONTEND-COMPONENTS.md Section 2.

### P2-2: Oversized Files (4 critical, 7 high)

Files exceeding 1,000 lines that should be split:

| File | Lines | Split Strategy |
|------|------:|---------------|
| `src/pages/MeetingDetailPage.tsx` | 1,751 | Extract 5+ chapter sections into sub-components, move business logic to hooks (see P1-1) |
| `src/pages/MonthlyWrappedPage.tsx` | 1,550 | Extract 10 inline slides into per-slide components (matching pattern used by every other report page) |
| `src/pages/InboxPage.tsx` | 1,372 | Extract file classification, drag-drop, entity assignment, file list into modules |
| `src/components/settings/SystemStatus.tsx` | 1,088 | Promote 5+ local section functions to separate files |

Files 500-1,000 lines that should be considered for split:

| File | Lines |
|------|------:|
| `src/pages/AccountDetailEditorial.tsx` | 949 |
| `src/pages/ActionDetailPage.tsx` | 855 |
| `src/pages/AccountsPage.tsx` | 837 |
| `src/pages/ActionsPage.tsx` | 835 |
| `src/pages/WeekPage.tsx` | 822 |
| `src/components/dashboard/DailyBriefing.tsx` | 752 |
| `src/components/settings/DiagnosticsSection.tsx` | 715 |

**Source:** FRONTEND-COMPONENTS.md Section 5.

### P2-3: Duplicate Structural Patterns (6 clusters)

| Cluster | Files | Shared % | Dedup Strategy |
|---------|-------|----------|---------------|
| Entity detail pages | AccountDetailEditorial, ProjectDetailEditorial, PersonDetailEditorial | ~80% | `EntityDetailEditorial` shell with entity-specific slots |
| Report slide pages | EbrQbrPage, SwotPage, AccountHealthPage, WeeklyImpactPage, RiskBriefingPage | ~70% | `useEditableReport` hook + `ReportPageShell` |
| Entity list pages | AccountsPage, ProjectsPage, PeoplePage | ~60% | Generic `EntityListPage` parameterized by type |
| Hero components | AccountHero, PersonHero, ProjectHero | ~75% | `EntityHero` base component |
| Appendix components | AccountAppendix, PersonAppendix, ProjectAppendix | ~50% | `EntityAppendix` with shared sections |
| Entity detail hooks | useAccountDetail, usePersonDetail, useProjectDetail | ~60% | Shared sub-hooks for enrichment, files, actions, archive |

**Source:** FRONTEND-COMPONENTS.md Section 6, FRONTEND-HOOKS.md Section "Duplicate Patterns".

### P2-4: Spacing Token Adoption Gap

The spacing token system has only 45 references across all files, while 200+ hardcoded pixel values exist. The root cause is a gap in the scale: `--space-sm` (8px) to `--space-md` (16px) leaves 6px, 10px, 12px, and 14px unaddressed. These four values account for ~65 of the hardcoded instances.

Options:
- Add intermediate tokens (`--space-xs-plus: 6px`, `--space-sm-plus: 12px`)
- Document that component-internal spacing is exempt from the token system

**Source:** FRONTEND-STYLES.md Section 4.

### P2-5: Missing Focus States (~20 interactive elements)

Interactive elements styled in CSS modules bypass Tailwind's base focus outline. Missing focus states on:
- Editorial briefing meeting action checkboxes, priority checkboxes, collapse buttons
- Lock overlay unlock button
- Meeting-intel plan inputs, ghost inputs, agenda items
- Context entry list action buttons
- Tour tips navigation buttons

**Source:** FRONTEND-STYLES.md Section 8.

### P2-6: Entity Hero Modules Lack Responsive Breakpoints

`AccountHero.module.css`, `PersonHero.module.css`, and `ProjectHero.module.css` have no media queries. The 76px serif headline will overflow on viewports narrower than ~600px.

**Source:** FRONTEND-STYLES.md Section 8.

### P2-7: Unused Token Definitions (8 tokens)

| Token | Defined In | References |
|-------|-----------|-----------|
| `--color-entity-account` | design-tokens.css:56 | 0 |
| `--color-entity-project` | design-tokens.css:57 | 0 |
| `--color-entity-person` | design-tokens.css:58 | 0 |
| `--color-entity-action` | design-tokens.css:59 | 0 |
| `--color-entity-user` | design-tokens.css:60 | 0 |
| `--transition-fast` | design-tokens.css:137 | 0 |
| `--transition-slow` | design-tokens.css:139 | 0 |
| `--space-4xl` | design-tokens.css:103 | 0 |

The 5 entity color aliases represent a missed abstraction layer. Components reference raw palette colors (`--color-spice-turmeric`) instead of semantic aliases (`--color-entity-account`). If entity color assignments change, every reference must be manually updated.

**Source:** FRONTEND-STYLES.md Section 1.

### P2-8: Dead CSS Rules and Sidebar Variables

| Item | Location | Lines |
|------|---------|-------|
| `.keyPeople*` legacy aliases (display: none) | `editorial-briefing.module.css` L258-260 | 3 |
| Empty `.attendeeTooltip` rule | `meeting-intel.module.css` L789-793 | 5 |
| Empty `.quickContextGlanceItem` rule | `editorial-briefing.module.css` L384-386 | 3 |
| 8 `--sidebar-*` variables for removed AppSidebar | `index.css` L50-58 | 9 |

**Cross-reference:** The sidebar variables correspond to the ghost `ui/sidebar.tsx` component (FRONTEND-COMPONENTS.md). Remove both together.

**Source:** FRONTEND-STYLES.md Section 6.

---

## 4. Remediation Roadmap

### Before v1.1.0 (immediate -- current dev cycle)

These items fix user-visible bugs or remove confusion that will compound during v1.1.0 development.

| # | Task | Files | Effort | Blocks |
|---|------|-------|--------|--------|
| 1 | Fix 3 broken CSS token references (P0-1) | `WeekPage.module.css`, `meeting-intel.module.css` | 15 min | -- |
| 2 | Add `weekNarrative` + `topPriority` to TS `WeekOverview` (P0-4) | `src/types/index.ts` | 10 min | WeekPage features |
| 3 | Remove `DbMeeting.accountId` phantom field (P0-3) | `src/types/index.ts` | 10 min | -- |
| 4 | Delete dead `useActivePreset.ts` (P1-7) | `src/hooks/useActivePreset.ts` | 5 min | -- |
| 5 | Extract `IntelligenceQuality` to named interface (P1-4) | `src/types/index.ts` | 20 min | Type drift |
| 6 | Delete dead types: `EmailSummaryData`, `EmailStats` (P2-1 cross-ref) | `src/types/index.ts` | 5 min | -- |
| 7 | Split `EmailSignal` into light/full variants (P1-5) | `src/types/index.ts` | 30 min | Email features |
| 8 | Fix cross-page action staleness: emit `action-updated` event from backend (P0-2) | `src-tauri/src/commands.rs`, `src/hooks/useActions.ts` | 1 hr | User-visible bug |
| 9 | Delete 10 dead UI primitives + 2 dead editorial components (P2-1 Category C) | 12 files, ~590 lines | 30 min | -- |
| 10 | Fix `TourTips.module.css` broken z-index and hardcoded color (Styles H1, H2) | `src/styles/TourTips.module.css` | 10 min | -- |
| 11 | Remove dead sidebar CSS variables + confirm `sidebar.tsx` is unused (P2-8) | `src/styles/index.css`, `src/components/ui/sidebar.tsx` | 20 min | -- |

**Estimated total: ~4 hours**

### During v1.1.0 (alongside intelligence schema redesign)

These items are structural improvements that align with v1.1.0's scope (entity intelligence, account health, reports).

| # | Task | Files | Effort | Blocks |
|---|------|-------|--------|--------|
| 12 | Extract shared entity detail sub-hooks (P1-2, P1-6) | New: `useEnrichment.ts`, `useEntityFiles.ts`, `useInlineAction.ts`. Modified: 3 detail hooks | 1 day | v1.1.0 entity work |
| 13 | Extract `useReportSave` hook (P1-2) | New: `useReportSave.ts`. Modified: 5 report pages | 2 hrs | v1.1.0 report work |
| 14 | Extract `useEntityMetadata` hook (P1-2) | New: `useEntityMetadata.ts`. Modified: 3 detail pages | 1 hr | v1.1.0 entity work |
| 15 | Deduplicate business logic into shared utilities (P1-1, P1-3) | New: `lib/schedule-utils.ts`, `lib/time-utils.ts`, `lib/report-utils.ts`. Modified: 10 files | 3 hrs | Testability |
| 16 | Decide on 6 ghost onboarding chapters (P2-1 Category A) | 6 files, 2,801 lines | 1 hr (decide) + variable (implement) | Onboarding completeness |
| 17 | Delete 3 superseded report components (P2-1 Category B) | 3 files, 460 lines | 15 min | -- |
| 18 | Split `MonthlyWrappedPage.tsx` into per-slide components (P2-2) | 1 file -> 11 files | 3 hrs | Report consistency |
| 19 | Add responsive breakpoints to entity hero modules (P2-6) | 3 CSS modules | 2 hrs | Mobile support |
| 20 | Adopt or remove entity color alias tokens (P2-7) | `design-tokens.css` + ~30 CSS module files | 2 hrs | Design system clarity |
| 21 | Add focus states to CSS-module interactive elements (P2-5) | ~8 CSS modules | 2 hrs | Accessibility |

**Estimated total: ~3 days**

### After v1.1.0 (tech debt reduction)

These are larger structural unifications that reduce long-term maintenance cost but are not blocking features.

| # | Task | Files | Effort |
|---|------|-------|--------|
| 22 | Create `EntityDetailEditorial` shell component (P2-3) | 3 detail pages + new shell | 2 days |
| 23 | Create `ReportPageShell` + `useEditableReport` (P2-3) | 5 report pages + new shell | 1 day |
| 24 | Create generic `EntityListPage` (P2-3) | 3 list pages + new component | 1 day |
| 25 | Create `EntityHero` base component (P2-3) | 3 hero components + new base | 1 day |
| 26 | Split `MeetingDetailPage.tsx` (1,751 lines) into sub-components (P2-2) | 1 file -> 6+ files | 1 day |
| 27 | Split `SystemStatus.tsx` (1,088 lines) into section files (P2-2) | 1 file -> 5+ files | 3 hrs |
| 28 | Add intermediate spacing tokens or document exemption (P2-4) | `design-tokens.css` + documentation | 1 hr |
| 29 | Migrate raw rgba() values to opacity tokens where possible (P2-4) | ~20 CSS module files | 3 hrs |
| 30 | Decompose `usePersonDetail` / `useProjectDetail` into sub-hooks (P1-6) | 2 hooks + new sub-hooks | 1 day |

**Estimated total: ~8 days**

---

## 5. Summary Statistics

### Codebase Size

| Metric | Count |
|--------|------:|
| Component files | 155 |
| Page files | 23 |
| Hook files | 33 |
| Type definition files | 3 (`index.ts`, `callout.ts`, `preset.ts`, `reports.ts`) |
| CSS files | 24 |
| Total TSX/TS lines | ~50,843 |
| Total hook lines | ~3,700 |
| Total CSS lines | ~5,000 (est.) |
| Named TypeScript types | 141 |

### Dead Code

| Category | Files | Lines |
|----------|------:|------:|
| Ghost components (unwired onboarding) | 6 | 2,801 |
| Ghost components (superseded reports) | 3 | 460 |
| Ghost components (dead UI primitives) | 10 | 590 |
| Ghost component (ProfileSelector) | 1 | 137 |
| Dead hook (`useActivePreset.ts`) | 1 | 23 |
| Dead types (`EmailSummaryData`, `EmailStats`) | -- | ~20 |
| Dead CSS rules (legacy aliases, empty rules, sidebar vars) | -- | ~20 |
| **Total ghost code** | **21+** | **~4,051** |

### Compliance Scores (from individual audits)

| Audit Area | Score | Key Gap |
|-----------|------:|---------|
| Style system (FRONTEND-STYLES.md) | 85/100 | Spacing token adoption (65/100), broken token refs |
| Typography compliance | 98/100 | Minor size deviations (9px, 18px) |
| Color system compliance | 85/100 | ~90 raw rgba() values, 3 broken token refs |
| Layout pattern compliance | 95/100 | Section rules and margin grids well-adopted |
| Type alignment (FRONTEND-TYPES.md) | Good | 3 phantom fields, 1 type conflation, 1 triplication |
| Hook architecture (FRONTEND-HOOKS.md) | Good | No caching layer, cross-page staleness, 6 missing hook wrappers |
| Event cleanup discipline | Excellent | All subscriptions properly cleaned up |
| Inline style prohibition | 100/100 | Zero violations |

### Issue Distribution

| Priority | Count | Lines Affected |
|----------|------:|---------------:|
| P0 (user-facing bugs) | 4 | ~100 |
| P1 (structural, blocks features) | 7 | ~2,500 |
| P2 (technical debt) | 8 | ~8,000 |
| **Total** | **19** | **~10,600** |

### Cross-Cutting Consolidation

Several findings from different audits collapse into single cleanup tasks:

| Combined Task | Components Audit | Hooks Audit | Types Audit | Styles Audit |
|--------------|-----------------|-------------|-------------|-------------|
| Delete `email-signal-list.tsx` + dead email types | Ghost component | -- | `EmailSummaryData`, `EmailStats` dead | -- |
| Delete `status-badge.tsx` + unused entity tokens | Ghost component | -- | -- | 5 unused `--color-entity-*` tokens |
| Delete `sidebar.tsx` + sidebar CSS vars | Ghost component | -- | -- | 8 dead `--sidebar-*` variables |
| Fix action staleness | Action called from 4 locations | Missing `useActionMutation` | -- | -- |
| Entity detail unification | 80% duplicate structure across 3 pages | 60% duplicate patterns across 3 hooks | Inline type duplication (`signals`) | -- |
| Report page unification | 70% duplicate structure across 5 pages | `save_report` called from 5 pages with no hook | `toArr()` duplicated in 5 pages | -- |
