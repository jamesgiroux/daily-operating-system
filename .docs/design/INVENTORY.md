# DailyOS surface inventory

**Last updated:** 2026-05-05 · reconciled against `src/router.tsx`, reference HTML, and `surface-manifest.json`.

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
| `covered-by` | No standalone reference/spec, but an equivalent canonical route or reference is explicitly named |
| `legacy` | Deprecated or non-routed implementation retained for migration context |
| `documented` | Prose mention in `NAVIGATION-ARCHITECTURE.md` / `INTERACTION-PATTERNS.md` only — softer than `referenced` |
| `gap` | Missing expected standalone reference and/or spec coverage; combine with `referenced` when a reference exists but the canonical spec is still missing |

A surface can hold multiple states (e.g. `spec+referenced`).

## Summary

- **26 router routes** in `src/router.tsx`
- **1 legacy non-route page implementation** retained for migration context (`AccountDetailEditorial`)
- **5 full-screen non-route surfaces** in `src/components/`
- **19 significant dialogs / sheets / banners** across `src/`

| Bucket | Total | `spec` | `referenced` | `covered-by` | `legacy` | `gap` |
|---|---:|---:|---:|---:|---:|---:|
| Router routes | 26 | 19 | 26 | 1 | 0 | 0 |
| Legacy page implementations | 1 | 0 | 0 | 1 | 1 | 0 |
| Non-route full-screen | 5 | 2 | 10 chapter/state refs | 0 | 0 | 0 (3 `documented`-only) |
| Dialogs / sheets | 19 | 2 | 2 | 0 | 0 | 15 (10 `documented`-only) |

## Routed pages — `src/pages/*.tsx`

| Surface | File | Route | JTBD | State | Reference | Spec | Linear gap | Notes |
|---|---|---|---|---|---|---|---|---|
| AccountDetailEditorial | `src/pages/AccountDetailEditorial.tsx` | — | Review a legacy flat account dossier with outlook, products, state of play, stakeholders, timeline, and work. | `legacy+covered-by` | `account.html` via AccountDetailPage | — | — | **Consolidation candidate.** Deprecated, no longer a route target; `account.html` now points canonical coverage at AccountDetailPage. CSS module still imported by child components. |
| AccountDetailPage | `src/pages/AccountDetailPage.tsx` | `/accounts/$accountId` | Review an account's health, context, and work in a three-view editorial dossier. | `referenced+spec` | `account.html` | `surfaces/AccountDetailPage.md` | — | Current route target. Consolidation candidate with deprecated AccountDetailEditorial. |
| AccountHealthPage | `src/pages/AccountHealthPage.tsx` | `/accounts/$accountId/reports/account_health` | Generate and edit a slide-based account health review. | `referenced+spec` | `reports/account-health.html` | `surfaces/AccountHealthPage.md` | — | Dedicated report spec added during the parity pass. |
| AccountsPage | `src/pages/AccountsPage.tsx` | `/accounts` | Browse, search, filter, create, and discover accounts. | `referenced` | `accounts.html` | — | — | Add-new flow tracked separately by DOS-369. |
| ActionDetailPage | `src/pages/ActionDetailPage.tsx` | `/actions/$actionId` | Inspect and update a single action — priority, due date, links, status, Linear push. | `referenced+spec` | `action-detail.html` | `surfaces/ActionDetailPage.md` | — | Added as standalone routed reference. |
| ActionsPage | `src/pages/ActionsPage.tsx` | `/actions` | Review suggested, active, and completed actions by priority and meeting context. | `referenced+spec` | `actions.html` | `surfaces/ActionsPage.md` | — | — |
| BookOfBusinessPage | `src/pages/BookOfBusinessPage.tsx` | `/me/reports/book_of_business` | Generate a leadership-ready portfolio review across the user's customer book. | `referenced+spec` | `reports/book-of-business.html` | `surfaces/BookOfBusinessPage.md` | — | Dedicated report spec added during the parity pass. |
| DailyBriefing (DashboardPage) | inline in `src/router.tsx` | `/` | View today's briefing, meeting prep, and what changed since the last scan. | `referenced+spec` | `briefing.html` | `surfaces/DailyBriefing.md` | — | Inline route component. Renamed from Dashboard per [DOS-360](https://linear.app/a8c/issue/DOS-360). |
| EbrQbrPage | `src/pages/EbrQbrPage.tsx` | `/accounts/$accountId/reports/ebr_qbr` | Generate and edit a customer-facing executive/quarterly business review deck. | `referenced+spec` | `reports/ebr-qbr.html` | `surfaces/EbrQbrPage.md` | — | Dedicated report spec added during the parity pass. |
| EmailsPage | `src/pages/EmailsPage.tsx` | `/emails` | Triage email intelligence, extract commitments and signals, act on reply debt. | `referenced+spec` | `emails.html` | `surfaces/EmailsPage.md` | — | The Correspondent now has a standalone routed reference. |
| HistoryPage | `src/pages/HistoryPage.tsx` | `/history` | Review past inbox processing activity and where files were classified or routed. | `referenced+spec` | `history.html` | `surfaces/HistoryPage.md` | — | Processing-history companion to InboxPage and EmailsPage. |
| InboxPage | `src/pages/InboxPage.tsx` | `/inbox` | Drop or import files, classify them, and route them to the right account/project. | `referenced` | `inbox.html` | — | — | Processing history is HistoryPage. |
| MePage | `src/pages/MePage.tsx` | `/me` | Maintain the user's profile, priorities, context, attachments, personal report entry points. | `referenced+spec` | `me.html` | `surfaces/MePage.md` | — | Personal reports route to WeeklyImpact, MonthlyWrapped, BookOfBusiness. |
| MeetingDetailPage | `src/pages/MeetingDetailPage.tsx` | `/meeting/$meetingId` | Prepare for or review a meeting — context, risks, room, plan, outcomes, transcript actions. | `referenced+spec` | `meeting.html` | `surfaces/MeetingDetail.md` | — | Spec reconciled to shipped source plus explicit extraction targets. |
| MeetingHistoryDetailPage | `src/pages/MeetingHistoryDetailPage.tsx` | `/meeting/history/$meetingId` | Preserve legacy meeting-history links by redirecting to the canonical route. | `covered-by` | `meeting.html` via MeetingDetailPage | `surfaces/MeetingDetail.md` | — | **Consolidation candidate.** Wrapper/redirect only. |
| MonthlyWrappedPage | `src/pages/monthly-wrapped/MonthlyWrappedPage.tsx` | `/me/reports/monthly_wrapped` | Generate a monthly retrospective of work, wins, watch items. | `referenced+spec` | `reports/monthly-wrapped.html` | `surfaces/MonthlyWrappedPage.md` | — | Dedicated report spec added during the parity pass. Lives in subdirectory. |
| PeoplePage | `src/pages/PeoplePage.tsx` | `/people` | Browse, search, create, archive, and clean up people across internal/external/unknown. | `referenced` | `people.html` | — | — | Add-new flow tracked by DOS-369. |
| PersonDetailEditorial | `src/pages/PersonDetailEditorial.tsx` | `/people/$personId` | Understand and maintain a person's relationship context, network, record, and work. | `referenced` | `person.html` | — | — | — |
| ProjectDetailEditorial | `src/pages/ProjectDetailEditorial.tsx` | `/projects/$projectId` | Understand and maintain a project's mission, trajectory, team, timeline, and work. | `referenced` | `project.html` | — | — | — |
| ProjectsPage | `src/pages/ProjectsPage.tsx` | `/projects` | Browse, search, create, and archive the project hierarchy. | `referenced+spec` | `projects.html` | `surfaces/ProjectsPage.md` | — | Add-new flow tracked by DOS-369. |
| ReportPage | `src/pages/ReportPage.tsx` | `/accounts/.../reports/$reportType`, `/me/reports/$reportType` | Render a generic saved report for account- or user-scoped report routes. | `referenced+spec` | `reports/generic-report.html` | `surfaces/ReportPage.md` | — | **Consolidation candidate.** Generic shell now has standalone coverage; dedicated report references cover named report examples. |
| RiskBriefingPage | `src/pages/RiskBriefingPage.tsx` | `/accounts/$accountId/reports/risk_briefing` | Generate and edit an executive risk briefing and recovery plan. | `referenced+spec` | `reports/risk-briefing.html` | `surfaces/RiskBriefingPage.md` | — | Dedicated report spec added during the parity pass. |
| SettingsPage | `src/pages/SettingsPage.tsx` | `/settings` | Configure identity, connectors, data, system, notifications, diagnostics. | `referenced+spec` | `settings.html` | `surfaces/Settings.md` | — | Spec source files reconciled to `src/pages/SettingsPage.tsx`, `SettingsPage.module.css`, and `src/features/settings-ui/*`. |
| SwotPage | `src/pages/SwotPage.tsx` | `/accounts/$accountId/reports/swot` | Generate and edit a SWOT analysis slide deck. | `referenced+spec` | `reports/swot.html` | `surfaces/SwotPage.md` | — | Dedicated report spec added during the parity pass. |
| WeekPage | `src/pages/WeekPage.tsx` | `/week` | Understand the upcoming week's meeting load and briefing readiness. | `referenced+spec` | `week.html` | `surfaces/WeekPage.md` | — | — |
| WeeklyImpactPage | `src/pages/WeeklyImpactPage.tsx` | `/me/reports/weekly_impact` | Generate and edit a weekly impact report — work moved, wins, watch items, next week. | `referenced+spec` | `reports/weekly-impact.html` | `surfaces/WeeklyImpactPage.md` | — | Dedicated report spec added during the parity pass. |

## Proposed release-candidate references

These are iteration references, not routed parity entries. Do not add them to
the strict surface manifest until source routing exists.

| Surface | File | Target route | JTBD | State | Reference | Spec | Notes |
|---|---|---|---|---|---|---|---|
| DailyBriefingDSpine | Proposed DailyBriefing redesign | `/` candidate | Explore the D-spine schedule-as-spine redesign using the current DailyBriefing reference foundation. | `proposed reference` | `briefing-d-spine.html` | `surfaces/DailyBriefingDSpine.md` | Built from `DayChart`, `MeetingSpineItem`, `EntityPortraitCard`, `ThreadMark`, `AskAnythingDock`, and current DailyBriefing chrome. |

## Full-screen non-route surfaces

These render at app-shell scale, not inside a page container.

| Surface | File | JTBD | State | Linear gap | Notes |
|---|---|---|---|---|---|
| DatabaseRecovery | `src/components/DatabaseRecovery.tsx` | Recover from unsafe database startup — restore backup, export copy, start fresh, or update app. | `documented` | — | Full-window startup gate; documented in `NAVIGATION-ARCHITECTURE.md` but no reference render. |
| EncryptionRecovery | `src/components/EncryptionRecovery.tsx` | Explain that encrypted DB can't be opened without macOS Keychain key; offer recovery or fresh-start. | `documented` | — | Full-window startup gate; same status as DatabaseRecovery. |
| LockOverlay | `src/components/LockOverlay.tsx` | Block app access while locked; unlock with Touch ID. | `documented` | — | Full-window startup gate. |
| OnboardingFlow | `src/components/onboarding/OnboardingFlow.tsx` | Guide first-run setup — Google, Claude Code, Glean, user context, first account, role, initial briefing. | `referenced+spec` | — | Chapter references exist under `reference/surfaces/onboarding/`; canonical shipped sequence is `surfaces/OnboardingFlow.md`. |
| StartupBriefingScreen | `src/components/startup/StartupBriefingScreen.tsx` | Hold the cold-start moment with branded splash/progress while DailyOS prepares context. | `referenced+spec` | — | Splash/progress references exist under `reference/surfaces/splash/` and are covered by the manifest fidelity audit; canonical spec is `surfaces/StartupBriefingScreen.md`. |

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
| ReportPage vs dedicated report pages (RiskBriefing, Swot, AccountHealth, EbrQbr, WeeklyImpact, MonthlyWrapped, BookOfBusiness) | ReportPage is a generic shell; the dedicated pages predate it. | Lookalike | Generic shell is now referenced; still investigate whether dedicated reports should migrate to one configured renderer. |
| Account/Project/Person CreateInlineFlow components | All three use `inline-create-form.tsx` + `bulk-create-form.tsx` but each page wires it slightly differently. | Config-only difference | Could collapse the wiring into a shared hook. Track in DOS-369. |
| AccountDialogs (ChildCreate / Archive) vs ProjectDetailEditorial sub-project dialog vs PersonMergePickerDialog | Each entity has a slightly different "create-child" pattern. | Lookalike | Audit during DOS-369 / DOS-373 (hero patterns are nearby). |
| StartupBriefingScreen used in 4 contexts | Cold-start, shell-only startup, welcome fade, dashboard progress state. Same component, four states. | Config-only difference | Canonical multi-state behavior now lives in `surfaces/StartupBriefingScreen.md`. |

## Outliers / edge cases

- **PersonMergePickerDialog** has no DS-SURF Linear issue. Could fold into DOS-369 (entity creation flows) since merge is in the same family of entity-management modals.
- **Generic ReportPage shell** — eight report references now cover named reports plus the wildcard shell. Remaining work is the product/implementation consolidation decision between generic and dedicated report rendering.

## Linear coverage map

Every gap above maps to a filed Linear issue (or is flagged as needing one):

| Linear issue | Covers |
|---|---|
| [DOS-369](https://linear.app/a8c/issue/DOS-369) — DS-SURF-03 | AccountCreateInlineFlow, AccountCreateChildDialog, ProjectCreateInlineFlow, ProjectCreateSubProjectDialog, PersonCreateInlineForm |
| [DOS-371](https://linear.app/a8c/issue/DOS-371) — DS-SURF-05 | ProfileSelector and any future onboarding chapter variants not in the shipped flow |
| **Unfiled** | PersonMergePickerDialog |

When DS-NAV-01 ([DOS-364](https://linear.app/a8c/issue/DOS-364)) builds the reference index, it should consume this file as the source of truth for surface enumeration, JTBD copy, and gap status.
