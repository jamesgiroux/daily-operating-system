# Frontend Components Audit

Comprehensive audit of all React components and pages in the DailyOS frontend.
Audited: 2026-03-02. Source: `src/components/` and `src/pages/`.

**Totals**: 155 component files, 23 page files, ~50,843 lines of TSX/TS.

---

## 1. Component Registry

### Pages (23 files)

| File | Lines | State | invoke() | listen() | Key Children |
|------|------:|:-----:|:--------:|:--------:|--------------|
| `pages/MeetingDetailPage.tsx` | 1,751 | Y | 8 | Y (3 events) | ActionRow, MeetingEntityChips, IntelligenceQualityBadge, ChapterHeading, FinisMarker, EditorialLoading, EditorialError, AgendaDraftDialog, FolioRefreshButton |
| `pages/MonthlyWrappedPage.tsx` | 1,550 | Y | 1 | N | AnimatedNumber, GeneratingProgress, Skeleton, Button |
| `pages/InboxPage.tsx` | 1,372 | Y | 1 | Y (drag-drop) | EditorialLoading, EditorialError, FinisMarker, GoogleDriveImportModal |
| `pages/AccountDetailEditorial.tsx` | 949 | Y | 4 | N | AccountHero, AccountAppendix, VitalsStrip, EditableVitalsStrip, StateOfPlay, StakeholderGallery, WatchList, UnifiedTimeline, TheWork, FileListSection, PresetFieldsEditor, ContextEntryList, EditorialLoading, EditorialError, FinisMarker |
| `pages/ActionDetailPage.tsx` | 855 | Y | 4 | N | ActionRow, EmptyState, ChapterHeading, FinisMarker |
| `pages/AccountsPage.tsx` | 837 | Y | N | N | EntityListShell, EntityRow, InlineCreateForm, BulkCreateForm, EmptyState, FinisMarker |
| `pages/ActionsPage.tsx` | 835 | Y | N | N | SharedActionRow, SharedProposedActionRow, PriorityPicker, EntityPicker, DatePicker, EmptyState, FinisMarker |
| `pages/WeekPage.tsx` | 822 | Y | 1 | N | ChapterHeading, EditorialLoading, EditorialError, FinisMarker, FolioRefreshButton, EmptyState |
| `pages/EmailsPage.tsx` | 784 | Y | 2 | N | EmptyState, FinisMarker, EmailEntityChip |
| `pages/MePage.tsx` | 679 | Y | N | N | ChapterHeading, EditorialLoading, FinisMarker, EmptyState, PresetFieldsEditor, EditableText, ContextEntryList |
| `pages/PeoplePage.tsx` | 685 | Y | 1 | N | EntityListShell, EntityRow, InlineCreateForm, BulkCreateForm, EmptyState, FinisMarker, AccountMergeDialog |
| `pages/ProjectDetailEditorial.tsx` | 730 | Y | 3 | N | ProjectHero, ProjectAppendix, VitalsStrip, EditableVitalsStrip, StateOfPlay, StakeholderGallery, WatchList, UnifiedTimeline, TheWork, FileListSection, PresetFieldsEditor, ContextEntryList, EditorialLoading, EditorialError, FinisMarker |
| `pages/ProjectsPage.tsx` | 520 | Y | N | N | EntityListShell, EntityRow, InlineCreateForm, BulkCreateForm, EmptyState, FinisMarker |
| `pages/PersonDetailEditorial.tsx` | 454 | Y | 2 | N | PersonHero, PersonAppendix, PersonInsightChapter, PersonRelationships, PersonNetwork, VitalsStrip, EditableVitalsStrip, StateOfPlay, StakeholderGallery, UnifiedTimeline, TheWork, FileListSection, PresetFieldsEditor, ContextEntryList, EditorialLoading, EditorialError, FinisMarker |
| `pages/EbrQbrPage.tsx` | 467 | Y | 1 | N | EbrCover, TheStorySlide, ValueDeliveredEbrSlide, MetricsSlide, NavigatedSlide, RoadmapSlide, NextStepsSlide, GeneratingProgress, EditorialLoading |
| `pages/SwotPage.tsx` | 459 | Y | 1 | N | SwotCover, QuadrantSlide, GeneratingProgress, EditorialLoading |
| `pages/AccountHealthPage.tsx` | 432 | Y | 1 | N | AccountHealthCover, WhereWeStandSlide, PartnershipSlide, ValueDeliveredSlide, WhatAheadSlide, GeneratingProgress, EditorialLoading |
| `pages/WeeklyImpactPage.tsx` | 414 | Y | 1 | N | CoverSlide, PrioritiesMovedSlide, TheWorkSlide, WatchSlide, IntoNextWeekSlide, GeneratingProgress, EditorialLoading |
| `pages/RiskBriefingPage.tsx` | 392 | Y | 2 | N | RiskCover, WhatHappenedSlide, StakesSlide, TheAskSlide, ThePlanSlide, BottomLineSlide, GeneratingProgress, EditorialLoading |
| `pages/HistoryPage.tsx` | 268 | Y | N | N | MeetingRow, EmptyState, FinisMarker |
| `pages/SettingsPage.tsx` | 221 | Y | N | N | YouCard, ConnectorsGrid, SystemStatus, DiagnosticsSection, ContextSourceSection, ActivityLogSection |
| `pages/ReportPage.tsx` | 144 | Y | N | N | ReportShell, SwotReport, AccountHealthReport, EbrQbrReport |
| `pages/MeetingHistoryDetailPage.tsx` | 9 | N | N | N | MeetingDetailPage (redirect) |

### Components by Subdirectory

#### `components/dashboard/` (8 files)

| File | Lines | State | invoke() | listen() | Parents |
|------|------:|:-----:|:--------:|:--------:|---------|
| `DailyBriefing.tsx` | 752 | Y | 1 | N | router.tsx (DashboardPage) |
| `BriefingMeetingCard.tsx` | 564 | Y | N | N | DailyBriefing |
| `DashboardEmpty.tsx` | 269 | Y | N | N | router.tsx (DashboardPage) |
| `DashboardSkeleton.tsx` | 122 | N | N | N | router.tsx (DashboardPage) |
| `StatusIndicator.tsx` | 220 | Y | N | N | Header |
| `Header.tsx` | 66 | N | N | N | router.tsx (RootLayout) |
| `RunNowButton.tsx` | 120 | Y | N | N | Header |
| `DashboardError.tsx` | 61 | N | N | N | router.tsx (DashboardPage) |

#### `components/editorial/` (10 files)

| File | Lines | State | invoke() | listen() | Parents |
|------|------:|:-----:|:--------:|:--------:|---------|
| `GeneratingProgress.tsx` | 267 | Y | N | N | EbrQbrPage, SwotPage, AccountHealthPage, WeeklyImpactPage, RiskBriefingPage, MonthlyWrappedPage, DashboardEmpty, PrimeBriefing |
| `EmptyState.tsx` | 114 | N | N | N | AccountsPage, ProjectsPage, PeoplePage, ActionsPage, EmailsPage, HistoryPage, WeekPage, MePage, ActionDetailPage |
| `EditorialLoading.tsx` | 73 | N | N | N | 8 pages + detail views |
| `EditorialError.tsx` | 31 | N | N | N | 8 pages + detail views |
| `ChapterHeading.tsx` | 53 | N | N | N | 32+ consumers (most-used editorial component) |
| `FinisMarker.tsx` | 42 | N | N | N | 20+ page/component consumers |
| `PullQuote.tsx` | 70 | N | N | N | AccountDetailEditorial, PersonDetailEditorial, ProjectDetailEditorial |
| `StateBlock.tsx` | 71 | Y | N | N | AccountDetailEditorial, ProjectDetailEditorial, PersonDetailEditorial, PersonInsightChapter |
| `TimelineEntry.tsx` | 76 | N | N | N | UnifiedTimeline |
| `BriefingCallouts.tsx` | 155 | N | N | N | **GHOST** |
| `EditorialEmpty.tsx` | 34 | N | N | N | **GHOST** |

#### `components/entity/` (13 files)

| File | Lines | State | invoke() | listen() | Parents |
|------|------:|:-----:|:--------:|:--------:|---------|
| `StakeholderGallery.tsx` | 694 | Y | 3 | N | AccountDetailEditorial, PersonDetailEditorial, ProjectDetailEditorial |
| `EditableVitalsStrip.tsx` | 484 | Y | N | N | AccountDetailEditorial, PersonDetailEditorial, ProjectDetailEditorial |
| `WatchList.tsx` | 305 | Y | N | N | AccountDetailEditorial, ProjectDetailEditorial, PersonDetailEditorial |
| `EntityListShell.tsx` | 228 | N | N | N | AccountsPage, ProjectsPage, PeoplePage |
| `FileListSection.tsx` | 225 | Y | 1 | N | AccountDetailEditorial, ProjectDetailEditorial, PersonDetailEditorial |
| `UnifiedTimeline.tsx` | 209 | Y | N | N | AccountDetailEditorial, ProjectDetailEditorial, PersonDetailEditorial |
| `TheWork.tsx` | 254 | Y | N | N | AccountDetailEditorial, ProjectDetailEditorial, PersonDetailEditorial |
| `IntelligenceQualityBadge.tsx` | 157 | N | N | N | AccountHero, PersonHero, ProjectHero, MeetingDetailPage, MeetingCard |
| `EngagementSelector.tsx` | 146 | Y | N | N | StakeholderGallery |
| `ContextEntryList.tsx` | 138 | Y | N | N | AccountDetailEditorial, ProjectDetailEditorial, PersonDetailEditorial, MePage |
| `PresetFieldsEditor.tsx` | 128 | N | N | N | AccountDetailEditorial, ProjectDetailEditorial, PersonDetailEditorial |
| `StateOfPlay.tsx` | 100 | Y | N | N | AccountDetailEditorial, ProjectDetailEditorial, PersonDetailEditorial |
| `EntityRow.tsx` | 99 | N | N | N | AccountsPage, ProjectsPage, PeoplePage |
| `VitalsStrip.tsx` | 63 | N | N | N | AccountDetailEditorial, ProjectDetailEditorial, PersonDetailEditorial |

#### `components/settings/` (10 files + 8 connectors)

| File | Lines | State | invoke() | listen() | Parents |
|------|------:|:-----:|:--------:|:--------:|---------|
| `SystemStatus.tsx` | 1,088 | Y | 5 | N | SettingsPage |
| `DiagnosticsSection.tsx` | 715 | Y | 2 | N | SettingsPage |
| `YouCard.tsx` | 658 | Y | 4 | N | SettingsPage |
| `ContextSourceSection.tsx` | 408 | Y | 2 | N | SettingsPage |
| `ActivityLogSection.tsx` | 300 | Y | 1 | N | SettingsPage |
| `ConnectorsGrid.tsx` | 208 | Y | 1+ (dynamic) | N | SettingsPage |
| `ConnectorDetail.tsx` | 59 | N | N | N | ConnectorsGrid |
| `styles.ts` | 97 | -- | -- | -- | (shared styles) |
| **Connectors (via registry):** | | | | | |
| `connectors/LinearConnector.tsx` | 593 | Y | 6 | N | ConnectorsGrid (via registry) |
| `connectors/ClayConnector.tsx` | 388 | Y | 8 | N | ConnectorsGrid (via registry) |
| `connectors/GoogleConnector.tsx` | 332 | Y | 2 | N | ConnectorsGrid (via registry) |
| `connectors/GoogleDriveConnector.tsx` | 277 | Y | 3 | N | ConnectorsGrid (via registry) |
| `connectors/ClaudeDesktopConnector.tsx` | 251 | Y | N | N | ConnectorsGrid (via registry) |
| `connectors/QuillConnector.tsx` | 231 | Y | 2 | N | ConnectorsGrid (via registry) |
| `connectors/GranolaConnector.tsx` | 198 | Y | 2 | N | ConnectorsGrid (via registry) |
| `connectors/GravatarConnector.tsx` | 174 | Y | 2 | N | ConnectorsGrid (via registry) |

#### `components/onboarding/` (15 files)

| File | Lines | State | invoke() | listen() | Parents |
|------|------:|:-----:|:--------:|:--------:|---------|
| `OnboardingFlow.tsx` | 301 | Y | 7 | N | router.tsx (RootLayout) |
| `chapters/AboutYou.tsx` | 449 | Y | 2 | N | OnboardingFlow (via render map) |
| `chapters/InternalTeamSetup.tsx` | 431 | Y | 1 | N | **GHOST** (not imported by OnboardingFlow) |
| `chapters/InboxTraining.tsx` | 427 | Y | 1 | Y (drag-drop) | **GHOST** (not imported by OnboardingFlow) |
| `chapters/DashboardTour.tsx` | 313 | Y | N | N | **GHOST** (not imported by OnboardingFlow) |
| `chapters/YouCardStep.tsx` | 266 | Y | 3 | N | OnboardingFlow |
| `chapters/ClaudeCode.tsx` | 249 | Y | N | N | OnboardingFlow |
| `chapters/PopulateWorkspace.tsx` | 237 | Y | 1 | N | **GHOST** (not imported by OnboardingFlow) |
| `chapters/PrimeBriefing.tsx` | 207 | Y | N | N | **GHOST** (not imported by OnboardingFlow) |
| `chapters/MeetingDeepDive.tsx` | 186 | N | N | N | **GHOST** (not imported by OnboardingFlow) |
| `chapters/GoogleConnect.tsx` | 176 | Y | N | N | OnboardingFlow |
| `chapters/EntityMode.tsx` | 128 | Y | 1 | N | OnboardingFlow |
| `chapters/FirstAccountStep.tsx` | 126 | Y | 2 | N | OnboardingFlow |
| `chapters/Workspace.tsx` | 155 | Y | 1 | N | OnboardingFlow |
| `chapters/Welcome.tsx` | 153 | N | N | N | OnboardingFlow |
| `chapters/Ready.tsx` | 175 | N | N | N | OnboardingFlow |
| `FolderTree.tsx` | 119 | N | N | N | Workspace, OnboardingFlow |
| `TourHighlight.tsx` | 28 | N | N | N | DashboardTour |

#### `components/layout/` (5 files)

| File | Lines | State | invoke() | listen() | Parents |
|------|------:|:-----:|:--------:|:--------:|---------|
| `MagazinePageLayout.tsx` | 203 | Y | N | N | router.tsx (RootLayout) |
| `FloatingNavIsland.tsx` | 257 | Y | N | N | MagazinePageLayout |
| `FolioBar.tsx` | 132 | N | N | N | MagazinePageLayout |
| `CommandMenu.tsx` | 200 | Y | 1 | N | router.tsx (RootLayout) |
| `AtmosphereLayer.tsx` | 42 | N | N | N | MagazinePageLayout |

#### `components/ui/` (38 files)

| File | Lines | State | invoke() | Parents (consumers) |
|------|------:|:-----:|:--------:|---------------------|
| `sidebar.tsx` | 724 | Y | N | router.tsx |
| `entity-picker.tsx` | 276 | Y | N | ActionsPage, MeetingEntityChips, PeoplePage, StakeholderGallery, InboxPage, LinearConnector |
| `EditableList.tsx` | 261 | Y | N | risk-briefing slides, weekly-impact slides |
| `meeting-entity-chips.tsx` | 220 | Y | 2 | MeetingDetailPage |
| `calendar.tsx` | 220 | Y | N | DatePicker |
| `EditableText.tsx` | 190 | Y | N | 32+ consumers (most-used editable component) |
| `email-entity-chip.tsx` | 140 | Y | 1 | EmailsPage, DailyBriefing |
| `agenda-draft-dialog.tsx` | 121 | Y | N | MeetingDetailPage |
| `date-picker.tsx` | 180 | Y | N | ActionsPage, ActionDetailPage |
| `button.tsx` | 64 | N | N | 39+ consumers |
| `input.tsx` | 21 | N | N | 10+ consumers |
| `dialog.tsx` | 147 | N | N | 7+ consumers |
| `command.tsx` | 174 | N | N | EntityPicker, CommandMenu |
| `badge.tsx` | 48 | N | N | 5+ consumers |
| `skeleton.tsx` | 13 | N | N | 8+ consumers |
| `tooltip.tsx` | 55 | N | N | 5+ consumers |
| `popover.tsx` | 87 | N | N | EntityPicker, DatePicker |
| `folio-refresh-button.tsx` | 52 | N | N | MeetingDetailPage, WeekPage, DailyBriefing, InboxPage |
| `alert-dialog.tsx` | 181 | N | N | AccountDetailEditorial, ProjectDetailEditorial, PeoplePage, PersonDetailEditorial |
| `card.tsx` | 86 | N | N | ConnectorsGrid, CommandMenu |
| `select.tsx` | 177 | N | N | SystemStatus, DiagnosticsSection |
| `separator.tsx` | 26 | N | N | sidebar |
| `sheet.tsx` | 145 | N | N | sidebar |
| `inline-create-form.tsx` | 40 | N | N | AccountsPage, ProjectsPage |
| `bulk-create-form.tsx` | 64 | N | N | AccountsPage, ProjectsPage |
| `priority-picker.tsx` | 41 | N | N | ActionsPage, ActionDetailPage |
| `Avatar.tsx` | 73 | Y | N | StakeholderGallery, PersonHero |
| `BrandMark.tsx` | 32 | N | N | FloatingNavIsland |
| `sonner.tsx` | 53 | N | N | router.tsx |
| `CyclingPill.tsx` | 72 | Y | N | **GHOST** |
| `copy-button.tsx` | 48 | Y | N | **GHOST** |
| `dropdown-menu.tsx` | 182 | N | N | **GHOST** |
| `email-signal-list.tsx` | 68 | N | N | **GHOST** |
| `list-row.tsx` | 76 | N | N | **GHOST** |
| `scroll-area.tsx` | 58 | N | N | **GHOST** |
| `search-input.tsx` | 29 | N | N | **GHOST** |
| `status-badge.tsx` | 62 | N | N | **GHOST** |
| `tab-filter.tsx` | 56 | N | N | **GHOST** |
| `collapsible.tsx` | 26 | N | N | **GHOST** |
| `label.tsx` | 24 | N | N | **GHOST** (Label is used in types but not as JSX import) |

#### `components/shared/` (4 files)

| File | Lines | State | invoke() | Parents |
|------|------:|:-----:|:--------:|---------|
| `ActionRow.tsx` | 376 | Y | N | ActionsPage, ActionDetailPage, MeetingDetailPage |
| `MeetingCard.tsx` | 133 | N | N | WeekPage, DailyBriefing |
| `MeetingRow.tsx` | 89 | N | N | HistoryPage |
| `ProposedActionRow.tsx` | 157 | N | N | ActionsPage, DailyBriefing |

#### `components/account/` (5 files)

| File | Lines | State | invoke() | Parents |
|------|------:|:-----:|:--------:|---------|
| `AccountAppendix.tsx` | 369 | N | N | AccountDetailEditorial |
| `AccountHero.tsx` | 274 | Y | N | AccountDetailEditorial |
| `AccountMergeDialog.tsx` | 237 | Y | N | PeoplePage |
| `LifecycleEventDrawer.tsx` | 200 | Y | N | AccountDetailEditorial |
| `WatchListPrograms.tsx` | 159 | N | N | AccountAppendix |

#### `components/person/` (5 files)

| File | Lines | State | invoke() | Parents |
|------|------:|:-----:|:--------:|---------|
| `PersonRelationships.tsx` | 395 | Y | 2 | PersonDetailEditorial |
| `PersonAppendix.tsx` | 294 | N | N | PersonDetailEditorial |
| `PersonHero.tsx` | 291 | Y | N | PersonDetailEditorial |
| `PersonInsightChapter.tsx` | 215 | N | N | PersonDetailEditorial |
| `PersonNetwork.tsx` | 140 | Y | N | PersonDetailEditorial |

#### `components/project/` (5 files)

| File | Lines | State | invoke() | Parents |
|------|------:|:-----:|:--------:|---------|
| `ProjectHero.tsx` | 197 | Y | N | ProjectDetailEditorial |
| `ProjectAppendix.tsx` | 180 | N | N | ProjectDetailEditorial |
| `TrajectoryChapter.tsx` | 180 | N | N | ProjectDetailEditorial |
| `HorizonChapter.tsx` | 170 | N | N | ProjectDetailEditorial |
| `WatchListMilestones.tsx` | 104 | N | N | ProjectAppendix |

#### `components/risk-briefing/` (6 files)

| File | Lines | State | invoke() | Parents |
|------|------:|:-----:|:--------:|---------|
| `ThePlanSlide.tsx` | 284 | Y | N | RiskBriefingPage |
| `StakesSlide.tsx` | 267 | Y | N | RiskBriefingPage |
| `TheAskSlide.tsx` | 262 | Y | N | RiskBriefingPage |
| `WhatHappenedSlide.tsx` | 248 | Y | N | RiskBriefingPage |
| `RiskCover.tsx` | 173 | Y | N | RiskBriefingPage |
| `BottomLineSlide.tsx` | 110 | Y | N | RiskBriefingPage |

#### `components/account-health/` (5 files + types)

| File | Lines | State | invoke() | Parents |
|------|------:|:-----:|:--------:|---------|
| `WhereWeStandSlide.tsx` | 350 | Y | N | AccountHealthPage |
| `WhatAheadSlide.tsx` | 338 | Y | N | AccountHealthPage |
| `ValueDeliveredSlide.tsx` | 194 | Y | N | AccountHealthPage |
| `PartnershipSlide.tsx` | 180 | Y | N | AccountHealthPage |
| `AccountHealthCover.tsx` | 143 | N | N | AccountHealthPage |
| `types.ts` | 31 | -- | -- | (shared types) |

#### `components/ebr-qbr/` (7 files)

| File | Lines | State | invoke() | Parents |
|------|------:|:-----:|:--------:|---------|
| `ValueDeliveredEbrSlide.tsx` | 269 | Y | N | EbrQbrPage |
| `MetricsSlide.tsx` | 219 | Y | N | EbrQbrPage |
| `NavigatedSlide.tsx` | 200 | Y | N | EbrQbrPage |
| `NextStepsSlide.tsx` | 193 | Y | N | EbrQbrPage |
| `TheStorySlide.tsx` | 188 | Y | N | EbrQbrPage |
| `EbrCover.tsx` | 137 | Y | N | EbrQbrPage |
| `RoadmapSlide.tsx` | 59 | N | N | EbrQbrPage |

#### `components/weekly-impact/` (5 files)

| File | Lines | State | invoke() | Parents |
|------|------:|:-----:|:--------:|---------|
| `TheWorkSlide.tsx` | 197 | Y | N | WeeklyImpactPage |
| `PrioritiesMovedSlide.tsx` | 179 | Y | N | WeeklyImpactPage |
| `IntoNextWeekSlide.tsx` | 158 | Y | N | WeeklyImpactPage |
| `WatchSlide.tsx` | 150 | Y | N | WeeklyImpactPage |
| `CoverSlide.tsx` | 89 | N | N | WeeklyImpactPage |

#### `components/swot/` (2 files)

| File | Lines | State | invoke() | Parents |
|------|------:|:-----:|:--------:|---------|
| `SwotCover.tsx` | 171 | Y | N | SwotPage |
| `QuadrantSlide.tsx` | 168 | Y | N | SwotPage |

#### `components/reports/` (7 files)

| File | Lines | State | invoke() | Parents |
|------|------:|:-----:|:--------:|---------|
| `MonthlyWrappedReport.tsx` | 277 | N | N | **GHOST** |
| `EbrQbrReport.tsx` | 241 | N | N | ReportPage |
| `ReportShell.tsx` | 184 | Y | N | ReportPage |
| `AccountHealthReport.tsx` | 147 | N | N | ReportPage |
| `SwotReport.tsx` | 165 | N | N | ReportPage |
| `WeeklyImpactReport.tsx` | 162 | N | N | **GHOST** |
| `ReportSection.tsx` | 35 | N | N | AccountHealthReport, EbrQbrReport, SwotReport, MonthlyWrappedReport, WeeklyImpactReport |
| `ReportExportButton.tsx` | 21 | N | N | **GHOST** |

#### Other standalone components

| File | Lines | State | invoke() | listen() | Parents |
|------|------:|:-----:|:--------:|:--------:|---------|
| `PostMeetingPrompt.tsx` | 449 | Y | N | N | router.tsx (RootLayout) |
| `inbox/GoogleDriveImportModal.tsx` | 537 | Y | 2 | N | InboxPage |
| `notifications/WhatsNewModal.tsx` | 228 | Y | N | N | router.tsx (RootLayout) |
| `notifications/UpdateBanner.tsx` | 143 | Y | N | N | router.tsx (RootLayout) |
| `devtools/DevToolsPanel.tsx` | 574 | Y | N | N | router.tsx (RootLayout) |
| `ProfileSelector.tsx` | 137 | Y | 1 | N | **GHOST** |
| `EncryptionRecovery.tsx` | 113 | Y | N | N | router.tsx (RootLayout) |
| `ICloudWarningModal.tsx` | 110 | Y | 1 | N | router.tsx (RootLayout) |
| `tour/TourTips.tsx` | 107 | Y | N | N | router.tsx (RootLayout) |
| `LockOverlay.tsx` | 43 | N | 1 | N | router.tsx (RootLayout) |
| `theme-provider.tsx` | 42 | Y | N | N | router.tsx (RootLayout) |
| `monthly-wrapped/AnimatedNumber.tsx` | 51 | Y | N | N | MonthlyWrappedPage |

---

## 2. Ghost Components

Components that are exported but never imported by any other file (including via relative path, registry, or router).

| File | Lines | Recommendation |
|------|------:|----------------|
| **Onboarding chapters (not wired into flow):** | | |
| `onboarding/chapters/DashboardTour.tsx` | 313 | **Wire in or delete** -- substantial component that references live data, designed for onboarding but not imported by OnboardingFlow. |
| `onboarding/chapters/InternalTeamSetup.tsx` | 431 | **Wire in or delete** -- fully built with invoke() calls. Was likely planned for onboarding but cut from the chapter list. |
| `onboarding/chapters/InboxTraining.tsx` | 427 | **Wire in or delete** -- has drag-drop listen(), invoke(), a complete UI. Never referenced. |
| `onboarding/chapters/PopulateWorkspace.tsx` | 237 | **Wire in or delete** -- has invoke("populate_workspace"). Built but unreferenced. |
| `onboarding/chapters/PrimeBriefing.tsx` | 207 | **Wire in or delete** -- loading/progress UI for initial briefing generation. |
| `onboarding/chapters/MeetingDeepDive.tsx` | 186 | **Wire in or delete** -- static educational content. |
| **Reports not used by ReportPage:** | | |
| `reports/MonthlyWrappedReport.tsx` | 277 | **Likely replaced** -- MonthlyWrappedPage uses its own custom Wrapped slides instead of this report format. Delete. |
| `reports/WeeklyImpactReport.tsx` | 162 | **Likely replaced** -- WeeklyImpactPage uses its own custom slide components instead. Delete. |
| `reports/ReportExportButton.tsx` | 21 | **Future use** -- stub for report export. Keep if planned; otherwise delete. |
| **UI primitives never used:** | | |
| `ui/CyclingPill.tsx` | 72 | **Delete** -- animated pill component, never used anywhere. |
| `ui/copy-button.tsx` | 48 | **Delete** -- wraps clipboard copy. MeetingDetailPage implements its own copy logic inline. |
| `ui/dropdown-menu.tsx` | 182 | **Keep (shadcn)** -- standard shadcn/ui primitive. May be used in future. |
| `ui/email-signal-list.tsx` | 68 | **Delete** -- was for email signal display but EmailsPage implements its own. |
| `ui/list-row.tsx` | 76 | **Delete** -- generic list row, superseded by EntityRow and ActionRow patterns. |
| `ui/scroll-area.tsx` | 58 | **Keep (shadcn)** -- standard shadcn/ui primitive. |
| `ui/search-input.tsx` | 29 | **Delete** -- simple wrapper, pages implement their own search. |
| `ui/status-badge.tsx` | 62 | **Delete** -- exports style maps but no component imports them. |
| `ui/tab-filter.tsx` | 56 | **Delete** -- generic tab filter, pages implement their own tab logic. |
| `ui/collapsible.tsx` | 26 | **Keep (shadcn)** -- standard shadcn/ui primitive. |
| `ui/label.tsx` | 24 | **Keep (shadcn)** -- Label used as type import, may be used in future forms. |
| **Other:** | | |
| `editorial/BriefingCallouts.tsx` | 155 | **Delete** -- callout cards, never used. DailyBriefing has its own callout rendering. |
| `editorial/EditorialEmpty.tsx` | 34 | **Delete** -- superseded by `editorial/EmptyState.tsx` which is the active empty state. |
| `ProfileSelector.tsx` | 137 | **Delete or wire in** -- profile switching UI with invoke("set_profile"). Not connected to any page. |

**Summary**: 23 ghost files totaling ~3,440 lines. 7 are shadcn/ui primitives to keep. 6 are substantial onboarding chapters (2,801 lines total) that were built but never wired into OnboardingFlow -- these represent the biggest waste and should be either integrated or removed. The remaining 10 are superseded or duplicate patterns.

---

## 3. Business Logic in Presentation

Components and pages containing business logic that should be extracted into custom hooks or utility modules.

### Critical (complex logic embedded in render files)

| File | Lines | Logic | Should Move To |
|------|-------|-------|----------------|
| `pages/MeetingDetailPage.tsx` | 48-79, 82-91, 106-207, 209-300+ | Time parsing (parseDisplayTimeMs), meeting start computation, attendee unification, prep data reconciliation, transient DB retry logic, transcript sync, refresh progress handling | `useMeetingDetail` hook (fetch + state) + `lib/meeting-utils.ts` (time parsing, attendee unification) |
| `pages/ActionsPage.tsx` | 30-69, 76-140 | Priority sorting, meeting-group label formatting with relative dates, action grouping by meeting context with time-band fallback | `lib/action-grouping.ts` or `useActionGroups` hook |
| `pages/InboxPage.tsx` | 54-120 | File classification logic (fileTypeClassifications, mdClassifications, classifyFile function), file state management | `lib/inbox-classify.ts` |
| `pages/MonthlyWrappedPage.tsx` | 73-110 | Schema normalization (normalizeMonthlyWrapped), defensive array/object coercion | `lib/monthly-wrapped-normalize.ts` |
| `pages/WeekPage.tsx` | 60-160+ | Meeting grouping by day, readiness computation, day-of-week labeling | `useWeekViewModel` hook (partially exists as test) |
| `pages/EmailsPage.tsx` | 30-100+ | Email grouping by entity, relevance scoring, signal filtering | `useEmailGroups` hook |
| `dashboard/DailyBriefing.tsx` | 48-140 | Time parsing, up-next meeting selection, unprepped-high-stakes detection, readiness computation, action sorting by relevance | `lib/briefing-utils.ts` |
| `settings/SystemStatus.tsx` | 22-69 | formatTime helper, cronToHumanTime converter, inline ChevronSvg component | `lib/schedule-utils.ts` (cron/time formatting) |
| `settings/DiagnosticsSection.tsx` | 40-69 | cronToHumanTime (duplicate of SystemStatus) | `lib/schedule-utils.ts` (shared) |
| `settings/ActivityLogSection.tsx` | 132-150 | groupByDay function for audit log entries | `lib/date-grouping.ts` |

### Moderate (invoke() calls that should be in hooks)

| File | invoke() calls | Should Move To |
|------|---------------|----------------|
| `entity/StakeholderGallery.tsx` | 3 (update_intelligence_field, update_stakeholders, link_person_entity) | `useStakeholderMutations` hook |
| `entity/FileListSection.tsx` | 1 (reveal_in_finder) | Acceptable (simple one-liner) |
| `person/PersonRelationships.tsx` | 2 (upsert_person_relationship, delete_person_relationship) | `usePersonRelationships` hook |
| `ui/meeting-entity-chips.tsx` | 2 (add_meeting_entity, remove_meeting_entity) | `useMeetingEntities` hook |
| `ui/email-entity-chip.tsx` | 1 (update_email_entity) | `useEmailEntity` hook |
| `settings/*Connector.tsx` | 2-8 each | Acceptable (settings are self-contained) |
| `onboarding/OnboardingFlow.tsx` | 7 | Acceptable (wizard is self-contained) |
| `onboarding/chapters/*.tsx` | 1-3 each | Acceptable (steps are self-contained) |

### Duplicate Logic

| Logic | Found In | Should Be |
|-------|----------|-----------|
| `cronToHumanTime()` | SystemStatus.tsx, DiagnosticsSection.tsx | Single `lib/schedule-utils.ts` |
| Time parsing (AM/PM string to ms) | DailyBriefing.tsx (parseDisplayTimeMs), BriefingMeetingCard.tsx (getTemporalState) | Single `lib/time-utils.ts` |
| `formatTime(iso)` helper | SystemStatus.tsx, ActivityLogSection.tsx | Single `lib/format-utils.ts` |
| Relative date formatting | ActionsPage.tsx, AccountHero.tsx, PersonHero.tsx, ProjectHero.tsx | Already exists in `lib/utils.ts` (formatRelativeDate) but ActionsPage re-implements |

---

## 4. Prop Drilling Instances

### Hero Component Prop Explosion

All three hero components (AccountHero, PersonHero, ProjectHero) receive 10-15+ props drilled from their parent detail page, including:
- `editName`, `setEditName`, `editHealth`/`editRole`/`editStatus`, `setEditHealth`/...
- `onSave`, `onSaveField`, `onEnrich`, `enriching`, `enrichSeconds`
- `onArchive`, `onUnarchive`

**Path**: DetailPage -> Hero -> (renders inline-editable fields)

This is 2-level prop drilling, not 3+, but the prop count (10-15 per hero) is a code smell. Consider a `useEntityEditor` hook that both the page and hero share via context.

### Report Slide `onUpdate` Chains

All report page types (RiskBriefingPage, EbrQbrPage, AccountHealthPage, WeeklyImpactPage, SwotPage) drill an `onUpdate` callback through to every slide component:

**Path**: ReportPage -> useMemo(content) -> SlideComponent({ content, onUpdate })

This is 1-level drilling and is acceptable, but the pattern is repeated 6 times identically across report pages with the same auto-save + debounce logic. A `useEditableReport` hook could centralize this.

### MeetingDetailPage -> ActionRow

`onRefresh` callback from MeetingDetailPage is drilled through to ActionRow (2 levels), which uses it after invoke() calls for complete/reopen/accept/reject.

---

## 5. Oversized Components (>200 lines)

| File | Lines | Recommendation |
|------|------:|----------------|
| `pages/MeetingDetailPage.tsx` | 1,751 | **Split urgently**. Extract: attendee unification logic, prep data loading, transcript handling, outcome display, and the 5+ chapter render sections into sub-components. |
| `pages/MonthlyWrappedPage.tsx` | 1,550 | **Split**. 10 full-screen slides are all inline. Extract each slide into its own component (like other report pages do). |
| `pages/InboxPage.tsx` | 1,372 | **Split**. File classification, drag-drop handling, entity assignment, and file list rendering should be separate. |
| `settings/SystemStatus.tsx` | 1,088 | **Split**. Contains 5+ independent sections (UpdateSection, ModelSection, ScheduleSection, HygieneSection, CaptureSection, LockSection). Each is already a local function -- promote to files. |
| `pages/AccountDetailEditorial.tsx` | 949 | Moderate. The editorial detail pages are long but well-structured with child components. |
| `pages/ActionDetailPage.tsx` | 855 | Moderate. Complex but single-purpose. |
| `pages/AccountsPage.tsx` | 837 | Moderate. Entity list pages share a pattern that could be unified. |
| `pages/ActionsPage.tsx` | 835 | Moderate. Grouping logic should extract to hook (see Section 3). |
| `pages/WeekPage.tsx` | 822 | Moderate. Has a test file for viewmodel (`weekPageViewModel.test.ts`) suggesting extraction was planned. |
| `pages/EmailsPage.tsx` | 784 | Moderate. Similar to ActionsPage -- grouping logic should extract. |
| `dashboard/DailyBriefing.tsx` | 752 | **Split**. Meeting selection, readiness computation, and the 3 main sections should be separate. |
| `pages/ProjectDetailEditorial.tsx` | 730 | Moderate. Same pattern as AccountDetailEditorial. |
| `settings/DiagnosticsSection.tsx` | 715 | **Split**. Multiple independent diagnostic panels (entity browser, queue viewer, signal inspector). |
| `entity/StakeholderGallery.tsx` | 694 | **Split**. Inline stakeholder editing, search, add, link-to-person -- extract modal/form portions. |
| `pages/PeoplePage.tsx` | 685 | Moderate. Entity list with merge dialog. |
| `pages/MePage.tsx` | 679 | Moderate. User profile page with multiple sections. |
| `settings/YouCard.tsx` | 658 | **Split**. Role selection, workspace config, personality, schedule -- 4 independent sections. |
| `settings/connectors/LinearConnector.tsx` | 593 | **Split**. Auth, sync, entity linking -- 3 distinct concerns. |
| `devtools/DevToolsPanel.tsx` | 574 | Moderate. Dev-only panel, complexity is acceptable. |
| `dashboard/BriefingMeetingCard.tsx` | 564 | **Split**. Meeting card with temporal state, progress bar, intel badges, prep quality -- extract temporal logic. |
| `inbox/GoogleDriveImportModal.tsx` | 537 | **Split**. File picker, preview, import -- 3 clear stages. |

---

## 6. Duplicate Pattern Clusters

### Cluster 1: Entity Detail Pages

`AccountDetailEditorial.tsx`, `ProjectDetailEditorial.tsx`, and `PersonDetailEditorial.tsx` share 80%+ of their structure:
- Hook-based data loading (useAccountDetail / useProjectDetail / usePersonDetail)
- Magazine shell registration with identical chapter nav
- VitalsStrip / EditableVitalsStrip toggle
- StateOfPlay, StakeholderGallery, WatchList, UnifiedTimeline, TheWork, FileListSection, PresetFieldsEditor, ContextEntryList sections
- Edit state management (editName, onSaveField pattern)
- EnrichingProgress overlay

**Recommendation**: Create `EntityDetailEditorial` shell component that accepts entity-specific Hero, Appendix, and custom sections via render props or slots.

### Cluster 2: Report Slide Pages

`EbrQbrPage`, `SwotPage`, `AccountHealthPage`, `WeeklyImpactPage`, `RiskBriefingPage` share:
- Load report -> generate if missing -> poll for completion -> display slides
- Auto-save with debounced `invoke("save_report")` on content change
- GeneratingProgress overlay during generation
- Slide-based layout with `onUpdate` callbacks

**Recommendation**: Extract `useEditableReport(reportType, entityId)` hook and `ReportPageShell` component.

### Cluster 3: Entity List Pages

`AccountsPage`, `ProjectsPage`, `PeoplePage` share:
- Entity list with search/filter
- EntityListShell (skeleton + error states)
- EntityRow rendering
- InlineCreateForm / BulkCreateForm for new entities
- EmptyState when list is empty

**Recommendation**: Create `EntityListPage` generic component parameterized by entity type.

### Cluster 4: Empty State Components

Three separate empty state implementations:
- `editorial/EmptyState.tsx` (114 lines) -- the active one, used by 9+ pages
- `editorial/EditorialEmpty.tsx` (34 lines) -- ghost, older version
- `dashboard/DashboardEmpty.tsx` (269 lines) -- dashboard-specific with Google auth and workflow triggers

**Recommendation**: Delete EditorialEmpty. DashboardEmpty is legitimately different (has workflow integration). EmptyState is the standard.

### Cluster 5: Hero Components

AccountHero (274), PersonHero (291), ProjectHero (197) share:
- Watermark asterisk background
- Editable name (h1, 76px serif)
- Intelligence quality badge
- Enrichment progress overlay
- Meta row with archive/unarchive/refresh actions
- Inline-editable secondary fields (health/role/status)

**Recommendation**: Extract `EntityHero` base component with slots for entity-specific badges and fields.

### Cluster 6: Appendix Components

AccountAppendix (369), PersonAppendix (294), ProjectAppendix (180) all render:
- Recent meetings list
- Recent emails list
- File attachments
- Entity-specific additional sections

**Recommendation**: Extract `EntityAppendix` with shared meeting/email/file sections and entity-specific slots.

---

## 7. Summary Statistics

| Metric | Count |
|--------|------:|
| Total component files | 155 |
| Total page files | 23 |
| Total lines of code | ~50,843 |
| Files with local state (useState/useReducer) | 103 |
| Files with invoke() calls | 48 |
| Files with listen() subscriptions | 2 |
| Ghost components (true unused) | 23 |
| Ghost component lines (wasted) | ~3,440 |
| Components >200 lines | 42 |
| Components >500 lines | 14 |
| Components >1000 lines | 4 |
| Duplicate logic instances | 4 clusters |
| Duplicate structural patterns | 6 clusters |

### Highest-Impact Improvements (by effort/reward)

1. **Extract MeetingDetailPage business logic** (1,751 lines, 63 logic operations) into `useMeetingDetail` hook and `lib/meeting-utils.ts`. Highest single-file complexity.

2. **Wire or delete 6 ghost onboarding chapters** (2,801 lines). These are fully built components rotting in the tree. Either complete the onboarding flow or remove the dead code.

3. **Unify entity detail pages** into a shared shell. Three 450-950 line files with 80% identical structure. One generic component + entity-specific slots would eliminate ~1,000 lines of duplication.

4. **Extract report page pattern** into `useEditableReport` hook. Five pages share identical load-generate-poll-save-display logic.

5. **Delete 10 dead UI primitives** (~700 lines). CyclingPill, copy-button, email-signal-list, list-row, search-input, status-badge, tab-filter, EditorialEmpty, BriefingCallouts, ProfileSelector.

6. **Split MonthlyWrappedPage** (1,550 lines) into per-slide components matching the pattern used by every other report page.

7. **Deduplicate cronToHumanTime** and time formatting helpers into shared utility modules.
