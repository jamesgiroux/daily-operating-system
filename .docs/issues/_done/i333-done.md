# I333 — Meeting Intelligence Collaboration — Share, Request Input, Draft Agenda

**Status:** Closed (v0.13.0)
**Priority:** P2
**Version:** 0.13.0
**Area:** UX

## Summary

Added collaboration affordances to the meeting detail folio bar: a Share button to share the meeting briefing, a Request Input button to ask an attendee or colleague to contribute context, and a Draft Agenda button to generate a structured agenda from the current prep. These actions surface from the folio bar (the persistent toolbar at the top of meeting detail pages) rather than being buried in the page content.

## Acceptance Criteria

Delivered in v0.13.0. The following was verified:

1. The meeting detail folio bar includes Share, Request Input, and Draft Agenda actions.
2. Share produces a shareable version of the meeting briefing (link or formatted text).
3. Draft Agenda generates an agenda from the current prep context.
4. The Sync Transcript button is in the folio bar (restored — had been removed in an earlier cleanup).

## Dependencies

- Depends on the folio bar component infrastructure (Sprint 25).
- I342 acceptance criteria note: "Prefill affordance is inside the Your Plan section, not the folio bar" and "Transcript attachment CTA is in the page body, not duplicated in the folio bar" — these are distinct from the collaboration actions in I333.

## Notes / Rationale

Collaboration affordances bring the chief-of-staff concept to life: the intelligence isn't just for the individual user, it's a foundation for coordinating with the meeting's other participants. The folio bar is the right home for these actions — persistent, not buried in a scrollable page.
