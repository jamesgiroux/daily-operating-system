# OnboardingFlow

**Tier:** surface
**Status:** shipped source reconciled
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `OnboardingFlow`
**`data-ds-spec`:** `surfaces/OnboardingFlow.md`
**Canonical name:** `OnboardingFlow`
**Source files:**
- `src/components/onboarding/OnboardingFlow.tsx`
- `src/components/onboarding/onboarding.module.css`
- `src/components/onboarding/chapters/*`

## Job

Guide a new user through connecting core data sources, defining themselves, creating the first account context, choosing entity mode, and priming the first briefing.

## Shipped Sequence

`Welcome -> GoogleConnect -> ClaudeCode -> GleanConnect -> YouCardStep -> FirstAccountStep -> EntityMode -> PrimeBriefing`

## Patterns Consumed

- `AtmosphereLayer`
- `FolioBar`
- `FloatingNavIsland`
- `OnboardingFlow`
- onboarding local chapter components

## Not Shipped In Current Flow

The following source-present chapters are not imported/rendered by `OnboardingFlow.tsx` as of 2026-05-05: `AboutYou`, `Workspace`, `InternalTeamSetup`, `InboxTraining`, `MeetingDeepDive`, `PopulateWorkspace`, `DashboardTour`, and `Ready`.

