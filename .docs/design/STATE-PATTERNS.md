# State Patterns

**Last audited:** 2026-03-15

This document describes the state management patterns used across DailyOS pages: what hooks each page uses, how data flows, loading/error/empty state handling, and refresh patterns.

---

## Core Patterns

### 1. Orchestrator Hook Pattern

**Used by:** Account Detail, Person Detail, Project Detail, Me, Dashboard

The dominant pattern for complex pages. A single orchestrator hook encapsulates all state for a page: loading, error, field editing, enrichment, sub-entity management. The hook returns a flat public API that the page component destructures.

```
Page Component
  └── useXxxDetail(id)          <- orchestrator hook
       ├── useState (core data, loading, error)
       ├── useXxxFields()       <- composed sub-hook (field editing)
       ├── useTeamManagement()  <- composed sub-hook (team CRUD)
       ├── useEnrichmentProgress() <- composed sub-hook
       ├── Event listeners (intelligence-updated, content-changed)
       └── Handler functions (save, enrich, archive, create)
```

**Orchestrator hooks:**

| Hook | Page | Key responsibilities |
|------|------|---------------------|
| `useAccountDetail(accountId)` | Account Detail | Core data, field editing (`useAccountFields`), team management (`useTeamManagement`), enrichment (`useEnrichmentProgress`), lifecycle events, programs, file indexing, action creation |
| `usePersonDetail(personId)` | Person Detail | Core data, field editing, enrichment, entity linking, merge flow, duplicate detection, archive, file indexing |
| `useProjectDetail(projectId)` | Project Detail | Core data, field editing, enrichment, archive, child project creation, file indexing |
| `useMe()` | Me | Core data, field-level save with optimistic updates, context entry CRUD |
| `useDashboardData()` | Dashboard | Discriminated union state (loading/error/empty/success), auto-refresh |

**Conventions:**
- `load()` -- full load with loading spinner (initial navigation)
- `silentRefresh()` -- background refresh, no loading state, no scroll reset
- `fetchDetail(showLoading: boolean)` -- internal shared method
- Sub-hooks are an internal concern; the page gets a flat return object

### 2. Direct Invoke Pattern

**Used by:** Meeting Detail, Action Detail, Emails, History, all Report pages

Simpler pages that manage their own state via direct `invoke()` calls without a dedicated orchestrator hook. State is managed with `useState` directly in the page component.

```
Page Component
  ├── useState (data, loading, error)
  ├── invoke("get_xxx") on mount
  ├── invoke("update_xxx") on user action
  └── useTauriEvent() for live updates
```

**When to use:** Pages with straightforward load-display-edit cycles that don't justify a separate hook file.

### 3. Filter/List Pattern

**Used by:** Actions, Accounts List, People List, Projects List

List pages with filter state, search, and navigation to detail views.

```
Page Component
  ├── useXxx() hook (data loading, CRUD)
  ├── Filter state (status, priority, search query)
  ├── URL search params for filter persistence
  └── EntityListShell + EntityRow for rendering
```

**Conventions:**
- URL search params via `validateSearch` in route definition
- Local filter/search state that doesn't persist across navigations
- `useRegisterMagazineShell()` for shell configuration

### 4. Report Pattern

**Used by:** Risk Briefing, Account Health, EBR/QBR, SWOT, Weekly Impact, Monthly Wrapped, Book of Business

All report pages follow the same state machine:

```
States: loading -> empty | content
  empty: Generate button -> generating (GeneratingProgress) -> content
  content: Regenerate action in folio -> generating -> content

Page Component
  ├── invoke("get_reports") on mount
  ├── invoke("generate_report") on generate
  ├── Inline editing with debounced invoke("save_report_content")
  ├── Keyboard navigation (number keys + arrow keys)
  └── useIntelligenceFeedback() for quality rating
```

---

## Shared State Infrastructure

### Shell Registration (`useMagazineShell`)

Every editorial page registers its shell configuration via `useRegisterMagazineShell()`:

```typescript
useRegisterMagazineShell({
  folioLabel: "Account",           // FolioBar publication label
  atmosphereColor: "turmeric",     // Background gradient color
  activePage: "accounts",          // FloatingNavIsland highlight
  backLink: { label: "Accounts", onClick: () => navigate({ to: "/accounts" }) },
  chapters: CHAPTERS,              // Chapter nav definitions
  folioActions: <RefreshButton />, // FolioBar right-side actions
  folioDateText: "...",            // FolioBar center date
  folioReadinessStats: [...],      // FolioBar readiness indicators
  folioStatusText: ">_ ready",    // FolioBar status
});
```

**Volatile folio state** (I563): For frequently-changing folio content (enrichment progress, save status), use `useUpdateFolioVolatile()` instead of re-registering the full config. This updates a ref without triggering shell re-renders.

### Tauri Event Listeners

Two patterns for listening to backend events:

1. **`useTauriEvent(eventName, callback)`** -- shared hook, auto-cleanup on unmount. Used for simple event-driven refreshes.

2. **`listen()` from `@tauri-apps/api/event`** -- direct API usage with manual cleanup. Used when event payload filtering is needed (e.g., filtering by `entityId`).

**Common events:**

| Event | Emitter | Consumers |
|-------|---------|-----------|
| `workflow-completed` | Workflow engine | Dashboard |
| `workflow-status-${name}` | Workflow engine | `useWorkflow()` hook |
| `calendar-updated` | Google Calendar sync | Dashboard, Week |
| `prep-ready` | Meeting prep queue | Dashboard, Meeting Detail |
| `entity-updated` | Intel queue | Dashboard |
| `emails-updated` | Email poller | Dashboard, Emails |
| `intelligence-updated` | Intel queue | Account/Person/Project Detail |
| `content-changed` | File watcher | Account Detail |
| `inbox-updated` | File watcher | Inbox |
| `user-entity-updated` | User entity save | Me |
| `background-work-status` | Intel queue | MagazinePageLayout (status indicator) |
| `config-updated` | Settings save | MagazinePageLayout (entity mode) |

### Data Freshness & Auto-Refresh

**Dashboard auto-refresh** (the most sophisticated):
- Window focus: silent refresh after 60-second debounce
- Tauri events: 300ms debounce to coalesce burst events
- Generation counter to discard stale responses from fast navigation
- `useTransition()` for non-blocking silent refresh updates

**Entity detail refresh:**
- Tauri event listener for `intelligence-updated` with entity ID filtering
- `silentRefresh()` preserves scroll position and loading state

**Calendar:**
- `useTauriEvent("calendar-updated")` triggers silent fetch
- `useTransition()` prevents content blink on update

**Workflow status:**
- `useWorkflow()` listens for `workflow-status-${name}` events in real-time
- Window focus triggers status re-fetch (replaces old 5s polling)

---

## Loading / Error / Empty State Matrix

| Page | Loading | Error | Empty | Notes |
|------|---------|-------|-------|-------|
| Daily Briefing | DashboardSkeleton | DashboardError | DashboardEmpty | Custom set. State managed in router.tsx. |
| Weekly Forecast | EditorialLoading | EditorialError | EmptyState | EmptyState with "no schedule" message. |
| Meeting Detail | EditorialLoading | EditorialError | n/a | Error if meeting not found. |
| Meeting History | n/a | n/a | n/a | Redirect only. |
| Actions | EditorialLoading | EditorialError | EmptyState | EmptyState per-tab (suggested, pending, completed). |
| Action Detail | Custom skeleton | Custom inline | n/a | Error if action not found. Retry button. |
| Account Detail | EditorialLoading | EditorialError | n/a | Error includes "not found" case. |
| Project Detail | EditorialLoading | EditorialError | n/a | Same pattern as Account Detail. |
| Person Detail | EditorialLoading | EditorialError | n/a | Same pattern as Account Detail. |
| Entity Lists | n/a | n/a | EmptyState | EmptyState with create CTA. |
| Risk Briefing | Skeleton (inline) | Custom inline | Custom + Generate CTA | GeneratingProgress during generation. |
| Account Health | Skeleton (inline) | Custom inline | Custom + Generate CTA | GeneratingProgress during generation. |
| EBR/QBR | Skeleton (inline) | Custom inline | Custom + Generate CTA | GeneratingProgress during generation. |
| SWOT | Skeleton (inline) | Custom inline | Custom + Generate CTA | GeneratingProgress during generation. |
| Weekly Impact | Skeleton (inline) | Custom inline | Custom + Generate CTA | GeneratingProgress during generation. |
| Monthly Wrapped | Skeleton (inline) | Custom inline | Custom + Generate CTA | GeneratingProgress during generation. |
| Book of Business | Skeleton (inline) | Custom inline | Custom + Generate CTA | GeneratingProgress during generation. |
| Generic Report | Text ("Loading...") | Text (error message) | n/a | Minimal -- delegates to ReportShell. |
| Emails | EditorialLoading | EditorialError | EmptyState | EmptyState with personality copy. |
| Inbox | EditorialLoading | EditorialError | Custom inline | Drop zone is the empty state. |
| History | EditorialLoading | EditorialError | EmptyState | EmptyState with "Go to inbox" CTA. |
| Settings | n/a | n/a | n/a | Sections handle own loading. |
| Me | EditorialLoading | EditorialError | n/a | Always has content (editable profile). |

### Component Sets

**Dashboard set** (bespoke -- used only by the Daily Briefing):
- `DashboardSkeleton` -- Full-page briefing skeleton
- `DashboardError` -- Error with retry
- `DashboardEmpty` -- Cold start with Generate CTA, Google auth check

**Editorial set** (standard for most magazine-layout pages):
- `EditorialLoading` -- Configurable skeleton line count
- `EditorialError` -- Error message with optional retry callback
- `EditorialEmpty` -- Serif italic title + sans description (deprecated in favor of EmptyState)
- `EmptyState` -- Rich empty state with headline, explanation, benefit, and action CTA

**Report set** (slide-deck pages):
- `Skeleton` (inline) -- Skeleton UI placeholders
- Custom inline empty state -- Generate CTA button
- `GeneratingProgress` -- Phased progress with editorial quotes

### Decision Tree

When building a new page, use this tree to decide which state components to use:

1. **Is this the dashboard?** Use DashboardSkeleton / DashboardError / DashboardEmpty
2. **Is this a slide-deck report?** Use Skeleton (inline) for loading, custom empty with Generate CTA, GeneratingProgress for generation
3. **Is this a magazine/editorial page?** Use EditorialLoading / EditorialError / EmptyState
4. **Is this a detail page?** Use EditorialLoading for loading, EditorialError for errors (no empty state -- show 404/not-found instead)

### Empty State Voice Guidelines

- Use Newsreader italic for the headline
- Use DM Sans for the description
- Tone: Gentle, not broken. "No actions yet" not "Error: 0 results"
- Include a next-step action when possible (CTA button)
- Use personality-driven copy where available (EmptyState + `getPersonalityCopy()`)

---

## Per-Page Hook Reference

| Page | Primary Hook(s) | Pattern |
|------|----------------|---------|
| Dashboard | `useDashboardData()`, `useWorkflow()` | Orchestrator |
| Week | `useCalendar()`, `useExecutiveIntelligence()` | Direct Invoke |
| Meeting Detail | (inline state), `useAgendaDraft()`, `useIntelligenceFeedback()` | Direct Invoke |
| Actions | `useActions()`, `useProposedActions()` | Filter/List |
| Action Detail | (inline state) | Direct Invoke |
| Accounts List | (inline state) | Filter/List |
| Account Detail | `useAccountDetail()` -> `useAccountFields()`, `useTeamManagement()`, `useEnrichmentProgress()` | Orchestrator |
| People List | (inline state) | Filter/List |
| Person Detail | `usePersonDetail()` | Orchestrator |
| Projects List | (inline state) | Filter/List |
| Project Detail | `useProjectDetail()` | Orchestrator |
| Emails | (inline state), `usePersonality()` | Direct Invoke |
| Inbox | `useInbox()` | Direct Invoke |
| History | (inline state) | Direct Invoke |
| Settings | `useGoogleAuth()`, `useGleanAuth()`, `useClaudeStatus()`, `useAppState()` | Direct Invoke |
| Me | `useMe()` | Orchestrator |
| All Reports | (inline state), `useIntelligenceFeedback()` | Report |

---

## Scroll & Reveal Patterns

### Chapter Observer (`useChapterObserver`)

Entity detail pages and meeting detail use chapter-based scroll navigation:

1. Page defines a `CHAPTERS` constant with `{ id, label, icon }` items
2. Each chapter section has a matching `id` attribute on its DOM element
3. `useChapterObserver(chapterIds)` uses IntersectionObserver to track which chapter is in view
4. FloatingNavIsland switches to `chapters` mode, showing chapter icons instead of page nav
5. Clicking a chapter icon smooth-scrolls to that section via `smoothScrollTo()`

### Reveal Observer (`useRevealObserver`)

Content fades in on scroll using CSS classes:

1. Elements receive `.editorial-reveal`, `.editorial-reveal-slow`, or `.editorial-reveal-stagger` classes
2. `useRevealObserver(ready, revision)` sets up IntersectionObserver
3. When an element enters the viewport, `.visible` class is added
4. CSS transitions handle the fade-in animation (600ms or 800ms)
5. `revision` parameter forces re-observation when data reloads

---

## Known Gaps

- **Report pages** use custom inline empty/loading states instead of shared components. These work but are not DRY -- each report page duplicates the Skeleton + empty + GeneratingProgress pattern.
- **ActionDetailPage** uses fully custom inline loading/error states instead of EditorialLoading/EditorialError.
- **HistoryPage** uses custom inline skeleton instead of EditorialLoading.
- **GenericReportPage** uses minimal text-only states -- acceptable since it's a fallback renderer.
