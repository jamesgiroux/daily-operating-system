# L0 Packet — v1.4.5 W1-C — DOS-465 Document/Entity Links + Ingestion Run Tracking

**Current revision:** V1.3 (cycle 3 fold, 2026-05-20). See §2 Changelog.

## 1. Header

- **Date:** 2026-05-20
- **Project:** v1.4.5 — Workspace Memory Refactor ([Linear](https://linear.app/a8c/project/v145-workspace-memory-refactor-cdb9d2c17102))
- **Wave:** W1 stage 1b (gates on W1-A merge; runs parallel with W1-B)
- **Issue:** [DOS-465 — Add document/entity links and ingestion run tracking](https://linear.app/a8c/issue/DOS-465)
- **Branch:** lands on `wave/v1.4.5-w1-stage1a` directly (V1.3 fold #5: wave-PR merge model prevents the v253/v254-before-v252 migration-skip race). No separate PR vs `dev`; PR #345 is the single atomic wave merge.
- **Migration slots claimed:** **v253, v254** (from v1.4.5 W1 block v250–v254 per wave-plan §Cycle 11)
- **L0 reviewer matrix:** architect-reviewer + codex challenge + codex consult. No `/cso`.
- **L2 reviewer matrix:** codex review + code-reviewer + architect-reviewer.

## 2. Changelog

- **V1.3 (2026-05-20 — cycle 3 fold):** Cycle 3 returned architect APPROVE + codex challenge BLOCK (3 substantive + UPSERT validated correct) + codex consult pending. Codex's findings are real — the tombstone guard is raceable as specified (pre-SELECT then INSERT not atomic), the user-relink API path is unclear, and in-progress idempotency isn't guarded. Folds:
  1. **Tombstone guard wrapped in `BEGIN IMMEDIATE` transaction** (challenge #1): V1.2 specified pre-SELECT then INSERT but didn't pin transactional boundary. A concurrent `reject_link` between SELECT and INSERT could let a classifier source insert an active row after tombstone. V1.3 explicitly requires the `add_link` flow for `attribution_source ∈ {Classifier, Backfill, DriveMetadata}` to wrap the SELECT-rejected + INSERT in `BEGIN IMMEDIATE` transaction:
     ```rust
     conn.execute("BEGIN IMMEDIATE", [])?;
     let tombstoned: Option<(DateTime<Utc>, String)> = conn.query_row(
         "SELECT rejected_at, rejected_reason FROM document_entity_links \
          WHERE file_id = ?1 AND entity_type = ?2 AND entity_id = ?3 AND rejected = 1",
         params![file_id, entity_type, entity_id], |r| Ok((r.get(0)?, r.get(1)?)),
     ).optional()?;
     if let Some((rejected_at, rejected_reason)) = tombstoned {
         conn.execute("ROLLBACK", [])?;
         return Err(LinkError::Tombstoned { rejected_at, rejected_reason });
     }
     let link_id = conn.query_row(
         "INSERT INTO document_entity_links (...) VALUES (...) \
          ON CONFLICT (file_id, entity_type, entity_id) WHERE rejected = 0 \
          DO NOTHING RETURNING link_id",
         params![...], |r| r.get(0),
     ).optional()?.unwrap_or_else(|| /* SELECT fallback for DO NOTHING no-row */ ...);
     conn.execute("COMMIT", [])?;
     ```
     `BEGIN IMMEDIATE` acquires a write lock immediately; no concurrent reject_link can interleave. EntityIntake/UserRelink/MCP/Frontmatter sources skip the tombstone-check step but still wrap their INSERT in a transaction for atomicity.
  2. **User-relink API path clarified** (challenge #2): V1.2 §10 said W4-A calls `override_link` for re-link but V1.2 §7 tested `add_link(... UserRelink)` for resurrection. The two are different operations:
     - **`add_link(file_id, entity_type, entity_id, UserRelink, ...)`**: creates a NEW link or resurrects a previously-rejected link. Used when W4-A's UI lets the user explicitly establish/re-establish a link (e.g., "this file IS about Acme" after a classifier mis-binding was rejected).
     - **`override_link(emitter, file_id, entity_type, entity_id, actor)`**: marks an existing-AND-active link as user-confirmed (sets `user_override_actor` + `user_override_at`). Does NOT resurrect rejected links; returns `LinkError::NotFound` if no active row exists. Used when the user explicitly endorses an existing classifier-attributed link without changing the link itself.
     §10 handoff updated: W4-A's re-link UI calls `add_link(... UserRelink)`; W4-A's "endorse" UI calls `override_link`. `reject_link` is the inverse of both.
  3. **In-progress idempotency guard** (challenge #3): V1.2 UNIQUE was `WHERE status='success'` only — duplicate in-progress runs for the same `(file_id, content_sha256, mode)` could execute. V1.3 service-layer `start_run` guard:
     - Pre-check via `find_by_idempotency_key` for **any** status (in_progress OR success).
     - If `status = 'success'` exists → return `RunsError::AlreadyCompleted { existing: ExistingRunReceipt }` (caller can choose to use the existing receipt).
     - If `status = 'in_progress'` exists AND `started_at` is within last 1 hour → return `RunsError::AlreadyInProgress { existing_run_id }` (the prior attempt is still live; caller can wait or poll).
     - If `status = 'in_progress'` exists but `started_at` is >1 hour old → mark prior as `aborted` (likely crashed), proceed with new run.
     - If no row exists → insert new run.
     Schema unchanged (UNIQUE WHERE status='success' is the storage-level fence; service-layer guard prevents duplicate in_progress).
  4. **`RunsError` extended**: adds `AlreadyInProgress { existing_run_id: IngestionRunId }` variant (V1.3 fold #3).
  5. **Migration ordering safeguard** (consult #1): if W1-C ships v253/v254 before W1-B v252 lands, the `version > current` runner permanently skips v252. V1.3 mitigation: this wave uses the **wave-PR merge model** (PR #345 = `wave/v1.4.5-w1-stage1a` → `dev`). W1-B + W1-C land on the wave branch via local merges in dependency order (W1-A → W1-B → W1-C → wave PR). The wave PR is a single atomic merge to dev. W1-C must NOT open a separate PR vs dev. Documented in §10 handoff + §1 branch.
  6. **`complete_run` idempotency convergence** (consult #2): wave AC requires "one record for same file hash". V1.3 service-layer `complete_run` flow:
     - For `mode != Forced`: if `find_by_idempotency_key(file_id, content_sha256, mode)` already returns `Some(receipt)` with `status = success`, return `Err(RunsError::AlreadyCompleted { existing: receipt })` BEFORE attempting the UNIQUE-violating UPDATE.
     - For `mode = Forced`: `retry_of_run_id` is required; the partial UNIQUE on `WHERE status = 'success'` will reject a duplicate success row at the SQL layer (caught + returned as `Err(RunsError::AlreadyCompleted)`).
     - Test `idempotency_unique_constraint_enforces_no_duplicate_successful_run` updated to assert: second `complete_run` returns `Err(RunsError::AlreadyCompleted)` (NOT generic `DbError`); total `status='success'` rows = 1; existing receipt is returned in the error envelope.
  7. **Downstream signature sweep** (consult #3): wave-plan + W4 handoff text still cites old `override_link/reject_link(file_id, entity_id, actor)` (without entity_type). V1.3 §10 handoff explicitly updates all callers to pass the triple: W4-A re-link UI → `add_link(file_id, entity_type, entity_id, UserRelink, ...)`; W4-A endorse UI → `override_link(emitter, file_id, entity_type, entity_id, actor)`; W2-C entity-intake → `add_link(file_id, entity_type, entity_id, EntityIntake, ...)`. The wave-plan text amendment is filed as a Cycle 13 wave-plan amendment to update the §Agent W1-C body and the W4-A/W2-C handoffs.

- **V1.2 (2026-05-20 — cycle 2 fold):** Cycle 2 returned architect CONDITIONAL APPROVE (2 SQL-spec tightening conditions) + codex challenge BLOCK (3 substantive — partial-unique doesn't enforce post-rejection, idempotency weaker than wave, service guard underspecified) + codex consult BLOCK (3 substantive — override_link/reject_link missing entity_type, UPSERT syntax wrong, test gap). Folds:
  1. **`override_link` + `reject_link` take `entity_type`** (consult #1 + architect #1): both APIs now signature `(emitter: &dyn SignalEmitter, file_id: &str, entity_type: crate::entity::EntityType, entity_id: &str, actor: &str, …) -> Result<(), LinkError>`. The unique key is the `(file_id, entity_type, entity_id)` triple, so every link-mutation API must carry all three. Reuses `crate::entity::EntityType` (canonical per W1-B V1.1 fold #2; no reinvention).
  2. **UPSERT syntax corrected** (architect #1 + consult #2): `INSERT INTO document_entity_links (...) VALUES (...) ON CONFLICT (file_id, entity_type, entity_id) WHERE rejected = 0 DO NOTHING RETURNING link_id`. The `WHERE rejected = 0` lives in the **conflict target**, not the action — SQLite UPSERT for partial unique indexes requires matching the partial predicate. §6 SQL block updated with the literal-correct form so the implementer doesn't relitigate.
  3. **Service-layer tombstone guard** (challenge #1 + #2 + consult #2): partial unique on `WHERE rejected = 0` correctly prevents duplicate ACTIVE rows but does NOT prevent a NEW active row after a previous row flips `rejected = 1`. V1.2 adds explicit pre-insert tombstone lookup in `add_link`:
     - For `attribution_source ∈ {Classifier, Backfill, DriveMetadata}`: pre-check `SELECT 1 FROM document_entity_links WHERE (file_id, entity_type, entity_id) = (?, ?, ?) AND rejected = 1` → if exists, return `LinkError::Tombstoned { rejected_at, rejected_reason }` without insert. Satisfies the Linear DOS-465 AC.
     - For `attribution_source ∈ {EntityIntake, UserRelink, McpPlacement, Frontmatter}`: bypass the tombstone check — user/MCP/explicit-attribution sources can intentionally resurrect a previously rejected link. `LinkError` adds `Tombstoned { rejected_at, rejected_reason }` variant.
  4. **Idempotency hardened with UNIQUE constraint** (challenge #3): v253 partial unique index `UNIQUE (file_id, content_sha256, mode) WHERE status = 'success'` replaces the V1.1 non-unique idempotency index. Service-layer `start_run` for `mode != Forced` does a `find_by_idempotency_key` pre-check; for `mode = Forced` (retry), the partial unique on `WHERE status = 'success'` allows the new in-progress row to coexist with the successful prior run.
  5. **Test additions** (challenge + consult test-gap findings): `user_relink_after_tombstone_succeeds` (UserRelink source bypasses the guard); `duplicate_active_classifier_attempt_is_noop` (classifier source returns existing link id without new insert); `idempotency_unique_constraint_enforces_no_duplicate_successful_run` (verifies the v253 UNIQUE actually rejects a second success row).
  6. **`add_link` return on conflict semantics** (architect #2): the implementer must follow `INSERT … ON CONFLICT DO NOTHING RETURNING link_id` with a `SELECT link_id WHERE …` fallback when RETURNING returns zero rows. Documented in §4 implementation notes.

- **V1.1 (2026-05-20 — cycle 1 fold):** Cycle 1 returned architect APPROVE (no findings), codex challenge BLOCK (5 findings), codex consult BLOCK (5 findings; 4 unique after dedup). Codex dissenters were right per memory `feedback_reviewer_dissent_is_signal`. Folds:
  1. **`RunsRepo::start_run` takes a request struct** (challenge #1 + consult #1): signature is `start_run(seed: StartRunSeed) -> Result<IngestionRunId, RunsError>` where `StartRunSeed { file_id: String, mode: IngestionMode, content_sha256: String, file_size_bytes: u64, extractor_version: String, retry_of_run_id: Option<String> }`. All v253 NOT NULL columns are populated by the seed; `retry_of_run_id` carries retry lineage.
  2. **`document_entity_links.user_override` field reinstated** (challenge #2 + consult #2): wave-plan W1-C body (line 565+) requires `user_override` as nullable timestamp + actor pair, matching W1-A's `WorkspaceFileLifecycle.user_override` precedent. V1.1 replaces V1.0's `actor` + `user_confirmed` columns with `user_override_actor: TEXT NULL` + `user_override_at: TEXT NULL` (mirrors W1-A column naming for consistency). `add_link` writes `user_override_*` NULL on initial classifier/intake creates; `override_link` populates both fields. W3-C's graph projection consumes the pair directly.
  3. **Tombstone semantics enforced via service guard + partial unique index** (challenge #3): `idx_del_active_unique UNIQUE (file_id, entity_type, entity_id) WHERE rejected = 0` added to v254. `add_link` is `INSERT … ON CONFLICT DO NOTHING WHERE rejected = 0` (SQL UPSERT semantics) — the partial unique guarantees no duplicate active row for the same `(file_id, entity_type, entity_id)` triple regardless of classifier race conditions. `reject_link` flips `rejected = 1` (which makes the row exit the partial unique scope, so a separate user-relink can later create a fresh active row). New test: `tombstone_prevents_classifier_resurrection` asserts that after `reject_link`, calling `add_link` with the same triple is a no-op (no new active row created).
  4. **Stale wave-plan refs swept** (challenge #4): V1.0 cited W4-C idempotency text as `(entity_id, content_sha256, client_dedup_key.unwrap_or_default())` but the W4-C key is actually a different shape. V1.1 corrects to cite the source-of-truth Linear issue body for the "rejected-link resurrection" AC (it's on DOS-465 issue: "Rejected links are not silently recreated by the next classifier pass"), not the wave plan.
  5. **Test coverage for `complete_run` state persistence + `add_link`/`list_links_for_file` round-trip added** (consult #4): explicit test cases `complete_run_persists_status_count_error_log` and `add_link_list_links_round_trip`.
  6. **`reject_link` scope confirmation** (consult #3): the API is justified by Linear DOS-465 issue body AC; V1.1 cites the issue rather than the wave plan to remove the ambiguity.

- **V1.0 (2026-05-20):** Initial packet. Drafted in parallel with W1-B before cycle 1 review.

## 3. Goal (verbatim from wave plan §Agent W1-C + DOS-465 issue body)

Introduce the `document_ingestion_runs` table — each run of the ingestion pipeline against a file produces a record with start time, file hash, file size, ingestion mode (initial/incremental/forced/backfill), status, claim count produced, and error log. Doc/entity links (`document_entity_links`) table associates workspace files with the entities they're relevant to, carrying the confidence and attribution for the association.

Per L0 question #10 (resolved cycle 2): `document_entity_links` ships as a relational table for v1.4.5, NOT as a `commit_claim` write. Justification: an entity-doc link is metadata about ingestion provenance, not an assertable claim about the world.

Per Linear DOS-465 issue body acceptance criteria: "Rejected links are not silently recreated by the next classifier pass." V1.1 enforces via partial unique index + service-layer guard (fold #3).

## 4. Files owned (exclusive)

### New migrations
- `src-tauri/src/migrations.rs` — slots **v253 + v254** registration.
- `src-tauri/src/migrations/253_document_ingestion_runs.sql` — see §6.
- `src-tauri/src/migrations/254_document_entity_links.sql` — see §6.

### Module content
- `src-tauri/src/services/workspace_ingestion/runs.rs` — fills the W1-A-pre-created placeholder:
  - `IngestionRunId(pub String)` newtype.
  - `IngestionMode { Initial, Incremental, Forced, Backfill }` enum (snake_case serde).
  - `StartRunSeed { file_id: String, mode: IngestionMode, content_sha256: String, file_size_bytes: u64, extractor_version: String, retry_of_run_id: Option<String> }` (V1.1 fold #1).
  - `IngestionRunStatus { InProgress, Success, Failed, Aborted }` enum.
  - `ExistingRunReceipt { run_id: IngestionRunId, completed_at: Option<DateTime<Utc>>, status: IngestionRunStatus, claim_count_produced: u64 }`.
  - `RunsError { NotFound, AlreadyCompleted { existing: ExistingRunReceipt }, AlreadyInProgress { existing_run_id: IngestionRunId }, ContentSha256Mismatch, DbError(String) }` enum + Display + Error. (V1.3 fold #4: `AlreadyInProgress` variant for in-progress idempotency guard; `AlreadyCompleted` now carries the existing receipt envelope so callers can use it directly per V1.3 fold #6.)
  - `RunsRepo::start_run(seed: StartRunSeed) -> Result<IngestionRunId, RunsError>`.
  - `RunsRepo::complete_run(run_id: &IngestionRunId, status: IngestionRunStatus, claim_count: u64, error_log: Option<serde_json::Value>) -> Result<(), RunsError>`.
  - `RunsRepo::find_by_idempotency_key(file_id: &str, content_sha256: &str, mode: IngestionMode) -> Result<Option<ExistingRunReceipt>, RunsError>`.
- `src-tauri/src/services/workspace_ingestion/link.rs` — fills the W1-A-pre-created placeholder:
  - `DocumentEntityLinkId(pub String)` newtype.
  - `LinkAttributionSource { EntityIntake, Frontmatter, Classifier, UserRelink, DriveMetadata, McpPlacement, Backfill }` enum (snake_case serde).
  - `DocumentEntityLink { link_id, file_id, entity_type, entity_id, attribution_source, confidence, rationale, actor, user_override: Option<lifecycle::UserOverride>, rejected, rejected_at, rejected_reason, created_at, updated_at }` (V1.1 fold #2: `user_override` matches W1-A's pattern; reuses `lifecycle::UserOverride` struct).
  - `LinkError { NotFound, DuplicateActive, AlreadyRejected, Tombstoned { rejected_at: DateTime<Utc>, rejected_reason: String }, EntityTypeUnknown, DbError(String) }` enum + Display + Error. (V1.2 fold #3: `Tombstoned` variant for the pre-insert tombstone-guard rejection path.)
  - `LinkRepo::add_link(file_id: &str, entity_type: crate::entity::EntityType, entity_id: &str, attribution_source: LinkAttributionSource, confidence: f64, rationale: Option<&str>, actor: &str) -> Result<DocumentEntityLinkId, LinkError>` — V1.2 fold #3 tombstone guard: classifier/backfill/drive sources pre-check for `rejected = 1` row → `LinkError::Tombstoned` if present. Entity-intake/user-relink/MCP/frontmatter sources bypass. UPSERT via `INSERT … ON CONFLICT (file_id, entity_type, entity_id) WHERE rejected = 0 DO NOTHING RETURNING link_id`; fallback `SELECT link_id WHERE …` when RETURNING returns zero rows.
  - `LinkRepo::list_links_for_file(file_id: &str, include_rejected: bool) -> Result<Vec<DocumentEntityLink>, LinkError>`.
  - `LinkRepo::override_link(emitter: &dyn contracts::SignalEmitter, file_id: &str, entity_type: crate::entity::EntityType, entity_id: &str, actor: &str) -> Result<(), LinkError>` — V1.2 fold #1: `entity_type` now in signature (matches unique key triple). Calls `emitter.emit_link_changed(file_id, entity_id, actor)` after row write; populates `user_override_actor` + `user_override_at`.
  - `LinkRepo::reject_link(file_id: &str, entity_type: crate::entity::EntityType, entity_id: &str, actor: &str, reason: &str) -> Result<(), LinkError>` — V1.2 fold #1: `entity_type` now in signature. Flips `rejected = 1`, writes `rejected_at`/`rejected_reason`.

### Tests
- `src-tauri/tests/workspace_ingestion_runs.rs` — `start_run` / `complete_run` / `find_by_idempotency_key` / retry lineage / state persistence (V1.1 fold #5).
- `src-tauri/tests/workspace_ingestion_link.rs` — `add_link` / `list_links_for_file` round-trip + `override_link` signal emission via mock + `reject_link` tombstone + classifier-resurrection prevention (V1.1 fold #3 + #5).

### NOT touched (deny list)
- `src-tauri/src/services/workspace_ingestion/{mod,contracts,lifecycle,registry,pipeline,extract,signals,graph,wiring}.rs` — W1-A or other-lane territory.
- `src-tauri/src/services/claims.rs`, `src-tauri/src/signals/**`.
- `abilities-runtime/**` — consume canonical types; never modify substrate.

## 5. Contracts referenced + K-in citations

| Contract | Location | W1-C relationship |
|---|---|---|
| `contracts::SignalEmitter` (trait, 5 methods, `Send + Sync`) | `services/workspace_ingestion/contracts.rs:209` (W1-A) | **consumes** (`override_link` takes `&dyn SignalEmitter`) |
| `contracts::NullSignalEmitter` (no-op default) | `…/contracts.rs:248` (W1-A) | **consumes** (test fixtures) |
| `lifecycle::UserOverride { actor_id, at }` | `services/workspace_ingestion/lifecycle.rs:53` (W1-A) | **consumes** (V1.1 fold #2: `document_entity_links.user_override_*` mirrors this shape) |
| W1-A `workspace_file_lifecycle.file_id` (UNIQUE) | v250 migration (W1-A) | **FK target** for both new tables |
| Highest registered migration on dev | v240 + W1-A v250+v251 + W1-B v252 | W1-C claims **v253 + v254** per cycle 11 |

### Anti-reinvention pre-grep (V1.1 re-grep)

- `abilities-runtime/src/abilities/trust/types.rs` — no name collisions with `IngestionRunId`, `IngestionMode`, `IngestionRunStatus`, `StartRunSeed`, `ExistingRunReceipt`, `DocumentEntityLinkId`, `LinkAttributionSource`, `DocumentEntityLink`, `LinkError`, `RunsError`.
- `abilities-runtime/src/abilities/provenance/source.rs` — `SourceAttribution` (canonical) — W1-C does NOT reinvent; `LinkAttributionSource` is a workspace-link-shaped enum distinct from claim-provenance `SourceAttribution`.
- `services/claims.rs` — `ClaimProposal` (W3-A consumer), `ClaimFeedbackInput`/`Outcome` — no collisions.
- `lifecycle::UserOverride` — explicitly consumed (V1.1 fold #2).

`tests/workspace_ingestion_no_substrate_reinvention.rs` (W1-A CI gate) catches any future drift.

### K-in `docs/solutions/` + `.docs/decisions/` grep

Re-run V1.1: `document_ingestion_runs`, `document_entity_links`, `override_link`, `ingestion_run`, `idempotency_key`, `tombstone`, `classifier_resurrection`. **Zero hits.** ADR anchors: ADR-0098 + ADR-0125. No K-in BLOCKED finding.

## 6. v253 + v254 migration column shapes

### v253 — `document_ingestion_runs`

```sql
-- v1.4.5 W1-C — ingestion run tracking. Append-only history.
CREATE TABLE IF NOT EXISTS document_ingestion_runs (
    id                      INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id                  TEXT NOT NULL UNIQUE,
    file_id                 TEXT NOT NULL,
    mode                    TEXT NOT NULL,                  -- 'initial' | 'incremental' | 'forced' | 'backfill'
    started_at              TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    completed_at            TEXT,
    status                  TEXT NOT NULL DEFAULT 'in_progress',
    content_sha256          TEXT NOT NULL,
    file_size_bytes         INTEGER NOT NULL,
    extractor_version       TEXT NOT NULL,
    claim_count_produced    INTEGER NOT NULL DEFAULT 0,
    error_log               TEXT,
    retry_of_run_id         TEXT,
    FOREIGN KEY (file_id) REFERENCES workspace_file_lifecycle(file_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_dir_file_id ON document_ingestion_runs (file_id);
CREATE INDEX IF NOT EXISTS idx_dir_status ON document_ingestion_runs (status);
-- V1.2 fold #4: UNIQUE partial index enforces idempotency at SQL layer (not just lookup).
-- Successful run rows are unique by (file_id, content_sha256, mode); Forced-mode retries
-- coexist because in-progress rows aren't in the partial scope, and a new successful retry
-- replaces the prior only via explicit re-run lineage tracking via retry_of_run_id.
CREATE UNIQUE INDEX IF NOT EXISTS idx_dir_idempotency_unique
    ON document_ingestion_runs (file_id, content_sha256, mode)
    WHERE status = 'success';
```

### v254 — `document_entity_links`

```sql
-- v1.4.5 W1-C — document/entity link table. V1.1 fold #2: user_override columns match W1-A
-- WorkspaceFileLifecycle precedent (user_override_actor + user_override_at nullable pair).
-- V1.1 fold #3: partial unique index enforces tombstone semantics — no duplicate active row
-- for the same (file_id, entity_type, entity_id) triple, regardless of classifier race.
CREATE TABLE IF NOT EXISTS document_entity_links (
    id                      INTEGER PRIMARY KEY AUTOINCREMENT,
    link_id                 TEXT NOT NULL UNIQUE,
    file_id                 TEXT NOT NULL,
    entity_type             TEXT NOT NULL,                  -- snake_case per crate::entity::EntityType
    entity_id               TEXT NOT NULL,                  -- canonical v1.4.0 entity-slug
    attribution_source      TEXT NOT NULL,                  -- 'entity_intake' | 'frontmatter' | 'classifier' | 'user_relink' | 'drive_metadata' | 'mcp_placement' | 'backfill'
    confidence              REAL NOT NULL DEFAULT 0.5,
    rationale               TEXT,
    actor                   TEXT NOT NULL,
    user_override_actor     TEXT,                           -- nullable; populated by override_link
    user_override_at        TEXT,                           -- nullable; populated by override_link
    rejected                INTEGER NOT NULL DEFAULT 0,
    rejected_at             TEXT,
    rejected_reason         TEXT,
    created_at              TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    updated_at              TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    FOREIGN KEY (file_id) REFERENCES workspace_file_lifecycle(file_id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_del_file_id ON document_entity_links (file_id);
CREATE INDEX IF NOT EXISTS idx_del_entity ON document_entity_links (entity_type, entity_id);
-- V1.1 fold #3: prevent classifier-resurrection at SQL layer — no duplicate active link
-- for same triple. V1.2 fold #2 documents the canonical SQLite UPSERT pattern:
--   INSERT INTO document_entity_links (...) VALUES (...)
--   ON CONFLICT (file_id, entity_type, entity_id) WHERE rejected = 0
--   DO NOTHING RETURNING link_id;
-- The WHERE clause lives in the conflict target (matches the partial unique predicate),
-- not the action clause.
CREATE UNIQUE INDEX IF NOT EXISTS idx_del_active_unique
    ON document_entity_links (file_id, entity_type, entity_id)
    WHERE rejected = 0;
-- V1.2 fold #3: lookup index for the service-layer tombstone guard.
-- add_link from classifier/backfill/drive sources runs a SELECT against this index
-- before insert; presence of a rejected row blocks the insert with LinkError::Tombstoned.
CREATE INDEX IF NOT EXISTS idx_del_rejected_lookup
    ON document_entity_links (file_id, entity_type, entity_id)
    WHERE rejected = 1;
```

## 7. Tests required

- **Migrations:** v253 + v254 apply cleanly after v252 (W1-B); FK to `workspace_file_lifecycle.file_id` enforced (CASCADE delete).
- **`RunsRepo::start_run + complete_run + find_by_idempotency_key`:**
  - `start_run(seed)` returns IngestionRunId; row present with all NOT NULL columns populated from seed.
  - **V1.1 fold #5:** `complete_run` persists `status`, `claim_count_produced`, `error_log` correctly; `find_by_idempotency_key` returns `Some(ExistingRunReceipt)` after success.
  - Repeat `start_run` with same `(file_id, content_sha256, mode)` after first success → `find_by_idempotency_key` returns the existing receipt.
  - `start_run` with `mode = Forced` + `retry_of_run_id = prior_run_id` → new row inserted; `retry_of_run_id` populated.
- **`LinkRepo::add_link / list_links_for_file`:**
  - **V1.1 fold #5:** `add_link` then `list_links_for_file(file_id, include_rejected=false)` returns the link; `include_rejected=true` returns rejected links too.
  - Multi-entity: same `file_id` with two different `(entity_type, entity_id)` → both rows present.
- **`LinkRepo::override_link` signal emission:** writes the link AND calls `emitter.emit_link_changed(file_id, entity_id, actor)` on a mock `SignalEmitter` recorder. Populates `user_override_actor` + `user_override_at` per V1.1 fold #2.
- **V1.1 fold #3 / V1.2 fold #3 — `tombstone_prevents_classifier_resurrection`:** `add_link(file_id, EntityType::Account, "acme", Classifier, ...)` → `reject_link(file_id, EntityType::Account, "acme", actor, reason)` → `add_link(file_id, EntityType::Account, "acme", Classifier, ...)` again → asserts `Err(LinkError::Tombstoned)` (service guard blocks); the rejected row remains; `list_links_for_file(file_id, include_rejected=false)` returns empty.
- **V1.2 fold #5 — `user_relink_after_tombstone_succeeds`:** same setup but the second `add_link` uses `attribution_source = UserRelink` → succeeds; a new active row is created (intentional user-driven resurrection per the AC carve-out).
- **V1.2 fold #5 — `duplicate_active_classifier_attempt_is_noop`:** `add_link(...)` → second `add_link(...)` with identical triple → returns the existing `DocumentEntityLinkId` (no error); single active row in the table.
- **V1.3 fold #6 — `idempotency_unique_constraint_enforces_no_duplicate_successful_run`:** `start_run + complete_run(success)` twice with same `(file_id, content_sha256, mode)` → second call returns `Err(RunsError::AlreadyCompleted { existing: receipt })` from the service-layer guard (NOT generic `DbError`). Test asserts: returned error envelope carries the existing receipt; total `status='success'` rows in the table = exactly 1. Generic `DbError` is **not** an acceptable outcome — service guard must catch the case before the SQL UNIQUE fires.
- **V1.3 fold #3 — `in_progress_idempotency_returns_already_in_progress`:** `start_run(seed)` → without calling `complete_run`, a second `start_run(seed)` with same `(file_id, content_sha256, mode)` returns `Err(RunsError::AlreadyInProgress { existing_run_id })` if the prior run is <1 hour old; if >1 hour old, the prior is marked `aborted` and the new run proceeds.
- **Multi-entity links:** one `file_id` can have multiple `(entity_type, entity_id)` active rows without partial-unique violation.
- **CI gates:** `cargo clippy --lib -- -D warnings` clean; `tests/workspace_ingestion_no_substrate_reinvention.rs` (W1-A) still passes.

## 8. Done when

- Migration slots **v253 + v254** used; both tables exist with full column set per §6; FK enforced; partial unique index on `document_entity_links` enforced.
- `services/workspace_ingestion/runs.rs` substantively filled with `RunsRepo` + `StartRunSeed` request struct + `RunsError` enum.
- `services/workspace_ingestion/link.rs` substantively filled with `LinkRepo` + `DocumentEntityLink` + `LinkAttributionSource` enum + `LinkError` enum. `override_link(emitter: &dyn contracts::SignalEmitter, …)` compiles against W1-A trait + `NullSignalEmitter`.
- `user_override_actor` + `user_override_at` columns present on `document_entity_links` (mirrors W1-A precedent per V1.1 fold #2).
- Tombstone semantics enforced via partial unique index + service guard (V1.1 fold #3); resurrection test green.
- All tests in §7 pass.
- `tests/workspace_ingestion_no_substrate_reinvention.rs` (W1-A's CI gate) still passes.
- `cargo clippy --lib -- -D warnings && cargo test` green.
- IL gate items 1–5 answered in commit message.
- `L2-status: passed` declared.
- L0 verdict posted as Linear comment on DOS-465.

## 9. Intelligence Loop gate

1. **Claim model.** Per L0 question #10 (cycle 2): both tables are relational metadata, not claims. W1-C commits no claims.
2. **Provenance + trust.** `document_ingestion_runs.content_sha256` provides tamper detection. `document_entity_links.{attribution_source, confidence, user_override_*}` are link-quality fields; downstream W3-A treats user-overridden + high-confidence links as trust-factor inputs.
3. **Signals + invalidation.** `override_link` emits via `contracts::SignalEmitter::emit_link_changed`. W3-B real impl maps to `SignalType::WorkspaceFileEntityLinkChanged`. W1-C does NOT reference the SignalType variant directly (trait-DI pattern preserved per W1-A cycle 4-6 fix).
4. **Runtime + surfaces.** `document_ingestion_runs` consumed by W5 backfill (idempotency), W4-A source-management block (history display), W3-C graph projection (run-status surface). `LinkRepo::add_link(... UserRelink)` consumed by W4-A re-link UI; `LinkRepo::add_link(... EntityIntake)` consumed by W2-C entity-seeded intake; `LinkRepo::override_link` consumed by W4-A endorse-UI (V1.3 fold #2: override is for existing-active endorsement only, NOT resurrection — resurrection goes through `add_link(UserRelink)`).
5. **Feedback loop.** `override_link` populates `user_override_*` AND emits signal. `reject_link` writes `rejected = 1` AND partial unique + service guard prevent silent classifier resurrection (Linear DOS-465 AC; V1.1 fold #3 + #6).

## 10. Handoff notes

- **W2-A (DOS-466)** consumes `RunsRepo::{start_run, complete_run, find_by_idempotency_key}`. Idempotency check before each ingestion attempt.
- **W3-A (DOS-470)** populates `extractor_version` + `claim_count_produced` after extraction.
- **W3-B (DOS-471)** wires real `WorkspaceSignalEmitter::emit_link_changed`; the trait-method surface W1-C compiles against in `override_link` is unchanged.
- **W4-A (DOS-472)** invokes:
  - `LinkRepo::add_link(file_id, entity_type, entity_id, UserRelink, ..., actor)` for the re-link UI (resurrects rejected links; creates new active row if tombstone exists per V1.3 fold #2).
  - `LinkRepo::override_link(emitter, file_id, entity_type, entity_id, actor)` for the "endorse this link" UI (marks an existing-AND-active link as user-confirmed; `LinkError::NotFound` if no active row).
  - `LinkRepo::reject_link(file_id, entity_type, entity_id, actor, reason)` for the "this isn't right" UI.
  - `RunsRepo::find_by_idempotency_key` for run-history display.
- **W2-C (DOS-468)** invokes `LinkRepo::add_link(file_id, entity_type, entity_id, EntityIntake, ..., actor)` from the entity-intake Gutenberg block (passes the full triple per V1.2 fold #1 + V1.3 fold #7 signature sweep).

No lane creates files in `services/workspace_ingestion/` beyond W1-A placeholders. No lane edits `mod.rs`. No lane reinvents canonical substrate primitives (W1-A CI grep gate enforces).
