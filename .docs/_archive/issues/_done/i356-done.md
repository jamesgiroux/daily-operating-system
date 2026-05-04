# I356 — Thread Position UI — Replies Needed

**Status:** Closed (v0.13.0)
**Priority:** P1
**Version:** 0.13.0 (partial work in v0.12.1, completed in v0.13.0)
**Area:** UX / Email

## Summary

Added a "Replies Needed" subsection to the daily briefing's Attention section — surfacing email threads where the ball is in the user's court (the last message in the thread is from the other party). This gives the user a clear, glanceable view of which email conversations require their response, without having to open Gmail and scan their inbox.

Thread position detection (knowing whether the user or the other party sent the last message) was built in the v0.12.0 email intelligence sprint. This issue surfaces that detection in the daily briefing UI.

## Acceptance Criteria

From the v0.13.0 brief, verified in the running app:

1. Open daily briefing. In the Attention section, a "Replies Needed" subsection appears.
2. The threads listed are ones where the last message in the thread is from the other party (ball in your court). Verify one thread manually against Gmail to confirm it's accurate.
3. If no threads require a reply, the subsection does not render (no empty state with a heading).

## Dependencies

- Depends on thread position tracking (v0.12.0 — I322 or equivalent).
- Related to I370 (thread position refresh) in v0.13.1 — v0.13.1 adds polling to update thread position between sync cycles.

## Notes / Rationale

"Replies Needed" is one of the most actionable outputs DailyOS can produce. It directly answers "what emails do I need to respond to?" without requiring the user to open Gmail. The rule is simple: if the last message in a thread is not from you, you owe a reply.
