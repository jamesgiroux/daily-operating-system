# Component Inventory

**Last audited:** 2026-03-15
**Rule:** Check this list before building a new component. If something similar exists, extend it.

---

## Layout Shell (4 components)

These wrap every page. Non-negotiable.

| Component | File | Job | Notes |
|-----------|------|-----|-------|
| **MagazinePageLayout** | `layout/MagazinePageLayout.tsx` | Page wrapper. Registers shell config (folio, nav, atmosphere). Renders content at max-width with proper padding. | Every page uses this. |
| **FolioBar** | `layout/FolioBar.tsx` | Fixed top bar. Brand mark, page label, date, context actions, status. | Frosted glass, 40px height. Context actions vary per page via shellConfig. |
| **FloatingNavIsland** | `layout/FloatingNavIsland.tsx` | Fixed right nav. Icon grid with tooltips. Active state color varies per page. Two modes: `app` (page nav) and `chapters` (scroll nav). | Frosted glass. |
| **AtmosphereLayer** | `layout/AtmosphereLayer.tsx` | Background radial gradients. Page-specific color. Breathing animation. | Fixed position, z: 0. |

---

## Editorial Components (11 components)

The building blocks of the magazine aesthetic.

| Component | File | Job | Compliance |
|-----------|------|-----|------------|
| **ChapterHeading** | `editorial/ChapterHeading.tsx` | Section header. Newsreader 28px + thin rule above. | Compliant |
| **FinisMarker** | `editorial/FinisMarker.tsx` | End-of-page marker. `* * *` with closing message. | Compliant |
| **PullQuote** | `editorial/PullQuote.tsx` | Focus callout. Turmeric left border, italic serif. | Compliant |
| **EditorialEmpty** | `editorial/EditorialEmpty.tsx` | Empty state. Serif italic title + sans description. | Compliant |
| **EditorialError** | `editorial/EditorialError.tsx` | Error state for editorial pages. | Compliant |
| **EditorialLoading** | `editorial/EditorialLoading.tsx` | Skeleton loading for editorial pages. | Compliant |
| **EmptyState** | `editorial/EmptyState.tsx` | Rich empty state with headline, explanation, benefit, action CTA. | Compliant |
| **GeneratingProgress** | `editorial/GeneratingProgress.tsx` | Loading state for report generation. Phased messages + quotes. | Compliant |
| **StateBlock** | `editorial/StateBlock.tsx` | Structured state display (working/struggling, momentum/headwinds). | Compliant |
| **TimelineEntry** | `editorial/TimelineEntry.tsx` | Timeline row with entity color accent. | Compliant. Has CSS module. |
| **BriefingCallouts** | `editorial/BriefingCallouts.tsx` | Callout boxes within briefing content. | Compliant |

---

## Entity Components (13 components)

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
| **EditableVitalsStrip** | `entity/EditableVitalsStrip.tsx` | Inline-editable vitals (I343). | Compliant | |
| **EngagementSelector** | `entity/EngagementSelector.tsx` | Engagement level picker for stakeholders. | Compliant | |
| **PresetFieldsEditor** | `entity/PresetFieldsEditor.tsx` | Role preset field editor. | Compliant | Should render inline per ADR-0084. |
| **ContextEntryList** | `entity/ContextEntryList.tsx` | CRUD list for professional context entries. Used on Me page. | Compliant | |
| **IntelligenceQualityBadge** | `entity/IntelligenceQualityBadge.tsx` | Quality indicator badge for meeting intelligence. | Compliant | |

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

### Person (5 components)

| Component | File | Job |
|-----------|------|-----|
| **PersonHero** | `person/PersonHero.tsx` | Person page hero. Name, assessment, temperature. Has CSS module. |
| **PersonAppendix** | `person/PersonAppendix.tsx` | Profile details, notes, files. |
| **PersonInsightChapter** | `person/PersonInsightChapter.tsx` | The Dynamic/Rhythm — relationship analysis. |
| **PersonNetwork** | `person/PersonNetwork.tsx` | Connected entities network view. |
| **PersonRelationships** | `person/PersonRelationships.tsx` | Relationship graph display. |

---

## UI Primitives (36 components)

Radix-based primitives. All are compliant. Used across the app.

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
| date-picker | `ui/date-picker.tsx` | Date selection (editorial calendar) |
| dialog | `ui/dialog.tsx` | Modal dialog |
| dropdown-menu | `ui/dropdown-menu.tsx` | Context/dropdown menus |
| email-entity-chip | `ui/email-entity-chip.tsx` | Entity color chip for email thread context |
| entity-picker | `ui/entity-picker.tsx` | Link entity to meetings |
| inline-create-form | `ui/inline-create-form.tsx` | Inline entity/action creation |
| input | `ui/input.tsx` | Text input |
| label | `ui/label.tsx` | Form label |
| meeting-entity-chips | `ui/meeting-entity-chips.tsx` | Entity color chips on meeting rows |
| popover | `ui/popover.tsx` | Positioned popup |
| priority-picker | `ui/priority-picker.tsx` | Action priority selector |
| ProvenanceLabel | `ui/ProvenanceLabel.tsx` | Field data lineage + conflict resolution. Mono labels, source attribution, accept/dismiss actions for Glean-sourced value conflicts. |
| ProvenanceTag | `ui/ProvenanceTag.tsx` | Data source provenance indicator (source + confidence) |
| select | `ui/select.tsx` | Dropdown select |
| separator | `ui/separator.tsx` | Visual divider |
| sheet | `ui/sheet.tsx` | Side drawer |
| sidebar | `ui/sidebar.tsx` | Legacy sidebar (dead code -- AppSidebar removed) |
| skeleton | `ui/skeleton.tsx` | Loading skeleton |
| sonner | `ui/sonner.tsx` | Toast notifications |
| tooltip | `ui/tooltip.tsx` | Hover tooltips (minor 2px radius exception) |
| agenda-draft-dialog | `ui/agenda-draft-dialog.tsx` | AI agenda draft modal |
| EditableText | `ui/EditableText.tsx` | Click-to-edit text (single/multiline). Used on Me page. |
| EditableList | `ui/EditableList.tsx` | Editable string list with add/remove. Used on Me page. |
| IntelligenceFeedback | `ui/IntelligenceFeedback.tsx` | Thumbs up/down feedback for intelligence quality. |
| folio-refresh-button | `ui/folio-refresh-button.tsx` | Refresh button for folio bar actions. |
| EditableInline | `ui/editable-inline.tsx` | Click-to-edit short inline text. Has CSS module. Extracted from ActionDetailPage. |
| EditableTextarea | `ui/editable-textarea.tsx` | Click-to-edit multiline text. Has CSS module. Extracted from ActionDetailPage. |
| EditableDate | `ui/editable-date.tsx` | Date picker via Popover + Calendar. Has CSS module. Extracted from ActionDetailPage. |

**Removed since last audit:** `copy-button`, `email-signal-list`, `list-row`, `scroll-area`, `search-input`, `tab-filter` -- no longer present in the codebase.

All UI primitives are compliant.

---

## Shared Components (8 components)

Cross-entity components used on multiple pages (meeting detail, briefing, entity detail).

| Component | File | Job |
|-----------|------|-----|
| **ActionRow** | `shared/ActionRow.tsx` | Compact action row with priority pill, status, entity link. Used in meeting detail, entity detail. Has CSS module. |
| **MeetingCard** | `shared/MeetingCard.tsx` | Rich meeting card with entity chips, prep status, time. Has CSS module. |
| **MeetingRow** | `shared/MeetingRow.tsx` | Compact meeting row for lists and timelines. |
| **ProposedActionRow** | `shared/ProposedActionRow.tsx` | Suggested action row with accept/dismiss buttons. |
| **HealthBadge** | `shared/HealthBadge.tsx` | Account health score badge with color coding. Has CSS module. |
| **DimensionBar** | `shared/DimensionBar.tsx` | Health dimension bar chart visualization. Has CSS module. |
| **StatusDot** | `shared/StatusDot.tsx` | Colored status indicator dot. Has CSS module. |
| **TalkBalanceBar** | `shared/TalkBalanceBar.tsx` | Meeting talk-time balance visualization. Has CSS module. |

---

## Meeting Components (1 component)

| Component | File | Job |
|-----------|------|-----|
| **PostMeetingIntelligence** | `meeting/PostMeetingIntelligence.tsx` | Post-meeting intelligence display (talk balance, key moments, outcomes). Has CSS module. |

---

## Dashboard Components (8 components)

| Component | File | Job |
|-----------|------|-----|
| **DailyBriefing** | `dashboard/DailyBriefing.tsx` | The main briefing — hero/schedule/attention/finis. Uses CSS module extensively. BEST PRACTICE example. |
| **BriefingMeetingCard** | `dashboard/BriefingMeetingCard.tsx` | Meeting card within the daily briefing schedule. |
| **DashboardSkeleton** | `dashboard/DashboardSkeleton.tsx` | Loading skeleton for dashboard |
| **DashboardError** | `dashboard/DashboardError.tsx` | Error state for dashboard |
| **DashboardEmpty** | `dashboard/DashboardEmpty.tsx` | Empty/cold-start state for dashboard. Generate CTA, Google auth check. |
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

## Account Health Report (5 components)

Slide-deck-style account health review. Scroll-snap sections.

| Component | File | Job |
|-----------|------|-----|
| **AccountHealthCover** | `account-health/AccountHealthCover.tsx` | Cover slide with name, assessment, health narrative |
| **PartnershipSlide** | `account-health/PartnershipSlide.tsx` | Relationship summary, engagement cadence |
| **WhereWeStandSlide** | `account-health/WhereWeStandSlide.tsx` | Working/struggling, expansion signals |
| **ValueDeliveredSlide** | `account-health/ValueDeliveredSlide.tsx` | Value items and risks |
| **WhatAheadSlide** | `account-health/WhatAheadSlide.tsx` | Renewal context, recommended actions |

---

## EBR/QBR Report (7 components)

Slide-deck-style quarterly business review. Scroll-snap sections.

| Component | File | Job |
|-----------|------|-----|
| **EbrCover** | `ebr-qbr/EbrCover.tsx` | Cover slide with account name, quarter label |
| **TheStorySlide** | `ebr-qbr/TheStorySlide.tsx` | Story bullets, customer quote |
| **ValueDeliveredEbrSlide** | `ebr-qbr/ValueDeliveredEbrSlide.tsx` | Value items delivered |
| **MetricsSlide** | `ebr-qbr/MetricsSlide.tsx` | Success metrics and numbers |
| **NavigatedSlide** | `ebr-qbr/NavigatedSlide.tsx` | Challenges and resolutions |
| **RoadmapSlide** | `ebr-qbr/RoadmapSlide.tsx` | Strategic roadmap |
| **NextStepsSlide** | `ebr-qbr/NextStepsSlide.tsx` | Action items |

---

## SWOT Report (2 components)

Slide-deck-style SWOT analysis. Scroll-snap sections.

| Component | File | Job |
|-----------|------|-----|
| **SwotCover** | `swot/SwotCover.tsx` | Cover slide with account name, summary |
| **QuadrantSlide** | `swot/QuadrantSlide.tsx` | Shared quadrant slide (strengths/weaknesses/opportunities/threats) |

---

## Weekly Impact Report (5 components)

Slide-deck-style weekly reflection. Scroll-snap sections.

| Component | File | Job |
|-----------|------|-----|
| **CoverSlide** | `weekly-impact/CoverSlide.tsx` | Cover with week label, stats, headline |
| **PrioritiesMovedSlide** | `weekly-impact/PrioritiesMovedSlide.tsx` | What moved forward |
| **TheWorkSlide** | `weekly-impact/TheWorkSlide.tsx` | Wins and accomplishments |
| **WatchSlide** | `weekly-impact/WatchSlide.tsx` | Items needing attention |
| **IntoNextWeekSlide** | `weekly-impact/IntoNextWeekSlide.tsx` | Carry-forward items |

---

## Book of Business (5 components)

Slide-deck-style portfolio review for leadership. Scroll-snap sections.

| Component | File | Job |
|-----------|------|-----|
| **CoverSlide** | `book-of-business/CoverSlide.tsx` | Cover with vitals strip, executive summary |
| **AttentionSlide** | `book-of-business/AttentionSlide.tsx` | Risks and opportunities |
| **SpotlightSlide** | `book-of-business/SpotlightSlide.tsx` | Per-account deep dive (one per slide) |
| **ValueThemesSlide** | `book-of-business/ValueThemesSlide.tsx` | Value delivered + cross-portfolio themes |
| **AskSlide** | `book-of-business/AskSlide.tsx` | Leadership asks (conditional) |

---

## Monthly Wrapped (12 components)

Spotify Wrapped-style monthly celebration. Decomposed from monolith into per-slide components.

| Component | File | Job |
|-----------|------|-----|
| **AnimatedNumber** | `monthly-wrapped/AnimatedNumber.tsx` | Animated count-up number display |
| **SplashSlide** | Pages: `monthly-wrapped/slides/SplashSlide.tsx` | Cover slide with month label |
| **VolumeSlide** | Pages: `monthly-wrapped/slides/VolumeSlide.tsx` | Stats/numbers with animated count-up |
| **TopAccountsSlide** | Pages: `monthly-wrapped/slides/TopAccountsSlide.tsx` | Most-touched accounts |
| **MeetingRhythmSlide** | Pages: `monthly-wrapped/slides/MeetingRhythmSlide.tsx` | Heat calendar, personality type |
| **MomentsSlide** | Pages: `monthly-wrapped/slides/MomentsSlide.tsx` | Key moments grid |
| **BiggestWinSlide** | Pages: `monthly-wrapped/slides/BiggestWinSlide.tsx` | Celebration slide |
| **ChallengesSlide** | Pages: `monthly-wrapped/slides/ChallengesSlide.tsx` | What tested you |
| **ActionsImpactSlide** | Pages: `monthly-wrapped/slides/ActionsImpactSlide.tsx` | Completed count, carry-forward |
| **LookingAheadSlide** | Pages: `monthly-wrapped/slides/LookingAheadSlide.tsx` | Next month focus |
| **CloseSlide** | Pages: `monthly-wrapped/slides/CloseSlide.tsx` | Sign-off message |

---

## Generic Report Components (5 components)

Shared report rendering infrastructure.

| Component | File | Job |
|-----------|------|-----|
| **ReportShell** | `reports/ReportShell.tsx` | Shared report wrapper with generate/regenerate actions |
| **ReportSection** | `reports/ReportSection.tsx` | Section layout within reports |
| **AccountHealthReport** | `reports/AccountHealthReport.tsx` | Read-only account health renderer (for ReportPage) |
| **EbrQbrReport** | `reports/EbrQbrReport.tsx` | Read-only EBR/QBR renderer (for ReportPage) |
| **SwotReport** | `reports/SwotReport.tsx` | Read-only SWOT renderer (for ReportPage) |

---

## Inbox Components (1 component)

| Component | File | Job |
|-----------|------|-----|
| **GoogleDriveImportModal** | `inbox/GoogleDriveImportModal.tsx` | Modal for importing files from Google Drive |

---

## Settings Components (13 components)

Redesigned as connections hub per I349. Includes per-connector configuration.

| Component | File | Job |
|-----------|------|-----|
| **YouCard** | `settings/YouCard.tsx` | User profile |
| **ConnectorsGrid** | `settings/ConnectorsGrid.tsx` | Integration cards grid |
| **ConnectorDetail** | `settings/ConnectorDetail.tsx` | Individual connection config |
| **DiagnosticsSection** | `settings/DiagnosticsSection.tsx` | System health |
| **SystemStatus** | `settings/SystemStatus.tsx` | Backend status |
| **DataPrivacySection** | `settings/DataPrivacySection.tsx` | Data purge, source management |
| **ActivityLogSection** | `settings/ActivityLogSection.tsx` | Audit log viewer |
| **DatabaseRecoveryCard** | `settings/DatabaseRecoveryCard.tsx` | DB backup and recovery |
| **ContextSourceSection** | `settings/ContextSourceSection.tsx` | Context source configuration |

### Connector Plugins (8 components)

| Component | File | Job |
|-----------|------|-----|
| **GoogleConnector** | `settings/connectors/GoogleConnector.tsx` | Google Calendar + Gmail OAuth |
| **GoogleDriveConnector** | `settings/connectors/GoogleDriveConnector.tsx` | Google Drive document import |
| **ClaudeDesktopConnector** | `settings/connectors/ClaudeDesktopConnector.tsx` | Claude Code integration |
| **ClayConnector** | `settings/connectors/ClayConnector.tsx` | Clay contact enrichment via Smithery |
| **LinearConnector** | `settings/connectors/LinearConnector.tsx` | Linear issue tracker |
| **GravatarConnector** | `settings/connectors/GravatarConnector.tsx` | Gravatar profile images |
| **QuillConnector** | `settings/connectors/QuillConnector.tsx` | Quill note-taking integration |
| **GranolaConnector** | `settings/connectors/GranolaConnector.tsx` | Granola meeting notes |

---

## Notifications (2 components)

| Component | File | Job |
|-----------|------|-----|
| **UpdateBanner** | `notifications/UpdateBanner.tsx` | App update available banner |
| **WhatsNewModal** | `notifications/WhatsNewModal.tsx` | What's New modal (release notes, shown after update) |

---

## Tour (1 component)

| Component | File | Job |
|-----------|------|-----|
| **TourTips** | `tour/TourTips.tsx` | Contextual tour tooltips for first-run guidance |

---

## Infrastructure Components (5 components)

Cross-cutting components that handle security, recovery, and system states.

| Component | File | Job |
|-----------|------|-----|
| **EncryptionRecovery** | `EncryptionRecovery.tsx` | Encryption key recovery flow |
| **LockOverlay** | `LockOverlay.tsx` | Screen lock overlay (security) |
| **PostMeetingPrompt** | `PostMeetingPrompt.tsx` | Post-meeting outcome capture prompt |
| **ICloudWarningModal** | `ICloudWarningModal.tsx` | Warning when workspace is on iCloud Drive |
| **DatabaseRecovery** | `DatabaseRecovery.tsx` | Database corruption recovery flow |

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
