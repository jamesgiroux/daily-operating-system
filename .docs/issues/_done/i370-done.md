# I370 — Thread Position Refresh — Detect User Replies Between Polls

**Status:** Open (0.13.1)
**Priority:** P1
**Version:** 0.13.1
**Area:** Backend / Gmail API

## Summary

"Replies Needed" (I356) shows threads where the ball is in the user's court. Currently, if the user replies to a thread via Gmail between DailyOS poll cycles, the thread stays in "Replies Needed" until the next full inbox poll detects the new message. This creates a stale view — the user has replied, but DailyOS still shows them the thread as needing a reply. This issue adds a mechanism to detect user replies between polls by querying recent sent mail or checking thread state.

## Acceptance Criteria

From the v0.13.1 brief, verified with real Gmail data in the running app:

1. DailyOS shows a thread in "Replies Needed" (ball in your court). Reply to it in Gmail. After the next poll cycle (≤15 min), the thread disappears from "Replies Needed."
2. Verify by checking `email_threads.user_is_last_sender` flipped from 0 to 1 for the thread.
3. The poll cycle queries recent sent mail (or thread state) to detect replies — not just incoming mail. Verify via logs.
4. If the other party replies again after your reply, the thread reappears in "Replies Needed" on the next poll.

## Dependencies

- Depends on I365 (inbox-anchored fetch) — thread state detection builds on the inbox fetch loop.
- Builds on thread position tracking established in v0.12.0.

## Notes / Rationale

"Replies Needed" is only useful if it stays accurate. A thread that shows up as needing a reply after the user has already replied is noise — it trains the user to ignore the section. Detecting user replies between polls is the correctness requirement for this feature to be trustworthy.
