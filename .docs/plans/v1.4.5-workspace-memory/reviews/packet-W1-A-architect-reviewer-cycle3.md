# L0 Cycle 3 — W1-A DOS-463 — architect-reviewer

**Date:** 2026-05-20
**Reviewer:** compound-engineering:ce-architecture-strategist
**Verdict:** **BLOCK** (3rd recurrence of substrate-reinvention class)

## Substrate-grep sweep (architect re-audit, cycle 2 miss)

Verified §4 compiles in shape (imports + `unimplemented!()` + Send+Sync on unit-struct Null impls). BUT the substrate-grep surfaces a 3rd recurrence of the class codex consult caught in cycle 2.

## Findings

**F1 (BLOCK)** — **`SourceAttribution` name collision.** §4 declares `pub struct SourceAttribution { file_id, ingestion_run_id }`. Established 7-field `SourceAttribution { data_source, identifiers, observed_at, source_asof, evidence_weight, scoring_class, synthesis_marker }` ships at `provenance/source.rs:303` and is used by 15+ abilities + tests (`account_overview.rs:21`, `get_entity_context.rs:12`, `prepare_meeting/synthesis.rs:16`). Two same-named types across the workspace is exactly the duality §6 frets about for `SubjectRef`/`DataSource`. Fix: rename to `WorkspaceSourceAttribution` OR fold `file_id`/`ingestion_run_id` into existing `SourceAttribution.identifiers: Vec<SourceIdentifier>`.

**F2 (BLOCK)** — **`SourceType` enum mirrors `WorkspaceFileKind` 1:1.** §4 declares 7-variant `SourceType` byte-identical to `WorkspaceFileKind` at `provenance/source.rs:201`. §6 admits "matches W1-A `SourceType` mirror" — the tell. Either drop `SourceType` and consume `WorkspaceFileKind` directly, or document why mirror is required.

**F3 (CONDITIONAL)** — **`WorkspaceClaimProposal` vs `services::claims::ClaimProposal`.** §4 `WorkspaceClaimProposal` (9 fields) vs canonical `dailyos_lib::services::claims::ClaimProposal` (`services/claims.rs:74`, 17 fields). The two converge at `commit_claim` time. Either (a) state explicitly that `WorkspaceClaimProposal` is the pre-`commit_claim` extraction shape and document the `into_claim_proposal()` mapping responsibility, or (b) have extractors emit `services::claims::ClaimProposal` directly with `lifecycle_state` + `initial_source_reliability` in `metadata_json`. Same class as cycle-2 `TrustFactorInput`.

## SubjectRef duality

No new shape. W1-A's choice + maintenance ticket pointer stand. OK.

## Convergence

9 (cycle 1) → 6 (cycle 2) → 3 (cycle 3) same-class hits. Third cycle the substrate-reinvention class has fired. Per memory `feedback_zoom_out_for_class_pattern_in_l2_loop`, this is the structural-sweep moment:

> **Before V1.3, the implementing agent (or L0 author) does a single full-pass `grep -rn '^pub struct\|^pub enum\|^pub trait' src-tauri/abilities-runtime/src/abilities/provenance/ src-tauri/abilities-runtime/src/abilities/trust/ src-tauri/src/services/claims.rs` and reconciles every name in §4 contracts.rs against it**, citing each in §6 table either as "consumes" or "deliberate workspace-scoped wrapper of X for reason Y."

If V1.3 does the audit + renames + cites, cycle 4 should converge.
