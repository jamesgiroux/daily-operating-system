# I384 — Parent Account Portfolio Intelligence — Two-Layer intelligence.json

**Status:** Open (0.13.3)
**Priority:** P1
**Version:** 0.13.3
**Area:** Backend / Intelligence

## Summary

A parent account (any account with child accounts) has a qualitatively different intelligence need than a leaf-node account. The current enrichment treats them identically — the parent gets the same prompt shape as a child BU. This issue implements two-layer intelligence for parent accounts: the AI enrichment prompt includes the `intelligence.json` of every direct child, and the resulting `intelligence.json` for the parent includes a `portfolio` field with portfolio health assessment, hotspot accounts, cross-BU patterns, and a portfolio narrative.

This is the intelligence layer required by the portfolio surface (I393). The portfolio section only appears on accounts with children; leaf-node accounts are unchanged.

## Acceptance Criteria

From the v0.13.3 brief, verified with real data in the running app:

1. The AI enrichment prompt for any account that has child accounts includes: the `intelligence.json` of every direct child (or a summarized excerpt if the child count is large), key active signals for each child, and the parent's own signals (from meetings tagged to the parent and user edits on the parent entity).
2. The resulting `intelligence.json` for a parent account contains a `portfolio` field that is absent from leaf-node intelligence. The `portfolio` field contains: `health_summary` (portfolio-level health assessment), `hotspots` (array of child accounts needing attention with reason), `cross_bu_patterns` (signal types or topics appearing in 2+ children), and `portfolio_narrative` (AI-synthesized executive view).
3. Verify with a real parent account that has 3+ children with existing intelligence: open the account detail page. The portfolio section renders with real content — not placeholder text, not null values. `hotspots` contains at least one entry if any child has a risk signal.
4. When a child account's `intelligence.json` is updated (by any means — enrichment, user edit, signal cascade), the parent account is enqueued in `intel_queue` for re-enrichment. Verify: update a child's intelligence; within one intel_queue cycle, the parent's `intelligence.json` `updated_at` timestamp changes.
5. Leaf-node accounts (no children) are unaffected. Their enrichment prompt and `intelligence.json` shape are unchanged. Verify: `SELECT id FROM accounts WHERE id NOT IN (SELECT DISTINCT parent_id FROM accounts WHERE parent_id IS NOT NULL)` returns leaf accounts; check one — no `portfolio` field in its `intelligence.json`.

## Dependencies

- Blocked by I385 (bidirectional propagation) — portfolio intelligence is most useful once child signals are propagating to the parent; do I385 first so the prompt has real signal data when portfolio enrichment runs for the first time.
- Required by I393 (parent detail page) — the page renders from `intelligence.json.portfolio`.
- See ADR-0087 decision 3.

## Notes / Rationale

A parent account without portfolio intelligence is just a container with a list of children. A parent account with portfolio intelligence is an executive view of the relationship: which BUs are healthy, which need attention, what's the pattern across the portfolio. This is the intelligence that makes DailyOS genuinely useful for users managing complex account hierarchies.
