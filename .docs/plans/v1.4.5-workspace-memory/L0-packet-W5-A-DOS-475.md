# L0 Packet - v1.4.5 W5-A - DOS-475 Workspace Source Backfill

**Issue:** DOS-475
**Branch/worktree:** `codex/v1.4.5-w5-backfill-validation`
**Base:** stacked on W4 PR #385 (`1e07a8f3`)
**Revision:** V0.3 after L0 cycle 3 pass
**Status:** L0 passed. No substrate implementation code edits have started in this worktree.
**Prepared:** 2026-05-25

## 0. Executive Verdict

W5-A implements conservative workspace-source registration, classification, and divergence reporting. It does not run the ingestion pipeline or create claims by default.

Linear DOS-475 is narrower than the stale W5 wave text: register existing files as sources, keep historical material conservative, and report source-registry/content-index/embedding/filesystem divergence. This packet supersedes the older claim-producing W5-A wording. Claim-producing E2E validation remains W5-B, but it must use already-reviewed ingestion paths such as entity intake, inbox ingestion, or placement fixtures, not historical backfill registration as an implicit claim producer.

Cycle 1 amendments now pinned:

- W5-A uses the next contiguous migration on this branch: registered v264, filename `264_workspace_backfill_state.sql`.
- Backfill-created source rows stay in `pending` or `pending_entity_assignment`; they are not `ingested` and therefore are excluded from `workspace_graph`/MCP and default runtime contexts until user promotion or separately reviewed extraction.
- Lifecycle-only registration emits no signal. New Backfill links emit only `WorkspaceFileEntityLinkChanged` through workspace-ingestion service APIs.
- No raw-path debug flag ships in W5-A.
- `content_index` and `content_embeddings` divergence is report-only.
- No bulk rollback command ships in W5-A. Backfill rows are identifiable and repairable via existing source-management actions; automated source tombstone/rollback is a follow-up if needed.
- Shared artifacts never include raw/unkeyed content hashes or hash prefixes.
- Opaque handles use existing reviewed local diagnostic/handle key material with domain-separated HMAC inputs; no hardcoded/global key.
- User promotion is not freshness attestation. Weak filesystem time remains weak until a reviewed path supplies confirmed `source_asof` or the user explicitly confirms freshness/time.

## 1. Active PR Coordination

| PR / branch | State | W5-A consequence |
| --- | --- | --- |
| #385 `codex/v1.4.5-w4-complete` | Open against `dev`; W5 is stacked on commit `1e07a8f3` | W5-A can build on W4 source-management surfaces locally, but final W5 PR should rebase after #385 lands. |
| account/Glean producer work | PR #366 merged | Do not touch `services/claims.rs`, `services/account_fact_claims.rs`, trust recompute, claim lifecycle helpers, or Glean/account producer code in W5-A. |
| W5-B DOS-476 | Blocked by DOS-475 | Validation starts only after backfill registration is implemented and merged. |

## 2. K-In Evidence

| Source | Constraint |
| --- | --- |
| Linear DOS-475 | Registers/classifies existing files conservatively; does not mass-create high-trust claims from historical content. |
| ADR-0101 | All domain mutations go through `services/`; bins and commands call service functions. |
| ADR-0103 | Maintenance/backfill work must be idempotent, dry-runnable, budgeted, and transactionally bounded. |
| ADR-0104 | Dry-run paths must be structurally write-free. |
| ADR-0105 | `source_asof` is first-class; unknown or weak timestamps get conservative treatment. |
| ADR-0107 | Workspace files use `DataSource::WorkspaceFile { kind }`; do not invent a new source taxonomy. |
| ADR-0108 / ADR-0120 | No raw paths, raw content, claim text, prompt/output bodies, or provenance blobs in surfaced receipts/logs. |
| ADR-0126 | Claim mutation stays in `services/claims.rs`; W5-A must not bypass `commit_claim` or add one-off claim scoring. |
| `docs/solutions/architecture-patterns/claim-producers-require-runtime-wide-trust-audit-2026-05-22.md` | A later claim-producing lane needs a runtime-wide trust/provenance audit. W5-A avoids that lane. |
| `docs/solutions/test-failures/repeated-full-migration-test-fixtures-2026-05-24.md` | Tests needing current schema should use `migrated_in_memory_for_tests` to avoid slow replay fixtures. |

## 3. Scope

### In Scope

- New backfill service/facade callable by a local CLI.
- New CLI `workspace_backfill` with dry-run default and explicit `--apply`.
- Migration v264 for resumable backfill run/item/operation state.
- Workspace scan over eligible files only.
- Source registration into `workspace_file_lifecycle` as `DataSource::WorkspaceFile { kind }`.
- Pending-review exposure metadata for every backfill-created source.
- Entity/category inference from trusted workspace path structure and existing active entity rows.
- Backfill-attributed document/entity links for confidently inferred account/person/project paths.
- Duplicate-content reporting by aggregate count plus opaque duplicate-group handles, not raw path, filename, raw content hash, or hash prefix.
- Report-only divergence between filesystem, `workspace_file_lifecycle`, `content_index`, and `content_embeddings`.
- Privacy-safe `BackfillSummary` with counts, reason codes, source classes, opaque duplicate-group handles, and stable opaque source handles.

### Out of Scope

- Running `IngestPipeline` by default.
- Direct calls to `commit_claim`.
- Any changes to `services/claims.rs`, account/Glean claim producers, trust recompute, source-purge helpers, or claim lifecycle helpers.
- Claim retraction or source-wide claim lifecycle mutation.
- Rewriting, moving, deleting, or normalizing user workspace files.
- Derived-cache repair, tombstoning, or deletion for `content_index` or embeddings.
- MCP gateway, handler, auth, bridge, transport, or tools/list edits.
- Lower-level signal bus semantics.
- Bulk rollback or source tombstone automation.

## 4. Planned Write Set Before Substrate Edits

Implementation may edit:

- `src-tauri/src/migrations/264_workspace_backfill_state.sql`
- `src-tauri/src/migrations.rs`
- `src-tauri/src/services/workspace_backfill.rs`
- `src-tauri/src/services/mod.rs`
- `src-tauri/src/services/workspace_ingestion/link.rs` only if needed for an `add_backfill_link_with_signal` service helper that returns inserted-vs-existing and emits `WorkspaceFileEntityLinkChanged` for newly inserted links
- `src-tauri/src/bin/workspace_backfill.rs`
- `src-tauri/Cargo.toml`
- focused Rust tests
- mock/dev seed updates only if the schema pre-commit gate requires them

The CLI binary does not call `ActionDb::open`; it calls a service entrypoint. Direct lifecycle/run/link SQL, if needed, stays under `services/workspace_ingestion/` service owners so the workspace mutation allowlist remains true.

## 4.1 Opaque Handle Key Management

W5-A must use existing reviewed local handle key material rather than inventing a secret.

- Prefer the workspace graph diagnostic key path (`local_install_diagnostic_key` / DB-backed local diagnostic key bytes) or an equivalent existing per-install encrypted local secret.
- Domain-separate every HMAC input:
  - `dailyos.workspace_backfill.workspace_root_fingerprint.v1`
  - `dailyos.workspace_backfill.source_handle.v1`
  - `dailyos.workspace_backfill.item_handle.v1`
  - `dailyos.workspace_backfill.duplicate_group.v1`
  - `dailyos.workspace_backfill.link_handle.v1`
- Handles must be stable across reruns on the same install/workspace and non-correlatable across installs/workspaces.
- Handle inputs may include internal file identity/content identity in local code, but emitted handles must never be raw IDs, unkeyed path digests, unkeyed content digests, or reversible encodings.
- The key is never hardcoded, committed, logged, printed, serialized in run-state JSON, or copied into Linear/PR/proof artifacts.
- Rotation/recovery behavior: if the local handle key is unavailable, dry-run/apply fail closed with `handle_key_unavailable`. A future key rotation may require regenerating opaque handles; W5-A does not implement rotation.
- Tests must prove handle stability across reruns, non-equality across distinct test keys/workspaces, and absence of raw file IDs/path/content digests in emitted summaries.

## 5. State Schema

Migration v264 adds three tables.

`workspace_backfill_runs`:

- `run_id` opaque UUID
- `mode`: `dry_run` or `apply`
- `status`: `running`, `completed`, `failed`, `aborted`
- `workspace_root_fingerprint`: domain-separated HMAC handle, not a raw path or unkeyed path digest
- `actor`: `system:workspace_backfill:v1`
- reason-code counts JSON only
- timestamps

`workspace_backfill_items`:

- `run_id`
- `source_handle`: stable opaque handle derived from canonical file identity/file_id with the local handle key; not run-namespaced
- `item_handle`: run-local opaque handle for cursor/debug correlation
- `file_id`: operational internal column allowed in the encrypted local DB, never emitted in CLI/log/Linear/PR summaries
- `content_sha256`: full internal content identity used only for idempotency/revalidation inside the encrypted local DB
- `duplicate_group_handle`: HMAC-derived opaque handle for duplicate reporting
- `candidate_kind`, `entity_type`, `entity_id`, `category`
- `exposure_state`: `pending_review`, `promoted`, `ignored`
- `source_time_basis`: `filesystem_mtime`
- `source_time_confidence`: `filesystem_unverified`
- `backfill_observed_at`
- `status`: `planned`, `applied`, `skipped`, `failed`
- `reason_code`
- timestamps

`workspace_backfill_operations`:

- `run_id`, `source_handle`, `operation_kind`
- `status`: `planned`, `applied`, `skipped`, `failed`
- `created_lifecycle`: boolean
- `updated_lifecycle_fields`: JSON list of field names only
- `created_link_handle`: stable opaque handle, nullable
- `reason_code`
- timestamps

Operation rows are an audit/resume ledger, not a rollback script. They store enough shape to prove idempotency and identify backfill-owned records without leaking paths/content. They do not store raw paths, filenames, entity names, file content, raw/unkeyed hash prefixes, claim text, prompt/output bodies, or surfaced raw file IDs.

## 6. Eligibility Policy

The scanner only considers regular files under the canonical workspace root after `WorkspaceSourceRegistry::open_validated` succeeds.

Skip by default:

- any hidden dot path or dotfile;
- `.claude`;
- `_today`, `_archive`, and other underscore-managed roots except `_inbox`;
- DailyOS generated exports such as `dashboard.json`, `dashboard.md`, `intelligence.json`;
- root `CLAUDE.md`;
- `.DS_Store`;
- structural `Internal` paths;
- unsupported extensions or binary/non-UTF-8 files;
- symlink, hardlink, cross-device, traversal, or race failures surfaced by the registry;
- deleted/missing files discovered from stale DB rows.

`_inbox` is eligible only for ordinary user-authored source files under `_inbox/`; managed/system residue inside `_inbox` still skips by the same hidden/generated/unsupported policy.

## 7. Entity And Category Inference

Path structure is a hint bounded by the DB, not by user-authored content.

Algorithm:

1. Load active account/person/project rows and their `tracker_path` values.
2. Canonicalize tracker paths relative to the workspace root.
3. Match each candidate by longest active tracker-path prefix.
4. If no tracker-path match exists, optionally fall back to exact canonical conventional roots (`Accounts/<name>`, `People/<name>`, `Projects/<name>`) only when there is exactly one active entity match.
5. After the entity prefix, treat the next segment as a category only if `WorkspaceCategory::from_slug` succeeds and `WorkspaceCategoryRegistry::validate` allows it for the entity type.
6. Unknown category-shaped segments are not guessed. The file remains entity-root source material with `category = None` or is `report_only` if ambiguity remains.
7. Archived, missing, duplicate, or ambiguous entity matches produce `report_only` / `pending_entity_assignment`; backfill does not create entities.
8. `_inbox` files use `WorkspaceFileKind::Inbox` and do not get new entity links from backfill unless an existing active link is already present.

YAML/frontmatter, file content, AI-generated markers, and filenames are untrusted hints and do not set actor, provenance, lifecycle, sensitivity, source time, or trust.

## 8. Mutation Flow

### Dry-Run

1. Build candidates and validate each candidate through `WorkspaceSourceRegistry::open_validated`.
2. Hash validated content only after the registry returns a safe file handle.
3. Classify source kind/category/entity from trusted path structure.
4. Compare planned state to lifecycle/link/content-index/embedding state.
5. Emit a non-mutating `BackfillSummary`.

Dry-run writes no rows, emits no signals, and does not update derived caches.

### Apply

1. Create or resume a `workspace_backfill_runs` row.
2. For each candidate in stable source-handle order, revalidate file identity and content hash before mutation.
3. Register missing lifecycle rows through `LifecycleRepo::insert_pending`.
4. Keep registered rows in `pending` when entity is inferred and `pending_entity_assignment` when entity is not resolved. Do not mark backfill-created rows `ingested`.
5. Record category/content hash through lifecycle service APIs when known.
6. Add Backfill-attributed links only for high-confidence entity matches and only when tombstone guards allow.
7. Emit `WorkspaceFileEntityLinkChanged` only when a new Backfill link is inserted. Lifecycle-only registration emits no signal. Do not emit `WorkspaceFileIngested` because W5-A does not create an ingestion run or call the pipeline.
8. Mark item/operation rows applied/skipped/failed so reruns are idempotent.

## 9. Visibility And Exposure

Backfill-created sources are pending review by default.

- `workspace_file_lifecycle.lifecycle_state` is never `ingested` from W5-A apply.
- `workspace_backfill_items.exposure_state = pending_review` for every apply-created source.
- `workspace_graph` already excludes non-`ingested` lifecycle rows; W5-A adds regression tests so pending-review backfill rows are not visible through MCP/workspace graph projections by default.
- Default runtime contexts do not include W5-A backfilled sources because W5-A creates no claims and no ingested graph rows.
- Source-management/local operator views may show pending sources through privacy-safe handles so the user can promote, relink, quarantine, or ignore them.

Promotion into ingestion/claims is a separate user action or separately reviewed extraction policy.

## 10. Source Time And Trust

The active freshness table gives `DataSource::WorkspaceFile { .. }` a 21-day half-life, but W5-A creates no claims and assigns no claim trust band.

For lifecycle source time:

- Use filesystem modified time only after `open_validated` succeeds.
- Store `source_time_basis = filesystem_mtime` and `source_time_confidence = filesystem_unverified` for every backfill row.
- Store `backfill_observed_at` separately.
- If mtime is missing, unparsable, earlier than the supported platform epoch, or more than five minutes in the future relative to `backfill_observed_at`, skip/report-only with `source_time_untrusted`.
- If mtime is future within tolerance, clamp to `backfill_observed_at`.
- Never use `now` as a silent fallback for unknown source time.

A fresh mtime on a historical file is not enough to make future claims `likely_current`. User promotion of a source is not freshness attestation. A `filesystem_unverified` source time remains weak until a reviewed ingestion path supplies confirmed `source_asof` or the user explicitly confirms freshness/time as a separate action. W5-B must include validation cases proving pending-review, promoted-without-freshness-attestation, and weak-time backfilled sources cannot become `likely_current` without that confirmation.

## 11. Divergence Reporting

W5-A reports, but does not repair, these divergence classes:

- filesystem candidate without lifecycle row;
- lifecycle row whose canonical file is missing;
- active document/entity link without lifecycle row;
- `content_index` row without lifecycle row;
- embedding row without source/lifecycle row;
- duplicate content hash across multiple source handles.

Apply mode does not tombstone or delete `content_index`, `content_embeddings`, claims, links, or lifecycle rows that W5-A did not create. Any derived-cache repair is a named follow-up with its own L0.

## 12. Intelligence Loop Fit

1. **Claim model:** W5-A creates no claims by default. It prepares registered workspace sources that a later extraction lane can process through `commit_claim`.
2. **Provenance + trust:** Source provenance is `DataSource::WorkspaceFile { kind }`; source time is filesystem-derived but explicitly weak/unverified; claim trust bands remain claim-layer concerns.
3. **Signals + invalidation:** Lifecycle-only registration emits no signal; new Backfill links emit `WorkspaceFileEntityLinkChanged` through workspace-ingestion service APIs.
4. **Runtime + surfaces:** Pending-review backfill rows stay out of MCP/workspace graph and default runtime contexts. Source-management/local operator views may list them with privacy-safe handles.
5. **Feedback loop:** W4 source-management actions can promote, relink, quarantine, or ignore registered sources. Existing tombstones/user overrides are respected; backfill never resurrects rejected links.

## 13. Tests Required

- Dry-run discovers eligible fixture files and produces expected counts without DB writes.
- CLI default is dry-run; `--apply` is required to mutate.
- CLI stdout/stderr/log summaries never contain raw absolute paths, relative paths, filenames, entity names parsed from paths, raw file content, raw file IDs, claim text, prompts, or outputs.
- Apply registers account/person/project workspace files without rewriting user files.
- Apply-created lifecycle rows are pending/pending-entity-assignment, never ingested.
- Pending-review backfill rows do not appear in MCP/workspace graph projections or default runtime contexts.
- Idempotency: running apply twice yields stable rows/counts, excluding run-local IDs and timestamps.
- Resumability: interrupted run resumes from item/operation state and completes without duplicating lifecycle/link rows.
- Duplicate content is reported by aggregate count plus opaque duplicate-group handle only.
- Managed/hidden/generated files skip with reason codes.
- Symlink/traversal/hardlink/cross-device registry failures skip safely.
- Existing rejected/tombstoned links are not resurrected.
- Backfill link insertion emits `WorkspaceFileEntityLinkChanged`; lifecycle-only registration emits no signal.
- Divergence report covers filesystem-only, lifecycle-only, content-index-only, and embedding-without-source cases and remains report-only.
- Weak/future/missing source time cases use reason codes and never fallback silently to now.
- Promotion without explicit freshness/time confirmation does not turn a weak filesystem timestamp into a `likely_current` claim input.
- Emitted summaries and proof artifacts do not contain `content_sha256`, `content_sha256_prefix`, or raw/unkeyed content hash material.
- Opaque handle tests cover stable reruns, key/workspace separation, and fail-closed behavior when key material is unavailable.
- Bin does not call `ActionDb::open`; it calls a service function.

## 14. Done When

- L0 passes after this scope correction.
- `workspace_backfill --dry-run` runs against a fixture workspace and returns a privacy-safe `BackfillSummary`.
- `workspace_backfill --apply` registers fixture sources idempotently and resumably.
- Backfill never rewrites user files.
- No historical file is mass-promoted into trusted claims.
- Required checks pass: `cargo clippy -- -D warnings`, `cargo test`, `pnpm tsc --noEmit`.
