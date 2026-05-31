# L0 Packet — v1.4.5 W1-A — DOS-463 Workspace File Lifecycle and Ownership Model

**Current revision:** V1.4 (cycle 4 mechanical-fix fold, 2026-05-20). See §2 Changelog.

## 1. Header

- **Date:** 2026-05-20
- **Project:** v1.4.5 — Workspace Memory Refactor ([Linear](https://linear.app/a8c/project/v145-workspace-memory-refactor-cdb9d2c17102))
- **Wave:** W1 stage 1a (gates W1-B and W1-C; gates all of W2/W3/W4/W5)
- **Issue:** [DOS-463 — Define workspace source lifecycle and ownership model](https://linear.app/a8c/issue/DOS-463)
- **Branch:** `wave/v1.4.5-w1-stage1a` (from `dev` @ `852a0118`)
- **Worktree:** `/private/tmp/dailyos-v145-w1a`
- **Migration slots claimed:** **v250, v251** (cycle 11 renumber)
- **Authority docs:** `.docs/plans/v1.4.5-waves.md` (cycle 10 unanimous APPROVE + cycle 11 slot renumber) + `.docs/plans/v1.4.5-w0-reuse-audit-2026-05-19.md` (with 2026-05-20 amendment)
- **L2 reviewer matrix:** codex review + code-reviewer + architect-reviewer

## 2. Changelog

- **V1.4 (2026-05-20 — cycle 4 mechanical-fix fold):** Cycle 4 returned architect APPROVE-ship + codex challenge BLOCK (5 findings) + codex consult BLOCK (3 findings). All findings are mechanical (compile-shape, missing type, count mismatch, downstream-doc sweep) — no new substrate-reinvention. Per memory `feedback_reviewer_dissent_is_signal`, codex dissent against architect's "ship" is genuine; folding all 5 unique findings:
  1. **`SourceAttribution::new(...)` doc-comment + §8 test wraps fixed** — `source_asof: Option<DateTime<Utc>>` (not bare), `document_id: DocumentId::new(file_id)` (not bare string). Updated both §4 §"Canonical 7-field SourceAttribution" doc-comment + §8 `SourceAttribution construction sanity` test text.
  2. **Unused `SourceIdentifier` import removed from `contracts.rs` `use`** — was imported only for the doc-comment reference, which fails `cargo clippy -- -D warnings`. The doc comment now fully qualifies (`abilities_runtime::abilities::provenance::source::SourceIdentifier::Document`).
  3. **`LifecycleError` defined in `lifecycle.rs` section** — V1.3 referenced it in `escalate_to_pending` return type but didn't define it. V1.4 §4 `lifecycle.rs` adds: `pub enum LifecycleError { FileNotFound, InvalidStateTransition { from: LifecycleState, to: LifecycleState }, DbError(String) }`.
  4. **Field count §8 `WorkspaceClaimProposal shape` test corrected from "9 fields" to "10 fields"** — matches §4 actual count (`claim_type, subject_ref, content, source_asof, data_source, sensitivity, source_attribution, ingestion_run_id, lifecycle_state, initial_source_reliability`).
  5. **Wave-plan downstream stale `SourceType` references swept** — wave plan amended with Cycle 12 block + per-lane fixes at lines 431, 433-436, 470, 615. Closes codex challenge cycle 4 #4 (the "L0 packet fixed but downstream authority can re-introduce reinvention" trap).
  - **CI grep gate scope acknowledged (challenge cycle 4 #5 partial):** §8 grep gate catches `^pub (struct|enum|trait)` patterns. It does NOT catch `pub(crate)`, `pub type` aliases, or same-shape renamed clones. This is a documented limitation; strengthening to a Rust-AST-aware lint is filed as a Codebase Maintenance follow-up (it would require parsing source.rs to know the canonical surface). The current gate catches the actual reinvention shape that fired three times in cycles 1–3 (always `pub struct/enum/trait`), which is the highest-value structural prevention available without a full lint rewrite.

- **V1.3 (2026-05-20 — cycle 3 structural-sweep fold):** Cycle 3 returned all three reviewers BLOCK with **same-class findings (substrate reinvention, 3rd consecutive cycle)** — architect: `SourceAttribution` + `SourceType` + `WorkspaceClaimProposal`/`ClaimProposal` mapping; codex challenge: mod.rs ordering conflict + `SourceType` reinvention + done-when test gap; codex consult: `SourceAttribution` reinvention + K-in citation gap + changelog wording. Per memory `feedback_zoom_out_for_class_pattern_in_l2_loop` (3rd recurrence = sweep IS the work), V1.3 runs the full structural substrate sweep and converges. Folds:
  1. **`SourceAttribution` reinvention removed.** `WorkspaceClaimProposal` now carries `source_attribution: abilities_runtime::abilities::provenance::source::SourceAttribution` (the canonical 7-field struct: `data_source`, `identifiers: Vec<SourceIdentifier>`, `observed_at`, `source_asof`, `evidence_weight`, `scoring_class`, `synthesis_marker`). W3-A's extractor constructs via `SourceAttribution::new(...)` with workspace-file-shape `SourceIdentifier::Document { document_id: file_id, chunk_id: None }`. The ingestion-run linkage moves to a separate top-level `WorkspaceClaimProposal.ingestion_run_id: String` field — different concept from source identification. Closes architect cycle 3 #1 + consult cycle 3 #1+#2.
  2. **`SourceType` enum dropped.** It was a 1:1 mirror of `WorkspaceFileKind` at `abilities-runtime/src/abilities/provenance/source.rs:201`. `contracts.rs` now imports `WorkspaceFileKind` directly and uses it where the V1.2 packet used `SourceType`. The migration column `source_type TEXT` stores serde-tag of `WorkspaceFileKind` (no semantic change; the storage shape is identical to what the mirror would have produced). Closes architect cycle 3 #2 + challenge cycle 3 #2.
  3. **`WorkspaceClaimProposal` vs `ClaimProposal` relationship documented.** `WorkspaceClaimProposal` is the **extraction-shape** owned by W1-A and produced by W3-A's `Extractor::extract`. The **commit-shape** is `dailyos_lib::services::claims::ClaimProposal` (`src-tauri/src/services/claims.rs:74`, 18 fields). The mapping is W3-A's responsibility (a `pub fn into_commit(self, actor: &str) -> ClaimProposal` method on the extraction shape that translates fields into the commit shape). The wave plan pinned the `WorkspaceClaimProposal` name through 10 cycles, so V1.3 keeps the wrapper (which has genuine value: it bundles extractor outputs with workspace-specific metadata BEFORE the canonical commit translation) and explicitly documents the wrapper-vs-canonical boundary. Closes architect cycle 3 #3.
  4. **`mod.rs` order: alphabetical** to match §8 `tests/mod_rs_shape.rs` gate. Reordered from V1.2's dependency-order to alphabetical: `contracts, extract, graph, lifecycle, link, pipeline, registry, runs, signals, wiring`. Closes challenge cycle 3 #1.
  5. **§6 K-in citation table expanded** to enumerate every substrate primitive `contracts.rs` touches with explicit "consumes" / "wraps for reason X" classification — `SourceAttribution`, `SourceIdentifier`, `ScoringClass`, `SynthesisMarker`, `Provenance`, `ClaimProposal` all added. Closes consult cycle 3 #2.
  6. **§8 CI grep gate: substrate-reinvention regression prevention.** New test asserts `grep -rE "^pub (struct\|enum\|trait) (SourceAttribution\|SourceType\|TrustFactorInput\|SourceIdentifier\|ClaimProposal\|Provenance)\b" src-tauri/src/services/workspace_ingestion/` returns zero lines. Catches future drift even if a future agent forgets the K-in sweep. Closes challenge cycle 3 #3 + structural-sweep memory `feedback_zoom_out_for_class_pattern_in_l2_loop`.
  7. **Changelog wording consistent** ("3rd recurrence of substrate-reinvention class") + V1.2 minor consult finding folded. Closes consult cycle 3 #4.

- **V1.2 (2026-05-20 — cycle 2 fold):** Cycle 2 folds (5 items, 2 classes): substrate-reinvention reverted (`TrustFactorInput` enum dropped → `initial_source_reliability: f64`); §4 reframed as frozen surface contract with `unimplemented!()` bodies; `tests/contracts_import.rs` import path corrected; `LifecycleState` variant test added; wave-plan line 125 stale-ref fixed. **V1.2 missed the broader substrate-reinvention class** (caught architect's cycle-2 acceptance but NOT `SourceAttribution` / `SourceType` / `ClaimProposal` shape).

- **V1.1 (2026-05-20 — cycle 1 fold):** 9 cycle-1 findings folded (slot renumber v200→v250; trait `Send + Sync` + signatures; `WorkspaceCategory` serde + responsibility split; `lifecycle_state` + invented `TrustFactorInput` added; `tests/contracts_import.rs`; CLAIM_TYPE_REGISTRY cite; architect doc nits). **V1.1 introduced the `TrustFactorInput` reinvention V1.2 reverted.**

- **V1.0 (2026-05-20):** Initial packet. **Inherited stale v200/v201 slot claim from W0 reuse audit.**

## 3. Goal (verbatim from wave plan §Agent W1-A)

Define and ship the authoritative lifecycle/ownership model for workspace files. Every file DailyOS monitors carries a lifecycle state (`pending`, `pending_entity_assignment`, `ingesting`, `ingested`, `superseded`, `rejected`, `quarantined`) and an ownership record linking it to a source type, entity (if known), and ingestion run. This is the schema + type contract all of W1 and W2 build on. `pending_entity_assignment` covers files that ingested into the registry but could not be matched to an entity; W2-D and W4-A both query for this state.

## 4. Files owned (exclusive)

### New migrations
- `src-tauri/src/migrations.rs` — slots **v250, v251** (cycle 11 renumber):
  - **v250** — `workspace_file_lifecycle` table (full column set per §6).
  - **v251** — additive `ALTER TABLE workspace_file_lifecycle ADD COLUMN category TEXT;` per cycle 8 Option B-prime.

### New module skeleton under `src-tauri/src/services/workspace_ingestion/`

**`mod.rs`** — `pub mod` declarations one per line, **alphabetical** (matches §8 test gate):
```rust
pub mod contracts;
pub mod extract;
pub mod graph;
pub mod lifecycle;
pub mod link;
pub mod pipeline;
pub mod registry;
pub mod runs;
pub mod signals;
pub mod wiring;
```
No `pub use` re-exports. No inline items. Subsequent lanes fill placeholder content but never edit this file.

**`contracts.rs`** — **frozen surface contract owned by W1-A** (function bodies are `unimplemented!()` for compileable shape; implementing agent fills them per doc comments; the signature shape is frozen — any change is a wave-plan amendment):

```rust
use std::fs::File;
use std::path::PathBuf;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use abilities_runtime::abilities::claims::ClaimType;
// V1.3 cycle-3 substrate-consumption fix: import canonical SourceAttribution + WorkspaceFileKind
// directly. W1-A does NOT define parallel SourceType / SourceAttribution types (V1.2 mirror +
// reinvention were caught by substrate-grep sweep).
// V1.4 cycle-4 fix: SourceIdentifier removed from import list — only referenced in doc-comment,
// which means importing here fails `cargo clippy -- -D warnings` (unused import).
// Doc comments fully qualify when they reference it.
use abilities_runtime::abilities::provenance::source::{
    DataSource, SourceAttribution, WorkspaceFileKind,
};
// SubjectRef reconciliation: two enums exist (abilities-runtime variant-tuple here vs db-layer
// struct-variant at src/db/claim_invalidation.rs:58). W1-A imports the abilities-runtime one
// by deliberate choice; downstream lanes inherit. Reconciliation tracked as Codebase Maintenance
// ticket in project b8e6aea4-d47e-4f3a-b03d-a05bec914aeb.
use abilities_runtime::abilities::provenance::subject::SubjectRef;
use abilities_runtime::types::ClaimSensitivity;
use super::lifecycle;

pub struct FileIdentity {
    pub canonical_path: PathBuf,
    pub device: u64,
    pub inode: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "name")]
pub enum WorkspaceCategory {
    Presentations,
    Transcripts,
    Meetings,
    Notes,
    Contracts,
    Attachments,
    /// Wire shape: `{"kind": "other", "name": "<lowercase_ascii_slug>"}`.
    /// W1-A's `from_slug` enforces only lexical slug shape (lowercase ASCII; `[a-z0-9_-]+`).
    /// Per-entity registry-allowedness is W1-B's `WorkspaceCategoryRegistry::validate`.
    Other(String),
}

impl WorkspaceCategory {
    /// Canonical wire+path slug. Single source of truth used by PlaceDocumentInput.category,
    /// PlaceDocumentReceipt.category, PlacementError::CategoryNotAllowed.allowed,
    /// WorkspaceGraphQueryInput.category_filter, file_links[].category, AND filesystem path.
    pub fn as_slug(&self) -> &str {
        unimplemented!("W1-A implementing agent: match each variant to its lowercase slug; Other(s) returns s")
    }

    /// Lexical slug parse. Returns None for slugs failing `^[a-z0-9_-]+$`. Does NOT verify
    /// per-entity registry-allowedness (that is W1-B's WorkspaceCategoryRegistry boundary).
    pub fn from_slug(s: &str) -> Option<Self> {
        unimplemented!("W1-A implementing agent: lex-validate; match known slugs; Other(s) only if lex-valid")
    }
}

pub enum RejectionReason {
    PathTraversalAttempt, SymlinkRefused, SymlinkRaced, OutsideWorkspace,  // W1-B emits these
    FileTooLarge, UnsupportedFormat,                                        // W2-A pipeline emits these
}

/// Extraction shape returned by W3-A's `Extractor::extract`. This is the wrapper that bundles
/// extractor output with workspace-specific metadata BEFORE conversion to the canonical commit
/// shape (`dailyos_lib::services::claims::ClaimProposal` at services/claims.rs:74, 18 fields).
///
/// W3-A is responsible for the `into_commit(self, actor: &str) -> ClaimProposal` mapping that
/// translates this extraction shape into the commit shape, packing workspace-specific bits
/// (`lifecycle_state`, `initial_source_reliability`, `ingestion_run_id`) into the commit shape's
/// `metadata_json` field per existing services::claims convention.
///
/// W1-A defines the extraction shape only; downstream commit happens through
/// `services::claims::commit_claim`, which is the single writer of `intelligence_claims`.
pub struct WorkspaceClaimProposal {
    pub claim_type: ClaimType,
    /// subject_ref entity ids are canonical v1.4.0 entity-slug format
    /// (consumed by services::claims::commit_claim).
    pub subject_ref: SubjectRef,
    pub content: serde_json::Value,
    pub source_asof: DateTime<Utc>,
    pub data_source: DataSource,
    pub sensitivity: ClaimSensitivity,
    /// Canonical 7-field SourceAttribution (consumed from existing substrate at
    /// abilities-runtime/src/abilities/provenance/source.rs:303). W3-A constructs via
    /// `SourceAttribution::new(
    ///     DataSource::WorkspaceFile { kind },
    ///     vec![abilities_runtime::abilities::provenance::source::SourceIdentifier::Document {
    ///         document_id: abilities_runtime::abilities::provenance::DocumentId::new(file_id),
    ///         chunk_id: None,
    ///     }],
    ///     observed_at,
    ///     Some(source_asof),                              // V1.4 cycle-4 fix: Some(...) wrap (real sig is Option<DateTime<Utc>>)
    ///     evidence_weight,                                // f32, validated 0.0..=1.0 by ::new
    ///     None,                                           // synthesis_marker: None for workspace files
    /// )`.
    pub source_attribution: SourceAttribution,
    /// Workspace-specific run linkage (NOT a SourceIdentifier — separate concept).
    pub ingestion_run_id: String,
    pub lifecycle_state: lifecycle::LifecycleState,
    /// One trust-factor input populated by W3-A's extractor (normalized 0.0–1.0). Satisfies
    /// wave-plan §Architecture invariants line 323 "at least one trust factor input from
    /// creation". At `services::claims::commit_claim` time, combined with corroboration query,
    /// contradiction query, freshness decay per data_source/source_asof to construct the full
    /// `abilities_runtime::abilities::trust::types::TrustFactorInputs` (11-field struct).
    /// W1-A does NOT reinvent any trust-factor type (V1.1's `TrustFactorInput` was caught
    /// in cycle 2 and reverted).
    pub initial_source_reliability: f64,
}

/// Extraction trait. W3-A's WorkspaceExtractor implements this; W2-A constructs the pipeline
/// with NullExtractor by default and wiring.rs swaps in WorkspaceExtractor after W3-A merges.
pub trait Extractor: Send + Sync {
    fn extract(
        &self,
        file: &File,
        identity: &FileIdentity,
        source_type: WorkspaceFileKind,
    ) -> Vec<WorkspaceClaimProposal>;
}

/// Signal-emission trait. Each method maps 1:1 to a SignalType::WorkspaceFile* variant in
/// W3-B's signals/policy_registry.rs (5 methods → 5 variants). W3-B's WorkspaceSignalEmitter
/// implements this; W2-A constructs the pipeline with NullSignalEmitter by default and wiring.rs
/// swaps in WorkspaceSignalEmitter after W3-B merges. W1-C's link::override_link consumes
/// `&dyn SignalEmitter` for emit_link_changed.
pub trait SignalEmitter: Send + Sync {
    fn emit_file_ingested(
        &self,
        file_id: &str,
        file_hash: &str,
        ingestion_run_id: &str,
        entity_id: Option<&str>,
    );
    fn emit_file_rejected(&self, file_id: Option<&str>, reason: RejectionReason);
    fn emit_file_pending_entity_assignment(&self, file_id: &str, ingestion_run_id: &str);
    fn emit_file_quarantined(&self, file_id: &str, reason: &str, actor: &str);
    fn emit_link_changed(&self, file_id: &str, entity_id: &str, actor: &str);
}

pub struct NullExtractor;
impl Extractor for NullExtractor {
    fn extract(&self, _: &File, _: &FileIdentity, _: WorkspaceFileKind) -> Vec<WorkspaceClaimProposal> {
        Vec::new()
    }
}

pub struct NullSignalEmitter;
impl SignalEmitter for NullSignalEmitter {
    fn emit_file_ingested(&self, _: &str, _: &str, _: &str, _: Option<&str>) {}
    fn emit_file_rejected(&self, _: Option<&str>, _: RejectionReason) {}
    fn emit_file_pending_entity_assignment(&self, _: &str, _: &str) {}
    fn emit_file_quarantined(&self, _: &str, _: &str, _: &str) {}
    fn emit_link_changed(&self, _: &str, _: &str, _: &str) {}
}
```

**`lifecycle.rs`** — substantive content owned by W1-A:
- `LifecycleState` enum with exactly 7 variants per §3:
  ```rust
  #[derive(Serialize, Deserialize)]
  #[serde(rename_all = "snake_case")]
  pub enum LifecycleState {
      Pending, PendingEntityAssignment, Ingesting, Ingested,
      Superseded, Rejected, Quarantined,
  }
  ```
  Canonical serde strings: `"pending"`, `"pending_entity_assignment"`, `"ingesting"`, `"ingested"`, `"superseded"`, `"rejected"`, `"quarantined"`.
- `WorkspaceFileLifecycle` struct mirroring the v250 row shape with cycle 8's `category: Option<WorkspaceCategory>` column.
- `UserOverride { actor_id: String, at: DateTime<Utc> }` struct.
- `escalate_to_pending(file_id: &str, actor: &str) -> Result<(), LifecycleError>` API stub (W4-A wires later).
- `LifecycleError` enum (V1.4 cycle-4 fix: V1.3 referenced this in `escalate_to_pending` return but didn't define it):
  ```rust
  pub enum LifecycleError {
      FileNotFound,
      InvalidStateTransition { from: LifecycleState, to: LifecycleState },
      DbError(String),
  }
  ```
- Trybuild-protected constructor enforcing the four required fields (`source_type`, `source_asof`, `data_source`, `lifecycle_state`).

**Empty placeholder files** (module-level `//!` doc-comment only):
- `registry.rs` (W1-B), `runs.rs` (W1-C), `link.rs` (W1-C), `pipeline.rs` (W2-A), `wiring.rs` (W2-A), `extract.rs` (W3-A), `signals.rs` (W3-B), `graph.rs` (W3-C).

### Touched (one line)
- `src-tauri/src/services/mod.rs` — add `pub mod workspace_ingestion;`, alphabetical between `versioning` and `#[cfg(test)] mod tests;`. No other edits.

## 5. Files NOT touched (deny list)

- `src-tauri/src/services/claims.rs`, `src-tauri/src/signals/**`, `src-tauri/src/processor/**`, `src-tauri/src/watcher.rs`, `src-tauri/src/google_drive/**`, `granola/**`, `quill/**`
- **`abilities-runtime/src/abilities/provenance/**` (W1-A consumes `SourceAttribution`, `SourceIdentifier`, `WorkspaceFileKind`, `DataSource`, `SubjectRef`, `ClaimType`, `ClaimSensitivity`; never modifies)**
- **`abilities-runtime/src/abilities/trust/**` (W1-A consumes `TrustFactorInputs`, `SourceReliabilityInput`, `SourceLifecycleState` implicitly via downstream commit_claim path; never modifies)**
- Any existing `migrate_v*` function
- Content of any submodule file W1-A creates as placeholder

## 6. Contracts referenced + K-in citations

### Substrate prerequisites verified live on `dev` (re-grepped 2026-05-20 cycles 1+2+3 + V1.3 full structural sweep)

| Contract | Location (verified) | W1-A relationship |
|---|---|---|
| `DataSource::WorkspaceFile { kind: WorkspaceFileKind }` | `…/provenance/source.rs:81` | **consumes** (PR #319 ADR-0107 amendment) |
| `WorkspaceFileKind` enum (7 variants) | `…/provenance/source.rs:201` | **consumes directly** (V1.2 had a 1:1 `SourceType` mirror; V1.3 sweep dropped it) |
| `ClaimType` | `…/abilities/claims.rs:130` | **consumes** |
| `ClaimSensitivity` (ADR-0125) | `…/types.rs:37` | **consumes** |
| `SubjectRef` (abilities-runtime, variant-tuple) | `…/provenance/subject.rs:8` | **consumes** ⚠ duality risk below |
| `CLAIM_TYPE_REGISTRY` | `…/abilities/claims.rs:710` | **referenced** (downstream W3-A consumer) |
| `DataSource` enum | `…/provenance/source.rs:73` | **consumes** |
| **`SourceAttribution` (7-field canonical)** | `…/provenance/source.rs:303` | **consumes** (V1.2 had a reinvented 2-field local struct; V1.3 sweep dropped it. Used by 15+ abilities including `account_overview.rs:21`, `get_entity_context.rs:12`, `prepare_meeting/synthesis.rs:16`) |
| **`SourceIdentifier::Document { document_id, chunk_id }`** | `…/provenance/source.rs:244` | **consumes** (W3-A constructs workspace-file attribution via this variant) |
| `ScoringClass` | `…/provenance/source.rs:227` | **consumes implicitly** (via `SourceAttribution.scoring_class`, set by `SourceAttribution::new`) |
| `SynthesisMarker` | `…/provenance/source.rs:295` | **consumes implicitly** (via `SourceAttribution.synthesis_marker = None` for workspace files) |
| `Provenance` (envelope) | `…/provenance/envelope.rs:343` | **referenced** (downstream consumer; W1-A does not construct) |
| `TrustFactorInputs` (11-field struct) | `…/trust/types.rs:118` | **referenced** (downstream commit_claim consumer; W1-A's `initial_source_reliability: f64` is one seed input) |
| `SourceReliabilityInput` | `…/trust/types.rs:176` | **referenced** (downstream consumer) |
| `SourceLifecycleState` | `…/trust/types.rs:188` | **referenced** (orthogonal — that is trust-axis lifecycle, distinct from `lifecycle::LifecycleState` here which is workspace-file lifecycle) |
| **`services::claims::ClaimProposal` (18-field commit input)** | `src-tauri/src/services/claims.rs:74` | **referenced** (W3-A's `WorkspaceClaimProposal::into_commit(actor)` produces this; W1-A defines extraction shape only, never commits) |
| Submodule + `contracts.rs` convention | `services/claim_receipt/{mod,contracts}.rs` | **follows existing pattern** |
| Highest registered migration on dev | `migrations.rs:927` (v240) | W1-A claims v250/v251 per cycle 11 |

### K-in obligation (grep `docs/solutions/` + `.docs/decisions/`)

Re-run 2026-05-20 V1.3 sweep: zero hits for `workspace_ingestion`, `workspace_file_lifecycle`, `pending_entity_assignment`, `FileIdentity`, `RejectionReason`, `SignalEmitter` (singular), `WorkspaceCategory`, `WorkspaceClaimProposal`, `initial_source_reliability`. Codex consult independently confirmed zero hits across cycles 1–3.

**No K-in BLOCKED finding.** Closest documented anchors: ADR-0098, ADR-0107 (with W0 amendment), ADR-0125, ADR-0130.

### v250 migration column shape

`workspace_file_lifecycle`:
- `id INTEGER PRIMARY KEY AUTOINCREMENT`
- `file_id TEXT NOT NULL UNIQUE`
- `canonical_path TEXT NOT NULL`
- `device INTEGER NOT NULL`, `inode INTEGER NOT NULL`
- `source_type TEXT NOT NULL` — serde-tag of `WorkspaceFileKind` (V1.3 sweep: stores the canonical `WorkspaceFileKind` snake_case string, not a mirror enum's serialization)
- `data_source TEXT NOT NULL` — JSON-serialized `DataSource::WorkspaceFile { kind }`
- `lifecycle_state TEXT NOT NULL DEFAULT 'pending'` — typed at service boundary
- `source_asof TIMESTAMP NOT NULL`
- `entity_id TEXT`, `entity_type TEXT` — nullable
- `content_sha256 TEXT` — nullable
- `user_override_actor TEXT`, `user_override_at TIMESTAMP` — nullable
- `created_at TIMESTAMP NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))`
- `updated_at TIMESTAMP NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))`
- Indexes: `idx_wfl_state (lifecycle_state)`, `idx_wfl_entity (entity_id, entity_type) WHERE entity_id IS NOT NULL`, `idx_wfl_pending_entity (lifecycle_state) WHERE lifecycle_state = 'pending_entity_assignment'`

### v251 migration shape
```sql
ALTER TABLE workspace_file_lifecycle ADD COLUMN category TEXT;
```

### Risk: SubjectRef + DataSource duality (single Codebase Maintenance ticket)

Two `SubjectRef` types exist (abilities-runtime variant-tuple vs db-layer struct-variant); two `DataSource` enums (abilities-runtime canonical vs `src/db/data_lifecycle.rs:315` mirror). W1-A imports the abilities-runtime variant for both and documents the choice inline. **One** Codebase Maintenance ticket (project `b8e6aea4-…`) tracks both reconciliations as a single sweep. Reconciliation is not W1-A in-scope work.

## 7. Intelligence Loop gate

1. **Claim model.** Lifecycle row is provenance carrier, not a claim. W1-A commits no claims.
2. **Provenance + trust.** Lifecycle row carries `source_type` (serde-tag of canonical `WorkspaceFileKind`), `source_asof`, `data_source` (`DataSource::WorkspaceFile { kind }`). Trust composition uses existing substrate (`TrustFactorInputs`, `SourceReliabilityInput`) via downstream `commit_claim` path. W1-A's `WorkspaceClaimProposal` consumes canonical `SourceAttribution` (V1.3 sweep eliminated all substrate reinvention).
3. **Signals + invalidation.** W1-A emits no signals. `SignalEmitter` trait compiles in W1 against `NullSignalEmitter` so W1-C's `link::override_link` lands without wave-timing inversion.
4. **Runtime + surfaces.** No ability consumes W1-A types in W1. Types defined for downstream consumption.
5. **Feedback loop.** `user_override: Option<UserOverride>` field; `lifecycle::escalate_to_pending` API for W4-A.

## 8. Tests required

- **Migration round-trip:** v250 applies against v240-baseline DB; v251 applies after v250.
- **Migration ordering:** `MIGRATIONS.iter().map(|m| m.version()).max() == Some(251)` after W1-A merge.
- **`LifecycleState` variant-set + serde-string test:** exactly 7 variants; serde strings match canonical list.
- **Trybuild test:** omitting `source_type` / `source_asof` / `data_source` / `lifecycle_state` from `WorkspaceFileLifecycle` constructor fails to compile.
- **`WorkspaceCategory` serde round-trip:** every variant serializes per `tag="kind", content="name"`; `as_slug() → from_slug()` round-trips; `Other(slug)` accepts lex-valid + rejects malformed.
- **`WorkspaceClaimProposal` shape:** trybuild asserts constructor enforces all **10 fields** (V1.4 cycle-4 count fix): `claim_type`, `subject_ref`, `content`, `source_asof`, `data_source`, `sensitivity`, `source_attribution: SourceAttribution` (canonical), `ingestion_run_id: String`, `lifecycle_state`, `initial_source_reliability: f64`.
- **`SourceAttribution` construction sanity:** test that `SourceAttribution::new(DataSource::WorkspaceFile{kind: WorkspaceFileKind::Inbox}, vec![SourceIdentifier::Document{document_id: DocumentId::new("test-file-id"), chunk_id: None}], Utc::now(), Some(file_mtime), 0.5_f32, None)` returns `Ok(_)` (verifies the canonical substrate accepts the workspace-shape inputs W3-A will produce; arg types match real signature at `provenance/source.rs:316-323`).
- **`SignalEmitter` trait + `NullSignalEmitter`:** 5-method surface; `Box<dyn SignalEmitter>` constructible; trait `Send + Sync` (test in spawned task).
- **`Extractor` trait + `NullExtractor`:** returns empty `Vec`; `Send + Sync`.
- **Shared-types import test:** `tests/contracts_import.rs` does `use dailyos_lib::services::workspace_ingestion::contracts::{WorkspaceCategory, FileIdentity, RejectionReason, WorkspaceClaimProposal, Extractor, SignalEmitter, NullExtractor, NullSignalEmitter};` (lib crate name `dailyos_lib` per `src-tauri/Cargo.toml:22`). Asserts each is `pub` and importable externally.
- **`mod.rs` shape test:** `tests/mod_rs_shape.rs` asserts exactly one `pub mod` line per submodule, **alphabetical order**, no `pub use`, no inline items.
- **CI grep gate — substrate-reinvention regression prevention (V1.3 cycle-3 structural fix):** `tests/no_substrate_reinvention.rs` shell-test asserts:
  ```sh
  ! grep -rE '^pub (struct|enum|trait) (SourceAttribution|SourceType|TrustFactorInput|SourceIdentifier|ClaimProposal|Provenance)\b' src-tauri/src/services/workspace_ingestion/
  ```
  Returns zero matches; catches future drift even if a future agent forgets the K-in sweep. Closes the 3-cycle substrate-reinvention class structurally.
- **Submodule placeholder skeleton:** every placeholder file exists with `//!` doc comment only.
- **`services/mod.rs` integration:** one-line addition; cargo build resolves.
- **Lint clean:** `cargo clippy -- -D warnings`.

## 9. Done when

- Migration slots **v250 + v251** used; `MIGRATIONS` slice max version is 251.
- `services::workspace_ingestion` submodule tree present per §4 (alphabetical `mod.rs`; `contracts.rs` + `lifecycle.rs` substantively filled; 8 placeholders with `//!` only).
- All shared types in `contracts.rs` per §4 frozen surface; all `pub` and externally importable per §8.
- `WorkspaceClaimProposal.source_attribution` is canonical `abilities_runtime::abilities::provenance::source::SourceAttribution` (NOT a reinvented local struct).
- `Extractor::extract` takes `source_type: WorkspaceFileKind` (canonical), NOT a `SourceType` mirror.
- `Extractor` + `SignalEmitter` traits (with `Send + Sync` + full signatures) + Null defaults compile.
- `LifecycleState` enum (7 variants with canonical serde strings) + `WorkspaceFileLifecycle` struct present.
- `WorkspaceClaimProposal.initial_source_reliability` is `f64` (NOT a reinvented trust enum); downstream trust composition uses canonical `TrustFactorInputs`.
- CI grep gate `tests/no_substrate_reinvention.rs` passes (zero `pub struct/enum/trait` matching reinvention list under `services/workspace_ingestion/`).
- `src-tauri/src/services/mod.rs` carries the new line.
- All §8 tests pass; `cargo clippy -- -D warnings && cargo test` green.
- IL gate items 1–5 in §7 answered affirmatively in commit message.
- `L2-status: passed` declared.
- L0 verdict posted as Linear comment on DOS-463.

## 10. Handoff notes

- **W1-B (DOS-464)** fills `registry.rs` + `WorkspaceCategoryRegistry`. Slot **v252**. `/cso`.
- **W1-C (DOS-465)** fills `runs.rs` + `link.rs`. Slots **v253 + v254**. Compiles against W1-A's `SignalEmitter` trait + `NullSignalEmitter`.
- **W2-A (DOS-466)** fills `pipeline.rs` + `wiring.rs`.
- **W3-A (DOS-470)** fills `extract.rs` (`WorkspaceExtractor` impl returns `Vec<WorkspaceClaimProposal>`); patches one line in `wiring.rs`. **Implements `WorkspaceClaimProposal::into_commit(actor) -> ClaimProposal` mapping** (the wrapper-to-canonical translation; packs `lifecycle_state`/`initial_source_reliability`/`ingestion_run_id` into `ClaimProposal.metadata_json` per existing services::claims convention). Populates `initial_source_reliability` from extraction confidence.
- **W3-B (DOS-471)** fills `signals.rs` (5 trait methods → 5 `SignalType::WorkspaceFile*` variants in `signals/policy_registry.rs`); patches one line in `wiring.rs`.
- **W3-C (DOS-489)** fills `graph.rs` + `workspace_graph` ability. `/plan-devex-review`.

No lane touches `mod.rs` after W1-A creates it. No lane creates a new file in `services/workspace_ingestion/` beyond placeholders. **No lane reinvents `SourceAttribution`, `SourceType`, `TrustFactorInput`, `SourceIdentifier`, `ClaimProposal`, or `Provenance` — the CI grep gate at §8 enforces this structurally.**
