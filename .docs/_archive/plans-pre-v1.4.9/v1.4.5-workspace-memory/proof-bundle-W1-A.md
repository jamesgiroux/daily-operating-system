# v1.4.5 W1-A — DOS-463 — proof bundle

**Issue:** [DOS-463 — Define workspace source lifecycle and ownership model](https://linear.app/a8c/issue/DOS-463)
**Branch:** `wave/v1.4.5-w1-stage1a` from `dev` @ `852a0118`
**L0 packet:** [`.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-A-DOS-463.md`](./L0-packet-W1-A-DOS-463.md) (V1.4, unanimous APPROVE after 6 cycles)
**L1 commit:** `34a4cf37`
**L0 packet commit:** `630fbad0`

## L-levels cleared

| Level | Status | Evidence |
|---|---|---|
| **L0 (Plan)** | UNANIMOUS APPROVE (cycle 6) | architect cycle 5 + codex challenge cycle 6 + codex consult cycle 6. Reviews in `reviews/packet-W1-A-*-cycle*.md`. |
| **L1 (Self)** | PASS | 14 W1-A tests green; `cargo clippy --lib -- -D warnings` green; commit `34a4cf37` carries IL gate answers + L2-status declaration. |
| **L2 (Diff)** | UNANIMOUS APPROVE (cycle 1) | architect-reviewer + code-reviewer + codex review all APPROVE against AC §9. 5 path-α findings consolidated to one Codebase Maintenance ticket. Reviews in `l2-reviews/`. |

## Acceptance criteria — verified line-by-line

| AC item (L0 §9) | Evidence |
|---|---|
| Migration slots **v250 + v251** used; `MIGRATIONS` max version is 251 | `src/migrations.rs:935-942` registers both; `migrations_v250_v251_apply_against_in_memory_db_and_register_columns` test green |
| Submodule tree present (alphabetical `mod.rs` + `contracts.rs` + `lifecycle.rs` + 8 `//!`-only placeholders) | `tests/workspace_ingestion_mod_rs_shape.rs` green; `git show 34a4cf37 -- src-tauri/src/services/workspace_ingestion/` shows 11 files |
| Shared types `pub` + externally importable | `tests/workspace_ingestion_contracts_import.rs` green (uses `dailyos_lib::services::workspace_ingestion::contracts::*` external import path) |
| `WorkspaceClaimProposal.source_attribution` is canonical `SourceAttribution` (not reinvented) | `contracts.rs:171`; canonical type re-exported at line 30; `tests/workspace_ingestion_no_substrate_reinvention.rs` green |
| `Extractor::extract` takes `source_type: WorkspaceFileKind` (canonical, not mirror) | `contracts.rs:197` |
| `Extractor` + `SignalEmitter` traits with `Send + Sync` + Null defaults compile | `null_extractor_is_send_sync_and_returns_empty` + `null_signal_emitter_methods_are_noops_callable_through_dyn` tests green |
| `LifecycleState` exactly 7 variants with canonical snake_case serde | `lifecycle_state_has_exactly_seven_variants_with_canonical_serde_strings` test green |
| `initial_source_reliability: f64` (NOT a reinvented trust enum) | `contracts.rs:186` |
| CI grep gate `tests/no_substrate_reinvention.rs` passes | `no_substrate_reinvention_in_workspace_ingestion` test green |
| `src/services/mod.rs` carries `pub mod workspace_ingestion;` | `git show 34a4cf37 -- src-tauri/src/services/mod.rs` shows single-line addition |
| `cargo clippy --lib -- -D warnings && cargo test` green | Both verified locally and independently by codex review |
| IL gate items 1–5 answered in commit message | `git show 34a4cf37` commit body has IL gate section |
| `L2-status` declared in commit message | `L2-status: not-run-acknowledged` in commit `34a4cf37`; L2 now passed (see L2 verdicts) |

## L2 path-α findings (route to Codebase Maintenance)

Five findings, none blocking. Consolidated maintenance ticket targets project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`. **NOTE:** Linear API returned 502 upstream_unavailable when filing the consolidated ticket (2026-05-20). Retry needed. Findings enumerated for reference:

1. **F1** — `WorkspaceCategory::Other(known_slug)` round-trip asymmetry (`contracts.rs:99-110`).
2. **F2** — u64 → i64 inode/device cast safety at W1-C persistence boundary (`lifecycle.rs:69-70`).
3. **F3** — `migrations_slice_max_version_is_at_least_251` test tautology (`tests/workspace_ingestion_unit.rs:241`).
4. **F4** — Missing trybuild compile-fail tests (L0 §8 mentioned them; not added).
5. **F5** — Substrate-reinvention grep gate scope (catches `pub struct/enum/trait` only).

## Class-pattern findings for K-out at W1 retro close

1. **W0 reuse-audit ceiling check** must grep `MIGRATIONS` slice version literals, not the migrations subdirectory listing. The audit's "v178 ceiling" claim missed the v240 leap; cycle 11 of L0 forced a slot renumber v200→v250.
2. **Substrate-reinvention is a recurring L0 class.** Fired 3 cycles in a row during W1-A L0 (`TrustFactorInput` cycle 2; `SourceAttribution` + `SourceType` + `ClaimProposal` cycle 3). Structural prevention via the CI grep gate (`tests/no_substrate_reinvention.rs`) shipped in this implementation. Future lanes inherit the gate.
3. **Wave-plan amendment hygiene:** substrate-promoting amendments need K-in grep against canonical `abilities-runtime` before naming new types. The wave plan's original cycle 6 amendment promoted `SourceType` without K-in checking; cycle 12 dropped it.

## Wave-plan amendments folded during L0 (in `.docs/plans/v1.4.5-waves.md`)

- **Cycle 11 (slot renumber):** v200–v219 → **v250–v269** above the live-dev v240 ceiling.
- **Cycle 12 (substrate-consumption sweep):** dropped `SourceType` mirror + local `SourceAttribution` reinvention. Per-lane references (W1-A/W2-A/W3-A) updated.

## Handoff for downstream lanes

W1-A is complete and unblocks:
- **W1-B (DOS-464)** — `registry.rs` + `WorkspaceCategoryRegistry`. Slot **v252**. Security-annotated (`/cso`). Compiles against W1-A's `contracts::{FileIdentity, RejectionReason, WorkspaceCategory}`.
- **W1-C (DOS-465)** — `runs.rs` + `link.rs`. Slots **v253 + v254**. Compiles against W1-A's `SignalEmitter` trait + `NullSignalEmitter` default. **Reviewers: flag u64→i64 inode/device cast at the persistence boundary per L2 path-α F2.**

No lane creates files in `services/workspace_ingestion/` beyond the placeholders W1-A pre-created. No lane edits `mod.rs`. No lane reinvents `SourceAttribution`, `SourceType`, `TrustFactorInput`, `SourceIdentifier`, `ClaimProposal`, or `Provenance` (CI grep gate enforces).
