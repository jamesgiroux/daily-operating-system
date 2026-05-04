# I369 — Contextual Email Synthesis — Entity-Aware Smart Summaries

**Status:** Open (0.13.1)
**Priority:** P1
**Version:** 0.13.1
**Area:** Backend / Intelligence

## Summary

Email summaries in DailyOS currently describe what the email says in isolation: "Jack sent a message about the EBR agenda." Contextual synthesis replaces this with a synthesis that connects the email to what DailyOS already knows about the entity: "Jack is confirming the Acme EBR agenda for Thursday. This aligns with the renewal discussion from your last meeting." The difference is entity-awareness — the synthesis prompt includes the entity's current intelligence, recent meeting history with the sender, and active signals.

This is the core intelligence behavior described in ADR-0085 and the primary quality improvement in v0.13.1.

## Acceptance Criteria

From the v0.13.1 brief, verified with real Gmail data in the running app:

1. Receive an email from a known contact at a known account (e.g., Jack at Acme). The email summary on the email page reads something like: "Jack is confirming the agenda for Thursday's EBR. This relates to the renewal discussion from your last meeting." NOT: "Email from Jack about EBR Agenda."
2. Receive an email from an unknown sender with no entity match. The summary is still useful ("New inquiry about enterprise pricing from an unknown contact at example.com") but acknowledges the lack of entity context.
3. Receive a low-information email (e.g., "Sounds good, thanks"). The summary says something like: "Jeff acknowledged your last message about the Medico expansion. No new information. Monitor." NOT: "Jeff sent a one-line reply."
4. The synthesis prompt includes: the email content, the resolved entity's current intelligence (from `entity_intel`), recent meeting history with the sender, and active signals for the entity. Verify by adding a debug log that prints the assembled prompt for one email — confirm it contains real entity data (account name, recent meeting date, signal text), not empty placeholders or nulls.
5. Synthesis runs as part of the enrichment pipeline (I367), not as a separate pass. Every enriched email has a `contextual_summary` in the DB. Verify: `SELECT count(*) FROM emails WHERE enrichment_state = 'enriched' AND contextual_summary IS NULL` returns 0.

## Dependencies

- Depends on I367 (mandatory enrichment) — synthesis is the enrichment content; the pipeline infrastructure comes first.
- Depends on I368 (DB persistence) — synthesis reads entity context from DB and writes `contextual_summary`.
- Related to ADR-0085 (Email as Intelligence Input) — this issue implements decision 3 (contextual synthesis, not raw summaries).

## Notes / Rationale

The litmus test from the v0.13.1 brief: open DailyOS in the morning. Without opening Gmail, understand what's happening in your email that affects your work today. Not "you have 12 emails" — "Jack confirmed the Acme EBR agenda, the Medico expansion thread has no new information, and Legal is waiting for your response on the SOW from Tuesday." Contextual synthesis is what makes that litmus test pass.
