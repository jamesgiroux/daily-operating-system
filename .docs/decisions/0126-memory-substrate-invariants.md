# ADR-0126 — Memory Substrate Invariants

**Status:** Proposed
**Date:** 2026-04-24
**Supersedes:** N/A
**Superseded by:** N/A

## Context

The v1.4.0 claim substrate (DOS-7 `intelligence_claims`, DOS-5 Trust Compiler, DOS-10 freshness decay, DOS-211 Provenance, DOS-212 DataSource taxonomy, DOS-213 prompt fingerprinting, DOS-263 invalidation_jobs, DOS-265 claim_edges, DOS-280 canonicalization) is large, interlocking, and built by multiple contributors over multiple releases. Without a unifying set of invariants, each contributor independently makes small decisions that drift apart over time. The result: two code paths that could in principle do the same thing end up doing subtly different things, ghost-resurrection bugs re-emerge after they were fixed, and every new feature ends up re-solving the "how do I store this?" problem.

A memory substrate that CSMs trust over many quarters needs a few structural guarantees that no single ticket enforces on its own. This ADR lists them so future tickets can point here instead of re-asserting them piecemeal.

The spirit is also aspirational: we looked at neuroscience's memory mechanisms (Hebbian strengthening, synaptic pruning, offline consolidation, schema-based assimilation, reconsolidation on retrieval) as a source of structural ideas. We adopt the mechanisms. We explicitly reject the failure modes the brain has because of its biological constraints — confabulation, irreversible forgetting, retrieval-induced distortion, schema-suppression of anomalies, one-winner contradiction resolution. Software has different constraints and can do better. This ADR locks down which side of that line the substrate sits on.

## Decision

The memory substrate is governed by the following invariants. Each is enforced in code (CI lint or test) where possible. Each feature ticket either honors these invariants or proposes an amendment to this ADR.

### 1. Immutable assertion core

Once a claim is written, its `text`, `claim_type`, `subject_ref`, `source_asof`, and `created_at` are immutable. No UPDATE of those columns from anywhere in the codebase except migration paths.

Derived columns (`trust_score`, `trust_computed_at`, `trust_version`, `claim_state`, `surfacing_state`, `demotion_reason`, `reactivated_at`, `superseded_by`) are allowed to mutate, routed exclusively through `services/claims.rs`.

Enforcement: CI lint allowlist per column (DOS-7 amendment D).

### 2. Additive weight updates

Corroboration strength accumulates via `UPDATE claim_corroborations SET strength = ?, reinforcement_count = ?, last_reinforced_at = ?` — the one exception to "append-only" in the corroboration layer, allowed because the underlying fact (same source corroborating the same claim again) is logically a reinforcement of an existing edge, not a new fact.

All other weight/trust changes produce new rows (signal events, feedback rows, trust recompute events), never destructive rewrites of the previous state.

Enforcement: write-path restricted to `services/claims.rs::record_corroboration`; all other UPDATEs on `claim_corroborations` rejected by CI lint.

### 3. Dormancy is recoverable, deletion is not

`DELETE FROM intelligence_claims` is forbidden outside user-data-deletion flows (GDPR/CCPA). Same for `claim_corroborations` and `claim_contradictions`.

Stale or unreinforced claims transition to `surfacing_state='dormant'`. Dormant claims stay in storage, queryable on opt-in, reactivatable on new evidence. The lifecycle is two-column (`claim_state` = user intent; `surfacing_state` = system rendering) to prevent ghost-resurrection bugs that come from conflating the two.

Enforcement: CI lint grep for `DELETE FROM intelligence_claims|claim_corroborations|claim_contradictions` outside the allowed paths.

### 4. Retrieval is additive, not distortive

Passive user engagement (view, dwell, skip) produces events that affect ranking and surfacing. It does not modify trust, does not rewrite claim text, does not alter provenance.

Explicit user judgment (correction, tombstone, confirmation) flows through `claim_feedback` per ADR-0123 typed feedback semantics. Explicit judgment affects trust via DOS-5's `fb` factor.

The two are not the same thing and never get conflated at the storage layer. Inflating "what is true" based on "what the user paid attention to" is a bug, not a feature.

### 5. Schema-based compression is allowed, anomaly suppression is not

When a new signal fits an existing claim schema (ADR-0125 claim type registry), it assimilates: strengthens corroborations, bumps freshness, does not produce a new surface notification.

When a new signal does NOT fit the schema (missing required claim, stale past typical freshness, unexpected presence, cadence break, out-of-range value), it surfaces as a deviation. Deviations are the foreground of the briefing, not the background.

Invariant: the schema assimilation pass is allowed to lower the weight of routine background claims. It is NOT allowed to lower the weight of any claim the deviation detector has flagged. Structural, not best-effort.

Enforcement: precedence check in the consolidation + trust compiler aggregation paths; unit test asserts deviation-flagged claims never lose weight via routine compression.

### 6. Contradiction is a fork, not a winner-picking

When a new signal directly contradicts an existing claim (same subject + claim_type, incompatible value, existing aggregate strength ≥ 0.4), the substrate creates a `claim_contradictions` row with `branch_kind='contradiction'`. Both claims stay active and queryable. Neither is authoritative for default display until reconciled.

Reconciliation happens via one of four explicit kinds (`user_picked_winner`, `evidence_converged`, `merged_as_qualified`, `both_dormant`). Auto-supersession for time-varying claim types (renewal_date, contract_end, ARR) records `reconciliation_kind='evidence_converged'` and uses ADR-0113's supersede path — no new supersession mechanism.

The brain picks a winner and forgets the loser. We fork and keep both lines, because we can.

### 7. Diversity of sources beats volume

Ten Gong transcripts of the same meeting are not ten sources. Three independent sources (Salesforce + Zendesk + Gong) asserting the same thing is stronger evidence than a hundred same-source repetitions. The corroboration math reflects this: each `(claim_id, data_source)` edge accumulates strength with diminishing returns, and claim-level confidence aggregates via noisy-OR over independent edges.

Same-source reinforcement: `strength += log(n+2)/log(n+1)` — approaches 1.0 asymptotically.
Independent-source aggregate: `1 - product(1 - strength_i)` — combines across distinct sources.

Enforced by DOS-7 amendment A and DOS-5 corr-factor amendment.

### 8. Homeostatic budgets by category, not by entity

An entity-wide scalar "attention budget" that demotes the lowest-weight claim when exceeded is wrong: it buries rare-but-important claims (first signal of a deviation) under common-but-routine ones (weekly status updates).

Budgets, if used, are per claim_type + sensitivity + surface. A critical-sensitivity health claim does not compete for the same attention slot as a routine adoption-metric. The claim type registry (ADR-0125) carries the per-type budget policy, not a global constant.

### 9. Engagement affects ranking; feedback affects trust

Two separate streams, two separate tables, never fused.

- Engagement events (view, dwell, expand, scroll-past, soft-dismiss) → `claim_engagement_events` → ranking input for the surfacing layer.
- Feedback events (correction, confirmation, tombstone) → `claim_feedback` (ADR-0123 typed) → DOS-5's `fb` factor → trust.

A claim the user ignores stays exactly as true as the evidence says it is. A claim the user corrects becomes a different claim, with the correction as a new source.

### 10. Substrate vocabulary is canonical

Drift in terminology across tickets is drift in implementation. The substrate uses a fixed vocabulary:

- **claim** — immutable assertion
- **corroboration** — evidence row supporting a claim, with strength
- **contradiction** — divergent claim on same subject + type, with reconciliation state
- **source** — a DataSource variant (DOS-212) with `source_asof`
- **edge** — claim-derived relationship in the semantic graph (DOS-265), distinct from corroboration evidence
- **signal** — runtime event per ADR-0115 emitted on substrate writes
- **engagement** — passive telemetry, ranking-only
- **feedback** — explicit user judgment, trust-affecting

Tickets that invent new terms for these concepts get rejected in review.

## Anti-patterns explicitly rejected

Each of these looked reasonable in isolation and failed review. Listing them here so future tickets don't re-propose:

- **`claim_source_edges` as a separate table alongside `claim_corroborations`.** Three edge tables is two too many. Evidence edges go on `claim_corroborations`.
- **`dormant_at` as a standalone column alongside `claim_state`.** Two lifecycle models → ghost-resurrection. Dormancy is a value in `claim_state` and a separate `surfacing_state` column.
- **`claim_branches` as a separate table alongside `claim_contradictions`.** The fork is a contradiction row with branch/reconciliation fields, not a parallel table.
- **Nightly consolidation as a bespoke `tokio::async_runtime::spawn` task.** Consolidation is an `invalidation_jobs.operation` (DOS-263) with deterministic idempotency keys.
- **`entity_graph_version` triggers for claim invalidation.** Singleton counter thrashing. Use DOS-310 per-entity invalidation.
- **Engagement-as-trust (`effective_strength * (1 + engagement_bonus)`).** Interesting or repeatedly-viewed claims becoming "more true" is a bug. Engagement affects ranking, not trust.
- **Hard-coded thresholds (`HOMEOSTATIC_BUDGET=100`, `DORMANT_THRESHOLD=0.15`, `STALE_WINDOW=180`).** Policy lives in the ADR-0125 claim type registry or role preset config (DOS-178), never magic numbers in code.
- **Homeostatic eviction by entity-wide weight.** Budget is by category + sensitivity + surface, not by entity-wide scalar.
- **`user_dismissed_soft` as a surfacing demotion reason.** User intent is a tombstone (`claim_state='tombstoned'`), not a system surfacing decision.
- **New claim-mutation APIs bypassing `services/claims.rs`.** All claim writes through the service. `intel_queue::write_enrichment_results` calls `commit_claim`, does not mint its own path.

## Consequences

**What this makes easier:**
- Future tickets know the invariants without re-reading every prior ticket.
- Review has a concrete list to check against: "does this ticket preserve the invariants in ADR-0126? If not, which one does it amend?"
- CI lints implementing these invariants catch drift automatically.
- The substrate accumulates structural properties over releases instead of accumulating exceptions.

**What this makes harder:**
- New features that would be easy to slam into a single column on `intelligence_claims` have to route through the right substrate layer. Some features will take slightly longer to ship as a result; the tradeoff is that the substrate doesn't decay over time.

**Who owns this ADR:**
- Schema shape: DOS-7.
- Trust math: DOS-5.
- Claim types + registry: ADR-0125.
- Canonicalization: DOS-280.
- Invalidation jobs: DOS-263.
- Semantic edges: DOS-265.

Amendments to this ADR happen when a feature ticket proposes a principled exception. Amendments are numbered and dated, not silently merged.

## References

- Codex adversarial review 2026-04-24 PM, DOS-312 bundle (the trigger).
- DOS-7 (intelligence_claims — amendments A, B, C, D, E trace back here).
- DOS-5 (Trust Compiler — corr factor amendment trace back here).
- DOS-125 (Claim anatomy, temporal_scope, sensitivity, claim type registry).
- DOS-113 (Claim ledger + supersede semantics, tombstone PRE-GATE).
- DOS-123 (Typed claim feedback semantics).
- Original framing: DailyOS Linear DOS-312 (closed; converted to this ADR).

## Amendments

None yet.
