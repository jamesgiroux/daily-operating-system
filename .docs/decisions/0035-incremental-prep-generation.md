# ADR-0035: Incremental prep generation for new meetings

**Date:** 2026-02
**Status:** Superseded by [ADR-0030](0030-weekly-prep-with-daily-refresh.md)

## Context

When the calendar poller (ADR-0032) detects a new meeting that wasn't in the morning briefing, the meeting appears on the dashboard with `overlay_status: new` and no prep. Currently the only way to get prep for this meeting is to re-run the entire briefing workflow (all three phases).

This is wasteful. The existing three-phase pattern (prepare → enrich → deliver) operates on the full day's schedule. A single new meeting shouldn't require re-fetching all emails, re-enriching all preps, and re-delivering the entire briefing.

## Decision

Decompose the three-phase pattern to support per-meeting prep generation. When a `new` meeting appears via calendar polling:

1. **Phase 1 (lightweight):** Generate a single-meeting directive with attendee info, account context, and meeting history — skip email/action/inbox gathering.
2. **Phase 2:** Run Claude Code enrichment scoped to one meeting.
3. **Phase 3:** Write a single prep JSON file to `_today/data/preps/` and update `schedule.json` to add the meeting entry with `hasPrepFile: true`.

The existing full-briefing workflow remains unchanged. This is an additive capability.

## Consequences

- New meetings get prep within minutes of appearing on the calendar, not hours
- Requires decomposing `prepare_today.py` into composable units (per-meeting vs full-day)
- Phase 2 prompt needs a single-meeting variant
- `deliver_today.py` needs an append-to-schedule mode (vs overwrite)
- Risk: partial briefing state if the incremental prep fails mid-way — mitigated by treating the prep file as atomic (write or don't)
- Depends on ADR-0032 (calendar polling detects new meetings) and ADR-0033 (calendar event ID linkage)
