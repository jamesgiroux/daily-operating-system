# I328 — Classification Expansion — All Meeting Types Get Intelligence

**Status:** Closed (v0.13.0)
**Priority:** P1
**Version:** 0.13.0
**Area:** Backend / Intelligence

## Summary

Prior to this issue, meeting intelligence was only generated for a subset of meeting types: external customer meetings, QBRs, and training sessions. Internal meetings, team syncs, 1:1s, and all-hands were excluded from the enrichment pipeline. This created a two-tier experience where some meetings had prep and others had nothing. This issue extended the entity-generic prep pipeline to cover all meeting types — every meeting gets intelligence based on the entities and people involved.

## Acceptance Criteria

Delivered in v0.13.0. The following was verified:

1. 1:1 meetings are enriched via the people-prep path (using person intelligence for the relevant person entity).
2. Internal team syncs are enriched using the internal account entity context.
3. All-hands meetings have intelligence based on the company/internal entity.
4. No meeting type is excluded from the `meeting_prep_queue` based on type alone.

## Dependencies

- Depends on I326 (meeting lifecycle state machine) and entity intelligence pipeline (v0.10.0+).
- Enables I329 (quality indicators) — quality indicators are meaningful only when all meetings have intelligence.

## Notes / Rationale

The entity-generic architecture introduced in v0.10.0 made this extension straightforward: the enrichment system already supports accounts, projects, and people. Meeting classification expansion maps each meeting type to the appropriate entity type for its enrichment context.
