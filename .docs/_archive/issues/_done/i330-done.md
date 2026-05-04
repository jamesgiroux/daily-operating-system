# I330 — Week Page ±7-Day Meeting Intelligence Timeline

**Status:** Closed (v0.13.0)
**Priority:** P1
**Version:** 0.13.0
**Area:** UX

## Summary

The Weekly Forecast page previously showed a simplified meeting list without intelligence context. This issue added a ±7 day meeting intelligence timeline as the primary content section of the week page — showing upcoming meetings with intelligence quality indicators, prep gap warnings, and days-until context, and past meetings with outcome indicators, follow-ups generated, and context seeds.

## Acceptance Criteria

Delivered in v0.13.0. The following was verified:

1. The week page shows a meeting timeline spanning from 7 days before today to 7 days ahead.
2. Future meetings show: intelligence quality badge, prep gap indicator, days until meeting.
3. Past meetings show: outcome indicator (checkmark + summary), follow-ups generated, context seeds.
4. The timeline respects the filtering rules in I361 (personal/focus/blocked events excluded).
5. Each meeting card uses the shared MeetingCard component (I362) with consistent visual treatment.

## Dependencies

- Depends on I326 (lifecycle), I327 (advance generation), I361 (filtering), I362 (shared card), I363 (data enrichment), I364 (timeline adoption).

## Notes / Rationale

The ±7 day window is intentionally narrow — the weekly forecast is about the current week with a few days of lookback, not a general-purpose calendar. The timeline replaced the earlier `MeetingRow variant="timeline"` component which was removed as part of I364.
