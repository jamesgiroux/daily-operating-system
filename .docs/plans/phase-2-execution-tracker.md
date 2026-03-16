# Phase 2 Execution Tracker (v1.0.0)

**Last updated:** 2026-03-10  
**Plan mode:** 3-lane execution  
**Policy:** No deferrals without documented follow-up issue, owner, and target wave/date.

## Wave sequencing (locked)

| Wave | Lane A | Lane B | Lane C |
|---|---|---|---|
| Wave 0 | Spec hardening + tracker + baseline audit | Spec hardening + tracker + baseline audit | Spec hardening + tracker + baseline audit |
| Wave 1 | I503 | I528 | I504 (+ I506 opportunistic) |
| Wave 2 | I508a | I508a | I508a |
| Wave 3 | I508b | I499 + I500 | I508c + I487 + I509 |
| Wave 4 | I505 | I505 | I505 |
| Wave 5 | I507 | I507 | I507 |

## Tracker matrix

| Issue | Depends on | Owner | Lane | Status | Validation gate |
|---|---|---|---|---|---|
| I503 | I511 | Unassigned | A | Done | ACs in `.docs/issues/i503.md` + cargo test + clippy |
| I528 | I511 | Unassigned | B | Done | ACs in `.docs/issues/i528.md` + revocation purge verification |
| I504 | None | Unassigned | C | Done | ACs in `.docs/issues/i504.md` + relationship persistence assertions |
| I508a | I503 | Unassigned | A/B/C | Done | Type coverage in Rust + TypeScript + DB persistence roundtrip verified |
| I508b | I508a | Unassigned | A | Done | Prompt schema coverage + local/remote fill behavior checks |
| I508c | I508a | Unassigned | C | Done | `semantic_gap_queries` dimension coverage and dedupe checks |
| I499 | I503 | Unassigned | B | Done | Health scoring ACs + sparse handling tests |
| I500 | I503 | Unassigned | B | Done | Org-score parsing ACs + fallback behavior |
| I487 | I528 | Unassigned | C | Done | New-only signal emission + purge compliance |
| I509 | I503, I508a | Unassigned | C | Done | Transcript interpretation + sentiment ACs |
| I505 | I528 | Unassigned | A/B/C | Done | Glean stakeholder ACs + validation gate |
| I506 | None | Unassigned | C | Done | Co-attendance ACs + deterministic ID dedup |
| I507 | I487, I504, I505 | Unassigned | A/B/C | Done | Scoped v1.0.0 feedback ACs |
| I536 (Phase 2a) | I511, I508a, I508b, I499, I503, I512 | Unassigned | Post-Phase-2 | Planned | Devtools scenario and seed integrity ACs |

## Baseline findings for delta implementation (historical snapshot)

1. I503 baseline
- **Resolved in Wave 1.** Foundation types are implemented and runtime now reads/writes structured `health` + `org_health` payloads.

2. I508a/b/c baseline
- **Resolved in Waves 2-3.** `IntelligenceJson` includes dimension fields and gap-query flow is dimension-aware for local ranking + Glean fan-out.

3. I504 baseline
- **Resolved in Wave 1.** Inferred relationship extraction, persistence, and reinforcement are active in enrichment flow.

4. I506 baseline
- Co-attendance linkage exists in hygiene and signal-rule hooks, giving a partial implementation base.
- Evidence: `src-tauri/src/hygiene.rs:1033`, `src-tauri/src/signals/rules.rs:810`.

5. I528 baseline
- **Resolved in Wave 1.** `data_lifecycle.rs`, `DataSource`, and `purge_source()` are implemented and wired.

## Execution notes

1. Every issue closes only when its own acceptance criteria are validated and cross-issue contracts remain green.
2. When partial code exists, complete by acceptance criteria instead of rewriting working behavior.
3. CI gates for each merge: cargo test, strict clippy, and issue-specific verification scripts/tests.

# Phase 2 Start Plan — Do Not Start with Full I508

## Summary
- Based on [Phase 2 graph](/Users/jamesgiroux/Documents/daily-operating-system/.docs/plans/v1.0.0.md) and [I508](/Users/jamesgiroux/Documents/daily-operating-system/.docs/issues/i508.md), the best start is not full I508.
- Correct kickoff: Wave 1 foundation in parallel (`I503`, `I528`, `I504`, optional `I506`), then `I508a`, then `I508b`.
- Parallelism mode locked to **3 lanes**.

## Implementation Plan
1. Wave 0 (spec and tracker hardening, same day)
- Normalize metadata mismatch: align I508 header/version with v1.0.0 Phase 2 plan.
- Keep I508 split explicit: `I508a` (types), `I508b` (prompt schema), `I508c` (semantic-gap evolution).
- Create a Phase 2 tracker matrix with columns: `issue`, `depends_on`, `owner`, `lane`, `status`, `validation_gate`.
- Record current partial-state findings so owners do delta work only (example: legacy `health_score/health_trend` still active in `io.rs`; `inferredRelationships` extraction exists but needs full path validation).

2. Wave 1 (parallel foundation)
- Lane A: `I503` end-to-end (Rust/TS types + DB migration + read/write compatibility).
- Lane B: `I528` end-to-end (`DataSource`, `purge_source()`, integration points).
- Lane C: `I504` implementation and tests; run `I506` as opportunistic side lane if capacity remains.
- Exit gate: all four issues merged or explicitly deferred with written rationale and follow-up issue.

3. Wave 2 (schema composition)
- Start `I508a` immediately after `I503` merges.
- Scope `I508a` to type/schema composition only; no prompt behavior changes.
- Include naming cleanup in this wave where schema/API identifiers are touched, to avoid carrying naming debt forward.

4. Wave 3 (parallel feature build)
- After `I508a`: run `I508b`, `I508c`, and `I509` in parallel.
- After `I503`: run `I499` and `I500` in parallel (independent of `I508b`).
- After `I528`: run `I487` in parallel.
- Critical path to monitor: `I503 -> I508a -> I508b` and `I528 -> I505`.

5. Wave 4 and Wave 5 (downstream integration)
- Wave 4: `I505` after `I528` and Glean validation gate pass.
- Wave 5: `I507` after `I487 + I504 + I505` (scoped-down v1.0.0 ACs only).

## Public Interface and Type Changes to Lock
- `IntelligenceJson` migrates from legacy scalar health fields to structured health model from `I503/I508a`.
- `semantic_gap_query` evolves to dimension-aware form in `I508c` without breaking existing callers during rollout (compat shim until all call sites migrate).
- `DataSource` and purge semantics from `I528` become mandatory source-of-truth for revocation-safe data handling.
- Relationship inference contract from `I504` must be stable before `I507` feedback routing work starts.

## Test Plan and Acceptance Gates
1. Per-issue gates
- Each issue must pass its own documented ACs plus `cargo test` and strict clippy gate used in the repo.
2. Cross-issue contract tests
- `I503 + I508a`: old intelligence rows load, migrate, and re-save without data loss.
- `I504 + I506 + I505`: relationship graph can contain AI, co-attendance, and Glean sources simultaneously with deterministic conflict behavior.
- `I528 + I487 + I505`: purge-on-revocation removes source-owned records and leaves non-owned data intact.
- `I499 + I500 + I508b + I509`: health computation and enrichment prompts produce expected structured outputs under sparse and rich data cases.
3. End-to-end Phase 2 validation
- Real-data flow: ingest transcript -> enrichment -> relationship updates -> health scoring -> briefing/account detail surfaces.
- Verify the Phase 2 outcome targets in [v1.0.0.md](/Users/jamesgiroux/Documents/daily-operating-system/.docs/plans/v1.0.0.md): health shown with confidence/context, 6 dimensions complete, multi-source relationships visible, glean signals integrated, sparse-data handling correct, correction loop reliable.

## Assumptions and Defaults
- Team operates in 3 parallel lanes.
- No additional gating from Phase 1 remains for Phase 2 kickoff.
- If any issue is discovered partially implemented, finish by AC compliance rather than rewriting.
- No deferrals without a documented follow-up issue, owner, and target wave/date.
