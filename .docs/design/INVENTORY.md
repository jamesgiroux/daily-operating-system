# DailyOS surface inventory

**Last updated:** 2026-05-03 · synthesized from `src/` analysis (DOS-363).

A surface-by-surface inventory of every DailyOS UI rendered at app-shell scale: routed pages, full-screen non-route components, and significant dialogs. The companion files for navigating these — index page (DOS-364) and JTBD journey maps (DOS-365) — read from this list.

## How this file is maintained

When new surfaces or significant dialogs are added/changed, the `arch-doc-updater.sh` hook appends to `.docs/design/_pending-inventory-updates.log`. Reconcile that log into this file as part of routine maintenance. The hook fires on:

- `src/pages/*.tsx`
- `src/components/onboarding/*.tsx`
- `src/components/startup/*.tsx`

Significant dialogs aren't auto-tracked (no clean grep) — add them manually when the design weight warrants.

## State key

| State | Meaning |
|---|---|
| `referenced` | Has `.docs/design/reference/surfaces/<name>.html` |
| `spec` | Has `.docs/design/surfaces/<Name>.md` canonical spec |
| `documented` | Prose mention in `NAVIGATION-ARCHITECTURE.md` / `INTERACTION-PATTERNS.md` only — softer than `referenced` |
| `gap` | None of the above |

A surface can hold multiple states (e.g. `spec+referenced`).

## Summary

- **25 routed pages** in `src/pages/*.tsx`
- **5 full-screen non-route surfaces** in `src/components/`
- **19 significant dialogs / sheets / banners** across `src/`

| Bucket | Total | `spec` | `referenced` | `gap` |
|---|---:|---:|---:|---:|
| Routed pages | 25 | 4 | 15 | 10 |
| Non-route full-screen | 5 | 0 | 0 | 5 (3 `documented`-only) |
| Dialogs / sheets | 19 | 2 | 2 | 15 (10 `documented`-only) |

## Routed pages — `src/pages/*.tsx`

| Surface | File | Route | JTBD | State | Reference | Spec | Linear gap | Notes |
|---|---|---|---|---|---|---|---|---|
| AccountDetailEditorial | `src/pages/AccountDetailEditorial.tsx` | — | Review a legacy flat account dossier with outlook, products, state of play, stakeholders, timeline, and work. | `referenced` | `account.html` | — | — | **Consolidation candidate.** Deprecated, no longer a route target; duplicate of AccountDetailPage. CSS module still imported by child components. |
| AccountDetailPage | `src/pages/AccountDetailPage.tsx` | `/accounts/$accountId` | Review an account's health, context, and work in a three-view editorial dossier. | `referenced` | `account.html` | — | — | Current route target. Consolidation candidate with deprecated AccountDetailEditorial. |
| AccountHealthPage | `src/pages/AccountHealthPage.tsx` | `/accounts/$accountId/reports/account_health` | Generate and edit a slide-based account health review. | `gap` | — | — | [DOS-370](https://linear.app/a8c/issue/DOS-370) | Report surface — DOS-370 covers all 7 report types. |
| AccountsPage | `src/pages/AccountsPage.tsx` | `/accounts` | Browse, search, filter, create, and discover accounts. | `referenced` | `accounts.html` | — | — | Add-new flow tracked separately by DOS-369. |
| ActionDetailPage | `src/pages/ActionDetailPage.tsx` | `/actions/$actionId` | Inspect and update a single action — priority, due date, links, status, Linear push. | `gap` | — | — | [DOS-368](https://linear.app/a8c/issue/DOS-368) | `actions.html` covers list view only. |
| ActionsPage | `src/pages/ActionsPage.tsx` | `/actions` | Review suggested, active, and completed actions by priority and meeting context. | `referenced` | `actions.html` | — | — | — |
| BookOfBusinessPage | `src/pages/BookOfBusinessPage.tsx` | `/me/reports/book_of_business` | Generate a leadership-ready portfolio review across the user's customer book. | `gap` | — | — | [DOS-370](https://linear.app/a8c/issue/DOS-370) | Report surface. |
| DailyBriefing (DashboardPage) | inline in `src/router.tsx` | `/` | View today's briefing, meeting prep, and what changed since the last scan. | `referenced+spec` | `briefing.html` | `surfaces/DailyBriefing.md` | — | Inline route component. Renamed from Dashboard per [DOS-360](https://linear.app/a8c/issue/DOS-360). |
| EbrQbrPage | `src/pages/EbrQbrPage.tsx` | `/accounts/$accountId/reports/ebr_qbr` | Generate and edit a customer-facing executive/quarterly business review deck. | `gap` | — | — | [DOS-370](https://linear.app/a8c/issue/DOS-370) | Report surface. |
| EmailsPage | `src/pages/EmailsPage.tsx` | `/emails` | Triage email intelligence, extract commitments and signals, act on reply debt. | `gap` | — | — | [DOS-367](https://linear.app/a8c/issue/DOS-367) | `inbox.html` covers the inbox; correspondent surface missing. |
| HistoryPage | `src/pages/HistoryPage.tsx` | `/history` | Review past inbox processing activity and where files were classified or routed. | `gap` | — | — | **No DS-SURF issue filed** | CSS exists but no reference render or spec. Worth filing a follow-up. |
| InboxPage | `src/pages/InboxPage.tsx` | `/inbox` | Drop or import files, classify them, and route them to the right account/project. | `referenced` | `inbox.html` | — | — | Processing history is HistoryPage. |
| MePage | `src/pages/MePage.tsx` | `/me` | Maintain the user's profile, priorities, context, attachments, personal report entry points. | `referenced` | `me.html` | — | — | Personal reports route to WeeklyImpact, MonthlyWrapped, BookOfBusiness. |
| MeetingDetailPage | `src/pages/MeetingDetailPage.tsx` | `/meeting/$meetingId` | Prepare for or review a meeting — context, risks, room, plan, outcomes, transcript actions. | `referenced+spec` | `meeting.html` | `surfaces/MeetingDetail.md` | — | Current implementation is pre/post-briefing oriented; spec describes the Wave 4 post-meeting recap target. |
| MeetingHistoryDetailPage | `src/pages/MeetingHistoryDetailPage.tsx` | `/meeting/history/$meetingId` | Preserve legacy meeting-history links by redirecting to the canonical route. | `referenced+spec` | `meeting.html` | `surfaces/MeetingDetail.md` | **Consolidation candidate.** Wrapper/redirect only. |
| MonthlyWrappedPage | `src/pages/monthly-wrapped/MonthlyWrappedPage.tsx` | `/me/reports/monthly_wrapped` | Generate a monthly retrospective of work, wins, watch items. | `gap` | — | — | [DOS-370](https://linear.app/a8c/issue/DOS-370) | Lives in subdirectory — easy to miss in inventories. |
| PeoplePage | `src/pages/PeoplePage.tsx` | `/people` | Browse, search, create, archive, and clean up people across internal/external/unknown. | `referenced` | `people.html` | — | — | Add-new flow tracked by DOS-369. |
| PersonDetailEditorial | `src/pages/PersonDetailEditorial.tsx` | `/people/$personId` | Understand and maintain a person's relationship context, network, record, and work. | `referenced` | `person.html` | — | — | — |
| ProjectDetailEditorial | `src/pages/ProjectDetailEditorial.tsx` | `/projects/$projectId` | Understand and maintain a project's mission, trajectory, team, timeline, and work. | `referenced` | `project.html` | — | — | — |
| ProjectsPage | `src/pages/ProjectsPage.tsx` | `/projects` | Browse, search, create, and archive the project hierarchy. | `referenced` | `projects.html` | — | — | Add-new flow tracked by DOS-369. |
| ReportPage | `src/pages/ReportPage.tsx` | `/accounts/.../reports/$reportType`, `/me/reports/$reportType` | Render a generic saved report for account- or user-scoped report routes. | `gap` | — | — | [DOS-370](https://linear.app/a8c/issue/DOS-370) | **Consolidation candidate.** Generic report shell vs the dedicated report pages — likely a refactor target. |
| RiskBriefingPage | `src/pages/RiskBriefingPage.tsx` | `/accounts/$accountId/reports/risk_briefing` | Generate and edit an executive risk briefing and recovery plan. | `gap` | — | — | [DOS-370](https://linear.app/a8c/issue/DOS-370) | Report surface. |
| SettingsPage | `src/pages/SettingsPage.tsx` | `/settings` | Configure identity, connectors, data, system, notifications, diagnostics. | `referenced+spec` | `settings.html` | `surfaces/Settings.md` | — | Spec source file note points at `src/features/settings-ui/` while router uses `SettingsPage.tsx` — drift to reconcile. |
| SwotPage | `src/pages/SwotPage.tsx` | `/accounts/$accountId/reports/swot` | Generate and edit a SWOT analysis slide deck. | `gap` | — | — | [DOS-370](https://linear.app/a8c/issue/DOS-370) | Report surface. |
| WeekPage | `src/pages/WeekPage.tsx` | `/week` | Understand the upcoming week's meeting load and briefing readiness. | `referenced` | `week.html` | — | — | — |
| WeeklyImpactPage | `src/pages/WeeklyImpactPage.tsx` | `/me/reports/weekly_impact` | Generate and edit a weekly impact report — work moved, wins, watch items, next week. | `gap` | — | — | [DOS-370](https://linear.app/a8c/issue/DOS-370) | Report surface. |

## Full-screen non-route surfaces

These render at app-shell scale, not inside a page container.

| Surface | File | JTBD | State | Linear gap | Notes |
|---|---|---|---|---|---|
| DatabaseRecovery | `src/components/DatabaseRecovery.tsx` | Recover from unsafe database startup — restore backup, export copy, start fresh, or update app. | `documented` | — | Full-window startup gate; documented in `NAVIGATION-ARCHITECTURE.md` but no reference render. |
| EncryptionRecovery | `src/components/EncryptionRecovery.tsx` | Explain that encrypted DB can't be opened without macOS Keychain key; offer recovery or fresh-start. | `documented` | — | Full-window startup gate; same status as DatabaseRecovery. |
| LockOverlay | `src/components/LockOverlay.tsx` | Block app access while locked; unlock with Touch ID. | `documented` | — | Full-window startup gate. |
| OnboardingFlow | `src/components/onboarding/OnboardingFlow.tsx` | Guide first-run setup — Google, Claude Code, Glean, user context, first account, role, initial briefing. | `gap` | [DOS-371](https://linear.app/a8c/issue/DOS-371) | Chapter-only `FloatingNavIsland` mode; CSS exists in reference assets. |
| StartupBriefingScreen | `src/components/startup/StartupBriefingScreen.tsx` | Hold the cold-start moment with branded splash/progress while DailyOS prepares context. | `gap` | [DOS-372](https://linear.app/a8c/issue/DOS-372) | Used as cold-start overlay, shell-only startup, welcome fade, dashboard progress state. |

## Significant dialogs / sheets / banners

| Surface | File | Triggered from | JTBD | State | Linear gap | Notes |
|---|---|---|---|---|---|---|
| AccountCreateInlineFlow | `src/pages/AccountsPage.tsx` + `inline-create-form.tsx` + `bulk-create-form.tsx` | AccountsPage folio "+ New" / empty-state CTA | Create one or many accounts, optionally with type and parent. | `gap` | [DOS-369](https://linear.app/a8c/issue/DOS-369) | Currently inline rather than modal. |
| AccountCreateChildDialog | `src/components/account/AccountDialogs.tsx` | Account detail FolioToolsDropdown "Create Team" / "Create Business Unit" | Create a child team or business unit under the current account. | `gap` | [DOS-369](https://linear.app/a8c/issue/DOS-369) | Modal entity creation. |
| AccountMergeDialog | `src/components/account/AccountMergeDialog.tsx` | Account detail FolioToolsDropdown "Merge" | Merge the current account into another and show moved-data results. | `documented` | — | Reference CSS exists but no standalone modal reference HTML. |
| AgendaDraftDialog | `src/components/ui/agenda-draft-dialog.tsx` | MeetingDetail Folio "Draft Agenda" | Generate, review, and copy a meeting agenda message — sending stays manual. | `documented` | — | Used exclusively by MeetingDetail. |
| CommandMenu | `src/components/layout/CommandMenu.tsx` | ⌘K, header search, MagazinePageLayout folio search | Search global entities, navigate routes, run quick actions. | `documented` | — | Global command palette; significant sub-composition. |
| DailyBriefingLifecycleCorrectionDialog | `src/components/dashboard/DailyBriefing.tsx` | DailyBriefing lifecycle card "Fix something" | Correct a proposed account lifecycle/renewal-stage change with notes. | `referenced+spec` | — | Dialog styling lives with DailyBriefing; reference covers the affordance. |
| DevToolsPanelSheet | `src/components/devtools/DevToolsPanel.tsx` | MagazinePageLayout dev badge / standalone wrench | Switch dev scenarios, onboarding states, integrations, account states, sandbox/live data. | `documented` | — | Controlled sheet variant from magazine shell. |
| GoogleDriveImportModal | `src/components/inbox/GoogleDriveImportModal.tsx` | InboxPage "Google Drive" import | Pick Drive files/folders, choose import/watch mode, link to entity. | `documented` | — | Custom modal after external picker. |
| ICloudWarningModal | `src/components/ICloudWarningModal.tsx` | RootLayout after `check_icloud_warning` returns a path | Warn that workspace is in iCloud-synced folder; offer dismiss. | `documented` | — | Custom modal, not Radix Dialog. |
| MeetingPasteTranscriptDialog | `src/pages/MeetingDetailPage.tsx` | MeetingDetail "Paste Transcript" | Paste plain-text or markdown transcript and process it through meeting intelligence. | `referenced+spec` | — | Reference covers the CTA; the open dialog isn't separately rendered. |
| PersonCreateInlineForm | `src/pages/PeoplePage.tsx` | PeoplePage folio "+ Add" | Create a person from email and name, then navigate to detail. | `gap` | [DOS-369](https://linear.app/a8c/issue/DOS-369) | Page-local inline form. |
| PersonMergePickerDialog | `src/pages/PersonDetailEditorial.tsx` | Person detail appendix merge action | Search for a target person before confirming merge of meetings, links, actions. | `gap` | **No DS-SURF issue filed** | Significant merge picker. |
| PostMeetingPrompt | `src/components/PostMeetingPrompt.tsx` | RootLayout via `usePostMeetingCapture` after meeting ends | Capture wins, risks, actions, or transcript before context disappears. | `documented` | — | Global floating capture overlay. |
| ProfileSelector | `src/components/ProfileSelector.tsx` | Not currently invoked | Choose a DailyOS profile that customizes setup. | `gap` | [DOS-371](https://linear.app/a8c/issue/DOS-371) | Ghost dialog per `FRONTEND-COMPONENTS.md`; likely superseded by OnboardingFlow. |
| ProjectCreateInlineFlow | `src/pages/ProjectsPage.tsx` + `inline-create-form.tsx` + `bulk-create-form.tsx` | ProjectsPage folio "+ New" / empty-state CTA | Create one or many projects from the project list. | `gap` | [DOS-369](https://linear.app/a8c/issue/DOS-369) | Currently inline. |
| ProjectCreateSubProjectDialog | `src/pages/ProjectDetailEditorial.tsx` | Project detail folio "+ Sub-Project" | Create a named sub-project under the current project. | `gap` | [DOS-369](https://linear.app/a8c/issue/DOS-369) | Modal entity creation. |
| TourTips | `src/components/tour/TourTips.tsx` | RootLayout after wizard completion | Four-step orientation card after onboarding. | `documented` | — | Floating corner card. |
| UpdateBanner | `src/components/notifications/UpdateBanner.tsx` | RootLayout / MagazinePageLayout when update context reports a version | Announce available app update; offer release notes or install/restart. | `documented` | — | Full-width app-shell banner; significant non-route surface. |
| WhatsNewModal | `src/components/notifications/WhatsNewModal.tsx` | UpdateBanner "What's New" / auto-show after update | Show release notes for the current app version in a focused modal. | `documented` | — | Custom modal; release-note markdown is sanitized before render. |

## Consolidation candidates (flagged during inventory)

These are duplicates or near-duplicates noticed during the inventory pass. Listed here for the running consolidation log ([DOS-374](https://linear.app/a8c/issue/DOS-374)).

| Candidate | What duplicates | Severity | Recommendation |
|---|---|---|---|
| AccountDetailEditorial vs AccountDetailPage | Two implementations of the account dossier; Editorial is deprecated but its CSS module is still imported by child components. | True duplicate | Delete Editorial after weaning child components off its module. |
| MeetingDetailPage vs MeetingHistoryDetailPage | History page is a redirect/wrapper to the canonical route. | True duplicate | Could collapse if history-link preservation can move to a router-level redirect. |
| ReportPage vs dedicated report pages (RiskBriefing, Swot, AccountHealth, EbrQbr, WeeklyImpact, MonthlyWrapped, BookOfBusiness) | ReportPage is a generic shell; the dedicated pages predate it. | Lookalike | Investigate during DOS-370 — possibly migrate dedicated reports to a single ReportPage with config. |
| Account/Project/Person CreateInlineFlow components | All three use `inline-create-form.tsx` + `bulk-create-form.tsx` but each page wires it slightly differently. | Config-only difference | Could collapse the wiring into a shared hook. Track in DOS-369. |
| AccountDialogs (ChildCreate / Archive) vs ProjectDetailEditorial sub-project dialog vs PersonMergePickerDialog | Each entity has a slightly different "create-child" pattern. | Lookalike | Audit during DOS-369 / DOS-373 (hero patterns are nearby). |
| StartupBriefingScreen used in 4 contexts | Cold-start, shell-only startup, welcome fade, dashboard progress state. Same component, four states. | Config-only difference | Document as canonical multi-state pattern in DOS-372. |

## Outliers / edge cases

- **HistoryPage** has no DS-SURF Linear issue. It's a `gap` for both reference and spec. Worth filing a follow-up issue or folding into DOS-367 (inbox/email surfaces) since it's tightly related to InboxPage processing. Recommendation: file `DS-SURF-07 — History surface reference` if treated separately, or add to DOS-367's scope.
- **PersonMergePickerDialog** has no DS-SURF Linear issue. Could fold into DOS-369 (entity creation flows) since merge is in the same family of entity-management modals.
- **SettingsPage spec drift** — `surfaces/Settings.md` references `src/features/settings-ui/` but the router uses `src/pages/SettingsPage.tsx`. Reconcile when the spec is next touched.

## Linear coverage map

Every gap above maps to a filed Linear issue (or is flagged as needing one):

| Linear issue | Covers |
|---|---|
| [DOS-367](https://linear.app/a8c/issue/DOS-367) — DS-SURF-01 | EmailsPage |
| [DOS-368](https://linear.app/a8c/issue/DOS-368) — DS-SURF-02 | ActionDetailPage |
| [DOS-369](https://linear.app/a8c/issue/DOS-369) — DS-SURF-03 | AccountCreateInlineFlow, AccountCreateChildDialog, ProjectCreateInlineFlow, ProjectCreateSubProjectDialog, PersonCreateInlineForm |
| [DOS-370](https://linear.app/a8c/issue/DOS-370) — DS-SURF-04 | AccountHealthPage, BookOfBusinessPage, EbrQbrPage, MonthlyWrappedPage, ReportPage, RiskBriefingPage, SwotPage, WeeklyImpactPage |
| [DOS-371](https://linear.app/a8c/issue/DOS-371) — DS-SURF-05 | OnboardingFlow, ProfileSelector |
| [DOS-372](https://linear.app/a8c/issue/DOS-372) — DS-SURF-06 | StartupBriefingScreen |
| **Unfiled** | HistoryPage, PersonMergePickerDialog |

When DS-NAV-01 ([DOS-364](https://linear.app/a8c/issue/DOS-364)) builds the reference index, it should consume this file as the source of truth for surface enumeration, JTBD copy, and gap status.
