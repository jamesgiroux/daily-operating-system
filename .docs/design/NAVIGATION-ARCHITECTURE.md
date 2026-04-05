# Navigation Architecture

**Last audited:** 2026-03-15

Complete route map, navigation components, and page connection graph.

---

## Route Map

All routes from `src/router.tsx`. Every route listed in `MAGAZINE_ROUTE_IDS` uses the magazine shell.

| Route | Page | Component | Shell |
|-------|------|-----------|-------|
| `/` | Daily Briefing | `DashboardPage` | Magazine |
| `/week` | Weekly Forecast | `WeekPage` | Magazine |
| `/actions` | Actions | `ActionsPage` | Magazine |
| `/actions/$actionId` | Action Detail | `ActionDetailPage` | Magazine |
| `/accounts` | Accounts | `AccountsPage` | Magazine |
| `/accounts/$accountId` | Account Detail | `AccountDetailEditorial` | Magazine |
| `/accounts/$accountId/risk-briefing` | Risk Briefing | `RiskBriefingPage` | Magazine |
| `/accounts/$accountId/reports/$reportType` | Generic Report | `ReportPage` | Magazine |
| `/accounts/$accountId/reports/account_health` | Account Health | `AccountHealthPage` | Magazine |
| `/accounts/$accountId/reports/ebr_qbr` | EBR/QBR | `EbrQbrPage` | Magazine |
| `/accounts/$accountId/reports/swot` | SWOT | `SwotPage` | Magazine |
| `/projects` | Projects | `ProjectsPage` | Magazine |
| `/projects/$projectId` | Project Detail | `ProjectDetailEditorial` | Magazine |
| `/people` | People | `PeoplePage` | Magazine |
| `/people/$personId` | Person Detail | `PersonDetailEditorial` | Magazine |
| `/emails` | Emails | `EmailsPage` | Magazine |
| `/inbox` | Inbox | `InboxPage` | Magazine |
| `/history` | History | `HistoryPage` | Magazine |
| `/meeting/$meetingId` | Meeting Detail | `MeetingDetailPage` | Magazine |
| `/meeting/history/$meetingId` | Meeting History | `MeetingHistoryDetailPage` | Magazine |
| `/me` | Me | `MePage` | Magazine |
| `/me/reports/weekly_impact` | Weekly Impact | `WeeklyImpactPage` | Magazine |
| `/me/reports/monthly_wrapped` | Monthly Wrapped | `MonthlyWrappedPage` | Magazine |
| `/me/reports/book_of_business` | Book of Business | `BookOfBusinessPage` | Magazine |
| `/me/reports/$reportType` | Me Report | `ReportPage` | Magazine |
| `/settings` | Settings | `SettingsPage` | Magazine |

**Total:** 26 routes, all using the magazine shell.

### Search Params

| Route | Params | Values |
|-------|--------|--------|
| `/actions` | `?search=` | Free text search |
| `/inbox` | `?entityId=` | Pre-filter by entity |
| `/people` | `?relationship=` | `all`, `external`, `internal`, `unknown` |
| `/people` | `?hygiene=` | `unnamed`, `duplicates` |
| `/settings` | `?tab=` | `you`, `connectors`, `system`, `diagnostics` (+ legacy: `profile`, `role`, `integrations`, `workflows`, `intelligence`, `hygiene`) |

---

## Shell Model

### Magazine Shell (All Pages)

All pages use `MagazinePageLayout`, which composes:

| Component | Position | Purpose |
|-----------|----------|---------|
| `FolioBar` | Fixed top | Brand mark, page label, date, context actions, search button |
| `FloatingNavIsland` | Fixed right | Page navigation icons or chapter navigation icons |
| `AtmosphereLayer` | Fixed background (z: 0) | Radial gradient ambiance, page-specific color, breathing animation |

**File:** `src/components/layout/MagazinePageLayout.tsx`

Shell configuration comes from two sources:
1. **Props** (router-level): `onFolioSearch`, `onNavigate`, `onNavHome`
2. **MagazineShellContext** (page-level): chapters, folioLabel, atmosphereColor, backLink, folioActions

Pages register their config via `useRegisterMagazineShell(shellConfig)`. This inverts the dependency so the router does not need to import page internals.

### Shell Config Shape

```typescript
{
  folioLabel: string;              // FolioBar publication label
  atmosphereColor: string;         // AtmosphereLayer color (turmeric, terracotta, etc.)
  activePage: string;              // FloatingNavIsland active highlight
  activeColor?: string;            // NavIsland active indicator color
  backLink?: { label, onClick };   // FolioBar back navigation (detail pages)
  chapters?: ChapterItem[];        // Switches NavIsland to chapter mode
  folioActions?: ReactNode;        // FolioBar right-side action buttons
  folioStatusText?: string;        // FolioBar status indicator
  readinessStats?: ReadinessStat[];// FolioBar readiness badges
  dateText?: string;               // FolioBar date display
  statusText?: string;             // FolioBar status text
}
```

---

## FloatingNavIsland

**File:** `src/components/layout/FloatingNavIsland.tsx`

Two modes: `app` (page navigation) and `chapters` (scroll navigation within a page).

### App Mode (Default)

11 navigation items in 4 groups, separated by thin dividers:

| Group | Items | Icons |
|-------|-------|-------|
| Home | Today (brand mark) | `BrandMark` |
| Main | This Week, Mail | `Calendar`, `Mail` |
| Entity | Dropbox, Actions, Me, People, Accounts/Projects | `Inbox`, `CheckSquare2`, `UserCircle`, `Users`, `Building2`/`FolderKanban` |
| Admin | Settings | `Settings` |

**Entity mode ordering:** The `entityMode` prop (from active role preset) controls whether Accounts or Projects appears first in the entity group:
- `'account'` (default): Accounts, then Projects
- `'project'`: Projects, then Accounts
- `'both'`: default order (Accounts first)

**Active state:** Active page gets a color highlight. Color varies by page via `activeColor` prop (turmeric, terracotta, larkspur, olive, eucalyptus).

**Me indicator dot:** When the user entity is empty (no name, company, title, etc.), the Me nav item shows a small dot prompting the user to fill in their profile.

### Chapter Mode

When a page provides `chapters` in its shell config, the NavIsland switches to chapter navigation:
- Same visual style as app mode (36px icon buttons, tooltips).
- Clicking a chapter icon calls `smoothScrollTo(chapter.id)` for smooth scroll.
- Active chapter is tracked via `activeChapterId` prop (set by `useChapterObserver` hook).
- Used by: AccountHealthPage (5 chapters), RiskBriefingPage (6 chapters), EbrQbrPage, SwotPage.

### Visual

- Frosted glass background.
- 36px icon buttons with `data-label` tooltips on hover.
- Brand mark (asterisk) at the top as home button.
- Dividers between groups.

---

## FolioBar

**File:** `src/components/layout/FolioBar.tsx`

Fixed top bar. 40px height. Frosted glass background.

| Section | Content |
|---------|---------|
| Left | Brand mark (links to `/`), publication label or back link |
| Center | Date text, readiness stats |
| Right | Status text, custom actions slot, search button |

**Back link:** On detail pages, the publication label is replaced with a clickable back link (e.g., "Back" on account health pages). Uses `window.history.back()` with fallback to parent route.

**Custom actions:** Pages can inject action buttons (e.g., "Regenerate" on report pages) via `folioActions` in the shell config.

---

## Command Menu (Global Search)

**File:** `src/components/layout/CommandMenu.tsx`
**Trigger:** `Cmd+K` or FolioBar search button

Global command palette providing:
1. Entity search across all types (accounts, people, projects, meetings, actions, emails)
2. Quick navigation to pages
3. Quick actions (run briefing, refresh data)

Results are grouped by entity type with consistent icons. Selection navigates to the entity detail page.

---

## Page Connection Graph

```
Daily Briefing (/)
  -> Meeting card -> /meeting/$meetingId
  -> Action item -> /actions/$actionId
  -> Email item -> /emails
  -> Entity chip -> /accounts/$accountId or /projects/$projectId

Weekly Forecast (/week)
  -> Meeting card -> /meeting/$meetingId

Accounts (/accounts)
  -> EntityRow -> /accounts/$accountId

Account Detail (/accounts/$accountId)
  -> Risk Briefing -> /accounts/$accountId/risk-briefing
  -> Account Health -> /accounts/$accountId/reports/account_health
  -> EBR/QBR -> /accounts/$accountId/reports/ebr_qbr
  -> SWOT -> /accounts/$accountId/reports/swot
  -> Stakeholder -> /people/$personId
  -> Meeting -> /meeting/$meetingId
  -> Action -> /actions/$actionId

Projects (/projects)
  -> EntityRow -> /projects/$projectId

Project Detail (/projects/$projectId)
  -> Stakeholder -> /people/$personId
  -> Meeting -> /meeting/$meetingId
  -> Action -> /actions/$actionId

People (/people)
  -> EntityRow -> /people/$personId

Person Detail (/people/$personId)
  -> Meeting -> /meeting/$meetingId
  -> Account -> /accounts/$accountId
  -> Project -> /projects/$projectId

Actions (/actions)
  -> Action row -> /actions/$actionId

Action Detail (/actions/$actionId)
  -> Entity chip -> /accounts/$accountId or /projects/$projectId
  -> Meeting link -> /meeting/$meetingId

Emails (/emails)
  -> Entity chip -> /accounts/$accountId or /projects/$projectId

Meeting Detail (/meeting/$meetingId)
  -> Entity chip -> /accounts/$accountId or /projects/$projectId
  -> Attendee -> /people/$personId

Me (/me)
  -> Weekly Impact -> /me/reports/weekly_impact
  -> Monthly Wrapped -> /me/reports/monthly_wrapped

Settings (/settings)
  -> Tab navigation via ?tab= search param
```

---

## Startup Gate

**File:** `src/routerStartupGate.ts`

Before the app renders normal routes, a startup gate evaluates conditions in strict priority order. The first matching condition takes over the entire viewport.

| Priority | Gate | Renders | Condition |
|----------|------|---------|-----------|
| 1 | `checking` | Blank centered div | Config check in progress |
| 2 | `encryption-recovery` | `EncryptionRecovery` | Encryption key missing from keychain |
| 3 | `database-recovery` | `DatabaseRecovery` | Database corruption detected |
| 4 | `lock` | `LockOverlay` | App is locked (idle timeout) |
| 5 | `onboarding` | `OnboardingFlow` | First run, no workspace configured |
| 6 | `app` | Normal routing | All conditions clear |

The gate is evaluated in `RootLayout` on every render. Recovery gates (encryption, database) take priority over lock and onboarding because data integrity must be resolved before the user interacts with the app.

---

## Global Overlays

These components render outside the page content area and are always available:

| Overlay | File | Trigger |
|---------|------|---------|
| `CommandMenu` | `layout/CommandMenu.tsx` | `Cmd+K` or search button |
| `PostMeetingPrompt` | `PostMeetingPrompt.tsx` | Auto after meeting ends |
| `WhatsNewModal` | `notifications/WhatsNewModal.tsx` | Auto on version update, or manual |
| `ICloudWarningModal` | `ICloudWarningModal.tsx` | Workspace on iCloud detected |
| `TourTips` | `tour/TourTips.tsx` | First-run guidance |
| `Toaster` | `ui/sonner.tsx` | Toast notifications (bottom-right) |
| `DevToolsPanel` | `devtools/DevToolsPanel.tsx` | Development tools |
| `UpdateBanner` | `notifications/UpdateBanner.tsx` | App update available |

---

## Router Implementation

**Library:** TanStack Router (`@tanstack/react-router`)
**File:** `src/router.tsx`

- Flat route tree (all routes are direct children of `rootRoute`).
- Route-specific report routes (`account_health`, `ebr_qbr`, `swot`) are registered BEFORE the generic `$reportType` catch-all to ensure they match first.
- `MAGAZINE_ROUTE_IDS` set determines shell selection. Currently all routes use magazine shell.
- Navigation from FloatingNavIsland uses a route map in `handleNavNavigate`:

```typescript
const routes = {
  today: "/",
  week: "/week",
  emails: "/emails",
  dropbox: "/inbox",
  actions: "/actions",
  me: "/me",
  people: "/people",
  accounts: "/accounts",
  projects: "/projects",
  settings: "/settings",
};
```

---

## Chapter-Based Scrolling

**Hook:** `useChapterObserver` in `src/hooks/useChapterObserver.ts`
**Scroll utility:** `smoothScrollTo` in `src/lib/smooth-scroll.ts`

Entity detail pages and report pages use chapter-based scrolling as an alternative to page-level navigation. When chapters are active, the FloatingNavIsland switches from page icons to chapter icons.

### How It Works

1. **Page defines chapters:** Each page with chapters declares a `CHAPTERS` constant:
   ```typescript
   const CHAPTERS = [
     { id: "headline", label: "The Brief", icon: <AlignLeft size={18} /> },
     { id: "risks", label: "Risks", icon: <AlertTriangle size={18} /> },
     { id: "the-room", label: "The Room", icon: <Users size={18} /> },
   ];
   ```

2. **DOM elements have matching IDs:** Each chapter section has `id={chapter.id}` on its container element.

3. **IntersectionObserver tracks visibility:** `useChapterObserver(chapterIds)` creates an observer with `rootMargin: "-40% 0px -60% 0px"` to detect which chapter is in the top 40% of the viewport.

4. **FloatingNavIsland renders chapters:** When `chapters` is provided in the shell config, the NavIsland renders chapter icons with active state tracking instead of page navigation.

5. **Click-to-scroll:** Clicking a chapter icon calls `smoothScrollTo(chapterId)`, which uses `scrollIntoView({ behavior: 'smooth' })` and sets a flag to prevent the IntersectionObserver from fighting with the scroll animation.

### Pages Using Chapter Navigation

| Page | Chapter Count | Chapters |
|------|---------------|----------|
| Account Detail | 9 | Hero, Vitals, State of Play, The Room, Watch List, The Work, The Record, Appendix, Finis |
| Person Detail | 9 | Hero, Vitals, The Dynamic, The Network, The Landscape, The Work, The Record, Appendix, Finis |
| Project Detail | 10 | Hero, Vitals, Trajectory, The Horizon, The Landscape, The Team, The Work, The Record, Appendix, Finis |
| Meeting Detail | 4 | The Brief, Risks, The Room, Your Plan |
| Account Health | 5 | Cover, Partnership, Where We Stand, Value Delivered, What's Ahead |
| Risk Briefing | 6 | Cover, Bottom Line, What Happened, The Stakes, The Ask, The Plan |
| EBR/QBR | 7 | Cover, The Story, Value Delivered, By the Numbers, What We Navigated, What's Ahead, Next Steps |
| SWOT | 5 | Cover, Strengths, Weaknesses, Opportunities, Threats |

---

## Deep Linking

### Entity Deep Links

Entity detail pages use URL parameters for direct access:
- `/accounts/{uuid}` -- account detail
- `/projects/{uuid}` -- project detail
- `/people/{uuid}` -- person detail
- `/meeting/{uuid}` -- meeting detail
- `/actions/{uuid}` -- action detail

These URLs are stable and can be shared or bookmarked. The `$accountId`, `$projectId`, etc. params are UUIDs.

### Report Deep Links

Reports are accessible via entity-scoped URLs:
- `/accounts/{uuid}/reports/account_health` -- account health report
- `/accounts/{uuid}/reports/ebr_qbr` -- EBR/QBR report
- `/accounts/{uuid}/reports/swot` -- SWOT report
- `/accounts/{uuid}/risk-briefing` -- risk briefing
- `/me/reports/weekly_impact` -- weekly impact
- `/me/reports/monthly_wrapped` -- monthly wrapped
- `/me/reports/book_of_business` -- book of business

### Search Param Deep Links

Certain pages accept search parameters for pre-filtering:
- `/actions?search=term` -- pre-filter actions list
- `/inbox?entityId=uuid` -- pre-filter inbox by entity
- `/people?relationship=external` -- filter to external contacts
- `/people?hygiene=duplicates` -- show duplicate candidates
- `/settings?tab=connectors` -- open Settings on Connectors tab

### Back Navigation

Detail pages use `backLink` in the shell config to provide back navigation in the FolioBar:
- Account Detail: "Accounts" -> `/accounts`
- Person Detail: "People" -> `/people`
- Project Detail: "Projects" -> `/projects`
- Action Detail: "Actions" -> `/actions`
- Report pages: entity name -> `/accounts/$accountId`
- Meeting Detail: "Today" -> `/`

The `backLink` replaces the brand mark + publication label with a clickable back arrow and label.

---

## Navigation Patterns Summary

| Pattern | Mechanism | When |
|---------|-----------|------|
| Page navigation | FloatingNavIsland (app mode) | Navigating between top-level pages |
| Chapter navigation | FloatingNavIsland (chapters mode) | Scrolling within entity detail or report pages |
| Entity navigation | EntityRow click, entity chips, CommandMenu | Navigating to entity detail pages |
| Back navigation | FolioBar backLink | Returning from detail to list page |
| Global search | CommandMenu (Cmd+K) | Finding any entity or page |
| Deep link | Direct URL with params | External links, bookmarks |
| Redirect | `<Navigate>` component | Legacy URL compatibility (e.g., meeting history) |
