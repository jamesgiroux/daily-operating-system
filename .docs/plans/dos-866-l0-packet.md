# DOS-866 L0 Packet v2 (post-review fold: K-in + feasibility + codex challenge)

Prior art (MANDATORY context): `docs/solutions/architecture-patterns/db-lock-storm-class-2026-05-27.md` (names this exact fn as W1-A decomposition target) and `.docs/plans/db-throughput-architecture.html` (W1-A design: decompose then chunk). The two starvation incidents (2026-06-10/11, DOS-866) are the measurement evidence that opens the W1-A gate. ADR-0133 §4: chunking is caller-side; atomicity shape belongs to ADR-0104 → small ADR amendment required (deliverable 4).

## D1 — Rank-then-cap at the PREPARE step (not glean parse)
Choke point: `intel_queue.rs` prepare step (~2540–2624), where `final_intel` + `projection_intelligence` are already filtered (`filter_suppressed_risks_and_wins` 2552/2610) before `PreparedEnrichment` (2618). This is post-reconcile and producer-agnostic (covers legacy + PTY paths; glean parse is the WRONG location — `reconcile_enrichment` re-merges DB snapshot at glean_provider.rs:1217/1232).
- Cap per dimension: const `MAX_ITEMS_PER_DIMENSION = 25` (risks, wins, etc.).
- RANK before cap: severity/urgency desc, then itemSource confidence desc, then sourcedAt recency; stable order fallback. Tail-drop without ranking is a correctness bug.
- Overflow → ONE `record_pipeline_failure("enrichment_parse_overflow", …)` with per-dimension dropped counts.
- AC1: oversized fixture with a high-severity item at position 300 → retained; low-value item dropped; exactly cap persisted; one pipeline-failure row.

## D2 — Phased persist (W1-A) with ordering + idempotency contract
Current single occupancy: `intel_queue.rs:2722–2743` → one `with_transaction` → `upsert_assessment_from_enrichment_in_active_transaction` (`services/intelligence.rs:2047`) which contains the per-claim commits (line 2070).
Restructure into phases, each its own db-service task so interactive commits interleave:
1. Phase A: cleared-dimension withdrawal (svc/intelligence.rs:2063).
2. Phase B×k: claim batches (~20/batch) via `commit_claim_shaped_intelligence_projection`.
3. Phase C (finalizer, ONLY after last batch): `withdraw_refreshed_projection_claims` (2077; its affected_subjects feed recompute 2099–2105), entity_assessment row write, legacy snapshot (2085), signal emit (2087), objectives reconcile (2110), recompute enqueue.
- Keep the single-tx variant intact for other callers (`upsert_assessment_from_enrichment` 2013, trust-recompute, ~12 test sites). New phased path used by intel_queue persist only.
- Idempotency/crash contract (replaces W1-A durable cursor; document in ADR amendment): per-field-path supersede semantics make claim commits re-runnable — a crash between batches leaves committed claims that the NEXT enrichment run supersedes/withdraws via Phase A+C; the surface never reads a partial assessment because the assessment row + signals + recompute land only in the finalizer.
- AC2: persist of N claims = ceil(N/20)+2 tasks; injected concurrent task interleaves between batches.
- AC3 (crash): injected failure after batch k → no assessment-row change, no signals; re-run converges to identical final state (property-style test).

## D3 — Over-cap subject cleanup (maintenance command, dry-run first)
- Reuse `withdraw_generated_projection_claims_for_field_path_roots_in_tx` family (services/claims.rs:11433 producer variant) + `withdraw_claim` (7321) — never raw SQL, never user-authored/corrected/corroborated claims (existing generated-only filters).
- Selection policy = SAME ranking as D1 (keep top-N per dimension), scoped per producer/source — not "oldest", which fights recompute.
- MUST also trim the legacy `entity_intelligence` snapshot dimensions to the retained set (else reconcile re-merges the 624 at next enrichment; glean_provider.rs:589–593 readback).
- Batched writes (D2 shape), `dry_run: bool` returning counts before apply (ADR-0103 pattern), audit counts on apply. Withdrawal (not dormancy) chosen: these are pathological generated claims, not stale facts — decision named against ADR-0126.
- AC4: fixture subject with 600 generated claims + bloated snapshot → dry-run reports, apply withdraws to ≤cap + trims snapshot; cleanup→recompute→cleanup is stable (no oscillation); user-authored claims untouched.

## D4 — ADR-0104 amendment (small doc)
`.docs/decisions/` amendment: enrichment persist atomicity relaxes from whole-enrichment to phase-ordered batches with finalizer barrier + supersede-based re-run idempotency. References ADR-0133 §4, W1-A, DOS-866 incidents.

## D5 — Quiet mode: VERIFY existing, don't build
`DAILYOS_DISABLE_BACKGROUND_WORKERS` / `_INTEL` (pty.rs:89–90, lib.rs:581–584 else-block wrapping all 12 workers + early backfill 330–333) already exists. Deliverable: verify coverage (one test or startup-log assertion), add per-worker skip log line if missing. NO new env var. Not a substitute for W1-B presence-aware admission (separate, unaffected).

## E2E AC (codex challenge #5)
AC5: pathological fixture (~1,300 claims) through the full persist while a composition commit is queued → composition completes within bounded tasks (not after all batches), final surface coherent (assessment+claims consistent), gates green (clippy -D warnings, cargo test, tsc).

## Execution
Codex sub-agent, isolated worktree branched from current dos-852 HEAD. All mutations via services/. No ephemeral issue refs in code comments.
