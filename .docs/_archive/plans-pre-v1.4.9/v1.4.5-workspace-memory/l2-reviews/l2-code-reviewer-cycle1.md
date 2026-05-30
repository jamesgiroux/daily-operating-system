# L2 Cycle 1 — W1-A DOS-463 — code-reviewer (correctness)

**Date:** 2026-05-20
**Reviewer:** compound-engineering:ce-correctness-reviewer
**Diff:** `34a4cf37` (wave/v1.4.5-w1-stage1a vs dev @ 852a0118)
**Verdict:** **APPROVE** (3 path-α nits, route to Maintenance project)

## Findings (all path-α, non-blocking per `feedback_l2_path_alpha_to_maintenance_project`)

**F1 (low)** — `WorkspaceCategory::Other(known_slug)` round-trip asymmetry (`contracts.rs:99-110`). `Other("presentations").as_slug() == "presentations"` but `from_slug("presentations") == Some(Presentations)`, not `Some(Other("presentations"))`. Documented behavior (`from_slug` matches known variants first), but a stricter constructor would prevent the foot-gun. **Maintenance follow-up.**

**F2 (low)** — `device`/`inode` u64 → i64 conversion at W1-C persistence boundary (`lifecycle.rs:69-70`). W1-A defines `u64`; SQLite INTEGER is i64. W1-C's `runs.rs` will need `try_into()` not `as i64`. Linux/macOS inodes can theoretically exceed `i64::MAX`. **Flag for W1-C reviewers; not a W1-A AC violation.**

**F3 (low)** — `migrations_slice_max_version_is_at_least_251` test tautology (`tests/workspace_ingestion_unit.rs:242-261`). Doesn't actually verify `MIGRATIONS` slice — only `include_str!`s the SQL files. The earlier `migrations_v250_v251_apply_against_in_memory_db_and_register_columns` (line 194) provides the real coverage. **Rename or strengthen as maintenance.**

## AC coverage verified

All §9 done-when items verified line-by-line:
- v250 + v251 wired in `migrations.rs:935-942`; MAX = 251.
- Alphabetical `mod.rs` matches gate.
- `contracts.rs` + `lifecycle.rs` substantively filled; 8 placeholders with `//!` only.
- All shared types `pub` + externally importable (`contracts_import` test).
- `WorkspaceClaimProposal.source_attribution` is canonical (`contracts.rs:171`).
- `Extractor::extract` takes `WorkspaceFileKind` (line 197).
- `Send + Sync` traits + Null defaults compile and assert (`workspace_ingestion_unit.rs:124-127`).
- `LifecycleState` 7 variants with snake_case serde (line 19 test).
- `initial_source_reliability: f64` (line 186 contracts), not reinvented enum.
- `no_substrate_reinvention.rs` gate passes.
- `services/mod.rs` carries new line.

**Verdict: APPROVE.** Three findings route to Codebase Maintenance project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb` per `feedback_l2_path_alpha_to_maintenance_project`.
