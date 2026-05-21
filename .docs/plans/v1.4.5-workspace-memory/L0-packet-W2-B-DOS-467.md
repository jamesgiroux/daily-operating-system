# L0 Packet - v1.4.5 W2-B - DOS-467 Services Refactor of Mutation Paths

**Current revision:** V1.4 (cycle-4 reviewer mechanical fold, 2026-05-21). See §2 Changelog.

## §0 Shared Contract Anchor

Authoritative W2 shared surface:

- `.docs/plans/v1.4.5-workspace-memory/W2-shared-contract.md`

This packet cites the shared contract by section and does not duplicate its full content.
Any conflict between this W2-B packet and the shared contract resolves in favor of the
shared contract. The key W2-B anchors are:

- §0 V1.2 §2.1 - canonical `IngestRequest` shape.
- §0 V1.2 §2.4 - canonical `file_id_from_identity` derivation.
- Shared contract §5 - `check_workspace_mutation_allowlist.sh` ownership.
- Shared contract §7 - category validation contract.
- Shared contract §10 - no substrate reinvention.

## §1 Header

- **Date:** 2026-05-21
- **Project:** v1.4.5 - Workspace Memory Refactor
- **Wave:** W2 stage 2b
- **Issue:** DOS-467 - Services refactor of mutation paths
- **Branch:** `wave/v1.4.5-w2-l0`
- **Worktree:** `/private/tmp/dailyos-v145-w2-l0`
- **Trust topology:** local-to-local single-user.
- **Migration slots claimed:** none
- **Authority docs:** `.docs/plans/v1.4.5-waves.md`; `.docs/plans/v1.4.5-workspace-memory/L0-packet-W1-A-DOS-463.md`; `src-tauri/src/services/workspace_ingestion/contracts.rs`; §0 shared contract
- **L2 reviewer matrix:** codex-review + code-reviewer + architect-reviewer
- **Pacing note:** This L0 runs against frozen W1 contracts and assumes W2-A has merged before L1 starts. W2-B must consume W2-A's merged `IngestPipeline::run(&Connection, IngestRequest)` surface from §0 V1.2 §2.1 rather than inventing a parallel mutation boundary.
- **L1 precondition:** First implementation step is to read W2-A's merged `pipeline.rs`, `registry.rs`, and §0 V1.2 §2.1. If the merged code conflicts with the shared contract, stop for a W2 contract amendment before editing W2-B call sites.

## §2 Changelog

- **V1.4 - 2026-05-21:** Lands the cycle-4 critical helper-disposition table immediately after the function-level call-site inventory, closing the third recurrence of the same unfolded finding. The table explicitly keeps `accounts::write_account_markdown`, `people::write_person_markdown`, `projects::write_project_markdown`, `accounts::sync_content_index_for_account`, and `projects::sync_content_index_for_project` with `dos7-allowed: entity-markdown-regen` call-site comments, and it also pins the inbox and Drive staging allowlist comments. Tightens §8 caller wiring around `watcher.rs::handle_account_changes`: W2-B uses `wiring::build_pipeline()?`, obtains `&Connection` with `db.conn_ref()`, keeps the account upsert as an entity-table write, keeps `write_account_markdown` with the allowlist comment, and routes any true workspace-file ingestion through `pipeline.run(conn, request)`.
- **V1.3 - 2026-05-21:** Adds trust topology declaration. Closes the helper-writer ambiguity: `write_person_markdown`, `write_account_markdown`, and `write_project_markdown` are explicitly allowlisted with `dos7-allowed: entity-markdown-regen` because they regenerate markdown summaries from entity DB state already canonical to the local user's workspace, not workspace-file ingestion. Adds concrete caller wiring: W2-B accesses the pipeline through `wiring::build_pipeline()` and passes `db.conn_ref()` as the `&Connection` from `ActionDb` watcher context. Adds failure-path test names for pipeline rejection after DB upsert, duplicate watcher-event idempotency, and real workspace edit lifecycle/run/event/markdown preservation proof. Fixes the `IngestPipeline::run` call shape to the instance method `pipeline.run(conn, request)`.
- **V1.2 - 2026-05-21:** Folds cycle-13 §13.3.2 and cycle-2 reviewer findings into this packet. Drive content writes at `src-tauri/src/google_drive/poller.rs:199` and `:208` are explicitly de-scoped to v1.4.6 per cycle-13 §13.3.2; file the v1.4.6 ticket for a `workspace_ingestion::staging` API that bridges remote Drive bytes to a validated `File`. Drops the self-vacating AC test names and removes all `if_supported` / `or_documents_staging_gap` language in favor of concrete CI gates. Drops the staging-gap escape valve language from §10/§11 and replaces it with explicit `dos7-allowed: drive-staging-v146` allowlist comments at the two Drive write sites. Replaces any `blake3` formula references with `sha256` per §0 V1.2 §2.4. Confirms §0 V1.2 §2.1 and W2-A are the canonical `IngestRequest` source; W2-B does not invent fields, DTOs, or a parallel mutation boundary.
- **V1.1 - 2026-05-21:** Folds §0 shared contract; reconciles §8 stub against W2-A's canonical `IngestRequest` (`file` + `identity` + `file_id` + `source_asof` + `source_type` + `entity` + `mode` + `category_hint`); replaces stringly `entity_id` with typed `EntityRef`; replaces `registry::open_validated` free-function call with method-call `WorkspaceSourceRegistry::open_validated`; explicitly de-scopes account/project/person/transcript DB writes and keeps them as-is; allowlists `watcher.rs:81` bootstrap with rationale; preserves existing `app_handle.emit` calls and does not refer to `emit_signal`; drops async run signature; clarifies CI script consumption: W2-A creates, W2-B verifies.
- **V1.0 - 2026-05-21:** Initial packet.

## §3 Goal

### Wave-plan goal, verbatim

Refactor existing mutation call sites in `commands/`, `watcher.rs`, `processor/`, `google_drive/`, `granola/`, `quill/` that bypass `services::` to route through `services::workspace_ingestion`. This is a mechanical refactor that makes the W1-A CI invariant (no direct writes outside service boundary) go from vacuous to enforced.

### V1.2 precision

W2-B's CI gate activates for newly-introduced direct workspace-file writes only. The two pre-existing Drive writes at `src-tauri/src/google_drive/poller.rs:199` and `:208` are explicitly allowlisted with `dos7-allowed: drive-staging-v146` rationale comments; their refactor moves to a v1.4.6 ticket per cycle-13 §13.3.2. W2-B does NOT introduce a new staging API.

Where existing call sites perform writes that cannot reduce to `IngestPipeline::run` (account/project/person upserts; transcript DB inserts), those writes stay in place. W2-B's CI gate covers direct workspace-file-mutation paths, not entity-table upserts.

The broad wave-plan nouns `commands/` and `processor/` remain historical context only. Function-level ownership for this lane is the inventory in §4. `src-tauri/src/commands/workspace.rs`, `src-tauri/src/processor/mod.rs`, and `src-tauri/src/processor/router.rs` remain outside W2-B.

## §4 Files Owned Exclusive

### Wave-plan ownership, verbatim

- **Files owned (exclusive):** Call sites (function-level): `src-tauri/src/watcher.rs` (the content-change handler that calls `accounts`, `people`, `projects` file I/O directly); `src-tauri/src/google_drive/poller.rs` (Drive sync write sites); `src-tauri/src/granola/poller.rs` (transcript sync write sites); `src-tauri/src/quill/poller.rs` (transcript sync write sites). The list of exact function names must be enumerated in the L0 plan from a fresh grep at W1 close: `rg 'fn.*_inline|fn.*process_file|fn.*sync_file|fn.*write_entity' src-tauri/src/` scoped to those files. **Cycle 2 amendment:** `src-tauri/src/commands/workspace.rs` is removed from W2-B ownership and assigned entirely to W2-D (inbox refactor lane). W2-B and W2-D no longer share files.

### V1.2 scope precision

- `src-tauri/src/watcher.rs` is owned only for file-write call sites and event-preservation checks named below.
- `src-tauri/src/google_drive/poller.rs` is owned only to annotate the two pre-existing Drive workspace-file writes at `:199` and `:208` with the cycle-13 §13.3.2 v1.4.6 de-scope rationale.
- `src-tauri/src/granola/poller.rs` and `src-tauri/src/quill/poller.rs` are owned for direct workspace-file mutation checks and event preservation. Their transcript DB writes stay in place.
- W2-B does not own `src-tauri/scripts/check_workspace_mutation_allowlist.sh` or `src-tauri/tests/workspace_mutation_allowlist_test.rs`; §0 shared contract §5 assigns creation to W2-A. W2-B consumes the gate.

### V1.2 direct file-write disposition

- `src-tauri/src/google_drive/poller.rs:199` - `std::fs::create_dir_all` - keep in v1.4.5; add `dos7-allowed: drive-staging-v146` lint comment because Drive remote-content staging is de-scoped to v1.4.6 per cycle-13 §13.3.2.
- `src-tauri/src/google_drive/poller.rs:208` - `std::fs::write` - keep in v1.4.5; add `dos7-allowed: drive-staging-v146` lint comment because Drive remote-content staging is de-scoped to v1.4.6 per cycle-13 §13.3.2.
- `src-tauri/src/watcher.rs:81` - `std::fs::create_dir_all` for inbox bootstrap - keep; add `dos7-allowed: inbox-bootstrap` lint comment because this creates the watched inbox directory and does not create a lifecycle row.
- All other workspace-file ingestion call sites in W2-B-touched files, beyond the explicit allowlists in this section, must route through §0 V1.2 §2.1 `IngestPipeline::run(&Connection, IngestRequest)` or fail CI.

### V1.3 entity-markdown helper disposition

| Helper | Disposition | Rationale |
|---|---|---|
| `accounts::write_account_markdown` | KEEP, allowlist with `dos7-allowed: entity-markdown-regen` | Entity-table data -> markdown summary file. Not workspace-file ingestion (no claim production). Local-to-local user files. |
| `people::write_person_markdown` | KEEP, allowlist with `dos7-allowed: entity-markdown-regen` | Same. |
| `projects::write_project_markdown` | KEEP, allowlist with `dos7-allowed: entity-markdown-regen` | Same. |
| `accounts::sync_content_index_for_account` | KEEP, allowlist | Content-index cache, not ingestion. |
| `projects::sync_content_index_for_project` | KEEP, allowlist | Same. |

### Required grep 1: function-name seed

Command:

```sh
rg -n 'fn .*_inline|fn .*process_file|fn .*sync_file|fn .*write_entity' src-tauri/src/watcher.rs src-tauri/src/google_drive/poller.rs src-tauri/src/granola/poller.rs src-tauri/src/quill/poller.rs
```

Result at L0 drafting time: zero lines. The current files do not use those suffix/name patterns; the exact enclosing functions below are therefore enumerated from the direct mutation grep plus function-boundary scan.

### Required grep 2: direct mutation lines

```text
src-tauri/src/google_drive/poller.rs:199:    std::fs::create_dir_all(&docs_dir)
src-tauri/src/google_drive/poller.rs:208:    std::fs::write(&file_path, content).map_err(|e| format!("Failed to write file: {}", e))?;
src-tauri/src/granola/poller.rs:438:                match db.upsert_action_if_not_completed(&db_action) {
src-tauri/src/watcher.rs:81:            if let Err(e) = std::fs::create_dir_all(&inbox_dir) {
src-tauri/src/watcher.rs:722:                if db.upsert_account(&account).is_ok() {
src-tauri/src/watcher.rs:763:                if db.upsert_project(&project).is_ok() {
src-tauri/src/quill/poller.rs:493:                        match db.upsert_action_if_not_completed(&db_action) {
```

### Required grep 3: signal emission lines

Command:

```sh
rg -n 'emit_signal' src-tauri/src/watcher.rs src-tauri/src/google_drive/poller.rs src-tauri/src/granola/poller.rs src-tauri/src/quill/poller.rs
```

Result at L0 drafting time: zero lines. W2-B preserves existing Tauri `app_handle.emit` calls; it does not add, remove, or test `emit_signal` calls in this lane.

Relevant existing event emissions:

```text
src-tauri/src/watcher.rs:95:        let _ = app_handle.emit(
src-tauri/src/granola/poller.rs:579:            let _ = app_handle.emit("transcript-processed", &outcome);
src-tauri/src/granola/poller.rs:586:            let _ = app_handle.emit("transcript-processed", &meeting_id.to_string());
src-tauri/src/quill/poller.rs:636:            let _ = app_handle.emit("transcript-processed", &outcome);
src-tauri/src/quill/poller.rs:643:            let _ = app_handle.emit("transcript-processed", &meeting_id.to_string());
```

### Required grep 4: DB write evidence in transcript pollers

Command:

```sh
rg -n 'db\.(upsert|insert|execute)' src-tauri/src/granola/poller.rs src-tauri/src/quill/poller.rs
```

Result:

```text
src-tauri/src/quill/poller.rs:415:                        let _ = db.insert_capture(
src-tauri/src/quill/poller.rs:428:                        let _ = db.insert_capture(
src-tauri/src/quill/poller.rs:441:                        let _ = db.insert_capture(
src-tauri/src/quill/poller.rs:493:                        match db.upsert_action_if_not_completed(&db_action) {
src-tauri/src/granola/poller.rs:337:                let _ = db.insert_capture(
src-tauri/src/granola/poller.rs:350:                let _ = db.insert_capture(
src-tauri/src/granola/poller.rs:363:                let _ = db.insert_capture(
src-tauri/src/granola/poller.rs:438:                match db.upsert_action_if_not_completed(&db_action) {
```

Additional line-pinned DB/state writes in the same semantic class, not matched by that exact regex:

```text
src-tauri/src/granola/poller.rs:322:            let _ = db.update_meeting_transcript_metadata(
src-tauri/src/granola/poller.rs:463:            let _ = crate::quill::sync::transition_state(
src-tauri/src/granola/poller.rs:483:                let _ = crate::quill::sync::transition_state(
src-tauri/src/quill/poller.rs:399:                    let _ = db.update_meeting_transcript_metadata(
```

### Function-level call-site inventory

`src-tauri/src/watcher.rs`:

- `pub fn start_watcher(...)` at `src-tauri/src/watcher.rs:62`; direct mutation at `src-tauri/src/watcher.rs:81` (`std::fs::create_dir_all(&inbox_dir)`). V1.2 explicitly allowlists this with rationale: directory bootstrap, not file mutation; no lifecycle row created. Required lint comment placeholder:

```rust
// dos7-allowed: inbox-bootstrap - directory bootstrap, not file mutation; no lifecycle row created
```

- `fn handle_people_changes(...)` at `src-tauri/src/watcher.rs:629`; mutation helpers at `src-tauri/src/watcher.rs:667` (`people::upsert_person_and_restore_entity_links`) and `src-tauri/src/watcher.rs:674` (`people::write_person_markdown`). The person/entity-table helper stays as-is. The markdown regeneration call is explicitly allowed with call-site rationale `dos7-allowed: entity-markdown-regen - entity DB state -> local markdown summary; not workspace-file ingestion`.
- `fn handle_account_changes(...)` at `src-tauri/src/watcher.rs:692`; direct entity-table mutation at `src-tauri/src/watcher.rs:722` (`db.upsert_account(&account)`) stays as-is. The markdown regeneration call at `src-tauri/src/watcher.rs:727` (`accounts::write_account_markdown`) is explicitly allowed with call-site rationale `dos7-allowed: entity-markdown-regen - entity DB state -> local markdown summary; not workspace-file ingestion`.
- `fn handle_project_changes(...)` at `src-tauri/src/watcher.rs:741`; direct entity-table mutation at `src-tauri/src/watcher.rs:763` (`db.upsert_project(&project)`) stays as-is. The markdown regeneration call at `src-tauri/src/watcher.rs:768` (`projects::write_project_markdown`) is explicitly allowed with call-site rationale `dos7-allowed: entity-markdown-regen - entity DB state -> local markdown summary; not workspace-file ingestion`.
- `fn handle_account_content_changes(...)` at `src-tauri/src/watcher.rs:783`; content-index mutation helper at `src-tauri/src/watcher.rs:814` (`accounts::sync_content_index_for_account`) stays as-is with explicit allowlist rationale: content-index cache, not ingestion. Preserve watcher-triggered enrichment; do not fold content-index cache behavior into claims.
- `fn handle_project_content_changes(...)` at `src-tauri/src/watcher.rs:850`; content-index mutation helper at `src-tauri/src/watcher.rs:881` (`projects::sync_content_index_for_project`) stays as-is with explicit allowlist rationale: content-index cache, not ingestion. Preserve watcher-triggered enrichment.
- `fn handle_user_attachment_changes(...)` at `src-tauri/src/watcher.rs:917`; processing call at `src-tauri/src/watcher.rs:935` (`crate::processor::process_user_attachment`) and embedding wake at `src-tauri/src/watcher.rs:940-947`. W2-B preserves the trigger; W2-D owns processor inbox routing and W2-A owns pipeline internals.

`src-tauri/src/google_drive/poller.rs`:

- `async fn sync_watched_source(...)` at `src-tauri/src/google_drive/poller.rs:93`; call sites at `src-tauri/src/google_drive/poller.rs:107` (`save_content_to_entity`), `src-tauri/src/google_drive/poller.rs:154` (`download_and_save_file`), and sync-token updates at `src-tauri/src/google_drive/poller.rs:123` and `src-tauri/src/google_drive/poller.rs:178` (`sync::mark_synced`).
- `pub fn save_to_entity_docs(...)` at `src-tauri/src/google_drive/poller.rs:186`; direct workspace-file mutations at `src-tauri/src/google_drive/poller.rs:199` (`std::fs::create_dir_all`) and `src-tauri/src/google_drive/poller.rs:208` (`std::fs::write`). These two writes remain in v1.4.5 with `dos7-allowed: drive-staging-v146` comments because cycle-13 §13.3.2 de-scopes the Drive remote-content staging API to v1.4.6.
- `fn save_content_to_entity(...)` at `src-tauri/src/google_drive/poller.rs:214`; delegates to `save_to_entity_docs` at `src-tauri/src/google_drive/poller.rs:227`.
- `async fn download_and_save_file(...)` at `src-tauri/src/google_drive/poller.rs:237`; delegates to `save_content_to_entity` at `src-tauri/src/google_drive/poller.rs:243`.

`src-tauri/src/granola/poller.rs`:

- `fn process_granola_document(...)` at `src-tauri/src/granola/poller.rs:244`; transcript processing call at `src-tauri/src/granola/poller.rs:299` (`process_fetched_transcript_without_db_with_kind`), DB/state writes at `src-tauri/src/granola/poller.rs:322`, `337`, `350`, `363`, `438`, `463`, `483`, and retry advance at `src-tauri/src/granola/poller.rs:496`. These are transcript metadata, capture, action, and sync-state writes; W2-B keeps them as-is because they are entity-table/state writes, not workspace-file writes.
- `fn emit_transcript_processed(...)` at `src-tauri/src/granola/poller.rs:561`; Tauri event emissions at `src-tauri/src/granola/poller.rs:579` and `src-tauri/src/granola/poller.rs:586`. Preserve these emissions after any refactor.
- `fn poll_once(...)` at `src-tauri/src/granola/poller.rs:112`; calls `emit_transcript_processed` at `src-tauri/src/granola/poller.rs:215`.
- `pub fn trigger_granola_sync_for_meeting(...)` at `src-tauri/src/granola/poller.rs:669`; calls `emit_transcript_processed` at `src-tauri/src/granola/poller.rs:779`.

`src-tauri/src/quill/poller.rs`:

- `async fn process_sync_row(...)` at `src-tauri/src/quill/poller.rs:102`; state-machine writes at `src-tauri/src/quill/poller.rs:121`, `134`, `155`, `187`, `223`, `253`, `275`, `299`, `312`, `348`, `372`, `527`, and `542`; transcript processing call at `src-tauri/src/quill/poller.rs:377`; DB writes at `src-tauri/src/quill/poller.rs:399`, `415`, `428`, `441`, and direct grep match at `src-tauri/src/quill/poller.rs:493` (`db.upsert_action_if_not_completed`). These are transcript metadata, capture, action, and sync-state writes; W2-B keeps them as-is because they are entity-table/state writes, not workspace-file writes.
- `fn emit_transcript_processed(...)` at `src-tauri/src/quill/poller.rs:622`; Tauri event emissions at `src-tauri/src/quill/poller.rs:636` and `src-tauri/src/quill/poller.rs:643`. Preserve these emissions after any refactor.

### V1.4 entity-markdown helper disposition (3rd-cycle recurring finding fold)

These helpers generate workspace files from entity-table state and are NOT workspace_ingestion mutations. They KEEP, with explicit `dos7-allowed: entity-markdown-regen` allowlist comments at each call site so the W2-A `check_workspace_mutation_allowlist.sh` script recognizes them.

| Helper | Defined at | Called from | Allowlist comment line |
|---|---|---|---|
| `accounts::write_account_markdown` | (look up at L1) | `watcher.rs:727` (handle_account_changes) | `// dos7-allowed: entity-markdown-regen` |
| `people::write_person_markdown` | (look up at L1) | `watcher.rs:674` (handle_people_changes) | `// dos7-allowed: entity-markdown-regen` |
| `projects::write_project_markdown` | (look up at L1) | `watcher.rs:768` (handle_project_changes) | `// dos7-allowed: entity-markdown-regen` |
| `accounts::sync_content_index_for_account` | (look up at L1) | `watcher.rs:814` (handle_account_content_changes) | `// dos7-allowed: entity-markdown-regen` |
| `projects::sync_content_index_for_project` | (look up at L1) | `watcher.rs:881` (handle_project_content_changes) | `// dos7-allowed: entity-markdown-regen` |
| `watcher.rs:81` (`std::fs::create_dir_all` for inbox bootstrap) | n/a | n/a | `// dos7-allowed: inbox-bootstrap` |
| `google_drive/poller.rs:199` (`std::fs::create_dir_all`) | n/a | n/a | `// dos7-allowed: drive-staging-v146` (cycle-13 §13.3.2) |
| `google_drive/poller.rs:208` (`std::fs::write`) | n/a | n/a | `// dos7-allowed: drive-staging-v146` (cycle-13 §13.3.2) |

The W2-A allowlist script reads these comments and skips the marked lines from the mutation-allowlist enforcement. ALL other workspace-file write paths in W2-B-owned files MUST route through `IngestPipeline::run` or fail the lint.

## §5 Don't Touch

- **Don't touch:** `services/claims.rs` (not a mutation-path caller; leave untouched); `processor/enrich.rs` (intel_queue enrichment is not a workspace-file mutation; separate concern); `src-tauri/src/accounts.rs` / `people.rs` / `projects.rs` standalone files except through owned call-site behavior in `watcher.rs`; `src-tauri/src/commands/workspace.rs` (W2-D owns; do not touch any function in this file); `src-tauri/src/processor/mod.rs` and `src-tauri/src/processor/router.rs` (W2-D owns); `src-tauri/scripts/check_workspace_mutation_allowlist.sh` and `src-tauri/tests/workspace_mutation_allowlist_test.rs` creation (W2-A owns per §0 shared contract §5).
- Do not add `workspace_ingestion::staging` helpers in W2-B. Cycle-13 §13.3.2 assigns the Drive remote-bytes-to-validated-`File` bridge to v1.4.6; W2-B only annotates the two existing Drive write sites and keeps the no-new-direct-write gate strict for every other workspace-file mutation path.

## §6 K-in Substrate Audit

| Contract / substrate | Location verified | W2-B relationship |
|---|---|---|
| Canonical `IngestRequest` | §0 V1.2 §2.1 | **Consumes.** Request carries `file`, `identity`, `file_id`, `source_asof`, `source_type`, typed `entity: Option<EntityRef>`, `mode`, and `category_hint`. No `source_path`, no `data_source`, no stringly `entity_id`. |
| Canonical `file_id` helper | §0 V1.2 §2.4 | **Consumes.** W2-B derives `file_id` with `pipeline::file_id_from_identity(&identity, workspace_root)`. No lane-local hash formula. |
| CI script ownership | §0 shared contract §5 | **Consumes.** W2-A creates `check_workspace_mutation_allowlist.sh`; W2-B keeps it green after this lane's edits. |
| Category validation | §0 shared contract §7 | **Consumes.** Callers validate `category_hint` upstream before constructing `IngestRequest`; W2-B does not introduce string category ingress. |
| Substrate reuse policy | §0 shared contract §10 | **Consumes.** Reuses W2-A pipeline surface; no new helpers and no parallel DTOs. |
| `WorkspaceSourceRegistry::open_validated` | `src-tauri/src/services/workspace_ingestion/registry.rs:148`; §0 V1.2 §2.1 | **Consumes.** Use the associated method call before constructing a request. No free-function `registry::open_validated` call. |
| `SignalEmitter` trait | `src-tauri/src/services/workspace_ingestion/contracts.rs:207-219`; ADR-0101 service-boundary-enforcement.md:27; ADR-0115 signal-granularity-audit.md:20 | **Preservation boundary only.** W2-B does not add W3-B signal variants. It must not remove existing event emission while W2-A/W3-B own pipeline signal wiring. |
| Signal swallow class | `docs/solutions/architecture-patterns/emit-or-log-wrapper-silent-error-swallow-class-2026-05-18.md:13` | **Preserves observed behavior.** Existing `app_handle.emit` best-effort calls remain. W2-B may add tests around payload shape; it does not silently drop emissions. |
| `services::claims::ClaimProposal` | `src-tauri/src/services/claims.rs:74` | **Referenced only.** W2-B does not write claims and does not edit `services/claims.rs`. |
| `services::claims::commit_claim` | `src-tauri/src/services/claims.rs:5581` | **Forbidden direct call in W2-B.** Claim production is W3-A through the pipeline extractor, not this lane. |
| Existing claim writer allowlist | `src-tauri/scripts/check_claim_writer_allowlist.sh`; commit substrate comment at `src-tauri/src/services/claims.rs:3-6`; update allowlist at `src-tauri/src/services/claims.rs:367` | **Preserved.** W2-B does not weaken claim writer lint. |

Reviewers must grep these documentation paths before L2 approval:

- `docs/solutions/README.md`
- `docs/solutions/architecture-patterns/capability-boundary-needs-crate-split-not-grep-2026-05-18.md`
- `docs/solutions/architecture-patterns/emit-or-log-wrapper-silent-error-swallow-class-2026-05-18.md`
- `docs/solutions/conventions/migration-filename-version-offset-2026-05-18.md`
- `docs/solutions/security-issues/prompt-channel-sensitivity-class-sweep-2026-05-18.md`
- `docs/solutions/test-failures/parallel-test-singleton-state-flake-2026-05-18.md`
- `docs/solutions/tooling-decisions/codex-worktree-isolation-incompatible-with-rescue-forwarder-2026-05-18.md`
- `docs/solutions/tooling-decisions/phpcs-warning-severity-zero-prevents-warning-only-ci-fails-2026-05-19.md`
- `docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md`
- `.docs/decisions/0042-per-operation-pipelines.md`
- `.docs/decisions/0050-universal-file-extraction.md`
- `.docs/decisions/0080-signal-intelligence-architecture.md`
- `.docs/decisions/0101-service-boundary-enforcement.md`
- `.docs/decisions/0115-signal-granularity-audit.md`

K-in grep at L0 returned no direct hits in `docs/solutions` or `.docs/decisions` for `check_workspace_mutation_allowlist`, `workspace_file_lifecycle`, `document_ingestion_runs`, or `document_entity_links`. Cycle-1 broader substrate hits are now folded through ADR-0101, ADR-0115, and the signal-swallow architecture note above.

## §7 Intelligence Loop Gate

1. *Claim model:* W2-B produces no claims directly, and W2-A's W2 pipeline shell produces zero claim proposals. Confirm by grep that no refactored call site contains a direct `INSERT INTO intelligence_claims` or direct `services::claims::commit_claim` call.
2. *Provenance + trust:* File-write call sites that can route through W2-A pass the validated `File`, `FileIdentity`, canonical `file_id`, and `source_asof` required by §0 V1.2 §2.1. Confirm no routed call site re-opens the path after `WorkspaceSourceRegistry::open_validated`.
3. *Signals + invalidation:* Existing `app_handle.emit` calls (`inbox-updated` at `watcher.rs:95`, `transcript-processed` at `granola:579/586` and `quill:636/643`) are preserved; no removal of any existing emission.
4. *Runtime + surfaces:* Watcher-triggered enrichment (`intel_queue` enqueue) must be preserved; W2-B is refactoring file-write paths, not enrichment triggers.
5. *Feedback loop:* No change to feedback path.
6. *Dependency security:* W2-B relies on §0 shared contract §7 for typed category validation and typed `EntityRef` construction. It must not widen the surface to accept raw `entity_type`, raw `entity_id`, or frontmatter category strings at this lane boundary.

## §8 Code Stub

This packet chooses `wiring::build_pipeline()?` for W2-B watcher call sites. Do not use a `state.workspace_intake_pipeline()?` extension method in this lane unless the shared W2 contract is amended first. The watcher already owns `ActionDb`; obtain the W2-A `&Connection` with `let conn = db.conn_ref();`.

Complete L1 refactor sketch for `watcher.rs::handle_account_changes` around `src-tauri/src/watcher.rs:692-740`: preserve the account table upsert, keep `write_account_markdown` with a `dos7-allowed: entity-markdown-regen` comment, and route any true workspace-file ingestion through W2-A's canonical pipeline. This pattern applies only when the caller has an existing workspace-relative file. It does not apply to Drive remote content; Drive writes are explicitly out of scope for v1.4.5 per cycle-13 §13.3.2.

```rust
// watcher.rs BEFORE: handle_account_changes around src-tauri/src/watcher.rs:722
if db.upsert_account(&account).is_ok() {
    let _ = accounts::write_account_markdown(workspace, &account, Some(&json), &db);
}
```

```rust
// watcher.rs AFTER shape for handle_account_changes around lines 692-740.
// The account upsert remains an entity-table write. Only a true workspace-file
// ingestion path goes through the exact §0 V1.2 §2.1 request shape.
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use std::path::Path;
use crate::entity::EntityType;
use crate::services::workspace_ingestion::contracts::WorkspaceCategory;
use crate::services::workspace_ingestion::pipeline::{
    file_id_from_identity, EntityRef, IngestError, IngestPipeline, IngestReceipt, IngestRequest,
};
use crate::services::workspace_ingestion::registry::WorkspaceSourceRegistry;
use crate::services::workspace_ingestion::runs::IngestionMode;
use crate::services::workspace_ingestion::wiring;
use abilities_runtime::abilities::provenance::source::{
    EntityId, WorkspaceFileKind,
};

fn handle_account_changes_after_w2b_refactor(
    db: &crate::db::ActionDb,
    workspace: &Path,
    workspace_root: &Path,
    account_json_relative_path: &Path,
    account: crate::db::DbAccount,
    json: serde_json::Value,
) -> Result<Option<IngestReceipt>, IngestError> {
    // KEEPS: entity-table write, not workspace-file mutation.
    if !db.upsert_account(&account).is_ok() {
        return Ok(None);
    }

    // If this watcher branch has a workspace-relative file that would otherwise
    // be written directly, route that ingestion through W2-A's pipeline.
    let pipeline = wiring::build_pipeline()?;
    let conn = db.conn_ref();
    let receipt = ingest_account_dashboard_after_upsert(
        &pipeline,
        conn,
        workspace_root,
        account_json_relative_path,
        &account,
    )?;

    // KEEPS: entity-table state -> local markdown summary, not workspace ingestion.
    // dos7-allowed: entity-markdown-regen - entity DB state -> local markdown summary; not workspace-file ingestion
    let _ = crate::accounts::write_account_markdown(workspace, &account, Some(&json), db);

    Ok(Some(receipt))
}

fn ingest_account_dashboard_after_upsert(
    pipeline: &IngestPipeline,
    conn: &Connection,
    workspace_root: &Path,
    account_json_relative_path: &Path,
    account: &crate::db::DbAccount,
) -> Result<IngestReceipt, IngestError> {
    let (file, identity) = WorkspaceSourceRegistry::open_validated(
        workspace_root,
        account_json_relative_path,
    )
    .map_err(IngestError::Rejected)?;

    let source_asof: DateTime<Utc> = identity
        .canonical_path
        .metadata()
        .and_then(|metadata| metadata.modified())
        .map_err(IngestError::Io)?
        .into();

    // §0 V1.2 §2.4: sha256_hex_lower(workspace_relative_path_bytes)[..16].
    let file_id = file_id_from_identity(&identity, workspace_root)
        .map_err(|_| IngestError::Rejected(crate::services::workspace_ingestion::contracts::RejectionReason::OutsideWorkspace))?;

    let entity = Some(EntityRef {
        entity_type: EntityType::Account,
        entity_id: EntityId::new(account.id.clone()),
        entity_name: Some(account.name.clone()),
    });

    let category_hint: Option<WorkspaceCategory> = None;

    let request = IngestRequest {
        file,
        identity,
        file_id,
        source_asof,
        source_type: WorkspaceFileKind::EntityDoc,
        entity,
        mode: IngestionMode::Incremental,
        category_hint,
    };

    pipeline.run(conn, request)
}
```

Stub constraints:

- W2-A is the canonical source for `IngestRequest`, `EntityRef`, `IngestReceipt`, `IngestError`, `IngestPipeline`, and `file_id_from_identity`; W2-B does not invent fields or DTOs.
- `IngestRequest` carries exactly `file`, `identity`, `file_id`, `source_asof`, `source_type`, `entity`, `mode`, and `category_hint` per §0 V1.2 §2.1.
- No `source_path` or `data_source` field exists in the W2-B request construction.
- `entity` is `Option<EntityRef>`, not `Option<String>` and not stringly `entity_id`.
- `IngestPipeline::run` has the synchronous signature `fn run(&self, conn: &Connection, request: IngestRequest) -> Result<IngestReceipt, IngestError>`.
- Call-site entry obtains the pipeline through `wiring::build_pipeline()?`, obtains the DB connection through `db.conn_ref()`, and calls the instance method as `pipeline.run(conn, request)`. Do not call a static `IngestPipeline::run(...)`.
- `WorkspaceSourceRegistry::open_validated(...)` is the method/associated call shape. Do not call a nonexistent `registry::open_validated(...)` free function.
- `file_id_from_identity` uses the §0 V1.2 §2.4 sha256 helper. No `blake3` formula or lane-local hash helper is allowed.
- No call site inserts into `intelligence_claims`.
- No call site calls `services::claims::commit_claim` directly.
- No call site opens a second path handle after `WorkspaceSourceRegistry::open_validated` returns `FileIdentity`.
- Existing transcript frontend events remain after any pipeline-adjacent work.
- Existing enrichment queue wake paths remain after watcher refactor.

Out-of-scope-for-W2-B writes:

- `google_drive/poller.rs:199` and `google_drive/poller.rs:208` are OUT-OF-SCOPE-v1.4.6 per cycle-13 §13.3.2. Keep them in v1.4.5 with `dos7-allowed: drive-staging-v146` comments and file the v1.4.6 Drive content staging API ticket.

```rust
// google_drive/poller.rs OUT-OF-SCOPE-v1.4.6 per cycle-13 §13.3.2.
// dos7-allowed: drive-staging-v146 - remote Drive bytes need v1.4.6 staging API
std::fs::create_dir_all(&docs_dir)?;

// dos7-allowed: drive-staging-v146 - remote Drive bytes need v1.4.6 staging API
std::fs::write(&file_path, content)?;
```

- `watcher.rs:667`, `watcher.rs:722`, and `watcher.rs:763` account/project/person upserts stay as-is. They are entity-table writes, not workspace-file writes.
- `watcher.rs:674`, `watcher.rs:727`, and `watcher.rs:768` entity markdown regenerators stay as-is with `dos7-allowed: entity-markdown-regen` rationale comments. They render canonical entity DB state into local markdown summaries; they are not workspace-file ingestion.
- `watcher.rs:814` and `watcher.rs:881` content-index helpers stay as-is with content-index-cache allowlist rationale. They update cache/enrichment surfaces; they are not ingestion.
- `granola/poller.rs:322`, `337`, `350`, `363`, `438`, `463`, `483` stay as-is. They are transcript metadata/capture/action/sync-state writes, not workspace-file writes.
- `quill/poller.rs:399`, `415`, `428`, `441`, `493` stay as-is. They are transcript metadata/capture/action writes, not workspace-file writes.

## §9 Tests Required

Regression tests prove that W2-B adds no new direct workspace-file writes, preserves existing event payloads, and builds §0 V1.2 §2.1 requests at every refactored ingestion call site. Cycle-4 F3 keeps the V1.3 watcher fixture additions: pipeline rejection after DB upsert, duplicate-event idempotency, and real workspace-edit lifecycle proof. There are no `*_if_supported` tests and no `*_or_documents_staging_gap` tests.

Required tests:

- `tests/workspace_ingestion_w2b_no_new_direct_writes.rs` - hard lint test that asserts no new `std::fs::write`, `std::fs::create_dir_all`, or `tokio::fs::write` appears in W2-B-touched files beyond the explicit `dos7-allowed` allowlist entries for `google_drive/poller.rs:199`, `google_drive/poller.rs:208`, and `watcher.rs:81`, plus the explicitly documented watcher helper-call allowlist entries for entity markdown regeneration and content-index caches.
- `watcher_account_change_preserves_account_upsert_and_builds_ingest_request`
- `watcher_project_change_preserves_project_upsert_and_builds_ingest_request`
- `watcher_people_change_preserves_person_table_write_and_builds_ingest_request`
- `tests/watcher_fixture_pipeline_rejection_after_db_upsert.rs` - verifies that if the pipeline rejects a file after the entity DB upsert succeeded, the entity row is NOT rolled back; writes are independent.
- `tests/watcher_fixture_duplicate_event_idempotency.rs` - verifies that the same file dropped twice produces one ingestion run.
- `tests/watcher_fixture_real_workspace_edit.rs` - verifies drop file -> lifecycle row -> ingestion run -> `app_handle.emit` fired -> markdown preserved.
- `watcher_account_content_change_preserves_content_index_enrichment_trigger`
- `watcher_project_content_change_preserves_content_index_enrichment_trigger`
- `watcher_user_attachment_change_preserves_embedding_queue_wake`
- `watcher_inbox_bootstrap_is_allowlisted_directory_bootstrap_not_file_mutation`
- `watcher_inbox_updated_emit_payload_snapshot_preserves_count_shape` - concrete snapshot for `app_handle.emit("inbox-updated", ...)`.
- `drive_save_to_entity_docs_allowlist_comments_match_cycle13_v146_descope`
- `granola_process_document_preserves_transcript_db_writes_as_out_of_scope`
- `granola_transcript_processed_emit_payload_snapshot_preserves_outcome_shape`
- `granola_transcript_processed_emit_payload_snapshot_preserves_string_fallback_shape`
- `quill_process_sync_row_preserves_transcript_db_writes_as_out_of_scope`
- `quill_transcript_processed_emit_payload_snapshot_preserves_outcome_shape`
- `quill_transcript_processed_emit_payload_snapshot_preserves_string_fallback_shape`
- `workspace_mutation_allowlist_lint_remains_green_after_w2_b_refactor`
- `workspace_mutation_allowlist_allows_only_drive_v146_inbox_bootstrap_entity_markdown_regen_and_content_index_comments`
- `workspace_mutation_allowlist_catches_direct_entity_doc_fs_write_outside_service`
- `ingest_request_source_asof_is_derived_from_validated_identity_metadata`
- `ingest_request_construction_does_not_reopen_path_after_open_validated`
- `ingest_request_compile_shape_uses_canonical_w2a_fields_and_test_pipeline` - compile-shape test that builds an `IngestRequest` with exactly §0 V1.2 §2.1 fields and runs it against the test pipeline through `run(&Connection, IngestRequest)`.
- `duplicate_watcher_events_are_idempotent_against_w2_a_pipeline_receipt`

Concrete event-preservation requirement:

- Snapshot `app_handle.emit("inbox-updated", InboxUpdate { count })` payload shape.
- Snapshot `app_handle.emit("transcript-processed", &outcome)` payload shape for both Granola and Quill.
- Snapshot `app_handle.emit("transcript-processed", &meeting_id.to_string())` fallback payload shape for both Granola and Quill.
- Verify no removal of any existing emission at `watcher.rs:95`, `granola:579/586`, or `quill:636/643`.

Compile-shape requirement:

- Each refactored call site constructs `IngestRequest { file, identity, file_id, source_asof, source_type, entity, mode, category_hint }` from §0 V1.2 §2.1.
- Each refactored call site derives `file_id` through the §0 V1.2 §2.4 sha256-based helper.
- Each refactored call site calls `WorkspaceSourceRegistry::open_validated(...)` before request construction and never re-opens the path after validation.
- Each refactored call site runs against the test pipeline through `let pipeline = wiring::build_pipeline()?; let conn = db.conn_ref(); pipeline.run(conn, request)`.

Standard gates:

- `cargo test`
- `cargo clippy -- -D warnings`
- W2-A-owned `src-tauri/scripts/check_workspace_mutation_allowlist.sh` remains green.
- Existing `src-tauri/scripts/check_claim_writer_allowlist.sh` remains green.

## §10 Done When

All newly introduced or refactored workspace-file ingestion paths in W2-B-owned functions route through W2-A's `IngestPipeline::run(&Connection, IngestRequest)` with the request shape from §0 V1.2 §2.1. The v1.4.5 exceptions are the explicit direct file-write and helper-call allowlist entries in §11.

Specific done-when clauses:

- `watcher.rs:81` has the `dos7-allowed: inbox-bootstrap` rationale comment and no lifecycle row is created for the bootstrap directory.
- `google_drive/poller.rs:199` and `google_drive/poller.rs:208` have `dos7-allowed: drive-staging-v146` rationale comments and remain unchanged behaviorally in v1.4.5 per cycle-13 §13.3.2.
- `watcher.rs:674`, `watcher.rs:727`, and `watcher.rs:768` have `dos7-allowed: entity-markdown-regen` rationale comments and keep regenerating markdown summaries from entity DB state.
- `watcher.rs:814` and `watcher.rs:881` content-index helper calls have allowlist rationale comments and keep cache/enrichment behavior.
- Account/project/person/transcript DB writes listed in §4 and §8 remain in place and are not treated as workspace-file mutation violations.
- CI invariant remains green with only the explicit `drive-staging-v146`, `inbox-bootstrap`, `entity-markdown-regen`, and content-index-cache allowlist entries.
- No existing `app_handle.emit` call is removed.
- `cargo test` and `cargo clippy -- -D warnings` are clean.

## §11 CI Invariants

This lane consumes W2-A's CI invariant; it does not create the script.

- `src-tauri/scripts/check_workspace_mutation_allowlist.sh` is created by W2-A per §0 shared contract §5.
- Cycle-13 §13.3.2 de-scopes the two Drive writes to v1.4.6. W2-B activates the allowlist for all other workspace-file-mutation paths.
- W2-B's CI obligation is that the script remains green after W2-B's refactor and fails on any new direct workspace-file write outside the service boundary.
- The only v1.4.5 direct `std::fs::*` allowlist entries are `src-tauri/src/google_drive/poller.rs:199`, `src-tauri/src/google_drive/poller.rs:208`, and `src-tauri/src/watcher.rs:81`.
- `google_drive/poller.rs:199` and `google_drive/poller.rs:208` require the `dos7-allowed: drive-staging-v146` rationale because Drive content staging is assigned to v1.4.6 by cycle-13 §13.3.2.
- `watcher.rs:81` requires the `dos7-allowed: inbox-bootstrap` rationale: directory bootstrap, not file mutation; no lifecycle row created.
- `watcher.rs:674`, `watcher.rs:727`, and `watcher.rs:768` require the `dos7-allowed: entity-markdown-regen` rationale: entity-table data to local markdown summary, not workspace-file ingestion.
- `watcher.rs:814` and `watcher.rs:881` require content-index-cache rationale: cache update/enrichment surface, not ingestion.
- Account/project/person/transcript entity-table writes listed in §4 are outside this W2-B allowlist scope and are not reasons for this script to fail.

W2-A-owned invariant tests that W2-B must keep green:

- `src-tauri/tests/workspace_mutation_allowlist_test.rs::lint_workspace_mutation_allowlist_passes_against_current_tree`
- `src-tauri/tests/workspace_mutation_allowlist_test.rs::lint_workspace_mutation_allowlist_catches_direct_db_upsert_outside_service`
- `src-tauri/tests/workspace_mutation_allowlist_test.rs::lint_workspace_mutation_allowlist_catches_direct_entity_doc_fs_write_outside_service`

Existing invariant preserved:

- `src-tauri/scripts/check_claim_writer_allowlist.sh` - existing claim writer lint. It remains the claim-table guard and must stay green; W2-B does not edit `services/claims.rs` or widen the claim writer allowlist.

Explicit non-activation in this lane:

- `src-tauri/scripts/check_workspace_signal_allowlist.sh` is W3-B's signal lane, not W2-B. W2-B only preserves existing event emissions and leaves W3-B's signal implementation boundary intact.

## §12 Reviewer Panel

- codex challenge
- codex consult
- architect-reviewer

**Pass rule:** unanimous APPROVE.

## §13 PATH-α Appendix

- Drive content staging API (cycle-13 §13.3.2): file a v1.4.6 ticket for `workspace_ingestion::staging` that bridges remote Drive bytes to a validated `File`; W2-B must not invent the helper locally.
- `mtime` in `FileIdentity` (cycle-2 codex path-α): consider carrying validated mtime from `open_validated` so callers do not need a second metadata read for §0 V1.2 §2.1 `source_asof`.
- AST-aware mutation lint (cycle-2/cycle-3 codex path-α): replace regex allowlist enforcement with Rust syntax parsing so entity-table writes, helper calls, file writes, comments, and imports are distinguished structurally.
- Event payload shape normalization: `inbox-updated` emits `{ count }` while some transcript fallback paths emit a string. Preserve current payloads in W2-B; normalize in a later signal/event compatibility lane.
