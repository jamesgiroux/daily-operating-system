# DailyBriefingDSpine

**Tier:** surface
**Status:** proposed reference candidate for v1.4.0
**Owner:** James
**Last updated:** 2026-05-06
**`data-ds-name`:** `DailyBriefingDSpine`
**`data-ds-spec`:** `surfaces/DailyBriefingDSpine.md`
**Reference:** `.docs/design/reference/surfaces/briefing-d-spine.html`
**Mockup source:** `/Users/jamesgiroux/Downloads/dailyos-design-system 2/project/mockups/briefing/variations/D-spine.html`

## Job

Explore the D-spine direction as an update to DailyBriefing using the existing
reference Daily Briefing as the foundation. The page keeps DailyOS chrome,
tokens, typography, margin grid, and inspector contracts while replacing the
current meeting list emphasis with a schedule-as-spine reading flow.

This is not a routed parity surface yet. It is an iteration surface for deciding
whether and how the D-spine redesign rolls into v1.4.0.

## Layout regions

1. **FolioBar / FloatingNavIsland / AtmosphereLayer** - unchanged app chrome.
2. **Lead** - DailyBriefing editorial hero treatment, not the standalone raw
   D-spine day strip.
3. **Today** - `DayChart` plus the new `MeetingSpineItem` stack.
4. **Moving** - `EntityPortraitCard` stack for account/person movement.
5. **Watch** - built from current briefing priority-row vocabulary and `Pill`
   actions until a dedicated watch pattern is approved.
6. **Ask** - `AskAnythingDock` with `ThreadMark` context affordances.
7. **FinisMarker** - editorial close.

## Local nav approach

The surface provides chapters to `FloatingNavIsland`:

- `lead`
- `schedule`
- `moving`
- `watch`
- `ask`

**No DayStrip.** The D-spine mockup has a day strip that replaces the app nav.
DailyBriefing has already rejected that direction in `DailyBriefing.md`; this
reference keeps FolioBar and FloatingNavIsland as the canonical chrome.

## Patterns consumed

- `FolioBar`
- `FloatingNavIsland`
- `AtmosphereLayer`
- `MarginGrid`
- `Lead` (surface-local current DailyBriefing hero treatment)
- `DayChart`
- `MeetingSpineItem`
- `EntityPortraitCard`
- `DailyBriefingAttentionSection` (watch section substrate)
- `ThreadMark`
- `AskAnythingDock`
- `FinisMarker`

## Primitives consumed

- `Pill`
- `MeetingStatusPill`
- `EntityChip` (available for entity references in the next iteration)
- `IntelligenceQualityBadge` (available if briefing quality labels are restored)
- `HealthBadge` (available if the schedule keeps compact health scores)

## Source alignment

Existing source-backed patterns reused:

- `src/components/dashboard/DayChart.tsx`
- `src/components/dashboard/EntityPortraitCard.tsx`
- `src/components/dashboard/AskAnythingDock.tsx`
- `src/components/ui/ThreadMark.tsx`
- `src/components/ui/Pill.tsx`
- `src/components/meeting/MeetingStatusPill.tsx`

New source-backed pattern added for this reference:

- `src/components/dashboard/MeetingSpineItem.tsx`
- `src/components/dashboard/MeetingSpineItem.module.css`

Current shipped DailyBriefing remains:

- `src/components/dashboard/DailyBriefing.tsx`
- `src/components/dashboard/BriefingMeetingCard.tsx`
- `src/components/shared/MeetingCard.tsx`
- `src/styles/editorial-briefing.module.css`

## Release gates

- Reference QA at desktop and mobile widths.
- Inspector overlay shows all major primitives/patterns and the proposed
  `DailyBriefingDSpine` surface tag.
- No new local nav pattern; FloatingNavIsland remains the chrome.
- No production route switch until a v1.4.0 implementation plan clears the
  user-facing surface review gate.
- If the redesign ships, routed DailyBriefing must either consume the extracted
  `MeetingSpineItem`/`DayChart`/`EntityPortraitCard` components or deliberately
  document why a local implementation is required.

## History

- 2026-05-06 - Reference candidate added from the D-spine mockup using current
  DailyBriefing reference structure and existing design-system components.
