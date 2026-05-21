# L0 Packet — v1.4.5 W1-C — DOS-465 Document/Entity Links + Ingestion Run Tracking

**Current revision:** V1.1 (cycle 1 fold, 2026-05-20). See §2 Changelog.

## 1. Header

- **Date:** 2026-05-20
- **Project:** v1.4.5 — Workspace Memory Refactor ([Linear](https://linear.app/a8c/project/v145-workspace-memory-refactor-cdb9d2c17102))
- **Wave:** W1 stage 1b (gates on W1-A merge; runs parallel with W1-B)
- **Issue:** [DOS-465 — Add document/entity links and ingestion run tracking](https://linear.app/a8c/issue/DOS-465)
- **Branch (proposed):** `feat/dos-465-document-entity-links` from `wave/v1.4.5-w1-stage1a` after W1-A merges
- **Migration slots claimed:** **v253, v254** (from v1.4.5 W1 block v250–v254 per wave-plan §Cycle 11)
- **L0 reviewer matrix:** architect-reviewer + codex challenge + codex consult. No `/cso`.
- **L2 reviewer matrix:** codex review + code-reviewer + architect-reviewer.

## 2. Changelog

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
  - `RunsError { NotFound, AlreadyCompleted, ContentSha256Mismatch, DbError(String) }` enum + Display + Error.
  - `RunsRepo::start_run(seed: StartRunSeed) -> Result<IngestionRunId, RunsError>`.
  - `RunsRepo::complete_run(run_id: &IngestionRunId, status: IngestionRunStatus, claim_count: u64, error_log: Option<serde_json::Value>) -> Result<(), RunsError>`.
  - `RunsRepo::find_by_idempotency_key(file_id: &str, content_sha256: &str, mode: IngestionMode) -> Result<Option<ExistingRunReceipt>, RunsError>`.
- `src-tauri/src/services/workspace_ingestion/link.rs` — fills the W1-A-pre-created placeholder:
  - `DocumentEntityLinkId(pub String)` newtype.
  - `LinkAttributionSource { EntityIntake, Frontmatter, Classifier, UserRelink, DriveMetadata, McpPlacement, Backfill }` enum (snake_case serde).
  - `DocumentEntityLink { link_id, file_id, entity_type, entity_id, attribution_source, confidence, rationale, actor, user_override: Option<lifecycle::UserOverride>, rejected, rejected_at, rejected_reason, created_at, updated_at }` (V1.1 fold #2: `user_override` matches W1-A's pattern; reuses `lifecycle::UserOverride` struct).
  - `LinkError { NotFound, DuplicateActive, AlreadyRejected, EntityTypeUnknown, DbError(String) }` enum + Display + Error.
  - `LinkRepo::add_link(file_id, entity_type, entity_id, attribution_source, confidence, rationale, actor) -> Result<DocumentEntityLinkId, LinkError>` — UPSERT semantics via partial unique index; returns existing link's id on conflict-do-nothing.
  - `LinkRepo::list_links_for_file(file_id, include_rejected) -> Result<Vec<DocumentEntityLink>, LinkError>`.
  - `LinkRepo::override_link(emitter: &dyn contracts::SignalEmitter, file_id: &str, entity_id: &str, actor: &str) -> Result<(), LinkError>` — calls `emitter.emit_link_changed(file_id, entity_id, actor)` after the row write; populates `user_override` per W1-A precedent (V1.1 fold #2).
  - `LinkRepo::reject_link(file_id, entity_id, actor, reason) -> Result<(), LinkError>` — flips `rejected = 1`, writes `rejected_at/rejected_reason`. Per DOS-465 issue AC "Rejected links are not silently recreated" (V1.1 fold #6).

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
-- Idempotency key index.
CREATE INDEX IF NOT EXISTS idx_dir_idempotency
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
-- V1.1 fold #3: prevent classifier-resurrection — no duplicate active link for same triple.
CREATE UNIQUE INDEX IF NOT EXISTS idx_del_active_unique
    ON document_entity_links (file_id, entity_type, entity_id)
    WHERE rejected = 0;
CREATE INDEX IF NOT EXISTS idx_del_rejected
    ON document_entity_links (rejected) WHERE rejected = 1;
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
- **V1.1 fold #3 — `tombstone_prevents_classifier_resurrection`:** `add_link(A, B, ...)` → `reject_link(A, B, ..., reason)` → `add_link(A, B, ...)` again (simulating classifier re-running) → asserts no new active row; the rejected row remains; `list_links_for_file(file_id, include_rejected=false)` returns empty.
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
4. **Runtime + surfaces.** `document_ingestion_runs` consumed by W5 backfill (idempotency), W4-A source-management block (history display), W3-C graph projection (run-status surface). `LinkRepo::override_link` consumed by W4-A (re-link action) and W2-C (entity-seeded intake first-link).
5. **Feedback loop.** `override_link` populates `user_override_*` AND emits signal. `reject_link` writes `rejected = 1` AND partial unique + service guard prevent silent classifier resurrection (Linear DOS-465 AC; V1.1 fold #3 + #6).

## 10. Handoff notes

- **W2-A (DOS-466)** consumes `RunsRepo::{start_run, complete_run, find_by_idempotency_key}`. Idempotency check before each ingestion attempt.
- **W3-A (DOS-470)** populates `extractor_version` + `claim_count_produced` after extraction.
- **W3-B (DOS-471)** wires real `WorkspaceSignalEmitter::emit_link_changed`; the trait-method surface W1-C compiles against in `override_link` is unchanged.
- **W4-A (DOS-472)** invokes `LinkRepo::override_link` from source-management block; consumes `RunsRepo::find_by_idempotency_key` for run-history display.
- **W2-C (DOS-468)** invokes `LinkRepo::add_link` with `attribution_source = EntityIntake` from the entity-intake Gutenberg block.

No lane creates files in `services/workspace_ingestion/` beyond W1-A placeholders. No lane edits `mod.rs`. No lane reinvents canonical substrate primitives (W1-A CI grep gate enforces).
