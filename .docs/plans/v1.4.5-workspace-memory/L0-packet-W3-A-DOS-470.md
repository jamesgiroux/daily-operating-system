# L0 Packet — v1.4.5 W3-A — DOS-470 Claim Proposals from Workspace Files

**Current revision:** V1.4 (approved after cycle 6, 2026-05-23). See §2 Changelog and §15-19 L0 results.

## 1. Header

- **Date:** 2026-05-23
- **Project:** v1.4.5 — Workspace Memory Refactor
- **Wave:** W3 stage 3a (W3-A — first real claim producer; merges alone)
- **Issue:** [DOS-470](https://linear.app/a8c/issue/DOS-470) — Convert document-derived facts into claim proposals
- **Branch:** `codex/v1.4.5-w3-a-dos-470`
- **Worktree:** `/Users/jamesgiroux/Documents/dailyos-repo/.worktrees/codex/v1.4.5-w3-a-dos-470`
- **Trust topology:** local-to-local single-user. Workspace files are user-controlled eligible evidence, not automatically authoritative metadata. Placement/action can be trusted as user intent; claim type, subject binding, sensitivity, confidence/trust, source attribution, and actor class remain system-owned policy decisions.
- **Migration:** None. W3-A ships no migration. Consumes W1's v250–v254 schema, the existing claim substrate, and the canonical primitives.
- **Authority docs:**
  - `.docs/plans/v1.4.5-waves.md:680-740` (W3-A section + W3 staging notes; cycle-12 substrate sweep + cycle-13 amendments)
  - `.docs/plans/v1.4.5-workspace-memory/retro-W2.md` (W2 close + `extract_stub` location)
- **L2 reviewer matrix:** `/codex review` + code-reviewer + architect-reviewer + `/cso`. W3-A converts user-controlled workspace content into durable claims, so claim/provenance/sensitivity/actor handling gets an explicit security lane even though there is no new external endpoint.
- **Pacing note:** Stage 3a is W3-A alone. W3-B (signal wiring) and W3-C (graph projection) cannot start until W3-A merges; their producers consume W3-A's claim output.
- **Coordination note:** W3-A intentionally avoids `abilities/claims.rs`, new `ClaimType` variants, `account_fact_claims.rs`, and `trust_recompute.rs` so it does not conflict with PR #366. If PR #366 lands first, rebase and use its generic recompute hook where available; do not duplicate that work in this lane.

## 2. Changelog

- **V1.0 — 2026-05-23:** Initial L0 packet. Drafted post-W2-close as the first W3 packet; cites W1's `contracts.rs` trait surface and `CLAIM_TYPE_REGISTRY` as load-bearing K-in.
- **V1.1 — 2026-05-23:** Cycle-2 amendment candidate after unanimous HOLD. Removes the nonexistent `RawObservation` fallback, authorizes the minimum contract/pipeline/context changes needed to commit claims, adds `/cso`, scopes W3-A away from PR #366 registry/trust-recompute files, and narrows binary/attachment handling to already-extracted UTF-8 text.
- **V1.2 — 2026-05-23:** Cycle-3 amendment candidate after a mixed cycle-2 result. Pins single-`ActionDb` commit wiring, narrows `UserNote` to account/project/person subjects, defines canonical subject JSON conversion, changes stale-source acceptance from trust-band degradation to freshness classification on current `dev`, and freezes the no-migration `error_log` JSON schema for drops/warnings/partial commit failures.
- **V1.3 — 2026-05-23:** Cycle-4 amendment candidate after cycle-3 mixed result. Amends the wave plan source of truth, removes the local `initial_source_reliability` trust seed from W3-A, requires canonical DB/link-backed subject resolution before commit, fixes unsupported-format attribution to W2-A `pipeline.rs`, and sets a `UserOnly` sensitivity floor for body-derived workspace notes unless a trusted service-owned policy explicitly lowers it.
- **V1.4 — 2026-05-23:** Cycle-5 amendment candidate after cycle-4 substrate HOLD. Replaces stale legacy-runtime consumer claims with live claim-aware ability/runtime consumers and removes `intelligence/io.rs` from W3-A ownership; prior extraction helpers remain read-only K-in only.

## 3. Goal (verbatim from wave plan §Agent W3-A)

> Extend `services::workspace_ingestion` with the claim extraction logic: given file content (structured JSON/markdown entity files, or extracted text from Drive/attachment files), produce typed `WorkspaceClaimProposal` records (the type is **W1-A-owned in `contracts.rs`** per cycle 6 promotion) using the existing `CLAIM_TYPE_REGISTRY`. This is the "intelligence" half of the ingestion pipeline — converting raw file content to structured claim proposals.

**Restating in plain terms:** W2-A shipped the pipeline shell with a `NullExtractor`, but live `pipeline.rs` already calls the extractor and discards the proposals. W3-A replaces that placeholder with a real extractor and the minimum pipeline-side commit orchestration required to preserve, validate, commit, and count supported proposals. W3-A only emits existing registered claim types. There is no generic `RawObservation` fallback in the current registry.

**V1.3 scope clarification:** W3-A handles validated UTF-8 workspace content. Raw PDF/DOCX/PPTX/binary extraction remains out of scope unless W2 adds a safe pre-extraction layer that passes extracted UTF-8 text through the same file-handle/identity protections. Attachment-derived text is in scope only after it is already represented as validated text content.

## 4. Files owned (exclusive)

- **`src-tauri/src/services/workspace_ingestion/extract.rs`** — owns `WorkspaceExtractor` and private parsing/mapping helpers. Extraction is pure content work: it reads the already-opened file handle and returns proposals or an extraction error/report. It does not open the DB, write claims, emit signals, or infer subject from frontmatter.
- **`src-tauri/src/services/workspace_ingestion/contracts.rs`** — the active V1.4 amendment authorizes the minimum contract changes W3-A needs: add an extraction context carrying system-owned metadata, make extraction fallible, remove the local `initial_source_reliability` seed, and ensure `WorkspaceClaimProposal` can convert cleanly into `services::claims::ClaimProposal`.
- **`src-tauri/src/services/workspace_ingestion/pipeline.rs`** — the active V1.4 amendment authorizes bounded orchestration changes: pass extraction context, resolve linked subjects from canonical DB/link state, preserve returned proposals, commit supported proposals through `commit_claim`, record the actual committed count, fail extractor/commit-error runs, and record `partial_commit: true` in `error_log` only when a commit error occurs after one or more successful commits. No category/lifecycle/registry ownership changes.
- **`src-tauri/src/services/workspace_ingestion/workspace_intake_impl.rs`** — the active V1.4 amendment authorizes context propagation into the blocking ingestion path. The adapter must gate non-Live modes before write work, carry a service-owned actor/observed-at snapshot, treat request entity DTOs as hints only, and preserve the original ability actor in metadata.
- **`src-tauri/src/services/workspace_ingestion/wiring.rs`** — swaps `NullExtractor` for `WorkspaceExtractor` in `build_pipeline()`. W3-B's later `NullSignalEmitter -> WorkspaceSignalEmitter` patch remains mechanically separate.
- **Tests under `src-tauri/tests/workspace_ingestion_*` and `extract.rs` unit tests** — W3-A owns the new regression coverage described in §10.

## 5. Don't touch

- **`services/claims.rs`** — caller, not modifier. Extraction commits through `commit_claim`; the claim service's own logic stays untouched.
- **`abilities/claims.rs::CLAIM_TYPE_REGISTRY`** — read-only in this lane. W3-A does not add `RawObservation` or any other claim type. A generic observation claim requires a separate ADR/L0 registry amendment and coordination with PR #366.
- **`services/account_fact_claims.rs`, `services/trust_recompute.rs`, `services/trust_extraction.rs`** — owned by PR #366 / runtime producer remediation. W3-A must not duplicate or edit those files. If they land first, W3-A may call an existing generic recompute hook only after rebasing and updating this packet.
- **`services/workspace_ingestion/{lifecycle,registry,runs,link,signals,graph,mod}.rs`** — all owned by other lanes unless this packet explicitly names a call-site adjustment. W3-A does not change lifecycle state definitions, path validation, category allowedness, link semantics, signal policy, or graph projection.
- **`src-tauri/src/intelligence/io.rs` and legacy `processor/**` extraction helpers** — read-only K-in only. W3-A may mine patterns but must not introduce path-reopening extraction work there; extraction reads the already-opened validated file handle.
- **`signals/policy_registry.rs` + `signals/invalidation.rs`** — W3-B's exclusive write surface. W3-A does not emit signals directly; signals flow from `commit_claim` per existing v1.4.1 infrastructure.
- **`signals/bus.rs`** — v1.4.1 owns the signal bus.
- **Markdown / preview rendering paths** — W4-B owns the sanitizer + preview block. W3-A's extraction reads file content but does not render it.

## 6. K-in substrate audit

### Substrate primitives W3-A consumes

| Primitive | Location | Use |
|---|---|---|
| `Extractor` trait | `services/workspace_ingestion/contracts.rs` (V1.4 amended by W3-A) | W3-A implements `WorkspaceExtractor` against this surface |
| `ExtractionContext` | `services/workspace_ingestion/contracts.rs` (new W3-A contract amendment) | System-owned file id, source_asof, resolved category, linked subject, run id, observed_at, and actor snapshot |
| `WorkspaceClaimProposal` | `services/workspace_ingestion/contracts.rs` (V1.4 amended by W3-A) | Output type of `Extractor::extract`; conversion input for `services::claims::ClaimProposal`; W3-A removes the unused local `initial_source_reliability` seed |
| `FileIdentity` | `services/workspace_ingestion/contracts.rs` (W1-A) | Identity carried through to provenance |
| `ClaimType` | `abilities_runtime::abilities::claims::CLAIM_TYPE_REGISTRY` | Closed set for claim-type resolution; no registry mutations in W3-A |
| `ClaimSensitivity` | `abilities_runtime::types` (per ADR-0125) | Sensitivity tier applied per claim |
| `SourceAttribution` | `abilities_runtime::abilities::provenance::source::SourceAttribution` (`:303`) | Canonical 7-field provenance; cycle-12 amendment confirmed W3-A consumes the canonical, not a local mirror |
| `SourceIdentifier::Document { document_id, chunk_id }` | `abilities_runtime::abilities::provenance::source` | Source identifier shape for workspace-file-derived claims |
| `DataSource::WorkspaceFile { kind: WorkspaceFileKind }` | per ADR-0107 and the live provenance substrate | DataSource tag carried on every emitted proposal |
| `commit_claim` | `services::claims::commit_claim` | Write path for proposals; W3-A is the caller, never a direct claim writer |
| existing trust behavior | `services::claims::commit_claim`; PR #366 `trust_recompute` when landed | W3-A does not seed arbitrary trust scores. Current branch gives `UserNote` an initial trust score; W3-A therefore tests stale `freshness`/`source_asof` behavior, not trust-band degradation, unless PR #366 lands first with a generic recompute hook |
| `IngestionRunId` | W1-C `services/workspace_ingestion/runs.rs` | Separate from `source_attribution`; carried as a parallel field for run-linkage |

### K-in obligation (grep before scoring)

L0 reviewers MUST grep before approving:

- `docs/solutions/**` for prior extraction patterns. Specifically: any existing `extract_*` helpers in `intelligence/io.rs`, any prior frontmatter parsing in `processor/router.rs` or the archived `prepare_inbox.py`, and any prior content-sniffing logic that should be reused rather than reinvented.
- `.docs/decisions/**` for ADRs that govern claim production. ADR-0107 (DataSource taxonomy), ADR-0108 (sensitivity rendering), ADR-0125 (envelope-actor binding), ADR-0124 (claim substrate allowances).
- `abilities_runtime/src/abilities/claims.rs` for the live closed registry. Review must confirm every W3-A claim type is already registered.
- `processor/` (the legacy path destined for v1.4.6 consolidation per DOS-770) for any existing fact-extraction logic worth mining, not reinventing.

### V1.3 contract amendment

The live W1/W2 surface is insufficient for W3-A because `Extractor::extract(&File, &FileIdentity, WorkspaceFileKind)` cannot populate subject, source_asof, run id, category, observed_at, or actor metadata. V1.3 amends the contract:

- Add `ExtractionContext` with `file_id`, `source_asof`, `source_type`, `resolved_category`, `linked_subject: Option<ResolvedLinkedSubject>`, `ingestion_run_id`, `observed_at`, and a sanitized invocation actor label.
- Change `Extractor::extract` to receive the context and return `Result<ExtractionReport, ExtractionError>`, where the report includes `proposals`, `dropped_facts`, and non-fatal warnings.
- Remove `initial_source_reliability` from `WorkspaceClaimProposal`. It is not consumed by `commit_claim` on current `dev`, and W3-A must not create a parallel local trust seed. Source freshness, provenance, sensitivity, and PR #366's generic trust recompute hook (if landed first) are the only trust inputs W3-A uses.
- Pipeline owns enrichment and commit orchestration. Frontmatter/body content never sets subject, actor, trust, sensitivity, source_asof, or claim type directly.
- Requests without a linked subject may ingest the file but must commit zero claims and record that no subject-bound claim was eligible.
- Extractor errors are distinct from intentional zero-claim results. Required structured sources fail the run before any commit attempt; optional unsupported content records zero proposals with a reason.

## 7. Security gate

**`/cso` is required at L2.** W3-A writes go through `commit_claim`, but this lane still converts user-controlled local content into durable claims. The security review targets actor mapping, prompt-injection-shaped content, frontmatter abuse, provenance/source references, sensitivity policy, and subject binding.

**What L0 reviewers should still check:**
- **Subject scoping.** Extracted claims must populate `subject_ref` only from canonical DB/link state. The request `entity_ref` DTO is a placement hint, not claim authority; W3-A must verify entity existence and id/name consistency, then create/read an active `document_entity_links` row before any claim commit. Cross-entity bleed (a workspace file linked to account A producing a claim attributed to account B) is a Suite-E and Suite-S concern.
- **Content disclosure.** Extracted claim text must not surface raw file content for sensitivity-elevated source types without the freshness/trust rendering controls required by ADR-0108.
- **Frontmatter trust.** Frontmatter fields are user-authored and not signed. `doc_type` may drive category/routing only. `confidence`, `source-of-truth`, `subject_ref`, `claim_type`, `sensitivity`, `actor`, `data_source`, and `source_asof` are ignored as metadata authority and may only be quoted as non-authoritative note content when the chosen claim type permits it.
- **Path leakage.** Extraction must not record file paths in claim text or `source_attribution` user-facing fields. `SourceIdentifier::Document { document_id }` uses opaque `file_id`, never path. Suite S check.
- **Binary content.** W1-B `open_validated` is path/race validation only. W2-A `IngestPipeline` rejects NUL/invalid UTF-8/unsupported text formats after the safe file handle is opened; extraction sees only validated text content. W3-A does not need to re-detect binary, but should defensively handle the case where `IngestPipeline` passes through a file with surprising encoding.

## 8. Intelligence Loop gate

The 5 mandatory questions from CLAUDE.md, answered for W3-A:

**1. Claim model.** Every committed proposal is a `WorkspaceClaimProposal` whose claim type is already present in `CLAIM_TYPE_REGISTRY`. W3-A does not add `RawObservation`. Unmapped body text has exactly two outcomes: a narrow `UserNote` claim when the file is a user/system note attached to a canonical DB/link-backed `Account`, `Project`, or `Person`, or a dropped-fact report entry when no registered claim type safely applies. Linked `Other` subjects are not eligible for `UserNote` and must drop with `unsupported_subject_kind`. Dropped facts are counted and tested; they are not silently treated as successful claim extraction.

**2. Provenance + trust.** Each proposal carries:
- `source_asof`: file mtime captured by workspace intake (`request.source_asof`), never caller/frontmatter-provided
- `observed_at`: service clock at ingestion/commit time, captured once per run
- `data_source`: ADR-0107 `DataSource::WorkspaceFile { kind: WorkspaceFileKind }` from the request
- `source_attribution`: canonical `SourceAttribution::new(DataSource::WorkspaceFile { kind }, vec![SourceIdentifier::Document { document_id, chunk_id }], observed_at, Some(source_asof), evidence_weight, None)`
- `ingestion_run_id`: parallel field for run-linkage (separate from `source_attribution`)
- `sensitivity`: body-derived workspace notes default to `UserOnly` so raw local note content does not enter Agent/MCP prompt readers. Frontmatter `privacy` cannot lower sensitivity. A future trusted category/source policy may lower sensitivity only through service-owned code with tests; W3-A ships no arbitrary lowering policy.
- trust behavior: W3-A does not invent trust scores. On current `dev`, `commit_claim` gives `UserNote` an initial `0.85` trust score, and `get_entity_intelligence` renders trust band from that score while exposing freshness separately. Therefore W3-A acceptance on current `dev` verifies `source_asof` and stale/aging/current freshness classification, not trust-band degradation. If PR #366's generic trust recompute service lands first, W3-A wires workspace commits into that existing service by amendment instead of adding a second recompute path.

**3. Signals + invalidation.** W3-A does not emit workspace signals directly. `commit_claim` performs claim-version invalidation for committed claims. W3-B's `WorkspaceFileIngested` signal fires at the pipeline boundary, not from `extract.rs`.

**4. Runtime + surfaces.** Committed proposals are consumed by live claim-aware ability/runtime readers, including `get_entity_context` / entity-context claim reads, `get_entity_intelligence`, `account_overview`, `list_open_loops`, and prepare/daily synthesis paths that already read committed claims. W3-A must populate `subject_ref` correctly so those readers find the claims. Body-derived workspace `UserNote` claims are `UserOnly`; Agent/MCP prompt readers filter them out unless a later trusted policy intentionally lowers sensitivity. W3-A does not promise legacy `build_intelligence_context()` / `gather_account_context()` prompt inclusion unless those readers already consume claim rows.

**5. Feedback loop.** User corrections to a workspace-derived claim propagate through existing claim-level feedback. Re-extract, quarantine, and relink are W4-A/W5 source-management affordances and are not required for this first producer.

## 9. Cycle-3 implementation shape

This is a plan packet, not final code, but L1 should follow this shape unless a reviewer amends it.

### Extraction context

Add the context object in `contracts.rs`, together with whatever small subject-ref type move/re-export is needed to avoid a `pipeline.rs` dependency from `contracts.rs`:

```rust
pub struct ExtractionContext<'a> {
    pub file_id: &'a str,
    pub identity: &'a FileIdentity,
    pub source_type: WorkspaceFileKind,
    pub source_asof: DateTime<Utc>,
    pub resolved_category: Option<&'a WorkspaceCategory>,
    pub linked_subject: Option<&'a ResolvedLinkedSubject>,
    pub ingestion_run_id: &'a str,
    pub observed_at: DateTime<Utc>,
    pub invocation_actor: &'a str,
}
```

`linked_subject` is the only source of `subject_ref`, and it is already verified against canonical DB/link state before extraction. The extractor must not infer a different subject from path, body text, YAML frontmatter, JSON fields, file title, account names, or domains.

### Subject/link resolution

- `workspace_intake_impl::parse_entity` remains DTO validation only. It may reject malformed entity type/id/name shapes, but it does not make the subject trusted.
- Before constructing `ExtractionContext`, pipeline resolves a `ResolvedLinkedSubject` from DB state. For entity-seeded intake, it verifies the target entity exists in the canonical entity table, verifies the caller-provided entity name matches the canonical stored entity name/path segment, then creates or reads an active `document_entity_links` row through `LinkRepo` with `LinkAttributionSource::EntityIntake`.
- For files with no verified active link, extraction may still ingest the file and mark it pending entity assignment, but commits zero claims with `unlinked_subject`.
- For a stale, nonexistent, rejected, mismatched, or `Other` entity link, W3-A commits zero claims and records the typed drop/rejection reason. It never commits against a raw request DTO.

### Claim-type policy

- `UserNote` is the only generic text-body claim W3-A may emit. It is allowed only when the file has a linked `Account`, `Project`, or `Person`, the resolved category/source kind is note-like user/system-authored content, and the body text is suitable to quote as a user/system note. It is not a fallback for arbitrary unsupported facts.
- Linked `Other`, `Meeting`, `Email`, global, multi, or unknown subjects are not eligible in W3-A. They ingest the file but commit zero `UserNote` claims and record `unsupported_subject_kind`.
- Existing typed claim kinds may be emitted only from explicit structured evidence that maps to a registered claim type and subject kind. L1 must grep `CLAIM_TYPE_REGISTRY` before adding a mapping.
- If no registered claim kind safely applies, the extractor returns a dropped-fact entry with a reason. It does not invent a kind, use `AccountFact` from PR #366, or promote unknown content to a durable assertion.
- W3-A does not seed arbitrary trust scores. Trust/freshness comes from the canonical claim substrate on current `dev`; if PR #366 lands first, W3-A may call its generic recompute hook after rebase instead of adding a duplicate recompute path.

### Sensitivity policy

- Body-derived workspace `UserNote` claims default to `ClaimSensitivity::UserOnly`, even though `UserNote` registry metadata defaults to `Internal`.
- Frontmatter `privacy`, `sensitivity`, `confidential`, or similar fields cannot lower this floor. They may only raise sensitivity if W3-A implements a service-owned raise-only mapping.
- W3-A ships no service-owned lowering policy. If a future trusted source/category policy lowers sensitivity to `Internal`, it must be explicit code with tests proving Agent/MCP prompt-reader eligibility is intentional.
- Structured deterministic claims, if any are added in W3-A, must choose the stricter of the registry default and the workspace sensitivity floor unless a trusted service-owned policy says otherwise.

### Frontmatter policy

| Field shape | W3-A behavior |
|---|---|
| `doc_type` | Category/routing input only; no claim metadata authority |
| `privacy` | May influence sensitivity only through an existing trusted category policy; otherwise ignored as metadata authority |
| `confidence`, `source-of-truth`, `trust`, `evidence_weight` | Ignored as authority; may be quoted only inside a permitted `UserNote` body |
| `subject_ref`, `account`, `company`, `domain`, `owner`, `actor` | Never used for subject or actor binding |
| unknown/custom keys | Ignored as metadata authority and counted only as dropped evidence if the parser otherwise sees a candidate fact |

### Pipeline commit flow

1. `workspace_intake_impl` stops discarding `AbilityContext` as `_ctx`.
2. Before starting blocking write work, the adapter gates non-Live modes with `ctx.services().check_mutation_allowed()` and captures a service-owned snapshot: observed time, sanitized ability actor label, `ctx.mode()`, `ctx.services().ability_id`, and any request-scoped metadata needed by the pipeline. A non-Live call returns a typed intake error before opening the DB.
3. `ingest_sync` opens exactly one `ActionDb`. It passes `&ActionDb` and `&ServiceContext` to `IngestPipeline::run`; the pipeline uses `db.conn_ref()` for lifecycle/run reads and `commit_claim(&ServiceContext, &ActionDb, proposal)` for claim writes. W3-A must not open a second DB connection in the commit loop and must not pass a bare `&Connection` to the claim commit path.
4. If the original `ServiceContext` cannot cross `spawn_blocking`, the blocking closure builds a short-lived service-owned commit context only after the pre-gate above, using `ServiceContext::new_live(...).with_actor("system:workspace_ingestion")` and the captured ability id when available. The original ability actor is stored in `metadata_json`, not used as claim authority. If PR #355 lands first, prefer its request-scoped context pattern after rebase.
5. `IngestPipeline::run` opens and validates the file exactly as W2-A does today; no path re-open is added in `extract.rs`.
6. After registry/category/link resolution, pipeline constructs `ExtractionContext` with the opaque file id, source_asof from the request/file identity, linked subject resolved from canonical DB/link state, ingestion run id, observed_at from the service snapshot, and sanitized invocation actor.
7. Pipeline calls `extractor.extract(&mut file, &context)` and keeps the returned `ExtractionReport`.
8. Pipeline pre-validates all proposals into commit-ready `services::claims::ClaimProposal` values before the first `commit_claim` call. Pre-validation covers claim type, actor class, allowed subject kind, canonical subject JSON, provenance JSON, `source_ref`, `metadata_json`, and path-leak guards.
9. Pipeline commits proposals through `commit_claim(&ServiceContext, &ActionDb, proposal)`. Extraction/conversion happen outside any bounded DB writer closure; claim service writes are the only claim DB writes.
10. Pipeline records the actual committed claim count, dropped-fact count, and warnings on the ingestion run using the `error_log` JSON contract below. A returned proposal that fails pre-validation records zero commits. A `commit_claim` failure after one or more successful commits marks the run `failed` with `partial_commit: true`; there is no new partial status in W3-A.
11. `wiring.rs` swaps `NullExtractor` for `WorkspaceExtractor`; `NullExtractor` remains test-only where useful.

### Commit conversion contract

`WorkspaceClaimProposal` must not serialize the abilities provenance tuple-style `SubjectRef` directly into `services::claims::ClaimProposal.subject_ref`. W3-A either replaces the extraction subject field with an explicit commit-subject JSON field or implements a single conversion helper that derives commit JSON from the pipeline-owned `EntityRef`:

| Linked entity type | Commit `subject_ref` JSON |
|---|---|
| `Account` | `{"kind":"account","id":"<entity_id>"}` |
| `Project` | `{"kind":"project","id":"<entity_id>"}` |
| `Person` | `{"kind":"person","id":"<entity_id>"}` |
| `Other` / unsupported | drop with `unsupported_subject_kind`; no commit attempt |

`source_ref` is `workspace_file:<file_id>` plus an optional `#chunk:<chunk_id>` suffix if chunking is later added. It never contains a filesystem path, file name, account name, domain, or title.

`metadata_json` for committed workspace claims is a JSON object with this minimum shape:

```json
{
  "producer": "workspace_ingestion",
  "ingestion_run_id": "<run_id>",
  "workspace_file_id": "<file_id>",
  "workspace_file_kind": "<workspace_file_kind_slug>",
  "resolved_category": "<category_slug|null>",
  "original_ability_actor": "<sanitized actor label>",
  "sensitivity_floor": "user_only",
  "schema_version": 1
}
```

### Run `error_log` JSON contract

No migration is added. W3-A stores drops, warnings, and commit failures in `document_ingestion_runs.error_log` as JSON when anything needs explanation. `error_log` may be `NULL` only when extraction produced and committed at least one claim with no drops/warnings/errors, or when there were intentionally no candidate facts and no warnings.

Minimum schema:

```json
{
  "schema_version": 1,
  "producer": "workspace_ingestion",
  "proposal_count": 0,
  "committed_count": 0,
  "dropped_count": 1,
  "partial_commit": false,
  "dropped_facts": [
    {
      "reason": "unlinked_subject|unsupported_subject_kind|unsupported_claim_type|empty_body|frontmatter_authority_ignored|unsupported_content_shape",
      "source": "frontmatter|body|structured",
      "field": "optional field/key/path",
      "claim_type_candidate": "optional registered-or-rejected type"
    }
  ],
  "warnings": [
    { "code": "frontmatter_authority_ignored", "message": "frontmatter authority fields ignored" }
  ],
  "commit_errors": [
    { "claim_type": "user_note", "reason": "commit_claim error string" }
  ]
}
```

Status mapping:
- `success` with `committed_count > 0`: normal committed run; `error_log` present only for warnings/drops.
- `success` with `committed_count == 0` and `dropped_count > 0`: intentional zero-claim run; `error_log` required so it is not indistinguishable from the old null-extractor success.
- `failed`: extractor error, commit pre-validation error that should fail the file kind, or any `commit_claim` error. If any claim committed before the failure, `partial_commit` is `true`.

## 10. Tests required

### Unit tests

- `frontmatter_doc_type_routes_only_does_not_create_claim` — `doc_type` can influence category/routing, but does not set claim type, trust, subject, actor, or sensitivity.
- `frontmatter_authority_fields_are_ignored` — `confidence`, `source-of-truth`, `subject_ref`, `actor`, and `source_asof` in YAML do not affect emitted proposal metadata.
- `unlinked_subject_commits_zero_claims_with_drop_reason` — extractor records why no subject-bound claim was eligible.
- `linked_other_subject_drops_as_unsupported_subject_kind` — a linked `Other` entity does not attempt `UserNote` commit.
- `note_body_can_emit_user_note_for_linked_subject` — linked note-like content emits a `UserNote` proposal with capped text and canonical metadata.
- `workspace_note_body_defaults_user_only` — body-derived `UserNote` proposals use `ClaimSensitivity::UserOnly` on current W3-A.
- `frontmatter_privacy_cannot_lower_sensitivity` — user-authored frontmatter cannot lower the body-derived sensitivity floor.
- `commit_subject_json_uses_object_shape_not_tuple_subject_ref` — conversion returns `{"kind":"account|project|person","id":"..."}` and never direct-serdes the provenance `SubjectRef`.
- `unsupported_fact_is_dropped_not_promoted` — unsupported body/frontmatter content returns a dropped-fact entry, not a claim with an invented type.
- `proposal_provenance_carries_all_required_fields` — emitted proposals carry source_asof, observed_at, data_source, source_attribution, ingestion_run_id, canonical commit subject JSON, sensitivity, and claim type.
- `no_claim_text_contains_file_path` — source path never leaks into claim text or user-facing provenance fields.

### Integration tests

- `fake_extractor_two_proposals_commits_and_counts_two` — regression for the current discard bug; returned proposals are committed and run count is two.
- `fake_extractor_error_fails_run_with_error_log` — extractor failure is not recorded as success with zero proposals.
- `workspace_extractor_note_flows_to_committed_claim` — fixture file flows through the real pipeline and lands one committed `UserNote` claim.
- `linked_subject_must_resolve_from_active_link` — request DTO alone is insufficient; claims commit only after canonical entity/link resolution.
- `entity_name_mismatch_does_not_commit_claim` — stale or mismatched entity hints do not create claims for the supplied id.
- `claim_subject_ref_matches_linked_entity_only` — cross-entity bleed regression.
- `frontmatter_subject_override_does_not_change_subject_ref` — malicious frontmatter cannot retarget a linked file.
- `agent_mcp_prompt_reader_excludes_workspace_user_only_note` — body-derived workspace notes do not become prompt input for Agent/MCP readers.
- `old_file_source_asof_renders_stale_freshness_on_current_dev` — validates `source_asof` drives stale freshness classification on current `dev`; if PR #366 lands first, this test is amended to validate the generic recompute hook and trust-band downgrade.
- `ingestion_run_records_claim_and_drop_counts` — run metadata distinguishes committed claims from intentionally dropped facts.
- `ingestion_run_error_log_schema_records_zero_claim_drops` — zero claims with dropped facts stores the schema-versioned `error_log`, not a null success indistinguishable from the old null extractor.
- `pipeline_uses_single_action_db_for_lifecycle_and_claim_commit` — regression guard that `pipeline.run` receives `&ActionDb` and does not open a second DB in the commit loop.

### CI gate activations

- Existing claim-writer allowlist must remain green; W3-A does not write `intelligence_claims` directly.
- `check_content_index_allowlist.sh` activation remains in scope only if the wave-plan invariant table still assigns it to W3-A after rebase. It must not expand this lane into content-index behavior.

## 11. Done when

- `WorkspaceExtractor` is wired as the production extractor and emits only existing registered claim types.
- The extractor receives `ExtractionContext` and never derives subject, actor, trust, sensitivity, source_asof, or data source from user-authored frontmatter/body text.
- `pipeline.rs` preserves extraction output, receives `&ActionDb` and `&ServiceContext`, resolves subjects from canonical DB/link state, commits proposals through `commit_claim` on the same `ActionDb` used for lifecycle/run bookkeeping, and records actual committed/dropped counts.
- `workspace_intake_impl.rs` propagates/gates service context instead of silently opening a hidden write path without actor/mode awareness.
- Tests prove returned proposals are committed and counted, extraction errors are distinct from zero-claim success, request DTOs alone cannot anchor claim subjects, `Other` linked subjects drop safely, canonical commit subject JSON is object-shaped, body-derived notes are `UserOnly`, `error_log` carries dropped-fact details, and frontmatter cannot override subject/provenance/trust/sensitivity metadata.
- No SQL migration, registry amendment, `AccountFact` dependency, `trust_recompute` duplicate, direct DB write, path leak, or raw file rendering is introduced.
- Required checks for L1: focused unit/integration tests for W3-A, `cargo clippy --lib -- -D warnings`, `cargo test --lib`, and `pnpm tsc --noEmit` if TypeScript surfaces or generated bindings are touched.
- W3-A retro entry captures any L0-cycle class patterns.

## 12. Reviewer panel

**L0 (this packet):**
- `/codex challenge` — adversarial pass; targets unmapped-fact handling, context propagation, pipeline commit orchestration, subject-ref bleed, and PR #366 overlap.
- `/plan-eng-review` or architect lens — confirms the active ownership expansion is the minimum viable change and does not take W3-B/W3-C work.
- `/codex consult` — substrate K-in grep against `CLAIM_TYPE_REGISTRY`, `commit_claim`, `SourceAttribution`, prior extract helpers, ADR-0107/0108/0124/0125, PR #366, PR #365, and PR #355.
- `/cso` or security lane — required because W3-A converts user-controlled local content into durable claims.

**L2 (PR):**
- `/codex review` — diff-bounded; targets extractor policy, provenance construction, commit orchestration, and run-count correctness.
- `code-reviewer` — diff-bounded implementation review.
- `architect-reviewer` — confirms W3-A only takes the approved contract/pipeline/context amendments.
- `/cso` — validates actor mapping, frontmatter abuse resistance, source/path disclosure, source_asof/freshness/trust behavior, and subject isolation.

`/plan-devex-review` is not required unless W3-A changes MCP/tool API shape.

## 13. Cycle-4 decisions for review

1. **Fallback policy:** W3-A uses narrow `UserNote` emission for linked account/project/person note-like content and otherwise records dropped facts. It does not add `RawObservation`, use PR #366's `AccountFact`, emit `UserNote` for linked `Other`, or create another generic fallback claim.
2. **Structured extraction:** W3-A may add small registered-kind mappings only when the source field is explicit, deterministic, subject-bound, and already covered by the live registry. No LLM inference, source-of-truth frontmatter, or broad entity-JSON schema is introduced in this lane.
3. **Feedback:** existing claim-level feedback is enough for W3-A. Re-extract, quarantine, relink, and source-management UI remain W4-A/W5 work.
4. **Suite E:** existing claim-substrate bundles are not enough by themselves; W3-A adds workspace-specific commit/provenance/subject/run-count fixtures.
5. **Migration:** none. If L1 discovers a schema or registry mutation is required, pause and amend the wave plan before coding.
6. **Substrate coordination:** W3-A avoids PR #366-owned files and registry changes. If PR #366 lands before implementation finishes, rebase and amend only to call its generic recompute hook where the hook is already available.
7. **Current-dev freshness vs trust:** W3-A verifies stale `source_asof` through freshness classification on current `dev`. It does not claim trust-band degradation for `UserNote` until the generic trust recompute substrate exists.
8. **Subject authority:** request entity DTOs are hints. The claim subject comes from canonical entity existence plus an active `document_entity_links` row.
9. **Sensitivity floor:** body-derived workspace notes default to `UserOnly`; no frontmatter-controlled lowering.

## 14. Cross-references

- W1 contracts: `src-tauri/src/services/workspace_ingestion/contracts.rs`
- W2-A pipeline: `src-tauri/src/services/workspace_ingestion/pipeline.rs`
- W2-A L0 packet: `L0-packet-W2-A-DOS-466.md` (shape template for this packet)
- W1 retro: `retro-W1.md`
- W2 retro: `retro-W2.md`
- Wave plan W3 section: `.docs/plans/v1.4.5-waves.md:680-740`
- ADR-0107 (DataSource), ADR-0108 (sensitivity rendering), ADR-0124 (claim substrate), ADR-0125 (envelope-actor binding)
- v1.4.5 wave plan HTML: `.docs/plans/v1.4.5-workspace-memory-waves.html`

## 15. L0 review result - 2026-05-23

**Historical V1.0 verdict: HOLD.** All review lanes found implementation blockers. Sections 3-13 have since advanced to the active V1.4 cycle-5 amendment candidate; reviewers should score the current sections, not the superseded V1.0 sketch.

### Review lanes run

- Codex challenge: HOLD.
- Codex consult / K-in check: HOLD.
- Architecture strategist: HOLD.
- Adversarial document review: HOLD.

### GitHub PR / incoming substrate context checked

- PR #366, `[codex] Add claim-backed account facts and producer plan`, is the most relevant incoming substrate. Its local plan at `/private/tmp/dailyos-account-fact-claims/.docs/plans/abilities-runtime-producer-remediation-waves.html` establishes a producer rule W3-A should adopt: deterministic read-model evidence may remain a section, but durable assertions must become claims and trigger trust recomputation in the same wave. It also requires render-policy coverage for MCP-projected fields.
- PR #366 adds an `AccountFact` claim producer path, but it does not add `RawObservation`. W3-A cannot treat `RawObservation` as an existing registry fallback.
- PR #365, `fix: stabilize foreground database contention`, reinforces DB-worker discipline: compute outside writer closures, keep writer closures DB-only and bounded, and avoid foreground read paths causing writes. If W3-A adds claim commit orchestration inside ingestion, it must name transaction boundaries and avoid long extraction work inside a DB writer.
- PR #355, `feat(mcp_v2): McpToolHandler request-scoped context + re-entrancy guard`, reinforces request-scoped context passing and bans hidden handler-local `ActionDb::open()` patterns. W3-A should not fabricate DB/service context inside extraction.

### Blocking findings

1. **`RawObservation` is not a registered claim kind.** The live `ClaimType` registry does not contain `RawObservation`, and PR #366 adds `AccountFact`, not `RawObservation`. The packet must either add a real registry amendment for a new fallback kind, or replace every `RawObservation` fallback/test with an existing registered kind and explicit actor/subject semantics.

2. **The extractor contract cannot populate the packet's proposal fields.** The live `Extractor` trait receives only `&File`, `&FileIdentity`, and `WorkspaceFileKind`. It does not receive `IngestRequest`, resolved entity, category, `file_id`, `source_asof`, ingestion run id, actor, or service context. The packet's sketch depends on those fields.

3. **The live pipeline discards extractor output and records success with zero proposals.** `IngestPipeline::run` already calls the extractor, stores `_discarded_proposals`, sets `claim_proposals = Vec::new()`, and completes the run with claim count `0`. Replacing a non-existent `extract_stub()` is not the work; W3-A needs an explicit pipeline/commit orchestration amendment or a prerequisite W2 follow-up.

4. **`commit_claim` integration is not wired.** `commit_claim` requires `ServiceContext` and `ActionDb`; the pipeline currently has a `&Connection`, and `workspace_intake_impl` drops the ability context as `_ctx`. The packet's claim that existing `commit_claim` actor enforcement covers W3-A is unsupported until context propagation is designed.

5. **Wave ownership conflicts with the implementation needed.** The wave plan says W3-A only swaps `wiring.rs` and does not touch `pipeline.rs`; the packet says W3-A patches `pipeline.rs`, and live code needs more than a one-line call-site swap. Amend W2/W3 ownership before L1.

6. **Provenance/source shape is stale.** The packet uses non-live helper names and an incorrect `SourceAttribution::new` shape. The live constructor takes `DataSource`, identifiers, `observed_at`, optional `source_asof`, evidence weight, and synthesis marker. `source_asof` should come from `request.source_asof`; `observed_at` should be the ingestion/commit clock.

7. **Frontmatter trust policy is unresolved.** Frontmatter is user-authored and must not set subject, claim type, trust, confidence, sensitivity, actor, evidence weight, data source, or `source_asof`. `doc_type` may drive category/routing only. Unknown keys should be ignored or treated as non-authoritative content, not auto-promoted to claims.

8. **Subject bleed is not structurally prevented.** `commit_claim` validates claim type against subject kind, but it does not prove the subject id matches the file's linked entity. W3-A must make the linked subject a pipeline-owned extraction context input or pipeline-side enrichment step, not a parser inference from frontmatter/body text.

9. **The test plan can miss silent zero-proposal success.** The current failure mode is green ingestion with zero committed claims even if an extractor would have returned proposals. W3-A needs fake-extractor regression tests that assert returned proposals are committed and counted, and extractor errors produce failed or partial-failed runs rather than indistinguishable zero-proposal success.

10. **L2 needs `/cso`.** Turning untrusted local workspace content into committed claims, with actor/context propagation and frontmatter sanitization, is a trust-boundary change even without a new MCP endpoint.

### L0 answers to packet open questions

1. **Frontmatter map:** mostly no claims. Existing frontmatter conventions include `area`, `account`, `business_unit`, `doc_type`, `date`, `privacy`, `source`, `created_by_agent`, and `summary`. `doc_type` is routing/category input. `privacy` may map to sensitivity if defined by a trusted category policy, not arbitrary user metadata. `confidence`, `owner`, `tags`, `source-of-truth`, and custom keys must not affect trust, subject, sensitivity, actor, or claim kind without a later explicit registry mapping.

2. **Entity JSON schema:** no canonical workspace entity-JSON schema was found. Reuse existing entity JSON readers (`AccountJson`/`AccountStructured`, `ProjectJson`/`ProjectStructured`, `PersonJson`/`PersonStructured`, and `entity_io::read_entity_json`) if W3-A keeps this scope. Otherwise de-scope structured entity JSON to a later packet and restrict W3-A to validated UTF-8 workspace text with explicit mapping.

3. **RawObservation chunking:** blocked because `RawObservation` does not exist. Chunking may still apply to provenance identifiers (`SourceIdentifier::Document { chunk_id }`) after the fallback claim-kind decision is resolved.

4. **Feedback:** existing claim-level feedback is enough for W3-A. Re-extract, quarantine, and relink are source-management affordances for W4-A/W5, not a requirement for the first extractor packet.

5. **Suite E:** existing Suite E/bundle coverage exercises the claim substrate generally, but not workspace-file-derived claim emission. Add workspace-specific fixtures that verify committed rows, provenance/source refs, trust band/freshness behavior, and run counts.

6. **Migration:** no SQL migration is required if W3-A maps to existing claim types and stores workspace metadata in existing proposal/metadata/provenance fields. A new fallback claim kind requires a registry/ADR/test amendment, and the wave plan must explicitly authorize it; it may not require a SQL migration.

### V1.0 minimum amendments that later revisions address

- Resolve the fallback claim-kind policy: add a real registered fallback claim kind by L0 amendment, or rewrite W3-A around existing claim types with no `RawObservation` references.
- Amend the extractor/pipeline contract. Either pass an `ExtractionContext` containing `file_id`, `source_asof`, source type, category, linked subject, run id, and actor/service context, or make extraction pure and enrich/commit proposals pipeline-side. The packet must name which module owns this.
- Amend wave ownership so the required `pipeline.rs`/context/commit work is explicitly authorized, or make it a prerequisite W2 follow-up before W3-A starts.
- Specify exact `commit_claim` wiring, including `ServiceContext`, `ActionDb`, actor class, `source_ref`, provenance JSON, metadata JSON, `observed_at`, and claim budget identity. Do not reopen DBs or synthesize context inside extraction.
- Replace the provenance sketch with live `SourceAttribution::new` usage and add tests for old file mtimes degrading freshness through `source_asof`.
- Freeze frontmatter policy and add hostile-frontmatter tests.
- Add fake-extractor and integration tests that prove non-empty extractor output commits and counts correctly, extractor errors do not become silent success, no-entity requests do not commit claims, and subject refs cannot cross entities.
- Add `/cso` to L2.

## 16. L0 review result - 2026-05-23 cycle 2

**Historical V1.1 verdict: HOLD by adversarial lane; PASS by substrate/K-in and security lanes.** Sections 3-13 later advanced to V1.2 and now V1.3; this section records the cycle-2 blockers folded into V1.2.

### Review lanes run

- Codex challenge: HOLD.
- Codex substrate/K-in consult: PASS.
- Security/trust review: PASS.

### Cycle-2 blockers folded into V1.2

1. **Single DB/context wiring.** `pipeline.run` needed an explicit `&ActionDb` / `&ServiceContext` signature and a no-second-DB rule. V1.2 pins that in §9 Pipeline commit flow.
2. **`UserNote` subject eligibility.** `UserNote` is registered for account/project/person subjects, while workspace intake accepts `other`. V1.2 narrows `UserNote` eligibility and adds unsupported-subject tests.
3. **Subject conversion.** Direct serde of the abilities provenance tuple `SubjectRef` does not match `commit_claim`'s object JSON parser. V1.2 freezes object-shaped commit subject JSON and forbids direct provenance-subject serialization.
4. **Freshness vs trust-band.** Current `dev` gives `UserNote` an initial `0.85` trust score and `get_entity_intelligence` renders freshness separately. V1.2 changes acceptance to stale freshness on current `dev`, with trust-band downgrade only if PR #366's recompute hook lands first.
5. **Run reporting shape.** No partial status or dropped-count columns exist. V1.2 defines the no-migration `error_log` JSON schema and status mapping.

## 17. L0 review result - 2026-05-23 cycle 3

**Historical V1.2 verdict: HOLD by substrate/K-in and security lanes; PASS by adversarial challenge lane.** Sections 3-13 in V1.3 are the cycle-4 amendment candidate intended to resolve the blockers below.

### Review lanes run

- Codex challenge: PASS.
- Codex substrate/K-in consult: HOLD.
- Security/trust review: HOLD.

### Cycle-3 blockers folded into V1.3

1. **Wave-plan contradiction.** `.docs/plans/v1.4.5-waves.md` still said W3-A must not touch `pipeline.rs` and should use `RawObservation`. V1.3 amends the wave plan and this packet together: W3-A owns bounded pipeline/intake/contract call-site changes needed to commit claims, and no `RawObservation` fallback exists.
2. **Local trust seed mismatch.** Live `WorkspaceClaimProposal` still carried `initial_source_reliability`, while the packet said W3-A should not seed arbitrary trust. V1.3 removes that local seed from the W3-A contract amendment and relies on source_asof/provenance/sensitivity plus PR #366's generic recompute hook if it lands first.
3. **Unsupported-format attribution.** V1.2 attributed unsupported-format rejection to W1-B `open_validated`. V1.3 corrects this: W1-B handles path/race validation; W2-A `pipeline.rs` performs NUL/UTF-8/unsupported-format rejection after safe open.
4. **Subject authority.** Request entity DTO validation is not enough to anchor a claim subject, and `commit_claim` only checks subject JSON shape/type compatibility. V1.3 requires canonical entity existence, id/name consistency, and an active `document_entity_links` row before any subject-bound claim commit.
5. **Sensitivity floor.** `UserNote` defaults to `Internal`, and Agent/MCP prompt readers admit `Public`/`Internal`. V1.3 sets body-derived workspace notes to `UserOnly` by default and forbids frontmatter-controlled lowering.

## 18. L0 review result - 2026-05-23 cycle 4

**Historical V1.3 verdict: HOLD by substrate/K-in lane; PASS by adversarial challenge and security lanes.** Sections 3-13 in V1.4 are the cycle-5 amendment candidate intended to resolve the blockers below.

### Review lanes run

- Codex challenge: PASS.
- Security/trust review: PASS.
- Codex substrate/K-in consult: HOLD.

### Cycle-4 blockers folded into V1.4

1. **Runtime consumers.** V1.3 named legacy `build_intelligence_context()` / `gather_account_context()` as if they currently read committed claim rows. V1.4 names the live claim-aware consumers instead: entity-context claim reads, `get_entity_intelligence`, `account_overview`, `list_open_loops`, and prepare/daily synthesis paths that already read committed claims.
2. **File ownership.** The wave plan still listed `src-tauri/src/intelligence/io.rs` under W3-A even though the packet narrowed scope to already-opened validated UTF-8. V1.4 removes `intelligence/io.rs` from W3-A ownership and keeps legacy extraction helpers as read-only K-in only.

## 19. L0 review result - 2026-05-23 cycle 6

**V1.4 verdict: PASS.** All L0 lanes passed after the focused cycle-6 rerun.

### Review lanes run

- Codex challenge: PASS.
- Security/trust review: PASS.
- Codex substrate/K-in consult: PASS.

### Residual L2 risks

- Rejected/tombstoned link handling must fail closed in implementation/tests so `EntityIntake` does not accidentally resurrect an invalid subject path.
- Provenance conversion must keep `DataSource::WorkspaceFile { kind }`, `SourceAttribution`, `source_ref`, and metadata path-safe and canonical.
- The blocking intake bridge must preserve one request-scoped `ActionDb`/`ServiceContext`, with bounded DB writes and no hidden DB opens in extraction or claim commit loops.
- Commit-loop reporting needs exact tests for zero proposals, drops, partial commits, warnings, extractor errors, and commit errors.
- If PR #366 lands first, W3-A must rebase onto its generic trust recompute hook without duplicating registry/trust code.
