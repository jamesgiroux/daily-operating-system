# L0 Cycle 4 — W1-A DOS-463 — architect-reviewer

**Date:** 2026-05-20
**Reviewer:** compound-engineering:ce-architecture-strategist
**Verdict:** **APPROVE — ship V1.3 to L1**

## Cycle-3 findings → V1.3 resolution

- **F1 `SourceAttribution` reinvention** → resolved. §4 imports canonical from `provenance/source.rs:303`; `ingestion_run_id` extracted to top-level field.
- **F2 `SourceType` mirror** → resolved. §4 imports `WorkspaceFileKind` directly; `Extractor::extract(... source_type: WorkspaceFileKind)`.
- **F3 `WorkspaceClaimProposal` vs `ClaimProposal` ambiguity** → resolved. §4 doc-comment defines extraction-vs-commit boundary; `into_commit(actor) -> ClaimProposal` ownership assigned to W3-A.

## Final substrate-grep on V1.3 §4 (zero collisions)

Ran `pub (struct|enum|trait) <name>` over `src-tauri/` excluding workspace_ingestion + tests for all 12 V1.3-introduced names: `FileIdentity`, `WorkspaceCategory`, `RejectionReason`, `WorkspaceClaimProposal`, `Extractor`, `SignalEmitter`, `NullExtractor`, `NullSignalEmitter`, `LifecycleState`, `WorkspaceFileLifecycle`, `UserOverride`, `LifecycleError`. **All zero matches.** Net-new vocabulary; no shadow of substrate.

## `SourceAttribution::new(...)` call shape sanity

§4 doc-comment + §8 test gate call: `SourceAttribution::new(DataSource::WorkspaceFile{kind}, vec![SourceIdentifier::Document{document_id, chunk_id: None}], observed_at, source_asof, evidence_weight, None)`. Real signature at `provenance/source.rs:316-323` takes `(data_source, identifiers, observed_at, source_asof: Option<DateTime<Utc>>, evidence_weight: f32, synthesis_marker: Option<SynthesisMarker>) -> Result<Self, SourceAttributionError>`. Arity, types, ordering all match. `source_asof` is `Option<DateTime<Utc>>` — §8 test wraps in `Some(file_mtime)`; implementing-agent nit, not a packet blocker. `scoring_class` correctly omitted (auto-derived inside `new`).

## Convergence

The class is closed structurally. §8's CI grep gate `tests/no_substrate_reinvention.rs` covers all 6 known reinvention names (`SourceAttribution|SourceType|TrustFactorInput|SourceIdentifier|ClaimProposal|Provenance`); future agents adding a 7th-shape reinvention will trip CI. K-in §6 is now per-primitive with consumes/referenced classification. No other reinvention classes visible in V1.3 §4 — `WorkspaceFileKind`, `DataSource`, `SubjectRef`, `ClaimSensitivity`, `ClaimType` all consumed, not paralleled. SubjectRef-duality risk explicitly routed to Codebase Maintenance project per §6.

Bias-toward-closure conditions both met: sweep is complete, gate enforces it. **Ship V1.3 to L1.**
