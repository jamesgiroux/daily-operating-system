# I374 — Email Dismissal Learning — Dismissed Senders/Types Adjust Future Classification

**Status:** Open (0.13.1)
**Priority:** P2
**Version:** 0.13.1
**Area:** Backend / Intelligence

## Summary

When a user repeatedly dismisses emails from a particular sender domain or email type, DailyOS should learn from this pattern and classify future emails from that domain lower (medium → low priority) without manual intervention. This is adaptive classification based on observed user behavior — the system learns what the user considers low-value and deprioritizes it automatically.

The learning is additive to mechanical classification, not a hard override: a high-urgency email from a "learned low" domain still gets classified high if urgency signals are present.

## Acceptance Criteria

From the v0.13.1 brief, verified with real data in the running app:

1. Dismiss 5 emails from the same sender domain over multiple days.
2. On subsequent polls, emails from that domain are classified lower (medium → low) without manual intervention.
3. The learning reads from `email_dismissals` table — verify the query groups by `sender_domain` and `email_type` with a threshold count.
4. The learning is additive to mechanical classification, not a hard override — a high-urgency email from a "learned low" domain still gets classified high if urgency keywords match.
5. A user can reset learned preferences from Settings.

## Dependencies

- Depends on `email_dismissals` table existing in the DB (may require a migration if not yet present).
- Independent of other v0.13.1 issues — can be built in parallel.

## Notes / Rationale

Adaptive classification respects the user's revealed preferences without requiring explicit configuration. The 5-dismissal threshold prevents a single accidental dismissal from permanently lowering a sender's priority. The "additive, not override" principle ensures high-signal emails from low-priority senders don't get buried.
