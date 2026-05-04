# DOS-204 — Chapter-by-Chapter Enrichment: tl;dr

**Date:** 2026-04-23
**Full ADR:** `.docs/decisions/0122-chapter-enrichment.md`

---

## Current waste (measurable bounds)

Exact numbers require 2 weeks of instrumented telemetry that does not currently exist. From static analysis:

- **The system is already dimension-parallel.** I574 fans out 6 independent PTY threads per enrichment run. Wall-clock cost is bounded by the slowest thread (~240s), not the sum. The "all-or-nothing" problem was real in v1.0 but the parallelization substantially addressed it.
- **The entity-level 2-hour TTL already prevents most redundant runs.** For a 50-account portfolio at typical signal volumes, the overwhelming waste source is `ProactiveHygiene` overnight sweeps, not signal-triggered re-enrichment.
- **Estimated savings from Option B (per-chapter TTL):** 30–40% of PTY calls on stable accounts, driven by `commercial_financial` and `strategic_context` dimensions that change at quarterly frequency but are re-enriched on a 2-hour cadence today. These two dimensions alone account for 2 of 6 PTY calls per run.
- **We cannot confirm this without a per-dimension change-detection hash in the audit log.** This is the measurement gap.

---

## Recommended approach

**Do nothing for Wave 3. Add instrumentation now. Revisit after 2 weeks of data.**

The fan-out is already happening. The coherence risk of per-dimension staleness is real and not yet priced. Before building a dimension-level TTL system, instrument each dimension run with a SHA-256 change-detection hash and measure how often each dimension actually produces different output. If more than 40% of runs produce no change on `commercial_financial` or `strategic_context`, build Option B (per-chapter TTL with 14-day and 30-day TTLs respectively). Do not build signal-driven dimension targeting (Options C/D) without first proving Option B's ROI — the coherence guard that C/D requires is significant engineering work with unclear payoff.

---

## Gate implications for Wave 3

**Wave 3 is not blocked.** DOS-203 (Health & Outlook tab), DOS-15 (Glean leading-signal), and DOS-207 (Context tab schema) can all proceed on the current all-or-nothing enrichment model without wasted implementation work. Specific implications:

- **DOS-203:** `ChapterFreshness` correctly shows entity-level `enriched_at`. All chapters share one freshness timestamp under the all-or-nothing model, which is accurate. No rework risk regardless of which option is eventually chosen — Option B would add a `dimensionFreshnessJson` field to the DTO, and ChapterFreshness would be updated to read per-dimension timestamps; this is an additive change, not a rewrite.
- **DOS-15:** Already architecture-correct. It runs as a supplemental async pass (`enrich_leading_signals`) independently of the 6 main dimensions. Enrichment model changes do not affect DOS-15.
- **DOS-207:** New context_signals_json column gets enriched on the same all-or-nothing schedule. If DOS-207 identifies a dimension that warrants its own TTL, that is a post-telemetry trigger to revisit Option B.

---

## What you need to decide before Wave 3 kicks off

**One decision, one action:**

1. **Decision:** Accept the recommendation (ship Wave 3 tabs on current model; add instrumentation; revisit chapter TTLs post-telemetry). If you want dimension-level freshness in the v1.2.2 release, accept the M-sized Option B scope now and defer DOS-203/15/207 by ~1 sprint.

2. **Action (regardless of decision):** Add per-dimension output hash logging to `run_parallel_enrichment()` in `src-tauri/src/intel_queue.rs`. This is 5–10 lines, no schema change, and will produce the data needed to make Option B defensible (or prove it unnecessary). This should be done now regardless of which option is chosen.
