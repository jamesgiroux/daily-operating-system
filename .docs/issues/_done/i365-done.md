# I365 — Inbox-Anchored Email Fetch — `in:inbox` Replaces `is:unread newer_than:1d`

**Status:** Open (0.13.1)
**Priority:** P0
**Version:** 0.13.1
**Area:** Backend / Gmail API

## Summary

The Gmail fetch query was using `is:unread newer_than:1d` — fetching only emails received in the last 24 hours that are currently unread. This had two problems: it missed emails that had been read but not archived (still in the inbox and relevant), and it didn't reflect actual inbox state. This issue changes the fetch query to `in:inbox` with a configurable time window (default 3 days), so DailyOS reflects the user's actual inbox — everything currently in Gmail's inbox, whether read or not.

The `unread` status is preserved as metadata on each email (so "Replies Needed" still knows what's been read), but it's no longer the filter criterion for what gets fetched.

## Acceptance Criteria

From the v0.13.1 brief, verified with real Gmail data in the running app:

1. Open Settings → Google section. Confirm email polling is active.
2. Read an email in Gmail (mark it as read) but do NOT archive it. Wait one poll cycle (≤15 min). Open DailyOS. The email still appears on the email page — it was not filtered out by `is:unread`.
3. Archive that email in Gmail. Wait one poll cycle. The email no longer appears on the email page.
4. Check logs: the Gmail API query contains `in:inbox`, not `is:unread newer_than:1d`.
5. The configurable time window (`newer_than:Xd`) defaults to 3 days. Verify emails up to 3 days old appear if still in inbox.
6. The `unread` status is preserved as metadata on the email object — verify `email.isUnread` (or equivalent) is `true` for unread emails and `false` for read emails, even though both are fetched.

## Dependencies

- Foundational for I366 (inbox reconciliation) — reconciliation compares current inbox against DB; needs inbox-anchored fetch first.
- Foundational for I370 (thread position refresh) — thread position detection needs to know what's in the inbox.
- See ADR-0085 (Email as Intelligence Input) — decision 5: "Inbox state is the source of truth."

## Notes / Rationale

`is:unread newer_than:1d` was a bootstrap query that made sense early in the product when email was a display surface. Now that email drives entity intelligence, the query must reflect actual inbox state. An email that's been read but not acted on is still a live signal — filtering it out because it's been read is wrong.
