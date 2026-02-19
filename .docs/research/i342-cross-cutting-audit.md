# I342 Cross-Cutting Audit: Navigation, Shared Components, Duplication Map

**Date:** 2026-02-18
**Scope:** Bird's-eye analysis of navigation structure, shared component inventory, and information duplication across all surfaces.

---

## 1. Navigation Structure

### 1.1 Two Shell Systems

The app runs two parallel navigation shells selected by route ID in `src/router.tsx:69`:

| Shell | Layout file | Nav component | When used |
|-------|-------------|---------------|-----------|
| **Magazine Shell** | `MagazinePageLayout.tsx` | `FloatingNavIsland` + `FolioBar` | All current pages (all routes in `MAGAZINE_ROUTE_IDS`) |
| **Sidebar Shell** | `AppSidebar.tsx` + `Header.tsx` | `SidebarProvider` + `AppSidebar` | Currently no pages (all routes migrated to magazine shell) |

**Finding:** The sidebar shell (`AppSidebar.tsx`) is dead code. Every route ID is in `MAGAZINE_ROUTE_IDS`. The sidebar groups "Today" (Today, This Week, Inbox) and "Workspace" (Actions, People, Accounts/Projects) plus Settings in footer -- but no route ever renders through it. The `SidebarProvider`, `SidebarInset`, and the entire `SidebarContent` branch in `RootLayout` (lines 167-184) are unreachable.

### 1.2 Magazine Shell Navigation

The `FloatingNavIsland` (right-margin toolbar) has two modes:

**App mode** (list pages, daily briefing, actions):
| Order | ID | Label | Group | Icon |
|-------|-----|-------|-------|------|
| Home | today | Today | -- | BrandMark asterisk |
| 1 | week | This Week | main | Calendar |
| 2 | inbox | Inbox | main | Inbox |
| -- | divider | -- | -- | -- |
| 3 | actions | Actions | entity | CheckSquare2 |
| 4 | people | People | entity | Users |
| 5 | accounts | Accounts | entity | Building2 |
| 6 | projects | Projects | entity | FolderKanban |
| -- | divider | -- | -- | -- |
| 7 | settings | Settings | admin | Settings |

**Chapter mode** (detail pages): Each detail page defines its own chapters array. The nav island switches from page-navigation icons to chapter-scroll icons.

**Finding:** The FloatingNavIsland always shows ALL entity types (Accounts AND Projects) regardless of `entityMode` config. The AppSidebar respects `entityMode` (account/project/both), but since AppSidebar is dead code, this distinction is moot. If entityMode is "account", a user still sees "Projects" in the floating nav.

### 1.3 FolioBar (Top Masthead)

Every magazine page registers a FolioBar config via `useRegisterMagazineShell()`:

| Surface | Publication Label | Atmosphere Color | Date Text | Readiness Stats |
|---------|------------------|-----------------|-----------|----------------|
| Daily Briefing | "Daily Briefing" | turmeric | Day name, date | X/Y prepped, N overdue |
| Weekly Forecast | "Weekly Forecast" | larkspur | WEEK N, date range | X/Y prepped, N overdue |
| Actions | "Actions" | terracotta | Day name, date | N to review, N pending, N overdue |
| Accounts list | "Accounts" | turmeric | (none) | N accounts, $X ARR |
| Account detail | "Account" | turmeric | (none) | Back link: "Accounts" |
| Projects list | "Projects" | olive | (none) | N projects |
| Project detail | "Project" | olive | (none) | Back link: "Projects" |
| People list | "People" | larkspur | (none) | N people |
| Person detail | "Person" | larkspur | (none) | Back link: "People" |
| Meeting detail | (meeting title) | turmeric | Time range | (save status) |
| Inbox | "Inbox" | turmeric | (none) | N unprocessed |
| Settings | "Settings" | turmeric | (none) | (none) |
| Emails | "Emails" | turmeric | (none) | (none) |
| History | "History" | turmeric | (none) | (none) |

### 1.4 Command Menu (Cmd+K)

Available from all pages. Contains:
- **Meeting search** (debounced, queries backend)
- **Navigate** group: Overview, Inbox, Calendar, Actions (only 4 of 8+ surfaces)
- **Quick Actions**: Run Morning Briefing, Refresh Dashboard

**Finding:** The command menu navigation is incomplete. Missing: People, Accounts, Projects, Settings, Emails, History. Labels inconsistent with nav: "Calendar" in command menu vs "This Week" in nav. "Overview" vs "Today".

### 1.5 Route Map

| Route | Page Component | Route Type |
|-------|---------------|------------|
| `/` | DashboardPage (DailyBriefing) | Index |
| `/week` | WeekPage | List |
| `/actions` | ActionsPage | List |
| `/actions/$actionId` | ActionDetailPage | Detail |
| `/accounts` | AccountsPage | List |
| `/accounts/$accountId` | AccountDetailEditorial | Detail |
| `/accounts/$accountId/risk-briefing` | RiskBriefingPage | Sub-detail |
| `/projects` | ProjectsPage | List |
| `/projects/$projectId` | ProjectDetailEditorial | Detail |
| `/people` | PeoplePage | List |
| `/people/$personId` | PersonDetailEditorial | Detail |
| `/meeting/$meetingId` | MeetingDetailPage | Detail |
| `/meeting/history/$meetingId` | MeetingHistoryDetailPage | Detail |
| `/inbox` | InboxPage | List |
| `/emails` | EmailsPage | List |
| `/history` | HistoryPage | List |
| `/settings` | SettingsPage | Config |

**Routes not in nav:** `/emails`, `/history`, `/meeting/history/$meetingId`, `/actions/$actionId`, all detail routes. These are reached only through links or command menu.

---

## 2. Shared Component Inventory

### 2.1 Editorial Design System Components (`src/components/editorial/`)

| Component | Purpose | Used on surfaces |
|-----------|---------|-----------------|
| `ChapterHeading` | Rule + serif title for chapter sections | WeekPage, MeetingDetailPage, all entity detail pages, onboarding chapters |
| `FinisMarker` | Three asterisks end mark | DailyBriefing, WeekPage, MeetingDetailPage, all entity detail pages, Settings, EmailsPage, HistoryPage, InboxPage, RiskBriefingPage |
| `PullQuote` | Centered italic serif with rules | WeekPage, StateOfPlay, TrajectoryChapter, PersonInsightChapter |
| `StateBlock` | Colored label + prose items | StateOfPlay (entity detail) |
| `WatchItem` | Type-colored dot + source badge | **NOT IMPORTED ANYWHERE** (dead code -- WatchList uses its own WatchItemRow) |
| `TimelineEntry` | Vertical timeline with dots | UnifiedTimeline (entity detail) |
| `EditorialLoading` | Pulsing placeholder blocks | MeetingDetail, ActionsPage, all entity detail pages, Settings, Emails, History, Inbox |
| `EditorialError` | Terracotta error + retry | MeetingDetail, all entity detail pages, Settings, Emails, History, Inbox |
| `EditorialEmpty` | Serif italic empty state | ActionsPage, all entity detail pages, Settings, Emails, History, Inbox |
| `GeneratingProgress` | Multi-phase progress display | WeekPage, DashboardEmpty, RiskBriefingPage |

**Finding:** `WatchItem.tsx` is dead code. The `WatchList` component defines its own `WatchItemRow` internally. Same visual pattern, different implementation.

### 2.2 Entity Shared Components (`src/components/entity/`)

| Component | Purpose | Used on surfaces |
|-----------|---------|-----------------|
| `VitalsStrip` | Horizontal metric strip | Account detail, Project detail, Person detail |
| `StateOfPlay` | Working/struggling + PullQuote | Account detail only |
| `WatchList` | Risks/wins/unknowns linen band | Account detail, Project detail, Person detail |
| `UnifiedTimeline` | Chronological record | Account detail, Project detail, Person detail |
| `TheWork` | Meetings + commitments | Account detail only |
| `StakeholderGallery` | People grid with engagement | Account detail, Project detail |
| `EngagementSelector` | Temperature/engagement picker | StakeholderGallery |
| `EntityListShell` | List page skeleton/error/header | AccountsPage, PeoplePage, ProjectsPage |
| `EntityRow` | List item row | AccountsPage, PeoplePage, ProjectsPage, PersonNetwork |

**Finding:** `StateOfPlay` and `TheWork` are used on Account detail only, despite being in the entity-generic directory. Project detail uses `TrajectoryChapter` instead of StateOfPlay and shows actions inline in `ProjectAppendix`. Person detail uses `PersonInsightChapter` instead. The three entity types share the outer framework (VitalsStrip, WatchList, UnifiedTimeline, FinisMarker) but diverge in the middle chapters.

### 2.3 Layout Components (`src/components/layout/`)

| Component | Purpose | Used from |
|-----------|---------|-----------|
| `MagazinePageLayout` | Wraps FolioBar + FloatingNavIsland + AtmosphereLayer | router.tsx (all magazine routes) |
| `FolioBar` | Top masthead | MagazinePageLayout |
| `FloatingNavIsland` | Right-margin nav toolbar | MagazinePageLayout |
| `AtmosphereLayer` | Background color wash | MagazinePageLayout |
| `CommandMenu` | Cmd+K search dialog | router.tsx |
| `AppSidebar` | Left sidebar navigation | router.tsx (dead code branch) |

### 2.4 UI Primitives (`src/components/ui/`)

| Component | Usage breadth | Notes |
|-----------|---------------|-------|
| `badge.tsx` | Moderate | Shadcn primitive |
| `button.tsx` | Wide | Shadcn primitive |
| `card.tsx` | Low | Mostly unused with editorial redesign |
| `dialog.tsx` | Moderate | Entity detail pages, settings |
| `alert-dialog.tsx` | Moderate | Destructive confirmations |
| `command.tsx` | CommandMenu only | Shadcn primitive |
| `status-badge.tsx` | Unknown | Custom status display |
| `priority-picker.tsx` | ActionsPage | Action create form |
| `entity-picker.tsx` | ActionsPage, forms | Entity assignment |
| `email-signal-list.tsx` | Unknown | Email signal display |
| `meeting-entity-chips.tsx` | DailyBriefing, MeetingDetailPage | Entity assignment chips |
| `list-row.tsx` | Unknown | Generic list row |
| `tab-filter.tsx` | Unknown | Filter tabs |
| `EditableText.tsx` | Entity detail pages | Inline editing (I261) |
| `copy-button.tsx` | MeetingDetailPage | Copy to clipboard |
| `search-input.tsx` | Unknown | Search input |
| `inline-create-form.tsx` | AccountsPage, ProjectsPage | Entity creation |
| `bulk-create-form.tsx` | AccountsPage, ProjectsPage | Bulk entity creation |
| `agenda-draft-dialog.tsx` | WeekPage, MeetingDetailPage | Meeting agenda drafting |
| `BrandMark.tsx` | FinisMarker, FolioBar, FloatingNavIsland, WeekPage | Asterisk brand mark |
| `Avatar.tsx` | BriefingMeetingCard | Person avatar display |

---

## 3. Duplication Map

### 3.1 Meetings — Same Data, Five Surfaces

| Data element | Daily Briefing | Weekly Forecast | Meeting Detail | Account Detail | Project Detail |
|---|---|---|---|---|---|
| Meeting title | Schedule row | Day group row | Hero headline | TheWork upcoming | TheWork upcoming |
| Meeting time | Schedule row (time) | Day group row (time) | Folio bar, hero | TheWork (date) | TheWork (date) |
| Meeting type | BriefingMeetingCard badge | Subtitle (account + type) | -- | TheWork badge | TheWork badge |
| Account/entity link | MeetingEntityChips | Subtitle text | MeetingEntityChips | (implicit) | (implicit) |
| Prep status | Expansion panel | Dot color + mono text | Full chapter | Readiness callout | -- |
| Stakeholders | KeyPeopleFlow (expansion) | -- | "The Room" chapter | StakeholderGallery | StakeholderGallery |
| Narrative/context | Lead story narrative | -- | "The Brief" chapter | -- | -- |
| Actions for meeting | MeetingActionChecklist | -- | -- | -- | -- |

**Ownership recommendation:**
- **Meeting Detail** should OWN all meeting intelligence (prep, stakeholders, context, risks).
- **Daily Briefing** should SURFACE a meeting preview and LINK to meeting detail.
- **Weekly Forecast** should SURFACE meeting schedule and readiness status, LINK to detail.
- **Entity Detail** should SURFACE upcoming meetings list, LINK to meeting detail.

**Current duplication concern:** The Daily Briefing's "Lead Story" renders a near-complete meeting briefing inline (narrative context, key people, prep grid, action checklist). This duplicates 60-70% of the Meeting Detail page. The "Read full intelligence" link at the bottom acknowledges this.

### 3.2 Actions — Same Data, Four Surfaces

| Data element | Daily Briefing | Weekly Forecast | Actions Page | Entity Detail |
|---|---|---|---|---|
| Action title | Priorities section row | Commitments row | ActionRow | TheWork ActionRow |
| Priority badge | P1/P2/P3 color | P1 color | P1/P2/P3 color | -- (accent bar) |
| Due date | Context line | Due context | Context line | Date text |
| Overdue status | Red styling, "Overdue" group | Red dot, overdue label | Red border, "X days overdue" | Red accent bar, "Overdue" group |
| Account context | -- | Account name | Account name in context | (implicit) |
| Completion toggle | Checkbox button | -- | Checkbox button | -- |
| AI suggested triage | "Review" section (accept/reject) | -- | "Proposed" tab (accept/reject) | -- |

**Ownership recommendation:**
- **Actions Page** should OWN the complete action list with full CRUD.
- **Daily Briefing** should SURFACE today's priorities (capacity-aware top N) and LINK to actions page.
- **Weekly Forecast** should SURFACE commitments due this week and LINK to actions page.
- **Entity Detail** should SURFACE actions for that entity and LINK to actions page.

**Current duplication concern:** Proposed action triage (accept/reject) appears on BOTH the Daily Briefing and the Actions Page with near-identical UI but independent component code. The Daily Briefing renders `ProposedActionRow` inline; the Actions Page has its own `ProposedActionRow`. Both use `useProposedActions()` hook.

### 3.3 Entity Info — Same Data, Three Surfaces

| Data element | Daily Briefing | Meeting Detail | Entity Detail |
|---|---|---|---|
| Account/project name | Meeting row subtitle, entity chips | Entity chips, meeting meta | Hero title |
| Account health | -- | Account snapshot items | Vitals strip |
| Account ARR | -- | Account snapshot items | Vitals strip |
| Person temperature | -- | Attendee context badges | Vitals strip (person detail) |
| Risks | -- | "Risks" chapter | WatchList component |
| Working/struggling | -- | -- | StateOfPlay component |
| Timeline (meetings/emails) | -- | -- | UnifiedTimeline component |
| Stakeholder list | Meeting: KeyPeopleFlow | "The Room" chapter | StakeholderGallery |

**Ownership recommendation:**
- **Entity Detail** should OWN all entity intelligence, context, and history.
- **Meeting Detail** should SURFACE entity context relevant to that meeting and LINK to entity detail.
- **Daily Briefing** should SURFACE entity names/links, nothing more.

### 3.4 Proposed Action Triage — Two Surfaces

| Element | Daily Briefing | Actions Page |
|---------|----------------|-------------|
| Action title | Serif 15px | Serif 17px |
| Source label | Mono 11px | Mono 13px + priority |
| Accept button | 24x24, green border, check SVG | 28x28, green border, check SVG |
| Reject button | 24x24, red border, X SVG | 28x28, red border, X SVG |
| Dashed left border | 2px dashed turmeric | 2px dashed turmeric |
| Max shown | 5 (with "see all" link) | All (in "proposed" tab) |
| Hook used | `useProposedActions()` | `useProposedActions()` |

**Finding:** Nearly identical UI rendered from two separate inline implementations. No shared `ProposedActionRow` component exists. The Daily Briefing version is smaller (24x24 buttons, 15px title). The Actions Page version is larger (28x28 buttons, 17px title). Both could share a single component with a `compact` prop.

### 3.5 Action Row Rendering — Four Implementations

| Implementation | Location | Font | Priority | Due date | Account | Toggle |
|---|---|---|---|---|---|---|
| DailyBriefing `PrioritizedActionItem` | DailyBriefing.tsx | CSS module | Colored | Capacity-aware | -- | Checkbox |
| WeekPage commitments | WeekPage.tsx inline | Inline style, serif 17px | Mono 11px | Due context | Account text | -- |
| ActionsPage `ActionRow` | ActionsPage.tsx | Inline style, serif 17px | Mono 11px | Context line | Account name | Checkbox |
| TheWork `ActionRow` | TheWork.tsx | Inline style, sans 14px | -- | Date only | -- | -- |

**Finding:** Four separate action-row implementations. None share code. All show title + due date + optional context but with slightly different styling and data. This is the single biggest duplication in the app.

### 3.6 Meeting Row Rendering — Three Implementations

| Implementation | Location | Elements shown |
|---|---|---|
| `BriefingMeetingCard` | DailyBriefing schedule | Time, title, type badge, entity byline, temporal state, expansion panel |
| WeekPage meeting row | WeekPage inline | Dot, time, title, account + type subtitle, prep status |
| TheWork meeting row | TheWork.tsx inline | Date, title, type badge |

**Finding:** Three separate meeting-row implementations. The DailyBriefing version is the most elaborate (expansion panel, temporal states, action checklists). The WeekPage version is compact. TheWork is minimal.

---

## 4. Design System Consistency Audit

### 4.1 Design Tokens Usage

The design token file (`src/styles/design-tokens.css`) is well-structured with:
- Paper palette (cream, linen, warm-white)
- Desk palette (charcoal, ink, espresso)
- Spice palette (turmeric, saffron, terracotta, chili)
- Garden palette (sage, olive, rosemary, larkspur)
- Entity color assignments (account=turmeric, project=olive, person=larkspur, action=terracotta)
- Typography (serif=Newsreader, sans=DM Sans, mono=JetBrains Mono, mark=Montserrat)
- Spacing scale (4px base grid)

### 4.2 Consistency Issues

1. **Inline styles vs CSS modules**: The DailyBriefing uses CSS modules (`editorial-briefing.module.css`). All other editorial pages use inline `style={}` objects. This means the same visual patterns (chapter heading, action row, meeting row) are implemented with two different styling strategies.

2. **Font size inconsistency for same elements**: Action titles vary from serif 15px (DailyBriefing) to serif 17px (ActionsPage, WeekPage) to sans 14px (TheWork). Priority badges vary from mono 11px to mono 13px.

3. **Color references**: Some components use design token vars (`var(--color-spice-terracotta)`), while others use raw values. The `WatchItem` editorial component uses vars correctly but is dead code. The `WatchList` component that replaced it also uses vars.

4. **`entityMode` not respected in nav**: The `FloatingNavIsland` always shows both Accounts and Projects in app mode. It does not read `entityMode` from config. Only the dead `AppSidebar` handled this.

### 4.3 Atmosphere Colors by Surface

| Color | Surfaces |
|-------|----------|
| turmeric | Daily Briefing, Accounts list, Account detail, Meeting detail, Inbox, Settings, Emails, History |
| larkspur | Weekly Forecast, People list, Person detail |
| terracotta | Actions |
| olive | Projects list, Project detail |

This aligns with the entity color assignments: Account=turmeric, Project=olive, Person=larkspur, Action=terracotta. The Daily Briefing defaulting to turmeric may be reconsidered since it's not account-specific.

---

## 5. Dead Code Summary

| File/Component | Status | Reason |
|---|---|---|
| `AppSidebar.tsx` | Dead code | All routes use magazine shell; sidebar branch unreachable |
| `Header.tsx` (dashboard) | Dead code | Only rendered in sidebar shell branch |
| `WatchItem.tsx` (editorial) | Dead code | Replaced by `WatchItemRow` inside `WatchList.tsx` |
| Sidebar UI primitives (`sidebar.tsx`) | Partially dead | Only used by `AppSidebar` |
| `entityMode` handling in nav | Dead behavior | `FloatingNavIsland` ignores it |

---

## 6. Cross-Surface Information Flow

### How the user navigates between surfaces:

```
Daily Briefing (/)
  |-- Featured meeting: "Read full intelligence" --> Meeting Detail
  |-- Schedule meeting click (upcoming) --> expands inline
  |-- Schedule meeting click (past) --> Meeting Detail
  |-- Proposed actions: "See all suggestions" --> Actions Page
  |-- Priority action click --> Action Detail
  |
Weekly Forecast (/week)
  |-- Meeting click --> Meeting Detail
  |-- Commitment click --> Action Detail
  |-- "X more" link --> Actions Page
  |
Meeting Detail (/meeting/$id)
  |-- Back link --> (no consistent back, relies on FolioBar)
  |-- Entity chips --> Account/Project/Person Detail
  |-- Stakeholder name click --> Person Detail
  |
Account Detail (/accounts/$id)
  |-- Back link: "Accounts" --> Accounts List
  |-- Meeting row --> (no link currently in TheWork)
  |-- Action row --> Action Detail
  |-- Stakeholder click --> Person Detail
  |-- "Reports" --> Risk Briefing
  |
Person Detail (/people/$id)
  |-- Back link: "People" --> People List
  |-- Timeline meeting click --> Meeting Detail
  |-- Network entity click --> Account/Project Detail
  |
Project Detail (/projects/$id)
  |-- Back link: "Projects" --> Projects List
  |-- Timeline meeting click --> Meeting Detail
  |-- Action link --> Action Detail
```

**Finding:** Navigation is mostly one-way (down into detail). There is no way to go from Meeting Detail back to the Daily Briefing schedule position. The FolioBar provides a back link on entity detail pages but not on meeting detail. Meeting Detail has no breadcrumb showing how the user got there (from daily briefing? from weekly forecast? from entity detail?).

---

## 7. Key Findings for I342

### What to cut (duplication that confuses rather than helps):

1. **Lead Story in Daily Briefing**: The inline meeting briefing duplicates 60-70% of Meeting Detail. Consider replacing with a compact preview + prominent link.

2. **Proposed action triage on Daily Briefing**: The accept/reject UI duplicates the Actions page "proposed" tab. Consider: either keep it on Daily Briefing only (triage during morning read) or on Actions page only (dedicated review surface), but not both with independent implementations.

3. **Four action row implementations**: Consolidate into 1-2 shared components with size/density variants.

4. **Three meeting row implementations**: Consolidate into 1-2 shared components.

### What to move:

1. **`entityMode` enforcement**: Should be applied in FloatingNavIsland, not just dead AppSidebar.
2. **Command menu nav**: Should cover all navigable surfaces, not just 4.

### What to merge:

1. **`WatchItem.tsx`** (dead) and `WatchItemRow` (in WatchList.tsx) -- delete WatchItem.tsx.
2. **Proposed action UI** -- extract shared component from DailyBriefing and ActionsPage.
3. **Action row UI** -- extract shared component usable across all surfaces.

### Structural observation:

The app has 18 routes but only 8 are in the primary nav. The "hidden" routes (emails, history, meeting history, action detail, all entity details) are discoverable only through in-page links or command menu. This is intentional for the editorial "magazine" model (you read the daily briefing, follow links for depth), but the command menu should be the primary discovery mechanism and it currently only covers 4 surfaces.
