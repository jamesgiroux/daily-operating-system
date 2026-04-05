# I225 — Gong Integration — Sales Call Intelligence + Transcripts

**Status:** Open (Integrations Queue)
**Priority:** P1
**Version:** Unscheduled (depends on Gong API access)
**Area:** Integrations

## Summary

Gong is the dominant call recording and intelligence platform for sales and CS teams. A DailyOS user with a Gong account has a rich store of recorded sales calls, AI-generated call summaries, deal intelligence, and competitor mentions sitting in Gong that never flows into DailyOS. This integration would pull Gong call data into the entity intelligence pipeline: call summaries, identified risks and opportunities, deal stage changes, and talk track analysis would become signals in the entity signal graph, enriching account and person intelligence without the user needing to drop transcripts manually.

## Acceptance Criteria

Not yet specified. At minimum: OAuth connection to Gong, pull of call summaries and topics for recent calls, entity resolution of Gong participants to DailyOS account/person entities, and signal emission for key Gong-detected events (deal risk, competitor mention, champion change).

## Dependencies

- Requires Gong API access (API key or OAuth credentials from the Gong account).
- The entity intelligence pipeline (v0.10.0+) and signal bus (ADR-0080) must be in place — they are.
- Not version-locked; pulled in when capacity and API access allow.

## Notes / Rationale

P1 priority because Gong users are a primary target user profile (CS managers, sales leaders) and the Gong data is uniquely valuable — it's recorded evidence of what's actually being said in customer calls, not just calendar metadata. The integration is blocked on Gong API access, not on DailyOS infrastructure readiness.
