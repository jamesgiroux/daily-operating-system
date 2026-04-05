# I366 — Inbox Reconciliation — Vanished Emails Removed from DailyOS on Each Poll

**Status:** Open (0.13.1)
**Priority:** P0
**Version:** 0.13.1
**Area:** Backend / Pipeline

## Summary

When a user archives an email in Gmail, it should disappear from DailyOS on the next poll. Currently, DailyOS only adds emails it sees — it doesn't remove emails that have disappeared from the inbox. This means archived emails persist in DailyOS indefinitely, creating a growing divergence between the Gmail inbox state and what DailyOS shows. This issue adds reconciliation: on each poll, compare the current inbox set against the DB, mark vanished emails as resolved, and inactivate their signals.

Email signals are marked inactive, not deleted — they're historical intelligence that remains useful even after the email is archived.

## Acceptance Criteria

From the v0.13.1 brief, verified with real Gmail data in the running app:

1. DailyOS shows 10 emails. Archive 3 in Gmail. After the next poll, DailyOS shows 7 emails.
2. The `email_threads` table marks threads for the 3 vanished emails with `resolved_at` timestamp.
3. Stale `email_signals` for vanished emails are marked inactive (not deleted — they're historical intelligence).
4. "Replies Needed" does not show threads for emails that have been archived in Gmail.
5. If an email reappears in the inbox (un-archived), it shows up again on the next poll.

## Dependencies

- Depends on I365 (inbox-anchored fetch) — reconciliation compares the current inbox set; needs inbox fetch to know the current set.
- Depends on I368 (persist to DB) — reconciliation reads from and writes to the `emails` table.

## Notes / Rationale

Reconciliation is the complement to fetching: you learn what arrived AND what left. Without reconciliation, DailyOS accumulates emails forever, the inbox view drifts from reality, and "Replies Needed" shows threads the user has already resolved. This is a correctness requirement, not an optimization.
