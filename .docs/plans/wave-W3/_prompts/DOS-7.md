**START** by reading `.docs/plans/wave-W3/_prompts/_common.md` from the repo cwd. That contains your role, constraints, deliverable, workflow, and template. Read it first.

Then, your specific assignment:

## Agent slot: W3-C — DOS-7

- **Title:** intelligence_claims persistence — design review vs deriving from provenance envelopes
- **Linear:** https://linear.app/a8c/issue/DOS-7 (fetch via linear MCP)
- **Output:** `.docs/plans/wave-W3/DOS-7-plan.md`
- **Wave plan section:** read `.docs/plans/v1.4.0-waves.md` §"Agent W3-C — DOS-7" (around lines 479-486)
- **Reviewers (3 stacked):** architect-reviewer + security-auditor + performance-engineer

## Things to be aware of

- DOS-7 is the heaviest ticket in W3 — schema for `intelligence_claims` + `claim_corroborations` + `claim_contradictions` + `agent_trust_ledger` + `claim_feedback` + `claim_repair_job`, plus 9-mechanism atomic backfill.
- The ticket was rewritten 2026-04-24 PM after Codex adversarial review (rounds 1+2). Memory-substrate amendments A–F (corroboration strength, surfacing lifecycle, contradiction branches, immutability allowlist, terminology, closed dups) all apply.
- DOS-308 cycle-2 absorbed implementation work into DOS-7's `commit_claim`. See `.docs/plans/v1.4.0-waves.md` lines 51-54 + DOS-308 cycle-2 amendment comment.
- Hard dependencies (must be merged before DOS-7 PR opens): DOS-308 (precondition: design contract + audit script + quarantine table), DOS-309 (already merged in W0), DOS-310 + DOS-311 (W1, already merged).
- Companion (parallel inside W3): DOS-301 derived-state writers — coordinate so W3-C ships schema + write path first, W3-D layers projection on top.

## Key code surfaces to grep

- `src-tauri/src/db/intelligence_feedback.rs` (DOS-308 territory after restructuring; DOS-7 builds on top)
- `src-tauri/src/services/claims.rs` — does not exist yet, this PR creates it
- `src-tauri/src/db/accounts.rs` — hard-delete CHAIN refactor (`set_team_member_role`, `remove_account_team_member`)
- `src-tauri/src/intel_queue.rs:2014` — existing `is_suppressed` caller
- `src-tauri/src/migrations/` — confirm migration numbering for new schema
- `.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md` — claims spec
- `.docs/decisions/0123-typed-claim-feedback-semantics.md`
- `.docs/decisions/0124-longitudinal-topic-threading.md`
- `.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md`
- `.docs/plans/storage-shape-review-2026-04-24.md` §Finding 1 (9 mechanisms inventory)
- `.docs/plans/claim-anatomy-review-2026-04-24.md`

## Coordination notes for §7

- W3-D (DOS-301) is your closest collaborator. The shared validators at `src-tauri/src/validators/json_columns.rs` are W3-D's territory; you call into them. Migration numbering must not collide.
- W3-E (DOS-294) consumes your `claim_feedback` skeleton table. Ship it.
- W3-F (DOS-296) adds `thread_id` column — coordinate migration ordering.
- W3-G (DOS-299) consumes your `source_asof` column.
- W3-H (DOS-300) adds `temporal_scope` + `sensitivity` columns — coordinate migration ordering.

Write the plan now.
