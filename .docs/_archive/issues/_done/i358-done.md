# I358 — Email Page — First-Class Nav Surface

**Status:** Closed (v0.13.0)
**Priority:** P1
**Version:** 0.13.0 (partial work in v0.12.1, completed in v0.13.0)
**Area:** UX / Email

## Summary

The email surface was not a first-class navigation destination — it was accessible only as a sub-section of the daily briefing or via deep link. This issue promoted email to a named nav item (Email) in the main navigation, with a dedicated page organized by meeting and entity context rather than a flat chronological list.

The email page is distinct from an inbox view — it shows emails organized by their relationship to the user's work (which account, which meeting, which person), not sorted by arrival time.

## Acceptance Criteria

From the v0.13.0 brief, verified in the running app:

1. The main navigation bar includes Email as a named nav item. It is not buried in settings or accessible only via deep link.
2. Clicking it loads the email page without a full page reload.
3. The email page organizes emails by meeting or entity context — not a flat chronological list.

## Dependencies

- Depends on email intelligence pipeline (v0.12.0) for the entity-contextual organization.
- Related to I365-I374 in v0.13.1 — the email page data quality will be significantly improved when those issues ship.

## Notes / Rationale

Navigation placement communicates product priorities. Email is a primary intelligence input and a primary action surface ("Replies Needed"). Burying it in the briefing undervalued the surface. Promoting it to main navigation declares that email intelligence is a first-class product concern, not a supplement.
