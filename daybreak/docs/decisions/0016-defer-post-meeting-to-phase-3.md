# ADR-0016: Defer post-meeting capture to Phase 3

**Date:** 2026-02
**Status:** Accepted

## Context

Post-meeting capture (win/risk/action quick entry after meetings) requires working calendar integration to detect meeting end times.

## Decision

Defer to Phase 3, after calendar integration (Phase 2/3A) is stable.

## Consequences

- Correct dependency ordering: calendar must work before meeting-end detection can work
- Users lose a feedback loop in MVP — they can't capture outcomes in the app
- Phase 3 can build capture on top of reliable calendar polling
