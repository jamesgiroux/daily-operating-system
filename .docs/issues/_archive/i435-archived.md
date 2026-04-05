# I435 — Token Optimization

**Status:** Open
**Priority:** P1
**Version:** 0.16.1
**Area:** Backend

## Summary

The app uses Claude API for entity enrichment, email enrichment, report generation, and meeting prep — but the model tier assignments have never been systematically audited. Some tasks (email extraction, SWOT analysis) are running on Sonnet when Haiku would produce equivalent output at lower cost. This issue audits every PTY call site, assigns the correct model tier to each, and measures the resulting reduction in daily call count.

## Acceptance Criteria

1. A written audit exists at `.docs/research/i435-token-audit.md` listing every PTY call site (from the I376 enrichment audit), its current model tier assignment, its actual prompt size (measured in production), and a recommendation (keep tier / downgrade to Haiku / batch with other calls).
2. Email enrichment uses Haiku tier, not Sonnet. Every call to `enrich_email` in `prepare/email_enrich.rs` uses `ModelTier::Extraction`. Verify: `grep -n "ModelTier" src-tauri/src/prepare/email_enrich.rs` — returns Extraction, not Synthesis.
3. Entity enrichment only fires when the trigger score (I408) exceeds threshold OR when the entity has never been enriched. The hardcoded "14-day staleness" path (now replaced by I408's trigger function) does not cause enrichments for entities with quality score > 0.75 and no new signals. Verify: an entity with `quality_score = 0.90` and no new signals since last enrichment does NOT appear in the intel_queue after a hygiene scan.
4. Report generation uses appropriate tiers: EBR/QBR and Risk Report use Synthesis. SWOT and Account Health Review use Extraction. Verify: `grep -n "ModelTier" src-tauri/src/` for each report type.
5. Measured token reduction: after implementing the audit recommendations, the average daily PTY call count on a typical workday (5 meetings, 20 emails, 50 entities) is ≤ 40 calls. Before optimization (baseline from enrichment_log), record the count. After: verify reduction.

## Dependencies

- The I376 enrichment audit doc (`.docs/research/i376-enrichment-audit.md`) is the starting inventory of PTY call sites. Read it before writing the i435 audit.
- I408 (trigger score quality-gated enrichment, shipped in v0.13.7) must be active and working correctly. The 14-day staleness path should already be replaced — but verify it is actually dead code before removing it.
- `ModelTier` enum is defined in `src-tauri/src/types.rs` (or equivalent). The `Extraction` variant maps to Haiku; `Synthesis` maps to Sonnet or Opus. Confirm the mapping before changing call sites.
- The `enrichment_log` table provides the baseline call count. Pull the baseline before making changes so the reduction is measurable against a real number, not an estimate.

## Notes / Rationale

The I376 audit doc at `.docs/research/i376-enrichment-audit.md` is the starting inventory. The v0.13.7 quality scoring (I406–I408) already addresses the biggest optimization (don't re-enrich high-quality entities). I435 ensures the second-biggest optimization (model tier correctness) is also implemented and measured.

The audit doc (criterion 1) should be written before any code changes. Measure first, then change. The audit should be honest about which call sites have uncertain recommendations — "this prompt is 2,000 tokens; Haiku may lose nuance for relationship analysis" is a valid finding that keeps the tier at Synthesis.

The ≤ 40 call target (criterion 5) is a threshold for a typical workday, not a hard cap. The point is to ensure the optimization is real and measurable — if the baseline is already ≤ 40, document that and close the criterion. The audit exists to find and fix the cases where it isn't.

Quality gates: do not downgrade a model tier without verifying output quality on at least 3 real entities. The `enrichment_log` stores `output_length` which is a weak proxy for quality — better to manually compare a few enrichments before and after a tier downgrade.
