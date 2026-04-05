# I348 — Email Digest Push — DailyOS Intelligence Summaries via Scheduled Email

**Status:** Open (0.14.0)
**Priority:** P2
**Version:** 0.14.0
**Area:** Distribution

## Summary

A scheduled email digest that pushes DailyOS intelligence summaries to the user's inbox — so the daily briefing can be consumed even when the user doesn't open the app. The digest would be a curated synthesis of the key intelligence for the day: upcoming meetings with prep status, critical actions, account signals requiring attention, and email threads needing response.

This inverts the usual app interaction model: instead of the user pulling information by opening DailyOS, DailyOS pushes a summary to wherever the user already is.

## Acceptance Criteria

Not yet specified for v0.14.0. Will be detailed in the v0.14.0 version brief. At minimum: a configurable daily email digest time, a formatted digest email containing the top items from the daily briefing, and an opt-in/opt-out control in Settings.

## Dependencies

- Requires email sending capability (SMTP or Gmail API send).
- Related to I258 (Report Mode) and I302 (PDF export) — digest might include an attached PDF.
- Part of the v0.14.0 distribution bundle.

## Notes / Rationale

Many users operate from their inbox all day and don't open secondary apps habitually. An email digest lowers the barrier to DailyOS adoption — the user gets value without a behavior change. The risk is spam: the digest must be high quality enough that users want it in their inbox, not just another notification to dismiss.
