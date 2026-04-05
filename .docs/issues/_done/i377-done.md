# I377 — Signal System Completeness — Emitter/Propagation/Consumer Map, Remove Dead Rules

**Status:** Open (0.13.2)
**Priority:** P1
**Version:** 0.13.2
**Area:** Code Quality / Signals

## Summary

The signal system (ADR-0080) has grown across multiple sprints, and there are likely propagation rules that listen for signal types that are never emitted — dead rules that consume processing time without producing value. Additionally, user field edits may not be emitting signals, breaking the chain from user correction to entity re-enrichment. This issue creates a complete signal map and remediates any gaps: dead rules removed or given emitters, edit handlers that don't emit signals are fixed.

## Acceptance Criteria

From the v0.13.2 brief, verified in the codebase and running app:

1. A written map exists at `.docs/research/i377-signal-map.md` covering: every `bus.emit()` call site (signal type, entity type, source file), every propagation rule in `rules.rs` (what it listens for, what it derives), and every downstream consumer (callouts, cadence, invalidation, email bridge, post-meeting).
2. Every propagation rule has at least one confirmed emitter in the map. Rules listening for signal types with no emitter are either (a) given an emitter or (b) removed as dead code.
3. User field edits emit signals. Every command handler that writes a user correction to the DB (field updates on accounts, people, projects, stakeholder edits, intelligence field overrides) also calls `bus.emit()` so the change enters the signal chain. Verify by auditing each edit command handler in `commands.rs` against the signal map — any handler that writes to the DB but does not emit a signal is either (a) given an emitter or (b) documented as intentionally signal-free with a rationale. The test: edit a field on a known account in the running app; within one intel_queue cycle, `intelligence.json` for that account reflects the edit without any manual refresh.
4. `post_meeting.rs` fires reliably after a meeting ends. Verify with real data: after a completed meeting, `SELECT * FROM signal_events WHERE source LIKE '%post_meeting%' ORDER BY created_at DESC LIMIT 5` returns rows with timestamps matching the meeting end time.
5. `email_bridge.rs` fires on every enriched email, not only emails linked to upcoming meetings. Verify: `SELECT DISTINCT entity_type FROM signal_events WHERE source LIKE '%email%'` returns at least `account` and `person` — not just `meeting`.
6. Person → account signal propagation fires correctly. After an email signal is emitted for a person, `signal_events` contains a derived signal for that person's linked account within one propagation cycle. Verify with a known person-account pair.

## Dependencies

- No code dependencies; findings may generate new issues.
- Related to I372 (email signal compounding in v0.13.1) — criterion 5 here verifies what I372 should have shipped.

## Notes / Rationale

The pre-0.13.0 audit found "3 of 6 propagation rules listening for signals never emitted." This issue systematically maps the full signal system to find any remaining dead rules and verify the signal chain is live end-to-end. The "user edits emit signals" requirement (criterion 3) is the behavioral contract established in the v0.13.2 design principle: no user-visible change should require a manual refresh.
