# Shipped Component Inventory

**Date:** 2026-05-05
**Scope:** DailyBriefing, MeetingDetail, Accounts/People/Projects lists, AccountDetail, Settings, and Onboarding.

## Classification Rule

The shipped app is the source of truth.

- **canonical/shipped** means a shared source component is consumed by shipped routed UI.
- **shipped-local/extraction-needed** means the UI is real and shipped, but still lives inline, in a surface-local component, or under a local class family.
- **implemented/unintegrated** means source exists, but no routed surface consumes it.
- **roadmap/planned** means the spec is a target and no shipped source exists under that name.

This audit corrects the previous failure mode: promoting Spine-D/prototype components into canonical status before the shipped app had adopted them.

## DailyBriefing

Source of truth: `src/components/dashboard/DailyBriefing.tsx`, `src/components/dashboard/BriefingMeetingCard.tsx`, and `src/styles/editorial-briefing.module.css`.

Shipped patterns now represented:

- `MeetingCard`
- `BriefingMeetingCard`
- `DailyBriefingAttentionSection`
- `HealthBadge`
- `FolioRefreshButton`
- `FinisMarker`

Demoted from canonical:

- `DayChart`, `EntityPortraitCard`, `ThreadMark`, and `AskAnythingDock` are implemented/unintegrated.
- `MeetingSpineItem` and `Lead` remain roadmap/planned.

## MeetingDetail

Source of truth: `src/pages/MeetingDetailPage.tsx`, `src/components/meeting/PostMeetingIntelligence.tsx`, and `src/components/shared/ActionRow.tsx`.

Shipped patterns now represented:

- `PostMeetingIntelligence`
- `TalkBalanceBar`
- `ActionRow`
- `IntelligenceFeedback`
- `FinisMarker`
- `EditableText`

The previous Wave 4 entries (`AgendaThreadList`, `PredictionsVsRealityGrid`, `SignalGrid`, `EscalationQuote`, `FindingsTriad`, `ChampionHealthBlock`, `CommitmentRow`, `RoleTransitionRow`) are real UI in the shipped app, but they ship as local `PostMeetingIntelligence` class families rather than exported components. They are now `shipped-local/extraction-needed`, not roadmap.

## Entity Lists

Source of truth: `src/pages/AccountsPage.tsx`, `src/pages/PeoplePage.tsx`, `src/pages/ProjectsPage.tsx`, `src/components/entity/EntityListShell.tsx`, and `src/components/entity/EntityRow.tsx`.

Shipped patterns now represented:

- `EntityListShell`
- `EntityRow`
- `Avatar`
- `HealthBadge`

## AccountDetail

Source of truth: `src/pages/AccountDetailPage.tsx`, `src/components/account/AccountViewSwitcher.tsx`, `src/components/entity/VitalsStrip.tsx`, and `src/components/work/WorkSurface.tsx`.

Shipped patterns now represented:

- `VitalsStrip`
- `AccountViewSwitcher`
- `WorkSurface`
- `HealthBadge`
- `EditableText`

`TrustBand`, `ClaimRow`, and `ReceiptCallout` remain implemented but unintegrated; AccountDetail does not currently import or render those names.

## Settings

Source of truth: `src/pages/SettingsPage.tsx` and `src/features/settings-ui/*`.

Shipped patterns now represented:

- `SurfaceMasthead`
- `FormRow`
- `YouCard`
- `SettingsSections`
- `ActivityLogSection`
- `DiagnosticsSection`
- `StatusDot`
- `Switch`
- `Segmented`

The Settings spec no longer claims the older 7-chapter redesign as shipped. Production chapters are `settings-you`, `settings-connectors`, `settings-data`, and `settings-system`; diagnostics is development-only.

## Onboarding

Source of truth: `src/components/onboarding/OnboardingFlow.tsx` and `src/components/onboarding/onboarding.module.css`.

Shipped sequence now represented:

`Welcome -> GoogleConnect -> ClaudeCode -> GleanConnect -> YouCardStep -> FirstAccountStep -> EntityMode -> PrimeBriefing`

`AboutYou`, `Workspace`, `InternalTeamSetup`, `InboxTraining`, `MeetingDeepDive`, `PopulateWorkspace`, `DashboardTour`, and `Ready` remain source-present or older chapter variants, but they are not the shipped `OnboardingFlow` sequence.

