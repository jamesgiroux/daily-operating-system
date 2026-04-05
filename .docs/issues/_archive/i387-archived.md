# I387 — Multi-Entity Signal Extraction from Parent-Level Meetings

**Status:** Deferred (P3)
**Priority:** P3
**Version:** Unscheduled
**Area:** Backend / Pipeline

## Summary

When a meeting is tagged to a parent account (e.g., "Crestview Media Enterprises"), the meeting may contain signals relevant to specific child accounts (e.g., "Crestview Media B2B" and "Crestview Media Retail" are mentioned separately in the transcript). Currently, all signals from that meeting land on the parent entity and stay there — the transcript processor doesn't perform content-level entity resolution to route signals to the appropriate children.

This issue would extend the transcript processor to detect mentions of child entities within parent-level meeting content and route signals to the appropriate child accounts rather than (or in addition to) the parent.

## Acceptance Criteria

Not yet specified. Deferred per ADR-0087 — "content-level entity resolution in the transcript processor; deferred per ADR-0087. User behavior is to tag at the parent level; bidirectional propagation (I385) provides sufficient coverage for now."

## Dependencies

- Deferred until I385 (bidirectional propagation) is shipped and the use case is validated in practice.
- Absorbed into I367 context: the existing mandatory enrichment pipeline is the foundation this would build on.

## Notes / Rationale

ADR-0087 decision 6: "Multi-entity signal extraction is a future concern." The expected user behavior is to tag meetings at the parent level. Bidirectional propagation (I385) partially addresses the need: parent-tagged meetings produce parent-level signals that cascade down to children via fan-out. Content-level multi-entity extraction is a refinement that becomes valuable once the portfolio intelligence system (I384, I393) is validated with real users.

P3 priority reflects that this is a quality refinement, not a missing capability. The system works without it; it just works at the parent level of resolution rather than the child level.
