# L0 Packet - v1.4.5 W2-D - DOS-469 `_inbox/` Refactor to Unresolved Queue

**Current revision:** V1.3 (cycle-3 micro-fold for local-to-local trust topology + compile-shape fixes, 2026-05-21). See §2 Changelog.

## 0. Shared contract anchor

This packet consumes `.docs/plans/v1.4.5-workspace-memory/W2-shared-contract.md` as the frozen W2 surface. Any conflict between this lane packet and that shared contract resolves in favor of the shared contract. This packet cites the shared contract by section and does not duplicate its type definitions.

Required anchors for W2-D L1:

- §0 §2.1: `IngestRequest` is constructed from `(File, FileIdentity)`, a deterministic `file_id`, `source_asof`, `WorkspaceFileKind`, typed `EntityRef`, `IngestionMode`, and optional `WorkspaceCategory`.
- §0 §2.2: `IngestReceipt.lifecycle_state_after` is the command-layer branch point for inbox badge state.
- §0 §2.4: `pipeline::file_id_from_identity(&identity, workspace_root)` is the only allowed `file_id` derivation.
- §0 §3: all lifecycle writes use W2-A's `LifecycleRepo`; W2-D does not update lifecycle rows directly.
- §0 §5: `check_workspace_mutation_allowlist.sh` is created by W2-A and must remain green after W2-D.
- §0 §6: canonical provenance for inbox files is `DataSource::WorkspaceFile { kind: WorkspaceFileKind::Inbox }`.
- §0 §10: W2-D consumes canonical primitives and does not re-invent request fields, entity IDs, lifecycle mutations, or error variants.

## 1. Header

- **Date:** 2026-05-21
- **Project:** v1.4.5 - Workspace Memory Refactor
- **Lane:** v1.4.5 W2-D - DOS-469 (`_inbox/` refactor to unresolved queue)
- **Wave:** W2 stage 2b
- **Issue:** DOS-469
- **Branch:** implementation branch created by L1 from current W2 base
- **Worktree:** `/private/tmp/dailyos-v145-w2-l0`
- **Sandbox posture:** workspace-write only; this packet is the only file written
- **Trust topology:** local-to-local single-user.
- **Migration slots claimed:** none
- **Authority docs:** `.docs/plans/v1.4.5-waves.md`, `.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-A-DOS-463.md`, and §0 shared contract
- **Required substrate before L1:** W1-A lifecycle/ownership model, W1-B `WorkspaceSourceRegistry::open_validated`, W1-C ingestion-run and link tracking, W2-A `IngestPipeline::run` + `LifecycleRepo`, the W1-extension PR adding `IngestionMode::Realtime` + canonical `UserRelink`, and W2-A V1.3 `LifecycleRepo::set_entity`
- **L2 reviewer matrix:** codex-review + code-reviewer + architect-reviewer
- **Pacing note:** depends on W2-A merged before L1

## 2. Changelog

- **V1.3 - 2026-05-21 - cycle-3 micro-fold.** Declares local-to-local single-user trust topology and drops multi-actor security gates while keeping data hygiene and compile correctness. Closes `process_inbox_file` entity-assignment bypass (cycle-3 codex challenge F1): `process_inbox_file` no longer accepts `entity: Option<EntityAssignmentInput>`; frontend call sites at `src/pages/InboxPage.tsx:382-384,789-792` continue calling `process_inbox_file` without `entityId`, and entity assignment is only through `assign_inbox_entity`. Reorders `assign_inbox_entity` per cycle-3 codex challenge F2: all pre-write validation runs before `LinkRepo::add_link`; the link is written at the end of the transactional block after `open_validated`, `file_id` derivation, and `reopened_file_id == assignment.file_id`. Fixes `IngestPipeline::run` instance-call shape (`let pipeline = wiring::build_pipeline(); pipeline.run(conn, request)`) per cycle-3 codex challenge F3 + consult F1. Adds W2-A V1.3 precondition `LifecycleRepo::set_entity` for updating `entity_id` + `entity_type` on the existing `pending_entity_assignment` row before pipeline rerun; this is not a lifecycle transition. Drops entity ACL/scope-helper requirements; keeps `EntityType::from_slug`, entity-id UUID/slug format, lifecycle-state, `source_type = 'inbox'`, and idempotency checks as data hygiene/correctness. Limits inbox listing and assignment to `source_type = 'inbox'` rows per cycle-3 codex consult F3. Notes the `pipeline` instance is obtained via `wiring::build_pipeline()`.
- **V1.2 - 2026-05-21 - cycle-13 + cycle-2 fold.** `IngestionMode::Realtime` is now valid via cycle-13 §13.1: the W1-extension PR adds `EntitySeeded` and `Realtime` to `src-tauri/src/services/workspace_ingestion/runs.rs:32`. `LinkAttributionSource::UserAssignment` was invented in V1.1; W1-C already ships canonical `UserRelink` at `src-tauri/src/services/workspace_ingestion/link.rs:45-53`, and V1.2 uses `LinkAttributionSource::UserRelink`. Claims `src-tauri/src/lib.rs:713-718` only for the single-line `assign_inbox_entity` Tauri command registration, annotated with `dos7-allowed: w2d-assign-command-registration`. Adds the `assign_inbox_entity` security gate: entity authorization, lifecycle-state validation, idempotency, and no duplicate active link rows. Clarifies assignment ordering: W2-D writes `LinkRepo::add_link` first, then re-invokes `IngestPipeline::run`; the pipeline handles `PendingEntityAssignment -> Ingesting` internally through §0 V1.2 §3 `LifecycleRepo`, and W2-D does not call `LifecycleRepo::transition` directly. Purges the remaining stale inbox-specific `DataSource` alias reference; §0 V1.2 §6 and cycle-13 §13.3.6 keep `DataSource::WorkspaceFile { kind: WorkspaceFileKind::Inbox }` canonical.
- **V1.1 - 2026-05-21 - cycle-1 fold.** Adds §0 shared-contract anchor; removes all direct lifecycle/run mutation SQL from the W2-D command and processor plan; routes lifecycle writes through W2-A's `LifecycleRepo` and ingestion-run writes through W1-C's `RunsRepo` via `IngestPipeline::run`; rewrites `IngestRequest` construction against §0 §2.1; replaces local inbox path opening with `WorkspaceSourceRegistry::open_validated`; uses canonical `DataSource::WorkspaceFile { kind: WorkspaceFileKind::Inbox }`; adds entity-assignment contract using `LinkRepo::add_link`; pins `copy_to_inbox` to post-copy pipeline ingestion; fixes `get_inbox_files` schema reads to v250 columns only; adds inbox-result and `inbox-updated` wire-shape regression coverage; names W3-B/DOS-471 and W5-B/DOS-476 as explicit deferred consumers.
- **V1.0 - 2026-05-21 - initial packet.**

## 3. Goal

Refactor `_inbox/` processing to route through the staged ingestion service instead of the current classify-and-route pattern. Files that cannot be matched to a known entity become `pending_entity_assignment` lifecycle records rather than staying as raw, untracked inbox files. The inbox remains a drop zone, but lifecycle state, not directory presence, drives the unresolved queue.

`copy_to_inbox` is deterministic in V1.2: a file copies into `_inbox/` and immediately triggers `IngestPipeline::run` through the staged path. The watcher's role for `_inbox/` becomes a secondary backstop only. W2-D must not rely on watcher ownership for the primary post-copy ingestion path.

Entity assignment is a separate command (`assign_inbox_entity`). `process_inbox_file` never accepts entity assignment input. `assign_inbox_entity` validates the lifecycle row, confirms `source_type = 'inbox'`, re-opens through `WorkspaceSourceRegistry::open_validated`, confirms the reopened identity derives the same `file_id`, updates the existing lifecycle row's entity fields through W2-A's `LifecycleRepo::set_entity`, writes the `UserRelink` link row through `LinkRepo::add_link`, and then re-invokes the pipeline. The pipeline itself handles the lifecycle transition. W2-D does not transition lifecycle directly.

At W2 time, ingestion records lifecycle and ingestion-run state only. Claim proposal extraction is W3-A's job; the W2-A pipeline shells out to a zero-claim extractor and W3-A replaces it with the real extractor. Real inbox-to-claim rendering validation is deferred to W5-B/DOS-476.

## 4. Files owned (exclusive)

### Verbatim wave-plan ownership

`src-tauri/src/processor/mod.rs` (the `process_file` top-level function and its routing - refactored to call `IngestPipeline::run` instead of direct `router::move_file` for claim-producing processing); `src-tauri/src/processor/router.rs` (the `resolve_destination` + `move_file` functions - preserved for file-move post-ingestion, not for claim extraction); **the six existing `src-tauri/src/commands/workspace.rs` inbox functions** - exact named functions: `get_inbox_files`, `process_inbox_file`, `process_all_inbox`, `enrich_inbox_file`, `get_inbox_file_content`, and `copy_to_inbox` (W2-D is the only owner of these per cycle 2 amendment; W2-B is denied). V1.3 also owns the new `assign_inbox_entity` command inside the same inbox command section. Updated to read from `workspace_file_lifecycle` where appropriate. Does not modify `processor/classifier.rs` or `processor/extract.rs`. Does not touch non-inbox functions in `commands/workspace.rs` (email/profile/iCloud handlers remain out of scope). V1.3 keeps ownership of `src-tauri/src/lib.rs:713-718` for a single-line command-registration addition only: add `commands::assign_inbox_entity` beside the existing inbox command registrations with `// dos7-allowed: w2d-assign-command-registration`. W2-A V1.3, not W2-D, picks up `LifecycleRepo::set_entity` in `src-tauri/src/services/workspace_ingestion/lifecycle.rs`.

### Pinned per-function ownership ranges

These ranges come from the required greps plus surrounding `nl -ba` reads in this worktree.

| File | Function | Lines | W2-D ownership note |
|---|---:|---:|---|
| `src-tauri/src/commands/workspace.rs` | `get_inbox_files` | 55-98 | Replace raw `_inbox/` filesystem listing/status-log enrichment with lifecycle-row read for unresolved queue output. Preserve `InboxResult` wire shape. |
| `src-tauri/src/commands/workspace.rs` | `process_inbox_file` | 108-149 | Route single-file processing through `WorkspaceSourceRegistry::open_validated` + `IngestPipeline::run`; preserve background-thread behavior as needed. |
| `src-tauri/src/commands/workspace.rs` | `process_all_inbox` | 159-181 | Batch all inbox drop-zone files through the same staged ingestion path; do not reintroduce direct claim production. |
| `src-tauri/src/commands/workspace.rs` | `enrich_inbox_file` | 192-238 | Reconcile existing AI enrichment entry point with lifecycle state; unresolved output becomes `pending_entity_assignment`. |
| `src-tauri/src/commands/workspace.rs` | `get_inbox_file_content` | 242-291 | Preview content by lifecycle row/path for pending unresolved files while retaining extraction fallback behavior. |
| `src-tauri/src/commands/workspace.rs` | `copy_to_inbox` | 303-411 | Keep duplicate-safe drop-zone copy behavior; after each successful copy, call the staged ingestion path directly. |
| `src-tauri/src/commands/workspace.rs` | `assign_inbox_entity` | New command in inbox section | Validate pending inbox lifecycle row, set entity via `LifecycleRepo::set_entity`, write `UserRelink`, then rerun pipeline. |
| `src-tauri/src/processor/mod.rs` | `process_file` | 54-260 | Replace classify-route-as-claim-producing path with staged ingestion call; legacy routing only as post-ingestion file placement. |
| `src-tauri/src/processor/router.rs` | `resolve_destination` | 108-197 | Preserve destination resolution for file placement after lifecycle/ingestion, not as claim extraction authority. |
| `src-tauri/src/processor/router.rs` | `move_file` | 226-254 | Preserve physical file move helper for post-ingestion placement only. |
| `src-tauri/src/lib.rs` | Tauri command registration block | 713-718 | Single-line addition of `commands::assign_inbox_entity` beside existing inbox commands, with `dos7-allowed: w2d-assign-command-registration` comment. |

### Precision amendments from cycle 1

- Shared private helpers in `src-tauri/src/processor/mod.rs` lines `<262` are in scope for refactor when they support `process_file`. Do not change `process_user_attachment` itself without a compile-driven reason.
- Verify the W2-B call site at `src-tauri/src/watcher.rs:935` (`process_user_attachment(workspace, path, Some(&db))`) still compiles after `processor/mod.rs` helper movement.
- `src-tauri/src/commands/workspace.rs:413` starts the Emails Command section comment; `EmailsResult` starts at `:420`; `get_all_emails` starts at `:428`. W2-D must not edit those email handlers.

## 5. Don't touch

- `src-tauri/src/watcher.rs` is W2-B-owned. W2-D preserves existing `inbox-updated` emission sites but does not edit them.
- `src-tauri/src/executor.rs` is not a W2-D-owned file. Preserve the existing event behavior by compatibility tests, not by changing executor code.
- `src-tauri/src/services/workspace_ingestion/pipeline.rs`, `lifecycle.rs`, and `runs.rs` are W2-A/W1-C substrate surfaces. W2-D consumes them.
- `src-tauri/src/services/workspace_ingestion/link.rs` is W1-C-owned. W2-D consumes `LinkRepo::add_link` and `LinkAttributionSource::UserRelink`; it does not edit link substrate.
- `src-tauri/src/processor/classifier.rs` and `src-tauri/src/processor/extract.rs` remain outside this lane.
- Non-inbox functions in `commands/workspace.rs` remain out of scope.

W2-D may add command-local helper functions inside the owned inbox section of `commands/workspace.rs`, including a helper that emits `inbox-updated` with a lifecycle-derived count. That helper must not move into or edit `watcher.rs`/`executor.rs`.

## 6. K-in substrate audit

| Substrate | Current evidence | W2-D consumption rule |
|---|---|---|
| §0 shared contract | §0 V1.2 §2.1 `IngestRequest`, §0 V1.2 §2.2 `IngestReceipt`, §0 V1.2 §3 `LifecycleRepo`, §0 V1.2 §6 `DataSource` canonicalization. | Treat §0 as the contract. If W2-A lands different names, update W2-D to the merged names without changing the lane semantics. |
| `IngestPipeline::run` | W2-A-owned API in `src-tauri/src/services/workspace_ingestion/pipeline.rs`, constrained by §0 §2.1-§2.2. | Obtain the instance via `wiring::build_pipeline()` and call `pipeline.run(&conn, request)` with an opened `File` + `FileIdentity`. Do not construct path-based or async-only request shapes unless W2-A changes §0. |
| `WorkspaceSourceRegistry::open_validated` | `src-tauri/src/services/workspace_ingestion/registry.rs:148-151` returns `(File, FileIdentity)`. | Replace command-local path opening with this trust boundary. No direct `File::open` for inbox ingestion. |
| `file_id_from_identity` | §0 §2.4 defines deterministic `file_id` derivation. | Derive `file_id` only through `pipeline::file_id_from_identity(&identity, workspace_root)`. |
| `LifecycleState::PendingEntityAssignment` | `src-tauri/src/services/workspace_ingestion/lifecycle.rs:38-45` defines canonical enum; serde string is `pending_entity_assignment`. | Use the canonical enum/storage string for unresolved inbox rows. Do not invent a parallel "needs entity" lifecycle. |
| W2-A pending transition responsibility | `src-tauri/src/services/workspace_ingestion/lifecycle.rs:26-28` documents W2-A emitting `emit_file_pending_entity_assignment` when intake cannot resolve entity. | Remove W2-D direct transition helpers. Pipeline/LifecycleRepo own the transition. |
| `LifecycleRepo` | §0 V1.2 §3 assigns lifecycle write helpers to W2-A. W2-A V1.3 must add `LifecycleRepo::set_entity(conn, file_id, entity_type, entity_id)` in `src-tauri/src/services/workspace_ingestion/lifecycle.rs`. | W2-D does not call `LifecycleRepo::transition` directly. Assignment first updates the existing pending row's `entity_id` + `entity_type` via `LifecycleRepo::set_entity`, then re-enters `IngestPipeline::run`; the pipeline owns `PendingEntityAssignment -> Ingesting` via `LifecycleRepo`. No raw lifecycle mutation SQL in command or processor code. |
| W1-C `RunsRepo` | `src-tauri/src/services/workspace_ingestion/runs.rs` owns ingestion-run start/complete/idempotency. | W2-D receives run correlation through `IngestReceipt`. Any run write goes through pipeline/RunsRepo, never command SQL. |
| `LinkRepo::add_link` | `src-tauri/src/services/workspace_ingestion/link.rs:215-224` requires `file_id`, typed `EntityType`, `entity_id`, `LinkAttributionSource`, confidence, rationale, and actor. | Entity assignment writes a canonical document/entity link before re-ingestion with `EntityRef`. |
| `IngestionMode` | Cycle-13 §13.1 extends `src-tauri/src/services/workspace_ingestion/runs.rs:32` from `Initial | Incremental | Forced | Backfill` to include `EntitySeeded | Realtime`; this lands via the explicit W1-extension PR named in §1. | `copy_to_inbox`, `process_inbox_file`, and `assign_inbox_entity` construct `IngestRequest { mode: IngestionMode::Realtime, ... }` only after W1-extension is present. |
| `LinkAttributionSource` | `src-tauri/src/services/workspace_ingestion/link.rs:45-53` ships `UserRelink`; cycle-13 §13.3.6 says V1.1 `UserAssignment` was invalid. | `assign_inbox_entity` uses `LinkAttributionSource::UserRelink`. No W2-D-local attribution variant is allowed. |
| Tauri command registration | `src-tauri/src/lib.rs:713-718` contains existing inbox command registrations. Cycle-13 §13.3.6 recommends claiming the block for a single-line `assign_inbox_entity` registration. | W2-D adds one registration line plus `dos7-allowed: w2d-assign-command-registration`; no other `lib.rs` edits. |
| `DataSource` / `WorkspaceFileKind` | `src-tauri/abilities-runtime/src/abilities/provenance/source.rs:73-82` defines `DataSource::WorkspaceFile { kind }`; `:201-208` defines `WorkspaceFileKind::Inbox`; ADR-0107 `:268-270` freezes the same shape. | Use `DataSource::WorkspaceFile { kind: WorkspaceFileKind::Inbox }` everywhere. |
| v250 lifecycle schema | `src-tauri/src/migrations/250_workspace_file_lifecycle.sql:20-36` defines `file_id`, `canonical_path`, `source_type`, `lifecycle_state`, `source_asof`, `entity_id`, `entity_type`, `content_sha256`, `created_at`, `updated_at` plus identity/audit columns. | `get_inbox_files` reads only existing columns. UI-only fields come from existing wire defaults, metadata, or preview path, not nonexistent DB columns. |
| Entity input hygiene | Local-to-local single-user trust topology; no multi-principal entity owner exists in this command surface. | Do not add an ACL/scope helper. `assign_inbox_entity` validates typed `entity_type` and `entity_id` format before writes; existence beyond link/lifecycle constraints remains data hygiene, not authorization. |
| Existing `inbox-updated` event | `src-tauri/src/watcher.rs:95`, `src-tauri/src/watcher.rs:454`, `src-tauri/src/executor.rs:909`. | Preserve event name and mixed payload compatibility. Command-owned emissions use lifecycle count; W3-B/DOS-471 consumes lifecycle signals. |
| Registry path behavior | `WorkspaceCategoryRegistry::resolve_path` treats `WorkspaceFileKind::Inbox` as `_inbox/{filename}` at `src-tauri/src/services/workspace_ingestion/registry.rs:424-428`. | Preserve inbox as drop zone; lifecycle state drives unresolved UI. |
| ADR-0104 | `.docs/decisions/0104-execution-mode-and-mode-aware-services.md`. | `IngestionMode::Realtime` is used for command-triggered inbox ingestion. |
| ADR-0105 | `.docs/decisions/0105-provenance-as-first-class-output.md`. | Provenance is an output of ingestion, not a command-local side effect. |
| ADR-0113 | `.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md`. | User assignment attribution must be explicit and distinguishable from automated classification. |
| ADR-0126 | `.docs/decisions/0126-memory-substrate-invariants.md:122`. | Do not mint new claim or lifecycle mutation APIs that bypass services. |

### K-in docs grep paths

Before L1 implementation, grep these paths for substrate names and stale vocabulary:

```bash
rg -n 'workspace_file_lifecycle|pending_entity_assignment|DataSource::WorkspaceFile|WorkspaceFileKind::Inbox|IngestPipeline|inbox-updated|LifecycleRepo|RunsRepo|LinkRepo|UserAssignment|UserRelink|IngestionMode::Realtime' docs/solutions .docs/decisions .docs/plans/v1.4.5-workspace-memory
```

## 7. Intelligence Loop gate

1. *Claim model:* Inbox files route through `IngestPipeline::run`, which records the ingestion run and lifecycle transitions. Claim production is W3-A's job; at W2 time the pipeline produces zero claims. Files without entity assignment remain as `pending_entity_assignment` lifecycle records. Real ingestion-to-claim rendering validation moves to W5-B/DOS-476.
2. *Provenance + trust:* `source_asof` is the file mtime captured after `WorkspaceSourceRegistry::open_validated` returns. DataSource is `DataSource::WorkspaceFile { kind: WorkspaceFileKind::Inbox }` per §0 V1.2 §6. Last sweep must show no stale inbox-specific `DataSource` aliases anywhere in the W2-D packet or implementation.
3. *Signals + invalidation:* Existing `app_handle.emit("inbox-updated", ...)` sites are preserved at `watcher.rs:95`, `watcher.rs:454`, and `executor.rs:909`. W2-D command-owned emissions may emit the same event with `{ count }` derived from unresolved lifecycle rows. New lifecycle state transitions emit signals through W3-B/DOS-471.
4. *Runtime + surfaces:* `get_inbox_files` returns lifecycle-backed unresolved queue entries, not a raw filesystem listing. The command reads only v250 lifecycle columns; `sizeBytes`, `preview`, and `suggestedEntityName` remain wire-shape fields populated from metadata/fallbacks or `None`, not DB columns.
5. *Feedback loop:* User assigns an entity to a `pending_entity_assignment` inbox file -> validate all inputs and source row -> re-open through `WorkspaceSourceRegistry::open_validated` -> verify derived `file_id` matches -> call `LifecycleRepo::set_entity` -> call `LinkRepo::add_link(conn, &file_id, entity_type, &entity_id, LinkAttributionSource::UserRelink, 1.0, Some("User assigned inbox file to entity"), "user")` -> re-invoke `pipeline.run(conn, request)` with `EntityRef`, `source_type: WorkspaceFileKind::Inbox`, and `mode: IngestionMode::Realtime`. The pipeline handles `PendingEntityAssignment -> Ingesting` through `LifecycleRepo` internally. W2-D does not call `LifecycleRepo::transition` directly and does not write lifecycle/run SQL inline.

### Security gate

`assign_inbox_entity` is a local trusted-user command surface. It must reject before link creation unless all data-hygiene and correctness checks pass:

- `file_id` must exist in `workspace_file_lifecycle`; missing rows return typed `NotFound`.
- `entity_type` must parse to typed `EntityType`; unknown slugs return typed `InvalidEntityType`.
- `entity_id` must be non-empty and match the accepted UUID/slug format; invalid values return typed `InvalidEntityId`.
- The lifecycle state for `file_id` must be `PendingEntityAssignment`; any other source state, including already `Ingested` or rejected/tombstoned file lifecycle rows, returns typed `InvalidLifecycleState`.
- The lifecycle row must have `source_type = 'inbox'`; this is a scope limit, not an authorization gate. Non-inbox pending files return typed `NotInboxFile` and never surface in inbox command output.
- Actor authorization is not a gate in the local-to-local single-user topology. `actor: QuarantineActor::User { user_id }` or the literal local actor `"user"` remains audit-trail data hygiene, but there is no scope check on an accessor.
- Entity ACL helpers are not required and must not be invented here. In single-user local mode, `entity_type` parsing and `entity_id` format validation are data hygiene; there is no separate entity owner to authorize against.
- Idempotency is a correctness gate: repeated assignment of the same `(file_id, entity_type, entity_id)` returns `Ok` with the existing active link/receipt semantics and does not create a duplicate active link row.
- Link errors from `LinkRepo::add_link` are mapped without stringly collapse: `NotFound`, `AlreadyRejected`, `EntityTypeUnknown`, and `Tombstoned` stay typed.

## 8. Code stub

This is a shape stub for L1. It is not a substrate definition. Exact module paths and helper names must follow W2-A/W1-C after merge, with §0 taking precedence.

```rust
struct InboxLifecycleRow {
    file_id: String, canonical_path: String, source_type: String,
    lifecycle_state: String, source_asof: String,
    entity_id: Option<String>, entity_type: Option<String>,
    content_sha256: Option<String>, created_at: String, updated_at: String,
}

#[tauri::command]
pub async fn get_inbox_files(state: State<'_, Arc<AppState>>) -> Result<InboxResult, String> {
    let rows: Vec<InboxLifecycleRow> = state
        .db_read(|conn| {
            // Read only columns present in migration v250. No size/preview/suggested-name
            // pseudo columns are allowed here.
            query_lifecycle_rows(
                conn,
                "SELECT file_id, canonical_path, source_type, lifecycle_state, \
                        source_asof, entity_id, entity_type, content_sha256, \
                        created_at, updated_at \
                   FROM workspace_file_lifecycle \
                  WHERE source_type = 'inbox' \
                    AND lifecycle_state IN ('pending_entity_assignment', \
                                            'pending', 'ingesting', 'rejected') \
                  ORDER BY updated_at DESC",
            )
        })
        .await
        .map_err(|e| e.to_string())?;

    let files: Vec<InboxFile> = rows.into_iter().map(|row| {
        let path = Path::new(&row.canonical_path);
        InboxFile {
            filename: path.file_name().and_then(|n| n.to_str()).unwrap_or(&row.file_id).into(),
            path: row.canonical_path,
            size_bytes: std::fs::metadata(path).map(|m| m.len()).unwrap_or(0),
            modified: row.source_asof,
            preview: None,
            file_type: InboxFileType::Other,
            processing_status: Some(row.lifecycle_state),
            processing_error: None,
            suggested_entity_name: None,
        }
    }).collect();

    let count = files.len();
    if files.is_empty() {
        Ok(InboxResult::Empty { message: "Inbox is clear".into(), files, count })
    } else {
        Ok(InboxResult::Success { files, count })
    }
}

fn ingest_opened_inbox_file(
    conn: &rusqlite::Connection,
    workspace_root: &Path,
    file: File,
    identity: FileIdentity,
    entity: Option<EntityRef>,
) -> Result<IngestReceipt, IngestError> {
    let source_asof = file_mtime_utc(&identity)?;
    let file_id = pipeline::file_id_from_identity(&identity, workspace_root)?;
    let pipeline = wiring::build_pipeline();
    pipeline.run(conn, IngestRequest {
        file,
        identity,
        file_id,
        source_asof,
        source_type: WorkspaceFileKind::Inbox,
        entity,
        mode: IngestionMode::Realtime,
        category_hint: None,
    })
}

#[tauri::command]
pub async fn process_inbox_file(
    filename: String,
    state: State<'_, Arc<AppState>>,
    app: AppHandle,
) -> Result<crate::processor::ProcessingResult, String> {
    let config = state
        .config
        .read()
        .clone()
        .ok_or("No configuration loaded")?;
    let workspace_root = PathBuf::from(&config.workspace_path);
    let path = workspace_root.join("_inbox").join(&filename);
    let receipt = state
        .db_write(|conn| {
            let (file, identity) = WorkspaceSourceRegistry::open_validated(&workspace_root, &path)
                .map_err(IngestError::Rejected)?;
            ingest_opened_inbox_file(conn, &workspace_root, file, identity, None)
        })
        .await?;

    if matches!(receipt.lifecycle_state_after, LifecycleState::PendingEntityAssignment) {
        emit_inbox_updated_from_lifecycle_count(&app, &state).await?;
    }

    match receipt.lifecycle_state_after {
        LifecycleState::Ingested => Ok(crate::processor::ProcessingResult::Routed {
            classification: "workspace_inbox".into(),
            destination: receipt.resolved_path.unwrap_or_default(),
        }),
        LifecycleState::PendingEntityAssignment => Ok(crate::processor::ProcessingResult::NeedsEntity {
            classification: "workspace_inbox".into(),
            suggested_name: String::new(),
        }),
        LifecycleState::Rejected | LifecycleState::Quarantined => Ok(crate::processor::ProcessingResult::Error {
            message: "Inbox ingestion failed".into(),
        }),
        _ => Ok(crate::processor::ProcessingResult::NeedsEnrichment),
    }
}

#[tauri::command]
// V1.3: validation BEFORE link write; pipeline owns lifecycle transition.
pub async fn assign_inbox_entity(
    state: State<'_, AppState>,
    file_id: String,
    entity_type: String,
    entity_id: String,
) -> Result<(), AssignError> {
    let conn = state.db_write()?; // transactional

    // Step 1: validate inputs (data hygiene, not security)
    let typed_entity_type = EntityType::from_slug(&entity_type)
        .ok_or(AssignError::InvalidEntityType)?;
    if entity_id.is_empty() || !is_valid_id_format(&entity_id) {
        return Err(AssignError::InvalidEntityId);
    }

    // Step 2: verify lifecycle row exists + state is PendingEntityAssignment + source_type = inbox
    let row = LifecycleRepo::get(&conn, &file_id)?.ok_or(AssignError::FileNotFound)?;
    if row.lifecycle_state != LifecycleState::PendingEntityAssignment {
        return Err(AssignError::WrongLifecycleState);
    }
    if row.source_type != WorkspaceFileKind::Inbox {
        return Err(AssignError::NotInboxFile);
    }

    // Step 3: re-open the file via the canonical trust boundary
    let workspace_root = state.workspace_root();
    let (file, identity) = WorkspaceSourceRegistry::open_validated(workspace_root, &row.canonical_path)
        .map_err(AssignError::from_open)?;

    // Step 4: verify reopened file_id matches assignment file_id
    let reopened_file_id = pipeline::file_id_from_identity(&identity, workspace_root)?;
    if reopened_file_id != file_id {
        return Err(AssignError::FileIdMismatch);
    }

    // Step 5: NOW all checks passed. Update lifecycle entity fields + write link.
    LifecycleRepo::set_entity(&conn, &file_id, typed_entity_type, &entity_id)?;
    LinkRepo::add_link(
        &conn,
        &file_id,
        typed_entity_type,
        &entity_id,
        LinkAttributionSource::UserRelink,
        1.0,
        Some("User assigned inbox file to entity"),
        "user",
    )?;

    // Step 6: re-invoke pipeline; pipeline transitions lifecycle internally.
    let pipeline = wiring::build_pipeline();
    let _receipt = pipeline.run(&conn, IngestRequest {
        file,
        identity,
        file_id: file_id.clone(),
        source_asof: row.source_asof,
        source_type: WorkspaceFileKind::Inbox,
        entity: Some(EntityRef {
            entity_type: typed_entity_type,
            entity_id: EntityId::from(entity_id.clone()),
            entity_name: None,
        }),
        mode: IngestionMode::Realtime, // valid via cycle-13 W1-extension PR
        category_hint: None,
    })?;

    Ok(())
}
```

L1 guardrails for the stub:

- `get_inbox_files` may use read SQL for lifecycle rows, but it must read only v250 columns listed above and must filter all rows by `source_type = 'inbox'`. It must not read `size_bytes`, `preview`, or `suggested_entity_name` from lifecycle rows.
- `process_inbox_file` opens through `WorkspaceSourceRegistry::open_validated`, derives `file_id` with §0 §2.4, builds §0 §2.1 `IngestRequest`, and branches on §0 §2.2 `lifecycle_state_after`. It does not accept `entity`, `entityId`, or any entity assignment input.
- Frontend `src/pages/InboxPage.tsx:382-384,789-792` must remove `entityId` from `process_inbox_file` invocations; the assignment button calls `assign_inbox_entity` as a separate user action.
- Delete any W2-D-local unmatched-entity transition helper. Pipeline + `LifecycleRepo` emit that transition per §0 §3 and `lifecycle.rs:26-28`.
- No direct lifecycle/run mutation SQL is allowed in `commands/workspace.rs` or `processor/mod.rs`.
- `copy_to_inbox` calls the same staged ingestion helper after each successful copy. Watcher processing remains a secondary backstop only.
- Entity assignment validates input, lifecycle state, `source_type = 'inbox'`, `open_validated`, and derived `file_id` before writes. It then calls `LifecycleRepo::set_entity`, writes through `LinkRepo::add_link` with `LinkAttributionSource::UserRelink`, and re-runs `pipeline.run` with a typed `EntityRef`. The caller does not call `LifecycleRepo::transition`; the pipeline owns the transition through §0 V1.2 §3.
- The retired V1.2 multi-actor input sketch is intentionally absent. The local audit actor is `"user"` (or the merged `QuarantineActor::User` equivalent if W2-A exposes that shape), not a command parameter.

## 9. Tests required

### Verbatim wave-plan tests

Inbox file produces a lifecycle record with `pending_entity_assignment` state when entity cannot be matched; inbox file with matched entity advances to `ingested` lifecycle state and writes an ingestion run record; zero claim proposals are expected at W2 time; `inbox-updated` event fires after ingestion; `cargo test` + `cargo clippy -D warnings` clean. Real-flow inbox-to-claim-render validation moves to W5-B/DOS-476.

### Regression-test names per refactored command

- `get_inbox_files_returns_pending_entity_assignment_lifecycle_rows`
- `get_inbox_files_excludes_resolved_ingested_non_inbox_rows`
- `get_inbox_files_preserves_inbox_result_wire_shape`
- `get_inbox_files_empty_state_preserved`
- `get_inbox_files_reads_only_v250_lifecycle_columns`
- `inbox_listing_filters_by_source_type.rs`
- `get_inbox_file_content_reads_pending_lifecycle_row_path`
- `get_inbox_file_content_rejects_path_traversal_from_lifecycle_filename`
- `process_inbox_file_unmatched_entity_creates_pending_entity_assignment_row`
- `process_inbox_file_matched_entity_records_ingested_run_with_zero_claims`
- `process_inbox_file_uses_open_validated_for_inbox_paths`
- `process_inbox_file_rejects_entity_param_compile_check.rs`
- `process_inbox_file_repeat_same_file_hash_is_idempotent`
- `process_all_inbox_routes_each_file_through_ingest_pipeline`
- `process_all_inbox_preserves_per_file_result_on_partial_failure`
- `enrich_inbox_file_unmatched_entity_transitions_lifecycle_not_status_log_only`
- `copy_to_inbox_preserves_duplicate_filename_resolution`
- `copy_to_inbox_triggers_staged_ingestion_after_copy`
- `copy_to_inbox_emits_inbox_updated_count_refresh_after_pipeline_receipt`
- `processor_process_file_uses_ingest_pipeline_for_inbox_processing`
- `processor_process_user_attachment_watcher_call_site_still_compiles`
- `router_resolve_destination_preserved_for_post_ingestion_placement`
- `router_move_file_preserves_unique_destination_suffix_behavior`
- `inbox_updated_event_payload_shape_accepts_watcher_count_payload`
- `inbox_updated_event_payload_shape_accepts_executor_unit_payload`
- `pending_entity_assignment_lifecycle_visible_in_inbox_query`
- `assign_inbox_entity_adds_user_relink_link_and_reruns_pipeline`
- `assign_inbox_entity_writes_zero_claim_ingestion_run_at_w2`
- `assign_inbox_entity_nonexistent_file_id_returns_typed_rejection`
- `assign_inbox_entity_wrong_lifecycle_state_returns_typed_rejection`
- `assign_inbox_entity_non_inbox_source_type_returns_typed_rejection`
- `assign_inbox_entity_repeated_same_file_entity_type_entity_is_idempotent`
- `assign_inbox_entity_rejected_file_lifecycle_returns_typed_rejection`
- `assign_inbox_entity_rejects_invalid_entity_type`
- `assign_inbox_entity_rejects_invalid_entity_id`
- `assign_inbox_entity_link_not_written_on_validation_failure.rs`
- `assign_inbox_entity_no_direct_lifecycle_transition.rs`
- `ingest_request_construction_accepts_ingestion_mode_realtime`
- `ingestion_mode_realtime_round_trips_storage_and_runtime`
- `link_attribution_user_relink_writes_fresh_active_row`
- `link_attribution_user_relink_bypasses_tombstone_guard_per_w1c_link_rs_743_755`

Keep all V1.2 typed-rejection coverage that still applies under local-to-local single-user topology: nonexistent file, wrong lifecycle state, non-inbox source type, rejected lifecycle, invalid `entity_type`, invalid `entity_id`, and idempotent repeat assignment. The retired actor/ACL rejection tests are replaced by the no-entity-param compile check and the explicit local audit actor rule.

### Security and CI gates

- `open_validated` path coverage proves no direct `File::open` for inbox ingestion.
- No direct lifecycle/run mutation SQL outside `src-tauri/src/services/workspace_ingestion/*.rs`.
- No direct `LifecycleRepo::transition` call from W2-D command/processor code.
- `check_workspace_path_validation.sh` green.
- `src-tauri/scripts/check_workspace_mutation_allowlist.sh` green once W2-A creates it per §0 §5.
- `cargo test`
- `cargo clippy -- -D warnings`

## 10. Done when

The W1-extension PR has landed before W2-D L1 starts, including `IngestionMode::Realtime` and `UserRelink` as the canonical user-assignment attribution. W2-A V1.3 has landed `LifecycleRepo::set_entity` in `src-tauri/src/services/workspace_ingestion/lifecycle.rs`.

Inbox commands and processor routing use `wiring::build_pipeline().run(...)`; `process_inbox_file` no longer takes any entity parameter; `pending_entity_assignment` files are visible in inbox command output only when `source_type = 'inbox'`; lifecycle transitions and ingestion run records are correct with zero proposals at W2 time; existing inbox UI behavior is preserved, including empty state, wire shape, preview path, and badge refresh.

`assign_inbox_entity` is implemented in the owned inbox command section, registered in `src-tauri/src/lib.rs:713-718`, writes `LinkAttributionSource::UserRelink`, re-invokes the pipeline with `IngestionMode::Realtime`, and has all assign-related tests green: nonexistent file, wrong lifecycle state, non-inbox source type, invalid entity type/id, repeated assignment idempotency, rejected-file lifecycle handling, link-not-written on validation failure, and no direct lifecycle transition call.

The transactional ordering is verified: `assign_inbox_entity` validates type/id, lifecycle row existence, `PendingEntityAssignment`, `source_type = 'inbox'`, canonical `open_validated`, and reopened `file_id` match before `LifecycleRepo::set_entity` and before `LinkRepo::add_link`. If any check fails, no link row exists.

Real-data proof is required before L2: drop a file into `_inbox/`, run `process_inbox_file`, verify the lifecycle row, verify the ingestion run row, verify `inbox-updated` refreshes the badge, and verify preview still works. Claim rendering remains deferred to W5-B/DOS-476.

No direct lifecycle/run SQL may remain in `commands/workspace.rs` or `processor/mod.rs`; no direct `LifecycleRepo::transition` call may appear in W2-D code; `src-tauri/scripts/check_workspace_mutation_allowlist.sh` must be green.

## 11. Reviewer panel

- codex challenge
- codex consult
- architect-reviewer

**Pass rule:** unanimous APPROVE.

## 12. PATH-alpha appendix

- Normalize `inbox-updated` payload shape (`watcher` emits `{ count }`, `executor` emits `()`) - file to DOS-751.
- Add a typed lifecycle-row query API instead of packet-local pseudo DAO - file to DOS-751 (cycle-3 codex consult F3 PATH-alpha).
- Off-by-15 line-comment in §4 - fixed inline in V1.1, not deferred to PATH-alpha.
- Add an mtime helper to `FileIdentity` so callers do not need a post-open metadata call after `WorkspaceSourceRegistry::open_validated`.
