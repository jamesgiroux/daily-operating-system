PASS

No pre-code blockers found in V0.3.

Reasons:
- The read/write split now matches ADR-0102: `score_salience` is non-mutating, while `recompute_salience_for_claim` is a separate service write path with `ctx.check_mutation_allowed()` and non-Live no-write tests.
- Ability exposure is closed: no caller-supplied actor, `mcp_exposure = None`, `client_side_executable = false`, no SurfaceClient/MCP actor path.
- v270 is planned as idempotent/retry-safe with `IF NOT EXISTS`, idempotent seed upsert, and partial-rerun tests.
- Current substrate supports the plan: W1-A salience DTOs, `ClaimType::Recommendation`, v269 indexes, claim feedback/corroboration/contradiction, canonicalization decisions, and semantic evidence are present.
- Privacy constraints are explicit: typed rationales only, no raw claim text/source paths/prompts/output bodies, no LLM ranking path.

Non-blocking watch items:
- `read.recommendations` is inert while MCP/SurfaceClient exposure is closed, but reconcile with future `dailyos.read.recommendations` before promotion.
- Novelty must avoid `claim_semantic_evidence.original_text`; use durable score/count evidence or return `None`.
- Implement v270 validation so reruns verify table shape, weight count, and weight sum, not just DDL success.