# Component Inventory

**Last audited:** 2026-02-20
**Rule:** Check this list before building a new component. If something similar exists, extend it.

---

## Layout Shell (3 components)

These wrap every page. Non-negotiable.

| Component | File | Job | Notes |
|-----------|------|-----|-------|
| **MagazinePageLayout** | `layout/MagazinePageLayout.tsx` | Page wrapper. Registers shell config (folio, nav, atmosphere). Renders content at max-width with proper padding. | Every page uses this. |
| **FolioBar** | `layout/FolioBar.tsx` | Fixed top bar. Brand mark, page label, date, context actions, status. | Frosted glass, 40px height. Context actions vary per page via shellConfig. |
| **FloatingNavIsland** | `layout/FloatingNavIsland.tsx` | Fixed right nav. Icon grid with tooltips. Active state color varies per page. | Frosted glass. Currently missing Emails nav item (I358). |
| **AtmosphereLayer** | `layout/AtmosphereLayer.tsx` | Background radial gradients. Page-specific color. Breathing animation. | Fixed position, z: 0. |

---

## Editorial Components (7 components)

The building blocks of the magazine aesthetic.

| Component | File | Job | Compliance |
|-----------|------|-----|------------|
| **ChapterHeading** | `editorial/ChapterHeading.tsx` | Section header. Newsreader 28px + thin rule above. | Compliant |
| **FinisMarker** | `editorial/FinisMarker.tsx` | End-of-page marker. `* * *` with closing message. | Compliant |
| **PullQuote** | `editorial/PullQuote.tsx` | Focus callout. Turmeric left border, italic serif. | Compliant |
| **EditorialEmpty** | `editorial/EditorialEmpty.tsx` | Empty state. Serif italic title + sans description. | Compliant |
| **EditorialError** | `editorial/EditorialError.tsx` | Error state for editorial pages. | Compliant |
| **StateBlock** | `editorial/StateBlock.tsx` | Structured state display (working/struggling, momentum/headwinds). | Compliant |
| **TimelineEntry** | `editorial/TimelineEntry.tsx` | Timeline row with entity color accent. | Compliant. Has CSS module. |
| **BriefingCallouts** | `editorial/BriefingCallouts.tsx` | Callout boxes within briefing content. | Compliant |
| **GeneratingProgress** | `editorial/GeneratingProgress.tsx` | Loading state for briefing generation. | Compliant |
| **EditorialLoading** | `editorial/EditorialLoading.tsx` | Skeleton loading for editorial pages. | Compliant |

---

## Entity Components (10 components)

Shared across Account, Project, and Person detail pages.

| Component | File | Job | Compliance | Notes |
|-----------|------|-----|------------|-------|
| **VitalsStrip** | `entity/VitalsStrip.tsx` | Horizontal metrics bar below hero. | Compliant | JetBrains Mono, color-coded values. |
| **StakeholderGallery** | `entity/StakeholderGallery.tsx` | Attendee/stakeholder grid with inline editing. | Compliant | The model for inline editing. |
| **WatchList** | `entity/WatchList.tsx` | Risks/Wins/Unknowns list. | Compliant | Accepts `bottomSection` prop for programs. |
| **StateOfPlay** | `entity/StateOfPlay.tsx` | Account: working/struggling analysis. | Compliant | |
| **TheWork** | `entity/TheWork.tsx` | Actions + upcoming meetings on entity. | Compliant | |
| **EntityListShell** | `entity/EntityListShell.tsx` | Shared shell for list pages (Accounts, Projects, People). | Compliant | |
| **EntityRow** | `entity/EntityRow.tsx` | Row in entity list. | Compliant | |
| **UnifiedTimeline** | `entity/UnifiedTimeline.tsx` | The Record — merged timeline of meetings, emails, captures. | Compliant | |
| **FileListSection** | `entity/FileListSection.tsx` | File listing in appendix. | Compliant | |
| **EditableVitalsStrip** | `entity/EditableVitalsStrip.tsx` | Inline-editable vitals (I343). | NEW | |
| **EngagementSelector** | `entity/EngagementSelector.tsx` | Engagement level picker for stakeholders. | Compliant | |
| **PresetFieldsEditor** | `entity/PresetFieldsEditor.tsx` | Role preset field editor. | Compliant | Should render inline per ADR-0084. |

---

## Entity-Specific Components

### Account (4 components)

| Component | File | Job | Notes |
|-----------|------|-----|-------|
| **AccountHero** | `account/AccountHero.tsx` | Account page hero. Name, assessment, health badges. | Has CSS module. |
| **AccountAppendix** | `account/AccountAppendix.tsx` | Lifecycle events, notes, files, BUs. | Reduced per I342 (no Value Delivered, Portfolio Summary). |
| **AccountMergeDialog** | `account/AccountMergeDialog.tsx` | Merge duplicate accounts. | |
| **LifecycleEventDrawer** | `account/LifecycleEventDrawer.tsx` | Add lifecycle event. | Drawer is acceptable (create workflow). |
| **WatchListPrograms** | `account/WatchListPrograms.tsx` | Active programs list. | Extracted from WatchList per I342 A15. |

### Project (4 components)

| Component | File | Job |
|-----------|------|-----|
| **ProjectHero** | `project/ProjectHero.tsx` | Project page hero. Has CSS module. |
| **ProjectAppendix** | `project/ProjectAppendix.tsx` | Milestones, description, notes, files. |
| **HorizonChapter** | `project/HorizonChapter.tsx` | Next milestone, timeline risk, decisions. |
| **TrajectoryChapter** | `project/TrajectoryChapter.tsx` | Momentum/headwinds. |
| **WatchListMilestones** | `project/WatchListMilestones.tsx` | Milestone tracking in watch list. |

### Person (2 components)

| Component | File | Job |
|-----------|------|-----|
| **PersonInsightChapter** | `person/PersonInsightChapter.tsx` | The Dynamic/Rhythm — relationship analysis. |
| **PersonNetwork** | `person/PersonNetwork.tsx` | Connected entities network view. |

---

## UI Primitives (37 components)

Radix-based primitives. Most are compliant. Used across the app.

### Compliant (35/37)

| Component | File | Brief description |
|-----------|------|-------------------|
| alert-dialog | `ui/alert-dialog.tsx` | Destructive confirmation dialog |
| Avatar | `ui/Avatar.tsx` | Profile image with fallback |
| badge | `ui/badge.tsx` | Label badges (uses Tailwind semantic tokens) |
| BrandMark | `ui/BrandMark.tsx` | The `*` asterisk in Montserrat 800 |
| bulk-create-form | `ui/bulk-create-form.tsx` | Multi-entity creation form |
| button | `ui/button.tsx` | CVA-based button variants |
| calendar | `ui/calendar.tsx` | Date calendar (editorial styled) |
| card | `ui/card.tsx` | Card container (featured content only!) |
| collapsible | `ui/collapsible.tsx` | Expand/collapse wrapper |
| command | `ui/command.tsx` | Command palette (cmdk) |
| copy-button | `ui/copy-button.tsx` | Copy-to-clipboard button |
| date-picker | `ui/date-picker.tsx` | Date selection (editorial calendar) |
| dialog | `ui/dialog.tsx` | Modal dialog |
| dropdown-menu | `ui/dropdown-menu.tsx` | Context/dropdown menus |
| email-signal-list | `ui/email-signal-list.tsx` | Email signal rows |
| entity-picker | `ui/entity-picker.tsx` | Link entity to meetings |
| inline-create-form | `ui/inline-create-form.tsx` | Inline entity/action creation |
| input | `ui/input.tsx` | Text input |
| label | `ui/label.tsx` | Form label |
| list-row | `ui/list-row.tsx` | Generic list row |
| meeting-entity-chips | `ui/meeting-entity-chips.tsx` | Entity color chips on meeting rows |
| popover | `ui/popover.tsx` | Positioned popup |
| priority-picker | `ui/priority-picker.tsx` | Action priority selector |
| scroll-area | `ui/scroll-area.tsx` | Custom scrollbar |
| search-input | `ui/search-input.tsx` | Search with icon |
| select | `ui/select.tsx` | Dropdown select |
| separator | `ui/separator.tsx` | Visual divider |
| sheet | `ui/sheet.tsx` | Side drawer |
| sidebar | `ui/sidebar.tsx` | Legacy sidebar (dead code — AppSidebar removed) |
| skeleton | `ui/skeleton.tsx` | Loading skeleton |
| sonner | `ui/sonner.tsx` | Toast notifications |
| tab-filter | `ui/tab-filter.tsx` | Tab-based filtering |
| tooltip | `ui/tooltip.tsx` | Hover tooltips (minor 2px radius exception) |
| agenda-draft-dialog | `ui/agenda-draft-dialog.tsx` | AI agenda draft modal |

### Non-Compliant (1/37)

| Component | File | Issue | Severity |
|-----------|------|-------|----------|
| **status-badge** | `ui/status-badge.tsx` | 24 hardcoded hex/rgba color values | HIGH |

See [VIOLATIONS.md](./VIOLATIONS.md) for details.

---

## Dashboard Components (5 components)

| Component | File | Job |
|-----------|------|-----|
| **DailyBriefing** | `dashboard/DailyBriefing.tsx` | The main briefing — hero/schedule/attention/finis. Uses CSS module extensively. BEST PRACTICE example. |
| **DashboardError** | `dashboard/DashboardError.tsx` | Error state for dashboard |
| **DashboardSkeleton** | `dashboard/DashboardSkeleton.tsx` | Loading skeleton for dashboard |
| **Header** | `dashboard/Header.tsx` | Legacy header (check if still used) |
| **RunNowButton** | `dashboard/RunNowButton.tsx` | Trigger briefing generation |
| **StatusIndicator** | `dashboard/StatusIndicator.tsx` | System status (`>_ ready`) |

---

## Risk Briefing (5 components)

Slide-deck-style risk report. Scroll-snap sections.

| Component | File | Job |
|-----------|------|-----|
| **RiskCover** | `risk-briefing/RiskCover.tsx` | Cover slide |
| **BottomLineSlide** | `risk-briefing/BottomLineSlide.tsx` | Executive summary |
| **WhatHappenedSlide** | `risk-briefing/WhatHappenedSlide.tsx` | What changed |
| **StakesSlide** | `risk-briefing/StakesSlide.tsx` | What's at stake |
| **TheAskSlide** | `risk-briefing/TheAskSlide.tsx` | Recommended action |
| **ThePlanSlide** | `risk-briefing/ThePlanSlide.tsx` | Action plan |

---

## Settings Components (5 components)

Redesigned as connections hub per I349.

| Component | File | Job |
|-----------|------|-----|
| **YouCard** | `settings/YouCard.tsx` | User profile |
| **ConnectionsGrid** | `settings/ConnectionsGrid.tsx` | Integration cards |
| **ConnectionDetail** | `settings/ConnectionDetail.tsx` | Individual connection config |
| **DiagnosticsSection** | `settings/DiagnosticsSection.tsx` | System health |
| **SystemStatus** | `settings/SystemStatus.tsx` | Backend status |

---

## Onboarding (10 components)

Multi-chapter onboarding flow.

| Component | File |
|-----------|------|
| **OnboardingFlow** | `onboarding/OnboardingFlow.tsx` |
| **FolderTree** | `onboarding/FolderTree.tsx` |
| **TourHighlight** | `onboarding/TourHighlight.tsx` |
| Welcome, GoogleConnect, AboutYou, Workspace, EntityMode, InternalTeamSetup, MeetingDeepDive, PrimeBriefing, PopulateWorkspace, ClaudeCode, DashboardTour, InboxTraining, Ready | `onboarding/chapters/` |

---

## Utility / Cross-Cutting

| Component | File | Job |
|-----------|------|-----|
| **CommandMenu** | `layout/CommandMenu.tsx` | `Cmd+K` command palette |
| **ProfileSelector** | `ProfileSelector.tsx` | Workspace profile switcher |
| **DevToolsPanel** | `devtools/DevToolsPanel.tsx` | Dev-only tools panel |
| **theme-provider** | `theme-provider.tsx` | Theme context (light mode only currently) |

---

## Dead Code (confirmed removed)

These should NOT exist. If you find them, delete them.

- `AppSidebar` — Replaced by FloatingNavIsland
- `WatchItem` — Replaced by WatchList internal rows
- `ActionList` / `ActionItem` — Legacy, replaced by editorial action rows
- `EmailList` — Not imported anywhere
