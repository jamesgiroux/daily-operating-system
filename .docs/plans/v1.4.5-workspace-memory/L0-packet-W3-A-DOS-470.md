# L0 Packet — v1.4.5 W3-A — DOS-470 Claim Proposals from Workspace Files

**Current revision:** V1.0 (initial draft, 2026-05-23). See §2 Changelog.

## 1. Header

- **Date:** 2026-05-23
- **Project:** v1.4.5 — Workspace Memory Refactor
- **Wave:** W3 stage 3a (W3-A — first real claim producer; merges alone)
- **Issue:** [DOS-470](https://linear.app/a8c/issue/DOS-470) — Convert document-derived facts into claim proposals
- **Branch (proposed):** `wave/v1.4.5-w3-a`
- **Worktree (proposed):** `/private/tmp/dailyos-v145-w3-a`
- **Trust topology:** local-to-local single-user. The producer runs inside the abilities-runtime under a James-only principal; output flows through `commit_claim` and the v1.4.1 signal infrastructure, both of which already enforce actor-attribution.
- **Migration:** None. W3-A ships no migration. Consumes W1's v250–v254 schema, the existing claim substrate, and the canonical primitives.
- **Authority docs:**
  - `.docs/plans/v1.4.5-waves.md:680-740` (W3-A section + W3 staging notes; cycle-12 substrate sweep + cycle-13 amendments)
  - `.docs/plans/v1.4.5-workspace-memory/retro-W2.md` (W2 close + `extract_stub` location)
- **L2 reviewer matrix:** codex-review + code-reviewer + architect-reviewer. **No `/cso` at L2 by default** — W3-A doesn't touch the trust boundary; it produces typed proposals through the existing `commit_claim` write path which already enforces sensitivity/actor policy. Add `/cso` only if L0 finds W3-A introduces a new MCP-reachable extraction surface or new prompt-channel reads.
- **Pacing note:** Stage 3a is W3-A alone. W3-B (signal wiring) and W3-C (graph projection) cannot start until W3-A merges; their producers consume W3-A's claim output.

## 2. Changelog

- **V1.0 — 2026-05-23:** Initial L0 packet. Drafted post-W2-close as the first W3 packet; cites W1's `contracts.rs` trait surface and `CLAIM_TYPE_REGISTRY` as load-bearing K-in.

## 3. Goal (verbatim from wave plan §Agent W3-A)

> Extend `services::workspace_ingestion` with the claim extraction logic: given file content (structured JSON/markdown entity files, or extracted text from Drive/attachment files), produce typed `WorkspaceClaimProposal` records (the type is **W1-A-owned in `contracts.rs`** per cycle 6 promotion) using the existing `CLAIM_TYPE_REGISTRY`. This is the "intelligence" half of the ingestion pipeline — converting raw file content to structured claim proposals.

**Restating in plain terms:** W2-A shipped `IngestPipeline::run` with a `NullExtractor` and a `extract_stub()` returning `Vec::new()`. W3-A swaps the null for the real `WorkspaceExtractor` impl of the W1-A `Extractor` trait. The extractor reads file content (via the read-only handle the pipeline already opened), parses by file type, maps each derived fact to a `claim_kind` against `CLAIM_TYPE_REGISTRY`, and emits `WorkspaceClaimProposal` records. The pipeline then commits each proposal via `services::claims::commit_claim`. **W3-A is the first real claim producer in the v1.4.5 substrate.**

## 4. Files owned (exclusive)

- **`src-tauri/src/services/workspace_ingestion/extract.rs`** — sole owner of content. W1-A pre-created the empty placeholder; W3-A fills it with the `WorkspaceExtractor` struct + impl of the `Extractor` trait defined in W1-A's `contracts.rs`. Extraction strategies per file type live here as private functions.
- **`src-tauri/src/intelligence/io.rs`** — read-only call additions for workspace file formats. W3-A may add new read-side helpers for entity-JSON/frontmatter/text parsing, but does not modify any write path here.
- **`src-tauri/src/services/workspace_ingestion/wiring.rs`** — single documented one-line patch: swap `NullExtractor` for `WorkspaceExtractor` in the `build_pipeline()` constructor call. This is the only W3-A touch in `wiring.rs`. The corresponding W3-B patch (NullSignalEmitter → WorkspaceSignalEmitter) is at a different argument position; the two patches are mechanically non-conflicting.
- **`src-tauri/src/services/workspace_ingestion/pipeline.rs`** — single documented edit: replace the `extract_stub()` call site with the real `extractor.extract(&file, &request)` trait call. **No restructuring of `pipeline.rs`** — W2-A owns the file permanently. W3-A's edit is bounded to swapping the stub call for the trait call; if the pipeline's stage shape needs broader change, raise it as a wave-plan amendment, do not edit pipeline structure in this packet.

## 5. Don't touch

- **`services/claims.rs`** — caller, not modifier. Extraction commits through `commit_claim`; the claim service's own logic stays untouched.
- **`abilities/claims.rs::CLAIM_TYPE_REGISTRY`** — read-only. If a file-derived fact doesn't map to an existing kind, it becomes a `RawObservation` claim (an existing kind), not a silently dropped fact or a silently invented new kind. **New `claim_kind` variants require an L0 amendment to this packet**, not a code-time decision.
- **`services/workspace_ingestion/{lifecycle,registry,runs,link,signals,graph,mod,contracts}.rs`** — all owned by other lanes (W1-A/B/C, W3-B, W3-C). W3-A only writes to `extract.rs` and applies the documented one-liners to `pipeline.rs` and `wiring.rs`.
- **`signals/policy_registry.rs` + `signals/invalidation.rs`** — W3-B's exclusive write surface. W3-A does not emit signals directly; signals flow from `commit_claim` per existing v1.4.1 infrastructure.
- **`signals/bus.rs`** — v1.4.1 owns the signal bus.
- **Markdown / preview rendering paths** — W4-B owns the sanitizer + preview block. W3-A's extraction reads file content but does not render it.

## 6. K-in substrate audit

### Substrate primitives W3-A consumes

| Primitive | Location | Use |
|---|---|---|
| `Extractor` trait | `services/workspace_ingestion/contracts.rs` (W1-A) | W3-A implements `WorkspaceExtractor` against this surface |
| `WorkspaceClaimProposal` | `services/workspace_ingestion/contracts.rs` (W1-A; cycle-6 promotion) | Output type of `Extractor::extract` |
| `FileIdentity` | `services/workspace_ingestion/contracts.rs` (W1-A) | Identity carried through to provenance |
| `ClaimType` | `abilities_runtime::abilities::claims::CLAIM_TYPE_REGISTRY` | Registry lookup for claim_kind resolution |
| `ClaimSensitivity` | `abilities_runtime::types` (per ADR-0125) | Sensitivity tier applied per claim |
| `SourceAttribution` | `abilities_runtime::abilities::provenance::source::SourceAttribution` (`:303`) | Canonical 7-field provenance; cycle-12 amendment confirmed W3-A consumes the canonical, not a local mirror |
| `SourceIdentifier::Document { document_id, chunk_id }` | `abilities_runtime::abilities::provenance::source` | Source identifier shape for workspace-file-derived claims |
| `DataSource::WorkspaceFile { kind: WorkspaceFileKind }` | per ADR-0107 (cycle-2 amendment added the variant) | DataSource tag carried on every emitted proposal |
| `commit_claim` | `services::claims::commit_claim` | Write path for proposals; W3-A is the caller, never a direct claim writer |
| `TrustFactorInputs` | existing v1.4.1 substrate | Composed via `freshness decay table` against `DataSource` variant |
| `IngestionRunId` | W1-C `services/workspace_ingestion/runs.rs` | Separate from `source_attribution`; carried as a parallel field for run-linkage |

### K-in obligation (grep before scoring)

L0 reviewers MUST grep before approving:

- `docs/solutions/**` for prior extraction patterns. Specifically: any existing `extract_*` helpers in `intelligence/io.rs`, any prior frontmatter parsing in `processor/router.rs` or the archived `prepare_inbox.py`, and any prior content-sniffing logic that should be reused rather than reinvented.
- `.docs/decisions/**` for ADRs that govern claim production. ADR-0107 (DataSource taxonomy), ADR-0108 (sensitivity rendering), ADR-0125 (envelope-actor binding), ADR-0124 (claim substrate allowances).
- `abilities_runtime/src/abilities/claims/` for the canonical `commit_claim` signature and any `assert_claim_invariants` helpers W3-A should call before submission.
- `processor/` (the legacy path destined for v1.4.6 consolidation per DOS-770) for any existing fact-extraction logic worth mining, not reinventing.

### Verified against §0 (W2 shared contract)

W3-A consumes the §0-V1.3 typed-DTO surface that W2-A bridges from raw slugs. The `IngestRequest` shape (canonical, post-V1.3) gives W3-A:
- `request.identity: FileIdentity` (read-only)
- `request.kind: WorkspaceFileKind` (drives extraction strategy selection)
- `request.category: Option<WorkspaceCategory>` (informs claim-kind weighting)
- `request.entity_ref: EntityRef` (subject_ref population for committed claims)

## 7. Security gate

**Not a `/cso` lane by default.** W3-A's writes go through `commit_claim`, which already enforces:
- actor attribution (Actor::User on this surface; no external-source MCP path)
- sensitivity tier (W3-A populates `ClaimSensitivity` per file content type; `commit_claim` enforces rendering policy)
- claim invariants (existing `assert_claim_invariants` helper)

**What L0 reviewers should still check:**
- **Subject scoping.** Extracted claims must populate `subject_ref` correctly from `request.entity_ref`. Cross-entity bleed (a workspace file linked to account A producing a claim attributed to account B) is a Suite-E and Suite-S concern.
- **Content disclosure.** Extracted claim text must not surface raw file content for sensitivity-elevated source types without trust-band degradation. ADR-0108 governs.
- **Frontmatter trust.** Frontmatter fields are user-authored but not signed; treat as `RawObservation` claim text, not as authoritative claim metadata. A frontmatter `confidence: 1.0` field does NOT set claim trust band — the freshness decay table + source kind do.
- **Path leakage.** Extraction must not record file paths in claim text or `source_attribution` user-facing fields. `SourceIdentifier::Document { document_id }` uses opaque `file_id`, never path. Suite S check.
- **Binary content.** `IngestPipeline` already rejects unsupported formats at W1-B (`open_validated`); extraction sees only validated text content. W3-A does not need to re-detect binary, but should defensively handle the case where `IngestPipeline` passes through a file with surprising encoding.

## 8. Intelligence Loop gate

The 5 mandatory questions from CLAUDE.md, answered for W3-A:

**1. Claim model.** Every extracted proposal is a `WorkspaceClaimProposal` with a `claim_kind` resolved against `CLAIM_TYPE_REGISTRY`. Unmapped facts become `RawObservation` claims (existing kind), not silently dropped and not silently invented. No new `claim_kind` variants without an L0 amendment.

**2. Provenance + trust.** Each proposal carries:
- `source_asof`: file mtime at extraction time (server-observed; never caller-provided)
- `data_source`: ADR-0107 `DataSource::WorkspaceFile { kind: WorkspaceFileKind }` from `request.kind`
- `source_attribution`: canonical 7-field `SourceAttribution::new(SourceIdentifier::Document { document_id: DocumentId::new(request.identity.file_id()), chunk_id })`
- `ingestion_run_id`: parallel field for run-linkage (separate from `source_attribution`)
- trust band: composed via existing `TrustFactorInputs` against the freshness decay table for the `DataSource` variant; W3-A does not compute trust band locally

**3. Signals + invalidation.** W3-A does not emit signals directly. `commit_claim` emits the existing v1.4.1 claim signals; W3-B's `WorkspaceFileIngested` signal fires at the pipeline boundary, not from `extract.rs`.

**4. Runtime + surfaces.** Committed proposals are consumed by `build_intelligence_context()` and `gather_account_context()` per existing v1.4.1 substrate. W3-A must populate `subject_ref` correctly so context queries find the claims. v1.4.4 WP block surfaces and the v1.4.4a Tauri briefing cutover read these claims through `get_entity_intelligence` — W3-A is the first producer that makes those reads return claim-backed content from workspace files (vs the existing meeting/calendar/email producers).

**5. Feedback loop.** User corrections to a workspace-derived claim propagate through the existing `services::feedback` path. The feedback event traces back to the source file via `source_attribution.source_identifier.document_id`. **Open question for L0:** does `services::feedback` need any extension to surface workspace-file-specific affordances (re-extract, quarantine, relink), or are existing claim-level feedback actions sufficient?

## 9. Code stub (sketch — L0 reviewers, do not bikeshed; this will tighten in cycles)

```rust
// services/workspace_ingestion/extract.rs

use crate::services::workspace_ingestion::contracts::{
    Extractor, FileIdentity, WorkspaceClaimProposal, IngestRequest,
};
use crate::services::claims;
use abilities_runtime::abilities::claims::{ClaimType, CLAIM_TYPE_REGISTRY};
use abilities_runtime::types::ClaimSensitivity;
use abilities_runtime::abilities::provenance::source::{SourceAttribution, SourceIdentifier, DocumentId};
use std::fs::File;
use std::io::Read;

pub struct WorkspaceExtractor;

impl Extractor for WorkspaceExtractor {
    fn extract(&self, file: &File, request: &IngestRequest) -> Vec<WorkspaceClaimProposal> {
        let content = match read_validated_content(file) {
            Ok(s) => s,
            Err(_) => return Vec::new(),  // upstream validation already rejected binary
        };

        // Strategy dispatch by WorkspaceFileKind + WorkspaceCategory.
        // Each strategy returns Vec<WorkspaceClaimProposal>; merge + dedupe.
        let mut proposals = Vec::new();
        proposals.extend(extract_frontmatter_facts(&content, request));
        proposals.extend(extract_text_observations(&content, request));
        proposals.extend(extract_structured_entity_json(&content, request));
        proposals
    }
}

fn extract_frontmatter_facts(content: &str, request: &IngestRequest) -> Vec<WorkspaceClaimProposal> {
    // Parse YAML frontmatter (first 4KB; W2-A's content_head pattern).
    // Map well-known fields (title, owner, tags, doc_type, source-of-truth markers) to existing
    // CLAIM_TYPE_REGISTRY kinds. Unmapped frontmatter fields are NOT auto-promoted to RawObservation;
    // only the text body produces RawObservation claims.
    todo!("L1 — frontmatter parsing + claim_kind mapping")
}

fn extract_text_observations(content: &str, request: &IngestRequest) -> Vec<WorkspaceClaimProposal> {
    // Generate at most one RawObservation claim per file from the text body.
    // Body length cap: 2KB excerpt; longer bodies surface as multiple claims chunked by paragraph
    // ONLY if the file kind is one of the chunk-friendly variants (markdown notes, transcripts).
    // Per CLAUDE.md "don't add error handling for scenarios that can't happen" — upstream W1-B
    // already rejected oversize/binary. Trust that boundary.
    todo!("L1 — body extraction + chunking decision")
}

fn extract_structured_entity_json(content: &str, request: &IngestRequest) -> Vec<WorkspaceClaimProposal> {
    // Only fires when WorkspaceFileKind is EntityJson AND content parses as the expected schema.
    // Maps known JSON field paths to claim kinds. Unknown JSON shapes return Vec::new() — they
    // do not fall through to text extraction (that would produce noise).
    todo!("L1 — structured entity-JSON extraction")
}

fn read_validated_content(file: &File) -> std::io::Result<String> {
    // Implementation note: file handle was opened upstream by WorkspaceSourceRegistry::open_validated.
    // W3-A reads from the handle, does not re-open by path.
    todo!("L1 — content read with size cap matching W2-A's content_head pattern")
}

// Provenance construction helper — used by all three extraction strategies.
fn proposal_with_provenance(
    claim_kind: ClaimType,
    claim_text: String,
    sensitivity: ClaimSensitivity,
    request: &IngestRequest,
) -> WorkspaceClaimProposal {
    let attribution = SourceAttribution::new(
        SourceIdentifier::Document {
            document_id: DocumentId::new(request.identity.file_id()),
            chunk_id: None,  // populate if chunking
        },
        /* source_asof */ request.identity.observed_at(),
        /* additional 5 canonical fields per SourceAttribution::new contract */
    );

    WorkspaceClaimProposal {
        claim_kind,
        claim_text,
        sensitivity,
        subject_ref: request.entity_ref.clone(),
        data_source: ADR-0107-tagged-from-request-kind(request.kind),
        source_attribution: attribution,
        ingestion_run_id: request.ingestion_run_id.clone(),
        // ...remaining required fields per WorkspaceClaimProposal definition
    }
}
```

### Code-stub invariants

- **No direct `intelligence_claims` writes.** Every proposal commits through `services::claims::commit_claim`. The existing `check_claim_writer_allowlist.sh` CI gate (W3-A merge gate activator) catches violations.
- **No new `claim_kind` variants.** If a fact's natural shape doesn't fit `CLAIM_TYPE_REGISTRY`, the fact becomes a `RawObservation` claim with the original text preserved. New variants require this packet to amend with the variant name and `policy_for` definition.
- **No re-opening files by path.** The pipeline passes a `&File` handle; extraction reads from the handle. This preserves the TOCTOU protection W1-B's `open_validated` provides.
- **No silent failures.** A failed extraction strategy returns `Vec::new()` and lets the ingestion run record success-with-zero-proposals, not a swallowed error. Errors that should fail the ingestion (corrupt UTF-8 on a kind that requires text) bubble up through the existing `IngestError` path W2-A defined.

### L1 implementation guardrails

- Body content cap: 2KB excerpt per RawObservation claim; chunking only for markdown notes + transcripts.
- Frontmatter parse cap: first 4KB only (matches W2-A's `content_head` pattern).
- One RawObservation claim per file body MAX, unless chunking applies.
- Frontmatter unknown fields are LOGGED for telemetry but not silently promoted to claims.

## 10. Tests required

### Unit tests (in `extract.rs` `#[cfg(test)] mod tests`)
- `frontmatter_doctype_maps_to_known_claim_kind` — fixture: file with `doc_type: meeting` produces a `MeetingNote`-shaped claim (or appropriate existing kind from registry grep).
- `frontmatter_unknown_field_does_not_create_claim` — unknown frontmatter keys logged but not promoted.
- `text_body_produces_at_most_one_raw_observation` — long bodies cap at one claim unless chunking applies.
- `chunked_markdown_produces_chunked_claims_with_chunk_ids` — chunking populates `SourceIdentifier::Document::chunk_id`.
- `entity_json_shape_match_produces_typed_claims` — fixture: well-formed entity-JSON file produces claims matching the JSON field-to-kind map.
- `entity_json_shape_mismatch_returns_empty` — malformed entity-JSON does NOT fall through to text extraction.
- `proposal_provenance_carries_all_required_fields` — every emitted proposal has `source_asof`, `data_source`, `source_attribution`, `ingestion_run_id`, `subject_ref`, `sensitivity`, `claim_kind`.
- `no_claim_text_contains_file_path` — Suite S unit assertion.

### Integration tests (in `tests/workspace_ingestion_w3a_*`)
- `pipeline_with_real_extractor_produces_real_claims` — replaces W2-A's null-extractor smoke test; fixture file flows through full pipeline and lands committed claims.
- `claim_subject_ref_matches_request_entity_ref` — cross-entity bleed regression test (account-A workspace file does NOT produce claims attributed to account-B).
- `trust_band_degrades_per_freshness_decay` — file with old mtime produces claim with `use_with_caution` or `needs_verification`, not `likely_current`. (Mirrors W5-A's trust discipline AC.)
- `unmapped_fact_becomes_raw_observation_not_dropped` — extraction produces a claim for a fact that doesn't match any registered kind; claim_kind is RawObservation.
- `ingestion_run_records_claim_proposal_count` — the W1-C `document_ingestion_runs` row records the actual count of proposals committed.

### CI gate activations (per W3-A merge)
- `check_claim_writer_allowlist.sh` already excludes `services/workspace_ingestion/**` as a direct writer of `intelligence_claims`; W3-A's first commit becomes a structural test of that exclusion (if extraction accidentally writes directly, CI catches it).
- `check_content_index_allowlist.sh` activates at W3-A merge — forbids direct `INSERT/UPDATE` against `content_index` / `embeddings` outside the existing cache services and ingestion cache hooks. W3-A owns this script per the wave plan's CI invariant table.

## 11. Done when

- `services/workspace_ingestion/extract.rs` ships `WorkspaceExtractor` implementing the W1-A `Extractor` trait.
- `wiring.rs` constructs `WorkspaceExtractor` in `build_pipeline()`; `NullExtractor` is no longer wired (but may remain in `contracts.rs` for test use).
- `pipeline.rs::run` calls `extractor.extract(&file, &request)` and commits each proposal via `commit_claim`.
- All extracted proposals carry mandatory provenance (`source_asof`, `data_source`, `source_attribution`, `ingestion_run_id`, `subject_ref`, `sensitivity`, `claim_kind`).
- No new `claim_kind` variants introduced.
- `check_content_index_allowlist.sh` ships green; existing `check_claim_writer_allowlist.sh` still green.
- All unit + integration tests above green; `cargo clippy --lib -- -D warnings` clean; `cargo test --lib` clean.
- W2-A's `extract_stub` is removed (or remains only as a test-only helper).
- W3-A retro entry in `retro-W3.md` captures any L0-cycle class patterns.

## 12. Reviewer panel

**L0 (this packet):**
- `/codex challenge` — adversarial pass; targets unmapped-fact handling, frontmatter trust, subject-ref bleed.
- `code-reviewer` (architect lens) — confirms file ownership scope; checks `pipeline.rs` edit is bounded to the one-line stub swap.
- `/codex consult` — substrate K-in grep against `CLAIM_TYPE_REGISTRY`, prior `extract_*` helpers, ADR-0107/0108/0124/0125.

**L2 (PR):**
- `/codex review` — diff-bounded; targets the extraction strategy logic and the provenance construction helper.
- `code-reviewer` — diff-bounded.
- `architect-reviewer` — confirms no architectural drift in `pipeline.rs` or `wiring.rs`.

**Skip at L2 unless L0 surfaces a trigger:**
- `/cso` — no new trust-boundary surface introduced; existing `commit_claim` enforces.
- `/plan-devex-review` — no MCP-consumed API in W3-A (DOS-489 / W3-C is the DX-annotated lane).

## 13. Open questions for L0 review

L0 reviewers should resolve these before L1 starts:

1. **Frontmatter field-to-claim-kind map.** Beyond `doc_type` (which W2-A's `auto_detect_category` already consumes for category routing), which frontmatter keys map cleanly to existing `CLAIM_TYPE_REGISTRY` kinds? Specifically: `owner`, `tags`, `source-of-truth`, `confidence`, custom-namespaced fields. L0 should produce a frozen frontmatter→claim-kind table that W3-A implements verbatim.

2. **Entity-JSON schema.** Does v1.4.5 have a canonical entity-JSON shape `WorkspaceExtractor` should parse against, or is this a per-file-kind ad-hoc shape? If canonical, cite the schema definition; if ad-hoc, define the minimum field set W3-A targets and what happens for files with extra fields.

3. **RawObservation chunking policy.** L0 should pin: when does a long body produce one chunked-claim-per-paragraph vs one whole-body claim? Suggested rule in code stub: chunking only for markdown notes + transcripts; everything else gets one body excerpt. Confirm or revise.

4. **`services::feedback` extension.** Is the existing claim-level feedback enough for workspace-file-derived claims, or does W3-A need to surface re-extract/quarantine/relink affordances that wire into source-management UI (W4-A)? If the latter, draft the feedback-extension scope here so W4-A can build against it.

5. **Suite E baseline.** W3 merge gate requires Suite E (bundles 1–18 from v1.4.1) green after ingestion wiring. L0 should confirm which bundles exercise the claim-emission path and whether they need fixture extension for workspace-derived claims, or whether they pass unchanged because `commit_claim` is the unifying write.

6. **Migration:** confirmed none. If L0 finds W3-A needs a schema mutation (e.g., a new column on `intelligence_claims` for workspace-specific metadata), pause for a wave-plan amendment and claim a slot from the v250–v269 block before coding.

## 14. Cross-references

- W1 contracts: `src-tauri/src/services/workspace_ingestion/contracts.rs`
- W2-A pipeline: `src-tauri/src/services/workspace_ingestion/pipeline.rs`
- W2-A L0 packet: `L0-packet-W2-A-DOS-466.md` (shape template for this packet)
- W1 retro: `retro-W1.md`
- W2 retro: `retro-W2.md`
- Wave plan W3 section: `.docs/plans/v1.4.5-waves.md:680-740`
- ADR-0107 (DataSource), ADR-0108 (sensitivity rendering), ADR-0124 (claim substrate), ADR-0125 (envelope-actor binding)
- v1.4.5 wave plan HTML: `.docs/plans/v1.4.5-workspace-memory-waves.html`
