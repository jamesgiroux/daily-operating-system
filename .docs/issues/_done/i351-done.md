# I351 — Actions Chapter on PersonDetailEditorial

**Status:** Closed (v0.13.0)
**Priority:** P1
**Version:** 0.13.0
**Area:** UX / Entity

## Summary

The Person detail page (PersonDetailEditorial) was missing an actions chapter. Account and project detail pages already surfaced associated actions in a dedicated chapter, but person detail did not. For 1:1 relationships — the primary relationship type surfaced on person detail — the actions arising from those meetings are often the most important follow-through items to track. This issue added an actions chapter to the person detail page, surfacing actions from 1:1 meetings with that person.

## Acceptance Criteria

From the v0.13.0 brief, verified in the running app:

1. Open the detail page for any person who has had at least one 1:1 meeting.
2. An "Actions" or equivalent chapter appears on the page — in the same structural position as actions on account and project detail pages.
3. At least one action from a 1:1 meeting with that person is surfaced there.
4. The actions chapter is absent for people with no meeting history (no empty chapter rendered).

## Dependencies

- Depends on entity actions infrastructure (I127, Sprint 10).
- Parity with account detail (I133) and project detail actions chapter.

## Notes / Rationale

Actions are how intent becomes reality. Showing actions on person detail closes the loop between the relationship intelligence ("Jim mentioned he'd send over the SOW") and the trackable follow-through. Without this chapter, person detail was purely retrospective (meetings, signals) with no forward-looking action context.
