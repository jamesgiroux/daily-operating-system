# I329 — Intelligence Quality Indicators

**Status:** Closed (v0.13.0)
**Priority:** P1
**Version:** 0.13.0
**Area:** UX

## Summary

The prior meeting card used a `hasPrep` boolean dot indicator to show whether a meeting had intelligence — a binary, internal-sounding signal. This issue replaced that with vocabulary-correct quality badges that reflect the actual intelligence state: Sparse (new meeting with minimal entity context), Developing (entity context exists but intelligence is still building), and Ready (full intelligence available). The `hasPrep` dot was removed entirely.

## Acceptance Criteria

From the v0.13.0 brief, verified in the running app:

1. Open daily briefing. Every meeting card shows a quality badge. No badge reads "Needs Prep", "hasPrep", or any internal state name.
2. Badge vocabulary reflects the actual intelligence state: a meeting with enriched intelligence shows a different badge than one still generating.
3. The `hasPrep` dot is removed from the schedule row — it does not appear anywhere.

## Dependencies

- Depends on I326 (lifecycle state machine) — quality indicators read from `meeting_prep_queue` state.
- Shared across I362 (shared meeting card component).

## Notes / Rationale

"hasPrep" was an internal boolean that leaked into the UI without meaningful user interpretation. The quality badge system communicates the same information in terms the user can act on: "Ready" means go, "Sparse" means the system is still building context.
