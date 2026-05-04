# I364 — Weekly Forecast Timeline Adoption — Replace MeetingRow with Shared Card

**Status:** Closed (v0.13.0)
**Priority:** P1
**Version:** 0.13.0
**Area:** UX

## Summary

The weekly forecast timeline was using a separate `MeetingRow variant="timeline"` component that didn't share code with the daily briefing's meeting cards. After extracting the shared `MeetingCard` component (I362) and enriching the timeline data (I363), this issue replaced all `MeetingRow variant="timeline"` usage in the weekly forecast with the shared `MeetingCard` component and deleted the timeline variant.

## Acceptance Criteria

From the v0.13.0 brief, verified in the running app:

1. Open the weekly forecast. The timeline chapter renders meeting cards that visually match the daily briefing's schedule section — same time format, same accent colors, same entity byline, same intelligence badges.
2. Click any meeting in the timeline. It navigates to `/meeting/$meetingId`.
3. Past meetings in the timeline show outcome indicators (checkmark + summary) — this existing behavior is preserved.
4. The "Review last meeting" link on future meetings with prior history is preserved.
5. `MeetingRow variant="timeline"` is deleted from `src/components/shared/MeetingRow.tsx`. Verify: `grep -r "variant.*timeline" src/` returns 0 results.
6. The earlier/today/ahead section structure is unchanged — only the rows inside each section are different.

## Dependencies

- Depends on I362 (shared meeting card extracted).
- Depends on I363 (timeline data enrichment with time/duration).
- Depends on I361 (timeline filtering applied).

## Notes / Rationale

Deleting `MeetingRow variant="timeline"` removed a divergent code path that would have continued to drift from the shared component over time. The deletion is the proof that the adoption is complete — not just that the new component is being used, but that the old one no longer exists to be accidentally used.
