# I362 — Shared Meeting Card — Extract Core Rendering from BriefingMeetingCard

**Status:** Closed (v0.13.0)
**Priority:** P1
**Version:** 0.13.0
**Area:** UX / Component

## Summary

Meeting cards were rendered differently on the daily briefing (via `BriefingMeetingCard`) and the weekly forecast (via `MeetingRow variant="timeline"`). The same meeting looked visually different depending on which page you were on — different accent colors, different time formats, different intelligence badge treatment. This issue extracted a shared `MeetingCard` component that both surfaces use, establishing a single visual identity for meetings across the app.

## Acceptance Criteria

From the v0.13.0 brief, verified in the running app:

1. The shared `MeetingCard` component exists at `src/components/shared/MeetingCard.tsx` with a co-located CSS module.
2. Customer/QBR/partnership/external meetings show a turmeric accent. 1:1 meetings show larkspur. Internal/team sync show linen. Verify on both the daily briefing and weekly forecast.
3. Past meetings render with muted/dimmed treatment on both pages.
4. Entity context (account or project name) appears below the meeting title on both pages.
5. Intelligence quality badges appear on both pages with the same visual treatment.
6. `BriefingMeetingCard` uses `MeetingCard` internally — it does not duplicate the base rendering. Verify: `grep -c "scheduleRow\|scheduleTime\|scheduleContent" src/components/dashboard/BriefingMeetingCard.tsx` returns 0 (those classes moved to the shared component).
7. No inline `style={{}}` props on the shared component — all styling via CSS module.

## Dependencies

- Enables I364 (weekly forecast timeline adoption).
- Depends on I329 (quality indicators) for the badge component.
- Depends on I363 (timeline data enrichment) for the time/duration data.

## Notes / Rationale

"One meeting, one visual identity" was a core thesis of v0.13.0. If a meeting looks like itself everywhere — same accent color, same time format, same entity context, same intelligence badge — the user builds trust that DailyOS is showing them consistent, reliable information. Divergent rendering across surfaces breaks that trust.
