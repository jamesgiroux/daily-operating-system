# DailyBriefingRedesign

**Tier:** surface
**Status:** canonical routed surface
**Owner:** James
**Last updated:** 2026-05-06
**`data-ds-name`:** `DailyBriefingRedesign`
**`data-ds-spec`:** `surfaces/DailyBriefingRedesign.md`
**Reference:** `.docs/design/reference/surfaces/briefing-redesign.html`
**Mockup source:** `/Users/jamesgiroux/Downloads/dailyos-design-system 2/project/mockups/briefing/variations/Daily Briefing redesign.html`

## Job

Render the canonical Daily Briefing with DailyOS chrome, tokens, typography,
margin grid, and inspector contracts. The surface uses a schedule-as-spine
reading flow backed by moving signals, predictions, and watch rows.

## Layout regions

1. **FolioBar / DayStrip / FloatingNavIsland / AtmosphereLayer** - app chrome
   plus proposed previous/current/next briefing-day navigation under FolioBar.
2. **Lead** - DailyBriefing editorial hero treatment without the extra date
   eyebrow or focus primitive.
3. **Today** - `DayChart` plus the new `MeetingSpineItem` stack.
4. **Moving** - current briefing attention-row vocabulary for account/person
   changes; no decorative card glyphs.
5. **Watch** - quiet rows with `InferredActionSelector` dropdowns.
6. **FinisMarker** - editorial close.

## Local nav approach

The surface provides chapters to `FloatingNavIsland`:

- `lead`
- `schedule`
- `moving`
- `watch`

`DayStrip` is included as a proposed Daily Briefing redesign-specific navigation candidate. It
does not remove global app navigation in the reference, but it is the intended
replacement for a separate Weekly Forecast route if the v1.4.0 redesign ships;
the reference hides the `This Week` nav item for this surface.

## Patterns consumed

- `FolioBar`
- `DayStrip`
- `FloatingNavIsland`
- `AtmosphereLayer`
- `MarginGrid`
- `Lead` (surface-local current DailyBriefing hero treatment)
- `DayChart`
- `MeetingSpineItem`
- `MovingRow`
- `WatchRow`
- `InferredActionSelector`
- `FinisMarker`

## Primitives consumed

- `Pill`

## Source alignment

Existing source-backed patterns reused:

- `src/components/dashboard/DayChart.tsx`
- `src/components/ui/Pill.tsx`

New source-backed pattern added for this reference:

- `src/components/dashboard/MeetingSpineItem.tsx`
- `src/components/dashboard/MeetingSpineItem.module.css`

Current shipped DailyBriefing route:

- `src/pages/DailyBriefingRedesign.tsx`
- `src/pages/DailyBriefingRedesign.module.css`

## Release gates

- Reference QA at desktop and mobile widths.
- Inspector overlay shows all major primitives/patterns and the
  `DailyBriefingRedesign` surface tag.
- Routed DailyBriefing consumes the extracted briefing patterns where they map
  cleanly to the shipped surface.

## History

- 2026-05-06 - Reference candidate added from the Daily Briefing redesign mockup using current
  DailyBriefing reference structure and existing design-system components.
- 2026-05-06 - Iteration removed Ask/ThreadMark, added DayStrip, moved
  MeetingSpineItem state tags into the time rail, switched DayChart labels to
  tooltips, and simplified Moving/Watch.
