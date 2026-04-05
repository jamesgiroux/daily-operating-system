# Page Architecture

**Last audited:** 2026-03-15

Every page in DailyOS has a stated job (ADR-0084). This document maps each page's intended structure against what's actually built, its JTBD (Job To Be Done), data sources, and key state patterns.

---

## 1. Daily Briefing (Dashboard)

**Route:** `/`
**File:** `src/components/dashboard/DailyBriefing.tsx` (rendered by `DashboardPage` in `router.tsx`)
**JTBD:** Morning situational awareness. Read top-to-bottom in 2-5 minutes. Know what matters before opening any other tool.
**Atmosphere:** Turmeric + Larkspur
**CSS:** `src/styles/editorial-briefing.module.css` (BEST PRACTICE -- use this as reference)

### Data Sources

- `get_dashboard_data` IPC command (returns `DashboardData` with schedule, actions, emails, freshness)
- `get_workflow_status` IPC command (workflow run state)
- Tauri events: `workflow-completed`, `calendar-updated`, `prep-ready`, `entity-updated`, `emails-updated`

### Structure

```
1. Day Frame (Hero + Focus merged)
   - 1-sentence AI narrative
   - Capacity: X hours free, N meetings
   - Focus: where to direct non-meeting energy

2. Schedule
   - Full temporal map
   - Up Next: first upcoming meeting expanded by default
   - Past meetings: outcomes summary

3. Attention (Review + Priorities merged)
   - Proposed action triage (high-signal, capped)
   - 2-3 actions: meeting-relevant or overdue
   - 3-4 urgent emails

4. Finis
```

### Current State: MOSTLY ALIGNED

| Section | Status | Notes |
|---------|--------|-------|
| Day Frame | Done | Hero + Focus merged. Capacity line present. |
| Schedule | Done | BriefingMeetingCard with Up Next expansion. |
| Attention | Done | AttentionSection replaces separate Review + Priorities. |
| Finis | Done | FinisMarker present. |

### Key State Patterns

- `useDashboardData()` -- discriminated union: loading | error | empty | success
- `useWorkflow()` -- workflow run status, manual trigger
- Auto-refresh on window focus (1-minute debounce), auto-refresh on Tauri events (300ms debounce)
- Loading: `DashboardSkeleton`, Error: `DashboardError`, Empty: `DashboardEmpty`

---

## 2. Weekly Forecast

**Route:** `/week`
**File:** `src/pages/WeekPage.tsx`
**JTBD:** Week-shape planning. Understand the topology of your week, identify priorities, find deep work windows.
**Atmosphere:** Larkspur
**CSS:** `src/pages/WeekPage.module.css`

### Data Sources

- `get_calendar_events` IPC command (week's calendar events)
- `get_executive_intelligence` IPC command (priorities, narrative)
- `weekPageViewModel.ts` -- transforms raw data into view model (tested)

### Structure

```
1. Hero (week narrative)
2. The Three (force-ranked priorities)
3. The Shape (multi-day density)
4. Meeting Intelligence Timeline (+-7 days) -- I330
5. Finis
```

### Current State: ALIGNED

| Section | Status | Notes |
|---------|--------|-------|
| Hero | Done | Week narrative, Newsreader 76px. |
| The Three | Done | Force-ranked priorities. |
| The Shape | Done | Day-by-day density rendering. |
| Timeline | Done (0.13.0) | +-7-day meeting timeline with IntelligenceQualityBadge. |
| Finis | Done | FinisMarker present. |

### Key State Patterns

- `useCalendar()` -- calendar event loading with Tauri event-driven silent refresh
- `useExecutiveIntelligence()` -- executive narrative and priorities
- View model pattern: `weekPageViewModel.ts` transforms raw data, tested independently
- Uses `useRegisterMagazineShell()` for shell configuration

---

## 3. Meeting Briefing

**Route:** `/meeting/$meetingId`
**File:** `src/pages/MeetingDetailPage.tsx`
**JTBD:** Brief you before a meeting (what to know, who's in the room, what to prepare) OR close it out after (outcomes, wins, risks, next actions).
**Atmosphere:** Turmeric (intense, focused)
**CSS:** `src/pages/meeting-intel.module.css`

### Data Sources

- `get_meeting_prep` IPC command (returns `FullMeetingPrep` with frozen prep data)
- `get_meeting_intelligence` IPC command (post-meeting intelligence)
- `get_meeting_outcomes` IPC command (user-captured outcomes)
- Tauri events: `prep-ready`, `intelligence-updated`

### Structure

```
[Post-meeting: Outcomes section at top]

1. The Brief
   - Key insight pull quote
   - Meeting metadata + entity chips
   - Before This Meeting (merged readiness + actions)

2. The Risks

3. The Room
   - Attendees with disposition, assessment, engagement

4. Your Plan
   - AI-proposed + user agenda items

5. Finis
```

### Current State: MOSTLY ALIGNED

| Section | Status | Notes |
|---------|--------|-------|
| Outcomes | Done | Summary, wins, risks, actions for past meetings. |
| The Brief | Done | Key insight, metadata, entity chips. |
| The Risks | Done | Risk rows. |
| The Room | Done | Attendee list with rich context. |
| Your Plan | Done | Agenda editor with draft/prefill. |
| Finis | Done | FinisMarker present. |

### Key State Patterns

- Direct `invoke()` calls (no dedicated hook -- page manages its own state)
- Chapter navigation via `CHAPTERS` constant + `useChapterObserver()`
- Scroll-linked reveals via `useRevealObserver()`
- `useAgendaDraft()` for AI agenda draft dialog
- `useIntelligenceFeedback()` for thumbs up/down feedback
- `useTauriEvent()` for `prep-ready` silent refresh

---

## 4. Meeting History Detail

**Route:** `/meeting/history/$meetingId`
**File:** `src/pages/MeetingHistoryDetailPage.tsx`
**JTBD:** Legacy redirect. Routes old meeting history URLs to the unified meeting detail page.

### Data Sources

None -- pure redirect component.

### Current State: COMPLETE

Redirects to `/meeting/$meetingId` via `<Navigate>`. No UI rendered.

---

## 5. Actions

**Route:** `/actions`
**File:** `src/pages/ActionsPage.tsx`
**JTBD:** Commitment inventory. Triage suggested actions, track pending work, review completions.
**Atmosphere:** Terracotta
**CSS:** `src/pages/ActionsPage.module.css`

### Data Sources

- `useActions()` hook -- loads from `get_actions_from_db` IPC command
- `useProposedActions()` hook -- loads suggested actions

### Structure

```
Tabs: Suggested | Pending | Completed

Suggested (default when suggestions exist):
  - Triage queue

Pending (primary view):
  - Temporal grouping: Overdue / This Week / Later

Completed:
  - Record of progress
```

### Current State: PARTIAL

| Section | Status | Notes |
|---------|--------|-------|
| Tab structure | Done | Suggested/Pending/Completed. |
| Smart default | Done | Defaults to Suggested when suggestions exist. |
| Temporal grouping | Done | Overdue / This Week / Later on pending tab. |
| Meeting-centric view | NOT DONE | Deferred. Currently temporal only. |
| FinisMarker | Done | FinisMarker present. |

### Key State Patterns

- `useActions(initialSearch)` -- filter state (status, priority, search), CRUD operations
- `useProposedActions()` -- proposed action loading and accept/dismiss
- Search query from URL search params (`?search=...`)
- Inline action creation form

---

## 6. Action Detail

**Route:** `/actions/$actionId`
**File:** `src/pages/ActionDetailPage.tsx`
**JTBD:** Execute and manage a single committed action. Edit context, priority, due date, entity links.
**Atmosphere:** Terracotta
**CSS:** `src/pages/ActionDetailPage.module.css`

### Data Sources

- `get_action_detail` IPC command
- `update_action_field` IPC command (field-by-field save)
- `complete_action` / `reopen_action` IPC commands

### Structure

```
1. Title band (status toggle + editable title)
2. Priority + Status strip (priority pill, status, waiting-on, source badge)
3. Priority picker
4. Context (editable textarea)
5. Reference (account, due date, created, completed, source)
6. Action bar (save status, mark complete/reopen)
Finis
```

### Current State: ALIGNED

| Section | Status | Notes |
|---------|--------|-------|
| Title band | Done | Inline-editable title with status toggle. |
| Priority strip | Done | Priority pill, status, waiting-on badge. |
| Context | Done | Editable textarea with auto-generated note. |
| Reference | Done | EntityPicker, Calendar date, source. |
| Finis | Done | FinisMarker present. |

### Key State Patterns

- Direct `invoke()` calls for load and save (no dedicated hook)
- Field-level save with debounce
- Uses `EditableInline`, `EditableTextarea`, `EditableDate` shared components

---

## 7. Accounts List

**Route:** `/accounts`
**File:** `src/pages/AccountsPage.tsx`
**JTBD:** Browse and find accounts. Portfolio overview with health indicators.
**Atmosphere:** Turmeric

### Data Sources

- `get_accounts` IPC command (returns list with health scores)

### Structure

```
1. Hero (page title, account count)
2. Search + filters
3. Entity list (EntityListShell + EntityRow)
Finis
```

### Current State: ALIGNED

Uses shared `EntityListShell` + `EntityRow` components from I223.

### Key State Patterns

- `useRegisterMagazineShell()` for shell configuration
- Local search/filter state
- Click-through navigation to `/accounts/$accountId`

---

## 8. Account Detail

**Route:** `/accounts/$accountId`
**File:** `src/pages/AccountDetailEditorial.tsx`
**JTBD:** Relationship dossier. Full picture to show up informed at any meeting about this account.
**Atmosphere:** Turmeric (warm, entity identity)
**CSS:** `src/pages/AccountDetailEditorial.module.css`

### Data Sources

- `useAccountDetail(accountId)` orchestrator hook, which composes:
  - `useAccountFields()` -- field editing, save/cancel
  - `useTeamManagement()` -- search, add, remove, inline create
  - `useEnrichmentProgress()` -- progressive enrichment tracking
- `get_account_detail` IPC command (returns `AccountDetail`)
- `get_entity_files`, `get_account_events` IPC commands (supplementary data)
- Tauri events: `intelligence-updated`, `content-changed`

### Structure

```
1. Hero (name, assessment lede, health/lifecycle badges)
2. Vitals Strip (ARR, health, lifecycle, renewal, NPS, meeting frequency)
3. State of Play (working/struggling)
4. The Room (stakeholders + your team)
5. Watch List (risks/wins/unknowns)
6. The Work (upcoming meetings + actions)
7. The Record (unified timeline)
8. Appendix (lifecycle events, company context, BUs, notes, files)
9. Finis
```

### Current State: MOSTLY ALIGNED

| Section | Status | Notes |
|---------|--------|-------|
| Hero | Done | AccountHero with CSS module. |
| Vitals | Done | VitalsStrip component. |
| State of Play | Done | StateOfPlay component. |
| The Room | Done | StakeholderGallery with inline editing. |
| Watch List | Done | WatchList + WatchListPrograms. |
| The Work | Done | TheWork component. |
| The Record | Done | UnifiedTimeline. |
| Appendix | Done | AccountAppendix (reduced). |
| Finis | Done | FinisMarker present. |

### Key State Patterns

- `useAccountDetail(accountId)` -- orchestrator hook with flat public API
- Composed sub-hooks: `useAccountFields`, `useTeamManagement`, `useEnrichmentProgress`
- `load()` (with loading spinner) vs `silentRefresh()` (background, no spinner)
- Chapter navigation via `CHAPTERS` constant + `useChapterObserver()`
- Tauri event listeners for `intelligence-updated` and `content-changed`

---

## 9. Risk Briefing

**Route:** `/accounts/$accountId/risk-briefing`
**File:** `src/pages/RiskBriefingPage.tsx`
**JTBD:** Slide-deck risk report for escalation. Generated from account intelligence, one idea per slide.
**Layout:** Scroll-snap sections (not standard magazine layout)
**Atmosphere:** Turmeric

### Data Sources

- `get_reports` IPC command (loads existing report)
- `generate_report` IPC command (triggers AI generation)

### Structure

```
1. Cover (entity name, date)
2. Bottom Line (executive summary)
3. What Happened (events driving risk)
4. The Stakes (what's at risk)
5. The Ask (recommended action)
6. The Plan (action plan)
7. Finis
```

### Current State: COMPLIANT

Best performer in design audits. Clean token usage, consistent patterns throughout.

### Key State Patterns

- Direct `invoke()` calls for report load/generate
- Slide-deck layout with scroll-snap and keyboard navigation (arrow keys)
- Loading: Skeleton, Empty: Generate button, Generating: `GeneratingProgress`
- `useIntelligenceFeedback()` for quality feedback

---

## 10. Account Health Report

**Route:** `/accounts/$accountId/reports/account_health`
**File:** `src/pages/AccountHealthPage.tsx`
**JTBD:** Pre-renewal health assessment. 5-slide editorial deck for internal review.
**Atmosphere:** Turmeric
**Layout:** Scroll-snap slide deck with keyboard navigation (1-5, arrow keys)

### Data Sources

- `get_reports` IPC command (loads existing report)
- `generate_report` IPC command (triggers AI generation)

### Structure

```
1. Cover (account name, overall assessment, health narrative)
2. The Partnership (relationship summary, engagement cadence, quote)
3. Where We Stand (working/struggling, expansion signals)
4. Value Delivered (value items, risks)
5. What's Ahead (renewal context, recommended actions)
Finis
```

### Current State: ALIGNED

| Section | Status | Notes |
|---------|--------|-------|
| All slides | Done | 5 dedicated slide components. |
| Inline editing | Done | All slides support content editing with debounced save. |
| Finis | Done | FinisMarker present. |

### Key State Patterns

- Report load/generate via `invoke()`
- Inline editing with debounced `save_report_content` calls
- Keyboard navigation (1-5 and arrow keys for slide jumping)
- Three states: Skeleton loading, GeneratingProgress, rendered content

---

## 11. EBR/QBR Report

**Route:** `/accounts/$accountId/reports/ebr_qbr`
**File:** `src/pages/EbrQbrPage.tsx`
**JTBD:** 7-slide quarterly business review deck. Customer-facing presentation material.
**Atmosphere:** Larkspur
**Layout:** Scroll-snap slide deck with keyboard navigation (1-7, arrow keys)

### Data Sources

- `get_reports` IPC command (loads existing report)
- `generate_report` IPC command (triggers AI generation)

### Structure

```
1. Cover (account name, quarter label, executive summary)
2. The Story (story bullets, customer quote)
3. Value Delivered (value items)
4. By the Numbers (success metrics)
5. What We Navigated (challenges and resolutions)
6. What's Ahead (strategic roadmap)
7. Next Steps (action items)
Finis
```

### Current State: ALIGNED

Same pattern as Account Health. All slides done, inline editing done.

---

## 12. SWOT Report

**Route:** `/accounts/$accountId/reports/swot`
**File:** `src/pages/SwotPage.tsx`
**JTBD:** 5-slide strategic SWOT assessment. Account-scoped analysis for planning.
**Atmosphere:** Olive (sage accent)
**Layout:** Scroll-snap slide deck with keyboard navigation (1-5, arrow keys)

### Data Sources

- `get_reports` IPC command (loads existing report)
- `generate_report` IPC command (triggers AI generation)

### Structure

```
1. Cover (account name, summary)
2. Strengths (QuadrantSlide, sage accent)
3. Weaknesses (QuadrantSlide, turmeric accent)
4. Opportunities (QuadrantSlide, larkspur accent)
5. Threats (QuadrantSlide, terracotta accent)
Finis
```

### Current State: ALIGNED

Shared QuadrantSlide with per-quadrant accent color. All quadrants support inline editing.

---

## 13. Generic Report (Fallback)

**Route:** `/accounts/$accountId/reports/$reportType` and `/me/reports/$reportType`
**File:** `src/pages/ReportPage.tsx`
**JTBD:** Render any report by entity + type combination. Fallback renderer for report types without dedicated pages.
**CSS:** `src/pages/report-page.module.css`

### Data Sources

- `get_reports` IPC command
- `generate_report` IPC command
- User entity lookup for `/me/reports/` routes

### Structure

```
1. ReportShell (title, generate/regenerate, entity context)
2. Type-specific renderer:
   - swot -> SwotReport
   - account_health -> AccountHealthReport
   - ebr_qbr -> EbrQbrReport
   - default -> JSON pretty-print
```

### Current State: ALIGNED

Delegates rendering to ReportShell and type-specific components.

---

## 14. Emails

**Route:** `/emails`
**File:** `src/pages/EmailsPage.tsx`
**JTBD:** Email intelligence surface. Surfaces important emails with entity context, signal extraction, and actionable insights.
**Atmosphere:** Turmeric
**CSS:** `src/pages/EmailsPage.module.css`, `src/styles/editorial-briefing.module.css` (shared)

### Data Sources

- `get_email_briefing_data` IPC command (enriched email data)
- `get_email_sync_stats` IPC command (sync status)
- `refresh_emails` IPC command (manual refresh)
- Tauri events: `emails-updated`

### Structure

```
1. Hero (title, sync stats)
2. Signal section (extracted email signals)
3. Email threads (grouped by importance/entity)
4. Entity-filtered views
Finis
```

### Current State: GOOD

Uses margin grid pattern, section rules, FinisMarker. Good compliance overall.

### Key State Patterns

- Direct `invoke()` calls (no dedicated hook)
- `usePersonality()` for personality-driven copy
- `useTauriEvent()` for `emails-updated` silent refresh
- `useTransition()` for non-blocking state updates
- Dismissed emails tracked via local `Set<string>` state

---

## 15. Inbox (File Drop Zone)

**Route:** `/inbox`
**File:** `src/pages/InboxPage.tsx`
**JTBD:** Route and classify uploaded files. Drop zone for documents that get processed, classified, and linked to entities.
**Atmosphere:** Olive

### Data Sources

- `useInbox()` hook -- file list with live updates from file watcher
- Tauri events: `inbox-updated`

### Structure

```
1. Hero (title, file count, drop zone)
2. File list (expandable rows with status, classification, routing)
3. Processing results (entity linking, enrichment prompts)
Finis
```

### Current State: ALIGNED

| Section | Status | Notes |
|---------|--------|-------|
| Hero | Done | Title with file count. |
| Drop zone | Done | Drag-and-drop + file browse. Google Drive import modal. |
| File list | Done | Expandable rows, status, classification. |
| Processing | Done | Inline entity picker for routing. |
| Finis | Done | FinisMarker present. |

### Key State Patterns

- `useInbox()` -- file list, count, loading, error, refresh
- `useTauriEvent()` for live inbox-updated events
- File classification and routing via `invoke()` calls

---

## 16. History

**Route:** `/history`
**File:** `src/pages/HistoryPage.tsx`
**JTBD:** File processing log. Review past inbox activity, see what was classified and where it went.
**Atmosphere:** Olive
**CSS:** `src/pages/HistoryPage.module.css`

### Data Sources

- `get_processing_history` IPC command

### Structure

```
1. Hero (title, entry count, thick rule)
2. Column headers (FILE, CLASS, STATUS, DESTINATION, TIME)
3. Entry rows (grid layout, error expansion)
Finis
```

### Current State: ALIGNED

| Section | Status | Notes |
|---------|--------|-------|
| Hero | Done | Serif title, entry count, thick rule. |
| Column headers | Done | Mono uppercase grid headers. |
| Entry rows | Done | Grid layout with status dots, error expansion. |
| Empty state | Done | EmptyState with personality-driven copy. |
| Finis | Done | FinisMarker present. |

### Key State Patterns

- Direct `invoke()` call on mount
- Local state: loading, error, history entries
- EmptyState with "Go to inbox" CTA

---

## 17. People List

**Route:** `/people`
**File:** `src/pages/PeoplePage.tsx`
**JTBD:** Browse and find people. Relationship overview with filters for internal/external/unknown.

### Data Sources

- `get_people` IPC command
- URL search params: `?relationship=all|external|internal|unknown` and `?hygiene=unnamed|duplicates`

### Structure

```
1. Hero (page title, count)
2. Relationship tabs (All / External / Internal / Unknown)
3. Hygiene filters (Unnamed / Duplicates)
4. Search
5. Entity list (EntityListShell + EntityRow)
Finis
```

### Current State: ALIGNED

Uses shared EntityListShell + EntityRow. URL-driven filter state.

### Key State Patterns

- `useRegisterMagazineShell()` for shell configuration
- Filter state from URL search params (`validateSearch`)
- Local search state

---

## 18. Person Detail

**Route:** `/people/$personId`
**File:** `src/pages/PersonDetailEditorial.tsx`
**JTBD:** Relationship dossier for an individual. Meeting history, connection network, relationship analysis.
**Accent:** Larkspur
**CSS:** `src/pages/PersonDetailEditorial.module.css`

### Data Sources

- `usePersonDetail(personId)` orchestrator hook
- `get_person_detail` IPC command
- Tauri events: `intelligence-updated`

### Structure

```
1. Hero (name, assessment, temperature, email, social)
2. Vitals (temperature, meeting frequency, last met, meeting count)
3. The Dynamic/Rhythm (relationship analysis)
4. The Network (connected entities)
5. The Landscape (Watch List)
6. The Work (actions)
7. The Record (timeline)
8. Appendix (profile, notes, files)
9. Finis
```

### Current State: ALIGNED

All sections present. FinisMarker present.

### Key State Patterns

- `usePersonDetail(personId)` -- orchestrator with load/silentRefresh, field editing, merge, delete, enrichment, entity linking
- Merge flow: search dialog -> confirm dialog -> navigate to merged person
- Duplicate candidate detection and suggested merges
- Chapter navigation via `useChapterObserver()`

---

## 19. Projects List

**Route:** `/projects`
**File:** `src/pages/ProjectsPage.tsx`
**JTBD:** Browse and find projects. Initiative overview with status indicators.

### Data Sources

- `get_projects` IPC command

### Structure

```
1. Hero (page title, count)
2. Search
3. Entity list (EntityListShell + EntityRow)
Finis
```

### Current State: ALIGNED

Uses shared EntityListShell + EntityRow components.

### Key State Patterns

- `useRegisterMagazineShell()` for shell configuration
- Local search/filter state

---

## 20. Project Detail

**Route:** `/projects/$projectId`
**File:** `src/pages/ProjectDetailEditorial.tsx`
**JTBD:** Initiative dossier. Momentum, milestones, team, timeline for a tracked project.
**Accent:** Olive
**CSS:** `src/pages/ProjectDetailEditorial.module.css`

### Data Sources

- `useProjectDetail(projectId)` orchestrator hook
- `get_project_detail` IPC command
- Tauri events: `intelligence-updated`

### Structure

```
1. Hero (name, assessment, status, owner)
2. Vitals (status, days to target, milestone progress, meeting frequency)
3. Trajectory (momentum/headwinds)
4. The Horizon (milestones, timeline risk, decisions)
5. The Landscape (Watch List)
6. The Team (stakeholders)
7. The Work (actions)
8. The Record (timeline)
9. Appendix (milestones full list, description, notes, files)
10. Finis
```

### Current State: MOSTLY ALIGNED

All sections present. Follows entity skeleton correctly.

### Key State Patterns

- `useProjectDetail(projectId)` -- orchestrator with load/silentRefresh, field editing, enrichment, archive, child project creation
- Chapter navigation via `useChapterObserver()`
- Tauri event listener for `intelligence-updated`

---

## 21. Settings

**Route:** `/settings`
**File:** `src/pages/SettingsPage.tsx`
**JTBD:** Connections hub. Manage integrations, view system health, configure data governance.
**Layout:** Magazine layout, 900px max-width
**CSS:** `src/pages/SettingsPage.module.css`

### Data Sources

- `get_config` IPC command (app configuration)
- `useGoogleAuth()` hook (Google OAuth status)
- `useGleanAuth()` hook (Glean connection status)
- `useClaudeStatus()` hook (Claude Code connection)
- URL search params: `?tab=you|connectors|system|diagnostics`

### Structure

```
Tabs: You | Connectors | System | Diagnostics

You:
  - YouCard (user profile summary)

Connectors:
  - ConnectorsGrid with per-connector cards
  - ConnectorDetail for individual connection config

System:
  - DataPrivacySection (data purge, source management)
  - ActivityLogSection (audit log viewer)
  - DatabaseRecoveryCard
  - ContextSourceSection

Diagnostics:
  - DiagnosticsSection (system health)
  - SystemStatus (backend status)
```

### Current State: ALIGNED

Redesigned per I349 as connections hub. ChapterHeading sections. FinisMarker present.

### Key State Patterns

- Tab state from URL search params
- Multiple connection status hooks: `useGoogleAuth()`, `useGleanAuth()`, `useClaudeStatus()`
- Per-connector configuration components in `settings/connectors/`
- `useAppState()` for demo mode and setup resumption

---

## 22. Me (User Profile)

**Route:** `/me`
**File:** `src/pages/MePage.tsx`
**JTBD:** Professional context + priorities. Everything you tell DailyOS about yourself shapes every briefing it produces. Zero-guilt priority model.
**Atmosphere:** Eucalyptus
**CSS:** `src/pages/MePage.module.css`

### Data Sources

- `useMe()` orchestrator hook
- `get_user_entity` IPC command (user profile data)
- `get_user_context_entries` IPC command (embedded knowledge)
- Tauri events: `user-entity-updated`

### Structure (ADR-0089/0090)

```
1. Hero (name, title at company)
2. About Me (name, title, company, focus, bio, role, metrics)
3. What I Deliver (value prop, success, product, pricing, differentiators, objections, competitive)
4. My Priorities (annual + quarterly, zero-guilt model)
5. My Playbooks (CS presets or methodology)
6. Context (ContextEntryList -- embedded knowledge)
7. Attachments (drop zone for documents)
Finis
```

### Current State: ALIGNED

| Section | Status | Notes |
|---------|--------|-------|
| All sections | Done | Full implementation with CSS module. |
| Finis | Done | FinisMarker present. |
| Folio actions | Done | Links to Weekly Impact and Monthly Wrapped reports. |

### Key State Patterns

- `useMe()` -- orchestrator with load, saveField, context entry CRUD
- Optimistic updates on field save (immediate local state, async backend)
- `saving` flag for save-in-progress indication
- Tauri event listener for `user-entity-updated`

---

## 23. Weekly Impact Report

**Route:** `/me/reports/weekly_impact`
**File:** `src/pages/WeeklyImpactPage.tsx`
**JTBD:** 5-slide weekly reflection. Personal impact report for the past week.
**Atmosphere:** Eucalyptus
**Layout:** Scroll-snap slide deck with keyboard navigation (1-5, arrow keys)

### Data Sources

- `get_reports` IPC command (loads existing report)
- `generate_report` IPC command (triggers AI generation)

### Structure

```
1. Cover (week label, stats, headline)
2. Priorities Moved (what moved forward)
3. The Work (wins, what you did)
4. Watch (items needing attention)
5. Into Next Week (carry-forward items)
Finis
```

### Current State: ALIGNED

All slides done, inline editing done.

### Key State Patterns

- Report load/generate via `invoke()`
- Inline editing with debounced save
- `useIntelligenceFeedback()` for quality feedback
- Keyboard navigation (1-5, arrow keys)

---

## 24. Monthly Wrapped

**Route:** `/me/reports/monthly_wrapped`
**File:** `src/pages/monthly-wrapped/MonthlyWrappedPage.tsx`
**JTBD:** Spotify Wrapped-style monthly celebration. Carousel of animated stats, top accounts, key moments.
**Atmosphere:** Eucalyptus
**Layout:** Full-viewport scroll-snap slides with CSS animations

### Data Sources

- `get_reports` IPC command (loads existing report)
- `generate_report` IPC command (triggers AI generation)

### Structure

```
1. Cover (month label, total conversations/entities)
2. By the Numbers (stat cards with animated count-up)
3. Top Accounts (most-touched accounts)
4. Meeting Rhythm (heat calendar, personality type)
5. Key Moments (defining moments grid)
6. Biggest Win (celebration slide)
7. Challenges (what tested you)
8. Actions Impact (completed count, carry-forward)
9. Looking Ahead (next month focus)
10. Sign-Off (closing message)
```

### Current State: ALIGNED

| Section | Status | Notes |
|---------|--------|-------|
| All 10 slides | Done | Per-slide components in `src/pages/monthly-wrapped/`. |
| Animations | Done | CSS keyframe animations, AnimatedNumber count-up. |
| Inline editing | Done | Content editing with debounced save. |
| Finis | n/a | Sign-off slide serves as closing. |

### Key State Patterns

- Report load/generate via `invoke()`
- AnimatedNumber helper for count-up animations
- Decomposed: orchestrator (~290 lines) + 10 per-slide components
- Shared CSS module for slide styles

---

## 25. Book of Business

**Route:** `/me/reports/book_of_business`
**File:** `src/pages/BookOfBusinessPage.tsx`
**JTBD:** Slide-deck portfolio review for leadership. Account health rollup with spotlight deep-dives.
**Atmosphere:** Turmeric
**Layout:** Scroll-snap slide deck
**CSS:** `src/pages/book-of-business.module.css`, `src/pages/report-slides.module.css`

### Data Sources

- `get_reports` IPC command (loads existing report)
- `generate_report` IPC command (triggers AI generation)
- `get_accounts` IPC command (account list for generation context)

### Structure

```
1. Cover (vitals strip, executive summary)
2. Attention (risks and opportunities)
3. Spotlight (per-account deep dive, one per slide)
4. Value Themes (value delivered + cross-portfolio themes)
5. The Ask (leadership asks, conditional)
Finis
```

### Current State: ALIGNED

All slides done with dedicated components.

### Key State Patterns

- Normalization layer guards against cached reports with old schema
- Dynamic slide count (spotlight slides generated per deep-dive account)
- `useIntelligenceFeedback()` for quality feedback
- `useRevealObserver()` for scroll-linked reveals

---

## Compliance Summary

| Page | Structure | Tokens | Typography | Layout | Vocab | Finis | Overall |
|------|-----------|--------|------------|--------|-------|-------|---------|
| Daily Briefing | A | A | A | A | B | A | **A** |
| Weekly Forecast | A | A | A | A | B | A | **A** |
| Meeting Detail | A | B | A | B | B | A | **B+** |
| Meeting History | n/a | n/a | n/a | n/a | n/a | n/a | Redirect |
| Actions | B | A | A | B | A | A | **A-** |
| Action Detail | A | A | A | A | A | A | **A** |
| Accounts List | A | A | A | A | B | A | **A** |
| Account Detail | A | A | A | A- | B | A | **A-** |
| Risk Briefing | A | A | A | A | A | A | **A+** |
| Account Health | A | A | A | A | A | A | **A** |
| EBR/QBR | A | A | A | A | A | A | **A** |
| SWOT | A | A | A | A | A | A | **A** |
| Generic Report | A | A | A | A | A | n/a | **A** |
| Emails | A | A | A | A | B | A | **A** |
| Inbox | A | B | A | B | A | A | **B+** |
| History | A | A | A | A | A | A | **A** |
| People List | A | A | A | A | B | A | **A** |
| Person Detail | A | A | A | A- | B | A | **A-** |
| Projects List | A | A | A | A | B | A | **A** |
| Project Detail | A | A | A | A- | B | A | **A-** |
| Settings | A | A | A | A | A | A | **A** |
| Me | A | A | A | A | A | A | **A** |
| Weekly Impact | A | A | A | A | A | A | **A** |
| Monthly Wrapped | A | A | A | A | A | n/a | **A** |
| Book of Business | A | A | A | A | A | A | **A** |
