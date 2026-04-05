# I394 — Week Page Past Meeting Duration Shows NaN

**Status:** Open
**Priority:** P1
**Version:** 0.13.1
**Area:** Frontend / Bug

## Summary

Past meetings on the Weekly Forecast timeline show `NaN` for their duration. `formatDurationFromIso` does not guard against NaN when the date strings it receives are invalid or missing. Since past meetings may have `end_time` values that are null, malformed, or not yet backfilled, the duration calculation produces NaN which renders directly in the UI.

## Acceptance Criteria

1. Open the Weekly Forecast page. No past meeting displays `NaN` for duration — either a formatted duration (e.g., "45m", "1h") or nothing renders (if end time is unknown).
2. `formatDurationFromIso` guards against all NaN-producing inputs: null start, null end, invalid ISO strings, end before start. Returns `null` or an empty string in those cases — never `"NaN"`.
3. The fix applies across all surfaces that call `formatDurationFromIso` — verify with `grep -r "formatDurationFromIso" src/` and confirm each call site handles the nullable return.
4. Past meetings with valid start and end times continue to display correct formatted durations.

## Dependencies

None. Isolated frontend bug fix.

## Notes / Rationale

Introduced when timeline data enrichment (I363) added duration display to `TimelineMeeting` rows. Past meetings sourced from `meetings_history` may have null or inconsistent `end_time` values — particularly for meetings imported from before the duration field was tracked. The guard needs to be in the utility function, not at every call site.
