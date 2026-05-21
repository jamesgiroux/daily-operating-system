# L0 Packet — v1.4.5 W2-A — DOS-466 Staged Ingestion Service: Pipeline Shell + Quarantine + auto_detect_category

**Current revision:** V1.6 (L2 cycle-1 codex BLOCK fold, 2026-05-21). See §2 Changelog.

## 1. Header

- **Date:** 2026-05-21
- **Project:** v1.4.5 — Workspace Memory Refactor
- **Wave:** W2 stage 2a (W2-A — staged ingestion service shell)
- **Issue:** DOS-466 — Staged ingestion service: pipeline shell + quarantine + `auto_detect_category`
- **Branch:** `wave/v1.4.5-w2-l0`
- **Worktree:** `/private/tmp/dailyos-v145-w2-l0`
- **Trust topology:** local-to-local single-user (WP block → loopback Tauri runtime, James-only principal).
- **Migration:** None. W2-A ships no migration and inherits W1's v250–v254 schema.
- **Authority docs:** `.docs/plans/v1.4.5-waves.md:601-641` + cycle-13 amendment §"Cycle 13 amendments (2026-05-21) — W2 L0 substrate-gap fix" + `.docs/plans/v1.4.5-w0-reuse-audit-2026-05-19.md`
- **Shared contract anchor:** `.docs/plans/v1.4.5-workspace-memory/W2-shared-contract.md` (cited below as **§0**; do not duplicate §0-owned type definitions).
- **L2 reviewer matrix:** codex-review + code-reviewer + architect-reviewer + `/cso`
- **V1.2 header note:** L2 matrix unchanged. Pacing note unchanged.
- **Pacing note:** This L0 runs against frozen W1 contracts before W1 lands L3 — L1 implementation is blocked until W1 merges + L3 clears.

## 2. Changelog

- **V1.6 — 2026-05-21 (L2 cycle-1 fold):** Codex L2 challenge returned BLOCK at 18:30Z with two findings. Architect / correctness / security reviewers returned APPROVE. Folds per `feedback_reviewer_dissent_is_signal`:
  1. **Finding 2 (§3 + §7 AC violation) — FIXED in cycle 2.** Frontmatter `doc_type` priority 1 was not terminal: a present-but-shape-invalid `doc_type` (e.g. `Bad`) fell through to filename-glob, letting filename intent override user-supplied invalid intent. Fix at `pipeline.rs::auto_detect_category_pure` — new internal helper `frontmatter_doctype_detection(content_head) -> Option<Option<WorkspaceCategory>>` distinguishes "priority 1 didn't fire" from "priority 1 fired with no match". Regression test at `tests/workspace_ingestion_w2a_auto_detect_category.rs::invalid_frontmatter_doctype_is_terminal_none_even_with_filename_match` (2 assertions covering shape-invalid + leading-digit cases).
  2. **Finding 1 (ServiceContext field shape) — PATH-α, NOT BLOCK.** Codex flagged that §9 stub specifies `workspace_intake: &'a dyn WorkspaceIntakeService` (non-Option borrowed ref) but impl uses `Option<Arc<dyn WorkspaceIntakeService>>`. Per `feedback_l2_path_alpha_to_maintenance_project`: this is code-stub drift, not an AC violation (§11 says "ServiceContext extended with workspace_intake; bootstrap registers IngestPipelineWorkspaceIntake" — satisfied), not an ADR-named contract violation, not a PR-introduced regression. Architect reviewer correctly noted the impl shape matches the established convention for builder-populated handles (`entity_context_reader`, `composition_commit`, `claim_receipt_reader` are all `Option<Arc<dyn ...>>`). Non-Option borrowed refs in ServiceContext (`Clock`, `SeededRng`, `ExternalClients`) are constructor params; handles need builder pattern. File under DOS-751 to revisit when W2-C's consumer pattern stabilizes — if `workspace_intake().expect(...)` proliferates, lift the field to non-Option.
- **V1.5 — 2026-05-21 (L1 kickoff):** Resolves three L1-preconditions from `L1-residuals-from-L0-cycle-5.md` against live substrate.
  1. **`entity_name` IS a path segment.** Live `registry.rs:448` (`PathBuf::from(entity_dir).join(entity_name)`) confirms. REVERTS V1.4 §7 / §9 wording marking it "display-only". The bridge (`workspace_intake_impl.rs`) MUST slug-validate `entity_name` via `is_valid_slug_shape` (registry.rs:464 pattern) before constructing `IngestRequest`; invalid → `WorkspaceIntakeError::InvalidEntityName(String)` (new variant added to §0 V1.4).
  2. **`workspace_root` threaded via constructor.** `IngestPipeline` holds `workspace_root: PathBuf` as a field; `pub fn build_pipeline(workspace_root: PathBuf) -> IngestPipeline` (still infallible). `pipeline.run()` revalidates `request.file_id == file_id_from_identity(&request.identity, &self.workspace_root)`. Also resolves W2-B residual #2 — W2-B call sites become `wiring::build_pipeline(workspace_root)`, no `?`.
  3. **§0 typed-DTO duplicate removed.** `W2-shared-contract.md` §4's stale typed-field `WorkspaceIntakeRequest` mirror (V1.2 leftover) deleted; canonical V1.3 raw-slug form is the only authority.
- **V1.4 — 2026-05-21:** Folds §0 V1.3 and cycle-4 mechanical findings without changing architecture.
  1. Absorbs §0 V1.3: `WorkspaceIntakeService` trait DTOs use raw slugs at the crate boundary; `LifecycleRepo` adds `set_entity` + `get` helpers.
  2. Makes `workspace_intake_impl.rs` the SLUG→TYPED translator: parse `WorkspaceIntakeRequest` raw slugs into `WorkspaceFileKind` / `IngestionMode` / `WorkspaceCategory`, parse entity type via `EntityType::from_str_lossy`, validate categories through `WorkspaceCategoryRegistry::validate`, then construct the internal typed `IngestRequest`.
  3. Adds the one-line `pub mod workspace_intake_impl;` declaration to `workspace_ingestion/mod.rs` per cycle-13 §13.2 and §0 V1.3 bridge expansion.
  4. Fixes cycle-4 mechanical bugs: `entity_name` is display-only and no longer slug-validated or used as a path segment; the no-direct-open grep catches bare `File::open`; extractor handoff requires rewinding the `&File` because W1 `Extractor::extract` accepts `&File` only; and the module declaration is explicitly owned by this packet.
- **V1.3 — 2026-05-21:** Folds local-to-local trust topology framing and cycle-3 compile-shape findings against W2-A V1.2.
  1. Adds the trust topology declaration to §1: local-to-local single-user (WP block → loopback Tauri runtime, James-only principal).
  2. Drops `resolved_path` scope-gated redaction. The user already has filesystem access on this surface, so the bridge passes the receipt path through.
  3. Drops scope-helper requirements. No `ctx.has_scope()` calls are needed, and abilities-runtime has no scope-context surface for this lane.
  4. Drops principal-binding test asks; `QuarantineActor::User { user_id }` remains typed for data hygiene, not actor-gate enforcement.
  5. Fixes cycle-3 compile-shape findings: `registry::open_validated` becomes `WorkspaceSourceRegistry::open_validated` across §6/§9/§10; `IngestPipeline::run` is used as instance `pipeline.run(&conn, request)`; abilities-runtime edits target the inline `pub mod services { ... }` block in `lib.rs:12-18`; and file-id mismatch uses new W2-A-local `IngestError::FileIdMismatch { expected, found }`, not `RejectionReason`.
- **V1.2 — 2026-05-21:** Folds cycle-13 wave amendment "W2 L0 substrate-gap fix" and cycle-2 reviewer findings against W2-A V1.1.
  1. `file_id_from_identity` formula switched to `sha256` via existing `sha2` + `hex` deps in `src-tauri/Cargo.toml:54-56`; `blake3` was wrong. Cite §0 V1.2 §2.4 and cycle-13 §13.2.1.
  2. `IngestError::Rejected(RejectionReason::{FileTooLarge, UnsupportedFormat})` consumes W1 UNIT variants per `src-tauri/src/services/workspace_ingestion/contracts.rs:125-132`; separate `RejectionMetadata { limit_bytes, found_bytes }` carries log/event metadata. Cite §0 V1.2 §2.3 and cycle-13 §13.2.2.
  3. Owns abilities-runtime bridge edits per cycle-13 §13.3.1: module declarations, `ServiceContext` field/accessor, dailyos_lib impl, and bootstrap registration.
  4. `WorkspaceIntakeService::ingest` is async per §0 V1.2 §4 and cycle-13 §13.2.4.
  5. Security gate now covers caller-provided `file_id` derivation mismatch. The former `EntityRef.entity_name` slug-validation requirement from V1.2 is superseded by V1.4 because §0 V1.3 marks `entity_name` display-only.
  6. IL gate Q3/Q5 now names `W3-B/DOS-471` and `W5-A/DOS-475` explicitly, per cycle-2 codex challenge F7.
  7. Adds resolved-path scope-gated enforcement test per cycle-2 `/cso` F4 and cycle-13 §13.4.
  8. Clarifies `IngestionMode` consumer mapping: W1-extension PR adds `EntitySeeded` + `Realtime` per cycle-13 §13.1; W2-A consumes the extended enum, not a local mapping.
- **V1.1 — 2026-05-21:** Folds cycle-1 reviewer findings by reconciling W2-A with the shared §0 addendum, removing local DTO drift, and making lifecycle, quarantine, file identity, CI ownership, and rejection privacy implementable without L1 invention.
  1. Folds shared-contract addendum §0 as the load-bearing contract for `IngestRequest`, `IngestReceipt`, `IngestError`, `file_id`, lifecycle helpers, CI script ownership, crate bridge, category validation split, and quarantine actor.
  2. Adds `lifecycle.rs` to W2-A owned files for `LifecycleRepo` write helpers while keeping W1-A's lifecycle types read-only.
  3. Adds W2-A ownership for `src-tauri/scripts/check_workspace_mutation_allowlist.sh` and extends the CI grep gate to all path-opening APIs per §0 §8.
  4. Replaces duplicate `IngestError::FileTooLarge` / `UnsupportedFormat` variants with `IngestError::Rejected(RejectionReason)` consumption per §0 §2.3.
  5. Pins `file_id` derivation through `file_id_from_identity(&FileIdentity, workspace_root)` per §0 §2.4.
  6. Adds typed `QuarantineActor` for quarantine mutations per §0 §9.
  7. Adds path-stripping for rejection logs/Tauri events and validates `doc_type` frontmatter slugs before storage/log use.
  8. Confirms W2-A ships no migration and inherits W1's v250–v254 schema.
- **V1.0 — 2026-05-21:** Initial packet.

## 3. Goal (verbatim from wave plan §Agent W2-A)

Implement `services::workspace_ingestion::pipeline::IngestPipeline` — the staged pipeline that takes a `(File, FileIdentity)` from `WorkspaceSourceRegistry::open_validated`, reads content (with sanitization), runs a stub `extract::extract_proposals_*` (W3-A replaces the stub with the real extractor) which returns `Vec::new()` for v1.4.5 W2, records the ingestion run via W1-C tables, and emits lifecycle progress hooks. **W2-A scope is narrowed to a non-claim-producing pipeline shell — no `commit_claim` calls in W2.** Claim production lands in W3-A. Cycle 2 amendment: this resolves the W2-A vs W3-A ownership contradiction (challenge finding #3) and lets DOS-466 land without depending on extraction logic that W3-A owns. W2-A also ships `pipeline::quarantine_source(file_id, reason, actor) -> Result<()>` — the quarantine state mutation that W4-A's source management UI invokes (challenge finding #6). **Cycle 8+9 amendment (Option B-prime):** W2-A also ships `pipeline::auto_detect_category(filename, content_head) -> Option<WorkspaceCategory>` — heuristic detection mined from the archived `prepare_inbox.py`. **Frozen detection rule table (cycle 9 deterministic-contract amendment):**

| Priority | Source | Match | Resolves to |
|---|---|---|---|
| 1 (highest) | Frontmatter field `doc_type` | `transcript` | `Transcripts` |
| 1 | Frontmatter field `doc_type` | `presentation` / `deck` / `slides` | `Presentations` |
| 1 | Frontmatter field `doc_type` | `meeting` / `1on1` | `Meetings` |
| 1 | Frontmatter field `doc_type` | `note` / `notes` | `Notes` |
| 1 | Frontmatter field `doc_type` | `contract` / `msa` / `sow` | `Contracts` |
| 1 | Frontmatter field `doc_type` | other registered string | matches `WorkspaceCategoryRegistry` `Other(s)` if present, else None |
| 2 | Filename glob (case-insensitive) | `*-transcript-*` | `Transcripts` |
| 2 | Filename glob | `*-deck-*` / `*.pptx` / `*.key` / `*-slides-*` | `Presentations` |
| 2 | Filename glob | `*meeting*` / `*-1on1-*` | `Meetings` |
| 2 | Filename glob | `*contract*` / `*msa*` / `*sow*` | `Contracts` |
| 3 (extension fallback) | Filename extension | `.pdf` (no other markers) | `Attachments` |
| 3 | Filename extension | `.md` / `.txt` (no other markers) | `Notes` |
| 4 (terminal) | (no match) | — | `None` — file lands at entity root, no sub-directory |

**Tie-break rules:** higher priority wins; within the same priority, first matched rule wins. **Validation:** auto-detected results MUST pass `WorkspaceCategoryRegistry::validate(category, entity_type)` — if the rule produces a category not allowed for the file's entity type, fall back to `None` (entity root) rather than silently using a disallowed category. **Content sniff bound:** `content_head` is the first 4KB of file content (sufficient for frontmatter parsing); never reads the full file. Pipeline records the resolved category in the lifecycle row and resolves the destination path via `WorkspaceCategoryRegistry` from W1-B.

**V1.3 §0 reconciliation:** the wave-plan name `auto_detect_category` is implemented as `auto_detect_category_pure(filename, content_head) -> Option<WorkspaceCategory>` plus registry-bound `validate_detected_category(conn, candidate, entity_type)` per §0 §7. W2-A resolves and returns `resolved_path` in the §0 §2.2 receipt; physical file moves remain with the caller lanes that own watcher, Drive, and `_inbox` mutation paths.

## 4. Files owned (exclusive)

- `src-tauri/src/services/workspace_ingestion/pipeline.rs` — sole owner permanently; defines the `IngestPipeline` struct and W2-A-owned public surface that USES the `Extractor` and `SignalEmitter` traits from W1-A's `contracts.rs`. It also owns the §0 §2 canonical DTO implementations in code; this packet cites §0 rather than duplicating their definitions.
- `src-tauri/src/services/workspace_ingestion/wiring.rs` — sole owner of content; W1-A pre-created the empty file shell. W2-A fills it with `pub fn build_pipeline() -> IngestPipeline` constructed with `contracts::NullExtractor` + `contracts::NullSignalEmitter` defaults.
- `src-tauri/src/services/workspace_ingestion/lifecycle.rs` — W2-A may add `LifecycleRepo::{insert_pending, transition, record_user_override, update_category, set_entity, get}` write helpers per §0 V1.3 §3. W1-A's existing `LifecycleState`, `WorkspaceFileLifecycle`, `UserOverride`, and `LifecycleError` types remain read-only.
- `src-tauri/src/services/workspace_ingestion/mod.rs` — single-line edit only: insert `pub mod workspace_intake_impl;` in the existing alphabetized module declaration list.
- `src-tauri/abilities-runtime/src/lib.rs:12-18` — edit the existing inline `pub mod services { ... }` block to add `workspace_intake`; do not invent a separate `services/mod.rs` file.
- `src-tauri/abilities-runtime/src/services/workspace_intake.rs` (NEW) — crate-boundary `WorkspaceIntakeService` async trait with raw-slug DTOs per §0 V1.3 §4 and cycle-13 §13.2.4. W2-C consumes this bridge rather than importing `dailyos_lib` from `abilities-runtime`.
- `src-tauri/abilities-runtime/src/services/context.rs` — add `workspace_intake` field + accessor to `ServiceContext` per §0 V1.3 §4 and cycle-13 §13.3.1.
- `src-tauri/src/services/workspace_ingestion/workspace_intake_impl.rs` (NEW) — dailyos_lib implementation of `WorkspaceIntakeService`; owns raw-slug parsing to typed `WorkspaceFileKind` / `IngestionMode` / `WorkspaceCategory`, category validation via `WorkspaceCategoryRegistry::validate`, validated open via `WorkspaceSourceRegistry::open_validated`, and `pipeline.run(&conn, request)` invocation inside `spawn_blocking`.
- `src-tauri/src/main.rs` OR Bootstrap location — one registration line wiring `IngestPipelineWorkspaceIntake` into `ServiceContext` construction.
- `src-tauri/scripts/check_workspace_mutation_allowlist.sh` (NEW) — W2-A creates the shared W2 mutation allowlist script per §0 §5.

**Cycle 3+4 dependency-injection pattern:** W3-A and W3-B do NOT edit `pipeline.rs` at all. Each later lane ships its real impl in its own submodule (which W1-A pre-created the empty shell for) and applies a single-line documented patch to `wiring.rs` to swap the Null impl for the real one — `wiring.rs` is the only cross-lane shared touch, with named function-level allowed edits per lane (W3-A patches the `Extractor` argument; W3-B patches the `SignalEmitter` argument; mechanically non-conflicting).

### Migration

- W2-A ships no migration. It inherits W1's v250–v254 schema. If L1 discovers a schema delta, that is a wave-plan amendment and a new L0 review.

## 5. Don't touch

- **Don't touch:** `processor/classifier.rs` (read-only call); `services/claims.rs` (no caller in W2-A — narrowed scope); `watcher.rs` (W2-B owns the watcher interaction); `google_drive/` mutation paths (W2-B owns); `_inbox/` refactor (W2-D owns); signal bus (W3-B owns the signal wiring); `services/workspace_ingestion/extract.rs` (W3-A owns); `services/workspace_ingestion/{registry,runs,link,signals,graph}.rs` (other lanes). `services/workspace_ingestion/lifecycle.rs` is intentionally not in this list because §4 gives W2-A owned write-helper edits. `services/workspace_ingestion/mod.rs` is forbidden except for W2-A V1.4's single `pub mod workspace_intake_impl;` declaration. No other crate-level `Cargo.toml` edits beyond the already-present `sha2` + `hex` dependencies at `src-tauri/Cargo.toml:54-56`; W2-A must not add a new hash dependency.

## 6. K-in substrate audit

### Substrate primitives W2-A consumes

| Primitive | Canonical location with file:line | How W2-A uses it |
|---|---|---|
| `Extractor` trait | `src-tauri/src/services/workspace_ingestion/contracts.rs:192` | `IngestPipeline` owns a `Box<dyn Extractor>` slot. W2-A wires `NullExtractor`; W3-A later swaps in the real extractor through `wiring.rs`, not by editing `pipeline.rs`. |
| `SignalEmitter` trait | `src-tauri/src/services/workspace_ingestion/contracts.rs:207` | `IngestPipeline` owns a `Box<dyn SignalEmitter>` slot. W2-A emits through the trait; W3-B later swaps in the real emitter through `wiring.rs`. |
| `FileIdentity` | `src-tauri/src/services/workspace_ingestion/contracts.rs:46` | `IngestRequest` carries the identity returned with the validated file handle. W2-A records device, inode, canonical path provenance in ingestion-run/lifecycle work. |
| `WorkspaceCategory` | `src-tauri/src/services/workspace_ingestion/contracts.rs:67` | `category_hint` and `auto_detect_category` use this canonical enum. W2-A must not define a parallel category enum or slug type. |
| `WorkspaceCategoryRegistry` | `src-tauri/src/services/workspace_ingestion/registry.rs:350` | Auto-detected categories are validated via `WorkspaceCategoryRegistry::validate` before use; disallowed detected categories fall back to `None` rather than being forced into a sub-directory. |
| `WorkspaceCategoryRegistry::validate` | `src-tauri/src/services/workspace_ingestion/registry.rs:358` | Enforces per-entity-type category allowedness for detected categories. Caller-provided `category_hint: Some(_)` is already validated before request construction per wave-plan cycle 10. |
| `WorkspaceCategoryRegistry::resolve_path` | `src-tauri/src/services/workspace_ingestion/registry.rs:418` | Provides the path-resolution contract for root vs category sub-directory placement; W2-A uses the same registry behavior rather than recreating routing rules. |
| `WorkspaceClaimProposal` | `src-tauri/src/services/workspace_ingestion/contracts.rs:148` | `Extractor::extract` returns `Vec<WorkspaceClaimProposal>`. W2-A records zero produced claims in W2; W3-A owns real claim production and later commit mapping. |
| `RejectionReason` | `src-tauri/src/services/workspace_ingestion/contracts.rs:125` | Size and binary-format rejection map to canonical `FileTooLarge` / `UnsupportedFormat` variants; pipeline rejection signals use this type. |
| `RejectionReason` unit variants | `src-tauri/src/services/workspace_ingestion/contracts.rs:125-132` | `FileTooLarge` and `UnsupportedFormat` are UNIT variants. Per §0 V1.2 §2.3, size/format metadata lives in separate `RejectionMetadata` log/event payloads, not in enum payloads. |
| `IngestError::FileIdMismatch { expected, found }` | `src-tauri/src/services/workspace_ingestion/pipeline.rs` | New W2-A-local error variant for caller-derived `file_id` mismatch. This is request-shape/data hygiene rejection, not a `RejectionReason` workspace-file rejection. |
| `WorkspaceSourceRegistry::open_validated` | `src-tauri/src/services/workspace_ingestion/registry.rs:148` | The pipeline boundary consumes the `(File, FileIdentity)` returned by this trust boundary. Any path-to-request adapter must call this first; `pipeline.rs` must never re-open a path directly. |
| `RunsRepo::start_run` | `src-tauri/src/services/workspace_ingestion/runs.rs:154` | Starts the `document_ingestion_runs` record with W1-C idempotency behavior before extraction. |
| `RunsRepo::complete_run` | `src-tauri/src/services/workspace_ingestion/runs.rs:238` | Completes the run as success/failure with `claim_count_produced = 0` for W2-A's no-claim pipeline shell. |
| `NullExtractor` | `src-tauri/src/services/workspace_ingestion/contracts.rs:223` | Default W2-A wiring uses this no-op extractor so the pipeline compiles and returns zero proposals until W3-A lands. |
| `NullSignalEmitter` | `src-tauri/src/services/workspace_ingestion/contracts.rs:238` | Default W2-A wiring uses this no-op emitter so signal calls are present without requiring W3-B signal bus changes. |
| `IngestionMode` | `src-tauri/src/services/workspace_ingestion/runs.rs:32` | W2-A consumes W1-C's canonical run mode enum as extended by the cycle-13 §13.1 W1-extension PR (`EntitySeeded` + `Realtime`). It does not define a local mapping for W2-C/W2-D. |
| `IngestionRunId` | `src-tauri/src/services/workspace_ingestion/runs.rs:27` | Returned in the ingestion receipt so callers can correlate lifecycle progress and run history. |
| `IngestionRunStatus` | `src-tauri/src/services/workspace_ingestion/runs.rs:63` | Used when completing runs as `Success` / `Failed`; W2-A must not define a second run-status enum. |
| `LifecycleState` | `src-tauri/src/services/workspace_ingestion/lifecycle.rs:38` | `IngestReceipt.lifecycle_state_after` (§0 §2.2) and `LifecycleRepo::transition` (§0 §3) use the canonical lifecycle enum. |
| `WorkspaceFileLifecycle` | `src-tauri/src/services/workspace_ingestion/lifecycle.rs:66` | `LifecycleRepo::insert_pending` writes this table shape; W2-A does not create a parallel row DTO. |
| `UserOverride` | `src-tauri/src/services/workspace_ingestion/lifecycle.rs:53` | `LifecycleRepo::record_user_override` records quarantine/user-correction audit data through the canonical user-override shape. |
| `LifecycleError` | `src-tauri/src/services/workspace_ingestion/lifecycle.rs:86` | `LifecycleRepo` write helpers return the canonical lifecycle error surface instead of inline SQL errors in `pipeline.rs`. |
| `DataSource::WorkspaceFile { kind }` | `src-tauri/abilities-runtime/src/abilities/provenance/source.rs:73` | Lifecycle provenance serializes workspace-file source through the canonical `DataSource` variant; no stale `WorkspaceInbox` mirror. See §0 §6. |
| `DocumentId` | `src-tauri/abilities-runtime/src/abilities/provenance/source.rs:65` | W2-A preserves stable `file_id` so W3-A can map it into document provenance without inventing a document-id type. |
| `EntityType` | `src-tauri/src/entity.rs:13` | `EntityRef` (§0 §2.1) and registry validation (§0 §7) pass typed entity kind; no stringly `entity_type` in pipeline logic. |
| `EntityType::from_str_lossy` | `src-tauri/src/entity.rs:41-49` | `workspace_intake_impl.rs` converts crate-boundary raw entity slugs through this existing helper. There is no `EntityType::from_slug` helper in shipped substrate. |
| `EntityId` | `src-tauri/abilities-runtime/src/abilities/provenance/source.rs:37` | `IngestRequest.entity` (§0 §2.1) consumes the canonical newtype through `EntityRef`; no stringly-typed entity ids in the public request surface. |
| `sha2` + `hex` deps | `src-tauri/Cargo.toml:54-56` | `file_id_from_identity` uses `Sha256` + `hex::encode` per §0 V1.2 §2.4 and cycle-13 §13.2.1. W2-A must not add `blake3`. |
| Canonical async ability shape | `src-tauri/abilities-runtime/src/abilities/account_overview.rs:106-115` | W2-A's bridge must support W2-C's canonical `pub async fn`, `ctx: &AbilityContext<'_>`, return-inner-type shape. See §0 V1.2 §13. |
| `ActorKind` variants | `src-tauri/abilities-runtime/src/abilities/registry.rs:468-486` | Existing variants are `Agent`, `User`, `Admin`, `System`, `SurfaceClient`, `McpClient`; no `WordPressRender` actor and no W2-A principal-differentiation gate. |
| `AbilityContext` | `src-tauri/abilities-runtime/src/abilities/registry.rs:740-768` | Ability fns receive `AbilityContext`, not `ServiceContext`; W2-A extends `ServiceContext` behind `ctx.services().workspace_intake()` per §0 V1.3 §4 and §13. |
| Inline abilities-runtime services module | `src-tauri/abilities-runtime/src/lib.rs:12-18` | The crate uses `pub mod services { ... }` inline in `lib.rs`; W2-A adds `workspace_intake` there and must not create `src-tauri/abilities-runtime/src/services/mod.rs`. |

### Shared-contract anchors consumed

- `IngestRequest` and `IngestReceipt` live in §0 §§2.1–2.2. W2-A implements those types in `pipeline.rs`, but this packet does not restate their fields. `IngestError` follows §0 §2.3 plus the V1.3 local addition below; §0 V1.2 §2.3 is authoritative that `RejectionReason` consumes W1 UNIT variants.
- V1.3 adds one W2-A-local `IngestError::FileIdMismatch { expected, found }` variant for caller-derived `file_id` mismatch. Do not encode this as `RejectionReason`.
- `file_id_from_identity` and `FileIdError` live in §0 V1.2 §2.4. All W2 callers derive file IDs through that helper; V1.2 uses `sha256_hex_lower(workspace_relative_path_bytes)[..16]`, not `blake3`.
- `LifecycleRepo` lives in §0 V1.3 §3. W2-A adds write helpers in `lifecycle.rs`, including `set_entity` and `get`; `pipeline.rs` must not inline lifecycle SQL.
- `WorkspaceIntakeService` and the ability/runtime crate bridge live in §0 V1.3 §4. The trait method is async, the trait DTO surface uses raw slugs, and the dailyos_lib `workspace_intake_impl.rs` converts those slugs to typed pipeline fields before constructing `IngestRequest`. The owned file list in §4 includes all module declaration, context, implementation, and registration edits required by cycle-13 §13.3.1.
- `check_workspace_mutation_allowlist.sh` ownership lives in §0 §5.
- `auto_detect_category_pure` plus registry validation split lives in §0 §7.
- The hardened path-opening CI gate lives in §0 §8.
- `QuarantineActor` and the typed quarantine signature live in §0 §9.
- Canonical ability shape lives in §0 V1.2 §13; W2-A's bridge is shaped to support that surface.
- Cycle-13 wave authority is `.docs/plans/v1.4.5-waves.md` §"Cycle 13 amendments (2026-05-21) — W2 L0 substrate-gap fix", especially §§13.1, 13.2.1-13.2.5, 13.3.1, and 13.4.

### Verified against §0

Load-bearing §0 sections for W2-A are §§2.1–2.4 (canonical typed internal request/receipt/error/file-id), V1.3 §3 (lifecycle write helpers including `set_entity` + `get`), V1.3 §4 (async raw-slug crate-boundary intake bridge), §5 (CI script ownership), §6 (`DataSource::WorkspaceFile { kind }` canonicalization), §7 (category sniff/validate split), §8 (path-opening grep gate), §9 (typed quarantine actor), §10 (substrate reuse policy), and §13 (canonical ability shape). Any L1 implementation that conflicts with these sections or with cycle-13 §§13.1-13.4 is a packet violation.

### K-in obligation (grep before scoring)

Reviewers must grep these paths before scoring:

- `docs/solutions/`
- `.docs/decisions/`

Required grep terms:

- `workspace_ingestion`
- `IngestPipeline`
- `IngestRequest`
- `quarantine_source`
- `auto_detect_category`
- `WorkspaceCategory`
- `WorkspaceCategoryRegistry`
- `WorkspaceClaimProposal`
- `RejectionReason`
- `SourceAttribution`
- `SourceType`
- `TrustFactorInput`
- `ClaimProposal`
- `Provenance`

No W2-A code may introduce a new substrate primitive when an existing W1/W0 substrate primitive covers the concept. A BLOCK on substrate reinvention must stop L1.

## 7. Security gate

DOS-466 is security-annotated. Untrusted file content enters the service. Required:

- Trust-topology trim removes multi-actor gates only. Compile correctness, Rust crate-boundary rules, slug/path validation, ADR-0093 indirect prompt-injection handling for untrusted document content, and ADR-0108 sensitivity redaction in logs/screenshots still apply.
- File content size limit enforced before reading (configurable, default 10MB; reject with `IngestError::Rejected(RejectionReason::FileTooLarge)` before any content processing). `RejectionReason::FileTooLarge` is a UNIT variant per §0 V1.2 §2.3; byte counts go only into separate `RejectionMetadata` for logs/events.
- Binary file detection before text extraction (reject non-text with `IngestError::Rejected(RejectionReason::UnsupportedFormat)` rather than attempting extraction). `RejectionReason::UnsupportedFormat` is a UNIT variant per §0 V1.2 §2.3; detected-format detail goes only into separate metadata.
- Filename sanitization: the ingestion run log records the original filename but internal keys use a normalized safe identifier.
- No shell command invocation from the pipeline (no `std::process::Command` for content extraction).
- Pipeline reads from the `(File, FileIdentity)` returned by `WorkspaceSourceRegistry::open_validated` only — never re-opens by path.
- Caller-provided `IngestRequest.file_id` MUST match `file_id_from_identity(&request.identity, workspace_root)` per §0 V1.2 §2.1 and §2.4. Mismatch returns typed `IngestError::FileIdMismatch { expected, found }`; L1 must not downgrade this to `DbError`, `Io`, `RejectionReason`, or an untyped string. §10 requires the negative test.
- **V1.5 fold:** `EntityRef.entity_name` IS a path segment per live `registry.rs:448` (`PathBuf::from(entity_dir).join(entity_name)`). The bridge MUST slug-validate via `is_valid_slug_shape` before constructing `IngestRequest`; invalid → `WorkspaceIntakeError::InvalidEntityName(String)`. Inside `pipeline.run()` we trust the type (validation happens at the bridge boundary).
- CI grep gate covers ALL path-opening APIs from §0 §8, verbatim:

```text
Forbidden in pipeline.rs source:
  - File::open
  - std::fs::File::open
  - std::fs::File::options
  - std::fs::OpenOptions
  - tokio::fs::File::open
  - tokio::fs::File::options
  - tokio::fs::OpenOptions
  - std::fs::read
  - std::fs::read_to_string
  - std::fs::metadata
  - memmap
  - std::process::Command
```

- `quarantine_source` uses typed `QuarantineActor` per §0 §9, not a free-form actor string. The V1.3 signature is `pub fn quarantine_source(conn: &Connection, file_id: &str, reason: &str, actor: QuarantineActor) -> Result<(), IngestError>;`; `QuarantineActor` itself is defined in §0 §9. In this local-to-local topology, the actor is data hygiene/audit context; do not add a scope check on the accessor.
- Frontmatter `doc_type` values used by `auto_detect_category_pure` must validate against `^[a-z][a-z0-9_-]{0,31}$` before storage or log use. Parse errors, invalid UTF-8/BOM/null-byte surprises, malformed YAML, or over-bound frontmatter are treated as `None`, never as propagated error strings containing file content.
- Rejection log lines and Tauri events MUST NOT include canonical path strings. Before emission, `std::io::Error` is reduced to `ErrorKind` + `raw_os_error()`; path-bearing context is kept out of logs/events for all rejection variants, especially traversal/outside-workspace cases.
- `/cso` at L0 plan AND L2 diff.
- Note: per-stage timeouts and line-length / parser bounds beyond size cap are filed to maintenance per cycle 2 path-α (CSO #2).

`/cso` reviews this packet at L0 AND reviews the implementation diff at L2.

## 8. Intelligence Loop gate

1. *Claim model:* W2-A produces NO claims (narrowed scope). The pipeline records the ingestion run, transitions lifecycle states, and calls a stub extractor that returns empty. Claim production happens in W3-A which replaces the stub. This narrowing is the cycle 2 fix for the producer/consumer contradiction.
2. *Provenance + trust:* Ingestion run records the `FileIdentity` (device + inode + canonical path) and SHA-256 content hash so W3-A's claim production can attribute correctly.
3. *Signals + invalidation:* Pipeline declares which signals it *will* emit (signal type list in source comments) but wiring deferred to `W3-B/DOS-471`. The `IngestPipeline` has a `signal_emitter: Box<dyn SignalEmitter>` slot that is a no-op `NullSignalEmitter` in W2; `W3-B/DOS-471` replaces it with the real emitter when wiring lands.
4. *Runtime + surfaces:* Consumed by W2-C (entity-seeded intake), W2-D (`_inbox` refactor), W4-A (quarantine action), W5-A (backfill).
5. *Feedback loop:* `quarantine_source(file_id, reason, actor)` is the user-correction service mutation that W4-A invokes; it transitions lifecycle to `quarantined`, records `user_override`, and (after `W3-B/DOS-471` wiring) emits `WorkspaceFileQuarantined` signal that triggers claim retraction in `W5-A/DOS-475` backfill scoping.

## 9. Code stub

`pipeline.rs` public surface is frozen to the shape below. `IngestRequest`, `IngestReceipt`, `EntityRef`, `FileIdError`, and `QuarantineActor` are owned by §0 V1.2 §§2.1–2.4 and §0 §9, so this section does not duplicate their definitions. `IngestError` follows §0 except for the V1.3 W2-A-local `FileIdMismatch` variant below. L1 fills bodies without changing signatures.

```rust
use std::path::Path;

use rusqlite::Connection;
use sha2::{Digest, Sha256};

use crate::entity::EntityType;

use super::contracts::{Extractor, FileIdentity, RejectionReason, SignalEmitter, WorkspaceCategory};
use super::registry::{WorkspaceCategoryRegistry, WorkspaceSourceRegistry};

pub struct IngestPipeline {
    pub extractor: Box<dyn Extractor>,
    pub signal_emitter: Box<dyn SignalEmitter>,
    pub max_file_bytes: u64,
    pub extractor_version: String,
}

impl IngestPipeline {
    pub fn new(
        extractor: Box<dyn Extractor>,
        signal_emitter: Box<dyn SignalEmitter>,
        max_file_bytes: u64,
        extractor_version: impl Into<String>,
    ) -> Self {
        Self {
            extractor,
            signal_emitter,
            max_file_bytes,
            extractor_version: extractor_version.into(),
        }
    }

    pub fn run(
        &self,
        conn: &Connection,
        request: IngestRequest,
    ) -> Result<IngestReceipt, IngestError> {
        // L1 fills:
        // 1. Re-derive file_id via file_id_from_identity(&request.identity, workspace_root).
        // 2. Reject caller-provided file_id mismatch with IngestError::FileIdMismatch { expected, found }.
        // 3. Resolve path through WorkspaceCategoryRegistry::resolve_path using
        //    entity_type + entity_name + category. V1.5: entity_name IS a path
        //    segment (registry.rs:448); bridge has already slug-validated it.
        // 4. Read from request.file only, enforcing max bytes and 4KB content_head.
        // 5. Seek back to start before calling Extractor::extract(&File, ...).
        // 6. Populate resolved_path in the receipt; no scope redaction in this topology.
    }

    pub fn validate_detected_category(
        conn: &Connection,
        candidate: Option<WorkspaceCategory>,
        entity_type: EntityType,
    ) -> Result<Option<WorkspaceCategory>, IngestError> {
        // L1 fills
    }

    pub fn auto_detect_category_pure(
        filename: &str,
        content_head: &str,
    ) -> Option<WorkspaceCategory> {
        // L1 fills
    }
}

pub fn file_id_from_identity(
    identity: &FileIdentity,
    workspace_root: &Path,
) -> Result<String, FileIdError> {
    // Exact body pinned in §0 V1.2 §2.4.
    let relative = identity
        .canonical_path
        .strip_prefix(workspace_root)
        .map_err(|_| FileIdError::OutsideWorkspace)?;
    let mut hasher = Sha256::new();
    hasher.update(relative.as_os_str().as_encoded_bytes());
    let hash = hasher.finalize();
    Ok(hex::encode(&hash)[..16].to_string())
}

pub fn quarantine_source(
    conn: &Connection,
    file_id: &str,
    reason: &str,
    actor: QuarantineActor,
) -> Result<(), IngestError> {
    // L1 fills
}
```

The only V1.3 W2-A-local error variant is for caller-derived `file_id` mismatch:

```rust
pub enum IngestError {
    FileIdMismatch { expected: String, found: String },
    Rejected(RejectionReason),
    // existing §0 variants...
}
```

Size and binary-format rejection uses UNIT `RejectionReason` variants per §0 V1.2 §2.3. Metadata is separate and redacted before logs/events:

```rust
pub struct RejectionMetadata {
    pub limit_bytes: Option<u64>,
    pub found_bytes: Option<u64>,
    pub detected_format: Option<String>,
}

return Err(IngestError::Rejected(RejectionReason::FileTooLarge));
```

`src-tauri/abilities-runtime/src/services/workspace_intake.rs` is the async crate-boundary trait from §0 V1.3 §4. The trait surface uses raw slugs because `abilities-runtime` cannot import dailyos_lib ingestion types:

```rust
use async_trait::async_trait;

use crate::abilities::registry::AbilityContext;

#[async_trait]
pub trait WorkspaceIntakeService: Send + Sync {
    async fn ingest(
        &self,
        ctx: &AbilityContext<'_>,
        request: WorkspaceIntakeRequest,
    ) -> Result<WorkspaceIntakeReceipt, WorkspaceIntakeError>;
}
```

The required DTO fields are `file_ref`, `source_type_slug`, `entity: Option<EntityRefDto>`, `mode_slug`, and `category_slug: Option<String>`. `EntityRefDto` carries `entity_type_slug`, `entity_id`, and `entity_name` (path-segment per V1.5; bridge slug-validates via `is_valid_slug_shape`).

`src-tauri/abilities-runtime/src/services/context.rs` grows the service accessor behind `AbilityContext::services()`; ability functions still receive `AbilityContext`, not `ServiceContext`, per §0 V1.2 §13:

```rust
pub struct ServiceContext<'a> {
    // existing fields...
    workspace_intake: &'a dyn WorkspaceIntakeService,
}

impl<'a> ServiceContext<'a> {
    pub fn workspace_intake(&self) -> &dyn WorkspaceIntakeService {
        self.workspace_intake
    }
}
```

`src-tauri/src/services/workspace_ingestion/workspace_intake_impl.rs` is the dailyos_lib bridge implementation and the only SLUG→TYPED translator. It validates the workspace-relative file reference through `WorkspaceSourceRegistry::open_validated`, derives `file_id` via §0 V1.2 §2.4, converts raw DTO slugs into the typed §0 V1.2 §2.1 `IngestRequest`, validates category via `WorkspaceCategoryRegistry::validate`, and runs sync rusqlite/pipeline work inside `spawn_blocking`. The bridge is constructed with a trusted `workspace_root`; `WorkspaceIntakeRequest` must not carry or override that root:

```rust
use std::path::PathBuf;

pub struct IngestPipelineWorkspaceIntake {
    workspace_root: PathBuf,
    // workspace_root bound at impl construction; NOT caller-overridable
}

#[async_trait::async_trait]
impl WorkspaceIntakeService for IngestPipelineWorkspaceIntake {
    async fn ingest(&self, ctx: &AbilityContext<'_>, req: WorkspaceIntakeRequest)
        -> Result<WorkspaceIntakeReceipt, WorkspaceIntakeError>
    {
        // Parse raw slugs to typed enums
        let source_type = WorkspaceFileKind::from_slug(&req.source_type_slug)
            .ok_or(WorkspaceIntakeError::InvalidSourceTypeSlug(req.source_type_slug.clone()))?;
        let mode = IngestionMode::from_slug(&req.mode_slug)
            .ok_or(WorkspaceIntakeError::InvalidModeSlug(req.mode_slug.clone()))?;
        let entity = req.entity.map(|e| {
            let entity_type = EntityType::from_str_lossy(&e.entity_type_slug); // NOTE: from_str_lossy NOT from_slug
            Ok::<_, WorkspaceIntakeError>(EntityRef {
                entity_type,
                entity_id: EntityId::new(e.entity_id),
                entity_name: e.entity_name,
            })
        }).transpose()?;
        // Convert + validate category via registry
        let conn = ctx.services().conn_ref()?;  // L1 wires this through; if accessor doesn't exist, ctx provides a db handle path
        let category_hint = if let Some(slug) = req.category_slug.as_deref() {
            let cat = WorkspaceCategory::from_slug(slug)
                .ok_or(WorkspaceIntakeError::InvalidCategorySlug(slug.to_string()))?;
            let entity_type = entity.as_ref().map(|e| e.entity_type);
            WorkspaceCategoryRegistry::validate(conn, &cat, entity_type)
                .map_err(|_| WorkspaceIntakeError::CategoryNotAllowed { allowed: vec![/* L1 enumerates */] })?;
            Some(cat)
        } else {
            None
        };
        // Open the file via the canonical trust boundary
        let (file, identity) = WorkspaceSourceRegistry::open_validated(&self.workspace_root, &req.file_ref)
            .map_err(WorkspaceIntakeError::from_rejection)?;
        let file_id = pipeline::file_id_from_identity(&identity, &self.workspace_root)?;
        // Construct typed IngestRequest
        let request = IngestRequest {
            file, identity, file_id: file_id.clone(),
            source_asof: identity.canonical_path.metadata()?.modified()?.into(),
            source_type, entity, mode, category_hint,
        };
        // Run pipeline
        let pipeline = build_pipeline();
        let receipt = pipeline.run(&conn, request)?;
        Ok(WorkspaceIntakeReceipt {
            run_id: receipt.ingestion_run_id.to_string(),
            file_id: receipt.file_id,
            content_sha256: receipt.content_sha256,
            lifecycle_state_after_slug: receipt.lifecycle_state_after.as_slug().to_string(),
            resolved_path: receipt.resolved_path,
        })
    }
}
```

`LifecycleRepo` also exposes the §0 V1.3 §3 helpers W2-D consumes:

```rust
pub fn get(
    conn: &Connection,
    file_id: &str,
) -> Result<Option<WorkspaceFileLifecycle>, LifecycleError>;

pub fn set_entity(
    conn: &Connection,
    file_id: &str,
    entity_type: EntityType,
    entity_id: &str,
    entity_name: Option<&str>,
) -> Result<(), LifecycleError>;
```

Bootstrap registration is a single wiring line in `src-tauri/src/main.rs` or the existing Bootstrap location:

```rust
service_context_builder.workspace_intake(&ingest_pipeline_workspace_intake);
```

### Code-stub invariants

- `pipeline.rs` implements the §0 V1.2 §§2.1–2.4 DTO/error/file-id contract and the §0 §9 quarantine actor contract; it must not introduce alternate request, receipt, file-id, or actor definitions. The only W2-A-local error addition is `IngestError::FileIdMismatch { expected, found }`.
- `IngestError` has no `FileTooLarge` or `UnsupportedFormat` variants. Size and binary-format rejection return `IngestError::Rejected(RejectionReason::FileTooLarge)` / `IngestError::Rejected(RejectionReason::UnsupportedFormat)` per §0 V1.2 §2.3, with `RejectionMetadata` separate for logs/events.
- `pipeline.rs` imports `Extractor`, `SignalEmitter`, `FileIdentity`, `WorkspaceCategory`, `WorkspaceClaimProposal`, `RejectionReason`, and `WorkspaceFileKind` from `contracts.rs`.
- `pipeline.rs` consumes `RunsRepo` / `StartRunSeed` / `IngestionRunStatus` from `runs.rs` and `LifecycleRepo` from `lifecycle.rs`; lifecycle SQL is centralized in `lifecycle.rs`.
- `pipeline.rs` may hold a `File` value in `IngestRequest`; it must not call `File::open`, `OpenOptions`, `std::fs::read*`, metadata-open helpers, mmap, or any path-open equivalent. The line-anchored CI gate in §10 enforces §0 §8.
- `auto_detect_category_pure` is pure and registry-free. `validate_detected_category` is the only registry validation surface and delegates to `WorkspaceCategoryRegistry::validate(conn, &candidate, entity_type)`, falling back to `None` on validation failure.
- `IngestReceipt.claim_proposals` is present for W3-A compatibility, but W2-A returns an empty vector and records `claim_count_produced = 0`.
- `WorkspaceIntakeService::ingest` is async, its DTO surface uses raw slug fields, and the dailyos_lib implementation converts to typed pipeline fields before constructing `IngestRequest`; sync rusqlite/pipeline work stays behind `tokio::task::spawn_blocking`.
- `file_id_from_identity` uses `Sha256` + `hex` exactly as pinned in §0 V1.2 §2.4; no `blake3` dependency or formula is allowed.
- Pipeline rewinds `request.file` before invoking `Extractor::extract`; W1 `Extractor::extract` accepts `&File` only, so there is no extractor-buffer-handoff alternative. Hashing must not leave the extractor reading EOF.

### L1 implementation guardrails

- Start from the handle in `IngestRequest.file`; do not derive a new path read from `identity.canonical_path`.
- Re-derive and compare `request.file_id` before any DB writes; mismatch is `IngestError::FileIdMismatch { expected, found }`.
- Compute `content_sha256` from the bounded content stream before `RunsRepo::start_run`.
- Enforce the 10MB default cap before allocating a full content buffer.
- Sniff only the first 4KB for frontmatter category detection, and prove this at the pipeline read boundary rather than only by unit-testing `auto_detect_category_pure`.
- Treat caller-provided `category_hint` as already registry-valid by contract.
- Treat auto-detected category output as provisional until registry validation passes.
- V1.5: `EntityRef.entity_name` IS the path segment between `{Accounts|People|Projects}/` and the category dir (live `registry.rs:448`). The bridge slug-validates via `is_valid_slug_shape` before `IngestRequest` is constructed; `pipeline.run()` trusts the typed field. Routing uses `entity_type` + `entity_name` + category, not entity_type + category alone.
- Pre-hash rejection paths (for example file too large before read allocation) do not create a `document_ingestion_runs` row because W1-C requires non-null `content_sha256`; they transition lifecycle to `Rejected` and emit a typed rejection. Post-hash handled failures complete the run as `Failed`.
- Preserve the no-claim W2 behavior even if `Extractor::extract` is swapped early in a local workspace.
- Complete failed runs with typed error detail; never leave a fresh run permanently `in_progress` on handled rejection paths.
- Emit via `SignalEmitter` only; do not call the signal bus directly from `pipeline.rs`.
- Strip canonical paths from rejection logs/Tauri events; reduce `std::io::Error` to `ErrorKind` + `raw_os_error()` before emission.
- Always populate `resolved_path` in the receipt and pass it through the bridge; no `read.entity_names` redaction applies in the local-to-local single-user topology.
- Keep all examples generic (`file_name.md`, `account-12345`); no customer-specific fixtures in this lane.

## 10. Tests required

End-to-end ingestion test against a fixture file (records ingestion run, `LifecycleRepo` transitions to `ingested`, category persists through `LifecycleRepo::update_category`, signal-emitter hook is called, claim count = 0); file size limit rejection through `IngestError::Rejected(RejectionReason::FileTooLarge)`; binary detection rejection through `IngestError::Rejected(RejectionReason::UnsupportedFormat)`; pre-hash rejection lifecycle row with no run row; idempotency test (re-ingesting same `(file_id, content_sha256, mode)` produces one successful ingestion run, not two); typed `quarantine_source` API test; rejection log/Tauri event privacy tests; caller-provided `file_id` mismatch rejection test; `LifecycleRepo::set_entity` / `LifecycleRepo::get` tests; raw-slug intake bridge parsing/validation tests; bridge registration/async wrapping tests; inline abilities-runtime services module test; `cargo test` + `cargo clippy -D warnings` clean.

Required test files/gates:

- `tests/workspace_ingestion_w2a_auto_detect_category.rs` table-driven tests:
  1. frontmatter `doc_type` priority wins over filename/extension;
  2. same-priority first match wins;
  3. filename globs are case-insensitive;
  4. extension fallback maps `.pdf` to `Attachments` and `.md` / `.txt` to `Notes`;
  5. `Other(slug)` registry hit survives validation;
  6. invalid-category or registry-disallowed result falls back to `None`;
  7. pure function tolerates malformed/over-bound frontmatter by returning `None` without leaking content.
- `tests/workspace_ingestion_w2a_content_head_bound.rs` proves the pipeline reads only the first 4KB for `content_head` before category sniffing. This must exercise `IngestPipeline::run` or the immediate pipeline helper, not only `auto_detect_category_pure(&str)`.
- `tests/workspace_ingestion_w2a_file_id.rs` asserts caller-provided `IngestRequest.file_id` must equal `file_id_from_identity(&identity, workspace_root)`; mismatch returns typed `IngestError::FileIdMismatch { expected, found }` before run/lifecycle writes.
- `tests/workspace_ingestion_w2a_lifecycle_repo.rs` covers `LifecycleRepo::{insert_pending, transition, record_user_override, update_category}` and asserts no inline lifecycle SQL is required outside `lifecycle.rs`.
- `tests/workspace_ingestion_w2a_lifecycle_transitions.rs` covers `LifecycleRepo::transition` legal vs illegal transition pairs, including illegal transitions returning `LifecycleError::InvalidStateTransition`.
- `tests/workspace_ingestion_w2a_lifecycle_repo_set_entity.rs` covers `LifecycleRepo::set_entity`, including a legal pending-row entity assignment and nonexistent-file-id rejection.
- `tests/workspace_ingestion_w2a_lifecycle_repo_get.rs` covers `LifecycleRepo::get`, including `None` for a nonexistent file_id and a populated `WorkspaceFileLifecycle` row for an existing file_id.
- `tests/workspace_ingestion_w2a_quarantine_actor.rs` asserts `quarantine_source` accepts typed `QuarantineActor` only, records `user_override`, transitions to `quarantined`, and is idempotent on an already-quarantined file.
- `tests/workspace_ingestion_w2a_rejection_privacy.rs` asserts rejection log lines and Tauri event payloads do not contain canonical path strings and serialize I/O errors only as `ErrorKind` + `raw_os_error()`.
- `tests/workspace_ingestion_w2a_file_cursor.rs` uses a fake extractor to verify hashing/content sniffing does not leave the extractor reading EOF; `seek(0)` happens before `Extractor::extract(&File, ...)`.
- `tests/workspace_ingestion_w2a_workspace_intake_impl.rs` asserts `IngestPipelineWorkspaceIntake::ingest` is async, uses `tokio::task::spawn_blocking` for sync pipeline work, binds `workspace_root` at bridge construction, converts raw `WorkspaceIntakeRequest` slugs to typed `IngestRequest` fields, converts through `WorkspaceSourceRegistry::open_validated` + `file_id_from_identity`, calls `pipeline.run(&conn, request)`, and passes through `resolved_path`.
- `tests/workspace_ingestion_w2a_intake_impl_slug_parse.rs` asserts each invalid raw slug returns the matching typed `WorkspaceIntakeError` variant (`InvalidSourceTypeSlug`, `InvalidModeSlug`, `InvalidCategorySlug`).
- `tests/workspace_ingestion_w2a_intake_impl_category_validate.rs` asserts `WorkspaceCategoryRegistry::validate` runs before pipeline invocation and disallowed categories return `CategoryNotAllowed`.
- `tests/workspace_ingestion_w2a_service_context_registration.rs` asserts `ServiceContext` exposes `workspace_intake()` and app bootstrap registers `IngestPipelineWorkspaceIntake`.
- `tests/workspace_ingestion_w2a_inline_mod_services.rs` asserts `src-tauri/abilities-runtime/src/lib.rs` declares `services::workspace_intake` inside the existing inline `pub mod services { ... }` block and that no `src-tauri/abilities-runtime/src/services/mod.rs` file is created for this lane.
- `src-tauri/tests/workspace_mutation_allowlist_test.rs` pins `check_workspace_mutation_allowlist.sh` scope: workspace lifecycle/run/link table writes plus direct workspace-file filesystem writes outside `services/workspace_ingestion`.
- Signal tests assert `emit_file_ingested`, `emit_file_rejected`, `emit_file_pending_entity_assignment`, and `emit_file_quarantined` are called through `SignalEmitter` and not through the signal bus directly.
- `src-tauri/scripts/check_workspace_mutation_allowlist.sh` exists, is executable, and is green.

### CI grep gate

Add `tests/workspace_ingestion_w2a_no_direct_open.rs`. This replaces the V1.0 substring-grep with a line-anchored check per §0 §8: strip comment lines, reject path-opening tokens in executable lines (including bare `File::open` after `use std::fs::File;`), and reject `OpenOptions` / `tokio::fs` / read-helper imports while allowing `std::fs::File` only as the §0 §2.1 request handle type.

```rust
#[test]
fn pipeline_never_opens_paths_directly() {
    let source = std::fs::read_to_string(
        "src-tauri/src/services/workspace_ingestion/pipeline.rs",
    )
    .expect("pipeline.rs should be readable");

    let forbidden = [
        "File::open",
        "std::fs::File::open",
        "std::fs::File::options",
        "std::fs::OpenOptions",
        "tokio::fs::File::open",
        "tokio::fs::File::options",
        "tokio::fs::OpenOptions",
        "std::fs::read",
        "std::fs::read_to_string",
        "std::fs::metadata",
        "memmap",
        "std::process::Command",
    ];
    let forbidden_imports = [
        "OpenOptions",
        "tokio::fs",
        "read_to_string",
        "metadata",
        "memmap",
        "Command",
    ];

    for (line_no, raw_line) in source.lines().enumerate() {
        let line = raw_line.trim_start();
        if line.starts_with("//") {
            continue;
        }
        if line.starts_with("use ") {
            for token in forbidden_imports {
                assert!(
                    !line.contains(token),
                    "pipeline.rs must not import path-opening helper {token:?} at line {}",
                    line_no + 1,
                );
            }
            continue;
        }
        for token in forbidden {
            assert!(
                !line.contains(token),
                "pipeline.rs must consume the validated File handle; forbidden token {token:?} at line {}",
                line_no + 1,
            );
        }
    }
}
```

Required final verification:

- `cargo test`
- `cargo clippy -- -D warnings`
- `tests/workspace_ingestion_w2a_no_direct_open.rs` green
- `src-tauri/scripts/check_workspace_mutation_allowlist.sh` green
- W2-A L2 `/cso` sign-off recorded

## 11. Done when

`pipeline.run(&conn, request)` consumes the canonical typed §0 V1.2 §2.1 request, revalidates caller-provided `file_id` with `IngestError::FileIdMismatch { expected, found }`, records an ingestion run when `content_sha256` exists, and produces zero claim proposals (W3-A's job to land them). `LifecycleRepo::{insert_pending, transition, record_user_override, update_category, set_entity, get}` present and tested; pipeline records ingestion run via RunsRepo and transitions lifecycle via LifecycleRepo (no inline SQL outside lifecycle.rs). **Cycle 10 contract reconciliation:** `category_hint` is contracted to be a registry-valid `WorkspaceCategory` value when `Some`. Caller-side validation (DOS-474, W2-C, W2-D, W5-A) is responsible for rejecting malformed/disallowed caller-provided slugs BEFORE constructing `IngestRequest` — the pipeline trusts the type. When `category_hint` is `None`, `auto_detect_category_pure()` runs as fallback and `validate_detected_category()` performs the registry-bound validation per §0 §7. The pipeline never sees an "invalid" hint at the type level. `workspace_intake_impl.rs` translates raw slugs to typed enums via `WorkspaceFileKind::from_slug`, `IngestionMode::from_slug`, `EntityType::from_str_lossy`, and `WorkspaceCategoryRegistry::validate` before constructing `IngestRequest`. `pipeline::quarantine_source` API present and tested with typed `QuarantineActor`; file content rejection paths return typed errors through §0 V1.2 §2.3; `resolved_path` is populated and passed through; `ServiceContext` extended with `workspace_intake`; bootstrap registers `IngestPipelineWorkspaceIntake`; `entity_intake` ability invocation (via W2-C) successfully reaches `IngestPipeline` through the bridge; no migration shipped; CI lint script `check_workspace_mutation_allowlist.sh` (modeled on `check_claim_writer_allowlist.sh`) green; all tests green; `/cso` L0 plan AND L2 diff approval recorded.

## 12. Reviewer panel

- codex challenge
- codex consult
- architect-reviewer
- `/cso`

**Pass rule:** unanimous APPROVE.

## 13. Path-α Appendix

- `IngestError::DbError` remains stringly-typed for now as a substrate-wide pattern; file under DOS-751 rather than widening W2-A.
- AST-based `pipeline.rs` no-direct-open lint for path-opening APIs should replace the line-anchored grep gate after W2, filed under DOS-751 (cycle-2 architect F7 plus prior `/cso` no-direct-open hardening).
- Drive content staging API for v1.4.6: de-scoped from W2-B per cycle-13 §13.3.2; future API bridges remote bytes to validated workspace `File` intake without direct Drive writes.
- v1.4.7 W3-A L0 packet must carry an ADR-0093 indirect-injection gate for the `content_head` / file-content to claim-content path (cycle-1 `/cso` F5).
- Bounded typed quarantine reason can replace `quarantine_source(reason: &str, ...)` after W2 if audit-string bounds become shared substrate rather than lane-local validation.
- Duplicate `EntityRef` names across crate boundaries should be cleaned up after W2 once the bridge DTOs settle.
