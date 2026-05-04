# I373 — Email Sync Status Indicator — Show Last Fetch Time, Stage, Error State

**Status:** Open (0.13.1)
**Priority:** P2
**Version:** 0.13.1
**Area:** Frontend / UX

## Summary

When the email pipeline fails (auth error, network timeout, enrichment failure), the user currently sees stale email data with no indication that anything is wrong. This issue adds a sync status indicator to the email page and daily briefing email section: last successful fetch time ("Updated 3 minutes ago"), current enrichment stage (fetching, enriching, complete), and error state if the last fetch failed. It also surfaces the `usingLastKnownGood` flag when data is from a stale fallback.

## Acceptance Criteria

From the v0.13.1 brief, verified with real data in the running app:

1. Open the email page. A status indicator shows: last successful fetch time (e.g., "Updated 3 minutes ago"), and whether enrichment is complete or in progress.
2. If the last fetch failed (auth error, network error), the indicator shows a warning with the error context — not a silent failure.
3. On the daily briefing, the email section shows a subtle "as of X:XX AM" timestamp.
4. The `EmailSyncStatus` type's `usingLastKnownGood` field is surfaced: if the current data is from a stale fallback, the UI indicates this.

## Dependencies

- Depends on I368 (DB persistence) — sync status reads enrichment state and last fetch time from the DB.

## Notes / Rationale

Silent failures are a trust problem. If the user sees the same emails for 3 days without realizing the sync has failed, they start to distrust DailyOS's freshness. The sync status indicator converts pipeline failures from hidden problems into explicit, actionable notifications.
