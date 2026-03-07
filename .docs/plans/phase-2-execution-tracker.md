# Phase 2 Execution Tracker (v1.0.0)

**Last updated:** 2026-03-07  
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
| I503 | I511 | Unassigned | A | Planned | ACs in `.docs/issues/i503.md` + cargo test + clippy |
| I528 | I511 | Unassigned | B | Planned | ACs in `.docs/issues/i528.md` + revocation purge verification |
| I504 | None | Unassigned | C | Planned | ACs in `.docs/issues/i504.md` + relationship persistence assertions |
| I506 | None | Unassigned | C (opportunistic) | Planned | ACs in `.docs/issues/i506.md` + no overwrite regressions |
| I508a | I503 | Unassigned | A/B/C | Planned | Type coverage complete in Rust + TypeScript |
| I508b | I508a | Unassigned | A | Planned | Prompt schema coverage + local/remote fill behavior checks |
| I508c | I508a | Unassigned | C | Planned | `semantic_gap_queries` dimension coverage and dedupe checks |
| I499 | I503 | Unassigned | B | Planned | Health scoring ACs + sparse handling tests |
| I500 | I503 | Unassigned | B | Planned | Org-score parsing ACs + fallback behavior |
| I487 | I528 | Unassigned | C | Planned | New-only signal emission + purge compliance |
| I509 | I503, I508a | Unassigned | C | Planned | Transcript interpretation + sentiment ACs |
| I505 | I528 | Unassigned | A/B/C | Planned | Glean stakeholder ACs + validation gate |
| I507 | I487, I504, I505 | Unassigned | A/B/C | Planned | Scoped v1.0.0 feedback ACs |
| I536 (Phase 2a) | I511, I508a, I508b, I499, I503, I512 | Unassigned | Post-Phase-2 | Planned | Devtools scenario and seed integrity ACs |

## Baseline findings for delta implementation

1. I503 baseline
- Missing foundation types in current code: no `AccountHealth`, `RelationshipDimensions`, `OrgHealthData` definitions found.
- Current intelligence model still uses legacy scalar health fields (`health_score`, `health_trend`) in runtime read/write paths.
- Evidence: `src-tauri/src/intelligence/io.rs:273`, `src-tauri/src/intelligence/io.rs:277`, `src-tauri/src/intelligence/prompts.rs:1680`, `src-tauri/src/intelligence/prompts.rs:1683`.

2. I508a/b/c baseline
- `IntelligenceJson` is present but not yet migrated to the full dimensioned schema from I508.
- Gap-query path is singular string API (`semantic_gap_query`) and not multi-query/dimension-aware.
- Evidence: `src-tauri/src/intelligence/io.rs:213`, `src-tauri/src/intelligence/prompts.rs:501`, `src-tauri/src/intelligence/prompts.rs:1111`.

3. I504 baseline
- Relationship extraction helper exists (`extract_inferred_relationships`), and `person_relationships` persistence primitives exist.
- Queue flow notes indicate inferred relationship write path is not fully active.
- Evidence: `src-tauri/src/intelligence/prompts.rs:1963`, `src-tauri/src/db/person_relationships.rs:130`, `src-tauri/src/intel_queue.rs:789`.

4. I506 baseline
- Co-attendance linkage exists in hygiene and signal-rule hooks, giving a partial implementation base.
- Evidence: `src-tauri/src/hygiene.rs:1033`, `src-tauri/src/signals/rules.rs:810`.

5. I528 baseline
- No `data_lifecycle.rs`, `DataSource` enum, or `purge_source()` implementation found in `src-tauri/src/db`.
- Evidence: no symbol matches for `enum DataSource` and `purge_source` in `src-tauri/src`.

## Execution notes

1. Every issue closes only when its own acceptance criteria are validated and cross-issue contracts remain green.
2. When partial code exists, complete by acceptance criteria instead of rewriting working behavior.
3. CI gates for each merge: cargo test, strict clippy, and issue-specific verification scripts/tests.
