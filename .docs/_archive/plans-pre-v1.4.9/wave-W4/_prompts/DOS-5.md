**START** by reading `.docs/plans/wave-W4/_prompts/_common.md` from the repo cwd. That contains your role, constraints, deliverable, workflow, and template. Read it first.

Then, your specific assignment:

## Agent slot: W4-A — DOS-5 + DOS-326

- **Title:** Trust Compiler + DOS-326 contamination fold-in
- **Linear:** https://linear.app/a8c/issue/DOS-5 and https://linear.app/a8c/issue/DOS-326 (fetch both via linear MCP)
- **Output:** `.docs/plans/wave-W4/DOS-5-plan.md`
- **Wave plan section:** `.docs/plans/v1.4.0-waves.md` §"Agent W4-A — DOS-5 Trust Compiler + DOS-326 contamination fold-in"
- **Reviewers (3 stacked):** architect-reviewer + performance-engineer + codex-consult

## Things to be aware of

- 5 canonical trust factors per ADR-0114 R1.4. `subject_fit_confidence` is a composer-local helper, NOT a 6th canonical factor. `unknown_timestamp_penalty` is NOT a separate factor — it folds as an input modifier on `freshness_weight` via `FreshnessContext { timestamp_known: bool }`.
- DOS-326: retire `src-tauri/src/intelligence/contamination.rs` entirely. Its cross-entity-coherence check becomes one factor inside the Trust Compiler — not a gate, not a separate module.
- Hard depends on W3-B (Provenance envelope / `SourceAttribution` shape), W3-C (claim rows + `claim_corroborations.strength`), W3-G (`source_asof` column + `FreshnessContext`), W3-H (`temporal_scope` + `sensitivity` columns). Read those W3 L0 plans to understand the upstream contracts.
- The corroboration formula (noisy-OR) has an open question filed on DOS-7: the formula may saturate wrong. Flag this in §10 and note that Trust Compiler tests must be co-authored with W3-C author once the formula is resolved.
- Trust Compiler must NOT be called from W3-C — W3-C creates the substrate, W4-A consumes it. No circular dep.
- Property test on geometric-mean math must be clippy clean. Floating-point gotchas: use `f64`, avoid `.sqrt()` on negative after subtraction.

## Files owned

- `src-tauri/src/abilities/trust/` (new directory + mod.rs)
- `FreshnessContext` struct (new, in trust/ or abilities/common/)
- Retirement of `src-tauri/src/intelligence/contamination.rs` (delete file, replace call sites with Trust Compiler cross-entity-coherence factor)
- ADR-0114 R1.3 amendment for `timestamp_known: bool`

## Key code surfaces to grep

- `src-tauri/src/intelligence/contamination.rs` — the module being retired; find all call sites
- `src-tauri/src/intelligence/io.rs` — existing trust fields, `effective_confidence`
- `.docs/decisions/0114-scoring-unification.md` — ADR-0114 R1.4 (5 canonical factors) + R1.3 (FreshnessContext)
- `.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md` — claim sources feeding trust
- `.docs/plans/wave-W3/DOS-7-plan.md` — W3-C contract: `claim_corroborations.strength`, `agent_trust_ledger`
- `.docs/plans/wave-W3/DOS-211-plan.md` — W3-B contract: `SourceAttribution` shape
- `.docs/plans/wave-W3/DOS-299-plan.md` — W3-G contract: `source_asof`, `FreshnessContext` input
- `.docs/plans/wave-W3/DOS-300-plan.md` — W3-H contract: `temporal_scope`, `sensitivity`
- `src-tauri/src/intel_queue.rs` — current trust score writes to understand what moves

## Specific design decisions to record in §3

- Trust Compiler as pure function: `compile_trust(claims: &[ClaimRow], context: TrustContext) -> TrustScore` — no DB writes inside compiler, side-effect free
- Where trust score is persisted (W3-C's `intelligence_claims.trust_score` + `trust_computed_at` — Trust Compiler only reads/writes via that contract)
- Geometric mean vs arithmetic mean for factor aggregation — confirm ADR-0114 pick
- Cross-entity-coherence factor: exact input shape replacing the `contamination.rs` check

## Coordination notes for §7

- W3-C (DOS-7) writes `trust_score` column — Trust Compiler reads from `claim_corroborations` and writes back via the W3-C service layer, not direct SQL.
- W3-G (DOS-299) owns `FreshnessContext` input population — if W3-G lands first, Trust Compiler imports its type; if W4-A lands first, define the type here and W3-G adopts it.
- W6-A (DOS-320) is the consumer of trust bands for rendering. Trust Compiler must emit a `TrustBand` enum (e.g., `High/Medium/Low/Unscored`) consumable by W6-A.

Write the plan now.
