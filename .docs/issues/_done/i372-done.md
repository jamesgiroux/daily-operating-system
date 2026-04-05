# I372 — Email-Entity Signal Compounding — Email Signals Flow into Entity Intelligence

**Status:** Open (0.13.1)
**Priority:** P1
**Version:** 0.13.1
**Area:** Backend / Signals

## Summary

Email signals currently have limited propagation into the entity signal graph. The `email_bridge.rs` module exists but was found (in the pre-0.13.0 audit) to only apply signals for emails linked to upcoming meetings, not for emails linked to accounts, projects, or people generally. This issue extends email signal emission to apply universally: every enriched email's extracted signals (commitment, sentiment, urgency, topic) flow into the signal graph for the resolved entity — whether that entity is an account, person, or project — and signal propagation fires afterward.

## Acceptance Criteria

From the v0.13.1 brief, verified with real data in the running app:

1. Receive an email with negative sentiment from a contact at Acme. Check the Acme account detail page. The email's sentiment signal has been emitted to the entity signal graph — visible in the signal timeline or affecting the entity's health indicators.
2. Receive a commitment email ("We'll send the contract by Friday"). The commitment is extracted AND a `commitment_received` signal is emitted for the related entity. Verify in `signal_events` table.
3. The email bridge (`signals/email_bridge.rs`) runs on EVERY enriched email, not just emails linked to upcoming meetings. Entity signals from emails apply to accounts, projects, and people — not just meetings. Verify: `SELECT DISTINCT entity_type FROM signal_events WHERE source LIKE '%email%'` returns at least `account` and `person`, not just `meeting`.
4. Signal propagation fires after email signals are emitted — if an email signal affects a person, it propagates to their linked accounts per existing propagation rules. Verify: after an email signal is emitted for a person, check `signal_events` for a corresponding propagated signal on their linked account.

## Dependencies

- Depends on I367 (mandatory enrichment) — signals are extracted as part of enrichment; needs enrichment to run first.
- Builds on existing signal bus (ADR-0080) and propagation rules.
- Related to I377 (signal system completeness in v0.13.2) which will audit and verify this path.

## Notes / Rationale

ADR-0085 decision 4: "Email signals compound with entity intelligence." This is what makes email a true intelligence input rather than a display surface — email events change what DailyOS knows about your accounts and people, not just what it shows in the email list. An email about Acme should update Acme's intelligence.
