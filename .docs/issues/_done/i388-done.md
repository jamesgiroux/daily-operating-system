# I388 — Project Hierarchy Intelligence — Two-Layer intelligence.json + Bidirectional Propagation for Project Entities

**Status:** Open (0.13.4)
**Priority:** P1
**Version:** 0.13.4
**Area:** Backend / Intelligence

## Summary

ADR-0087 defines a portfolio intelligence model that applies to any entity type with a parent/child relationship — not only accounts. v0.13.3 implements it for accounts. v0.13.4 applies the identical architecture to project entities: parent projects become portfolio surfaces for Marketing, Product, and Agency users, with the same two-layer intelligence model (own signals + portfolio synthesis from children) and the same bidirectional signal propagation (child signals accumulate at parent; significant parent signals cascade to children).

The vocabulary and prompt shape adapt to the project context rather than the account context: campaigns, workstreams, milestones, and program health rather than account health, renewal risk, and spend.

## Acceptance Criteria

Not yet specified. Will be detailed in the v0.13.4 version brief. Should mirror the I384 and I385 acceptance criteria adapted for project entities:

- Parent projects have a `portfolio` field in `intelligence.json` populated with project-portfolio-appropriate vocabulary.
- Child project signals propagate upward to parent projects at 60% confidence.
- Parent project signals fan out to direct children at 50% confidence (≥0.7 threshold).
- Project detail pages show a Portfolio chapter for parent projects.
- Leaf-node projects are unaffected.

## Dependencies

- Logically follows I384 (account portfolio intelligence) and I385 (bidirectional propagation for accounts) — the same pattern, applied to projects.
- Related to I389 (entity-mode-aware surface ordering) — `entityModeDefault: "project"` preset users get the portfolio surface for projects first.
- See ADR-0087 decision 5.

## Notes / Rationale

ADR-0087 explicitly generalized the portfolio model: "The two-layer parent intelligence model and bidirectional propagation apply to any entity type with a parent/child relationship." v0.13.4 is the natural next step after v0.13.3 validates the model with accounts. The implementation should be nearly identical — the main work is project-appropriate prompt vocabulary and verifying the entity type routing.
