# Implementation Plan: DOS-296

## Revision history

- v1 (2026-05-01) - initial L0 draft.

## 1. Contract restated

DOS-296 is a substrate-only allowance for longitudinal topic threading. It adds a nullable thread reference to claim storage and an additive `thread_ids` list to Provenance, but does not define what a thread means or how claims enter one. Load-bearing ticket lines: "Add nullable `thread_id TEXT NULL` column to `intelligence_claims` table." "`Provenance.provenance_schema_version` stays at `1` - additive, optional, default-empty field per ADR-0105 §1 forward-compatibility rules." "`dedup_key` formula does NOT include `thread_id` - same-content claims that participate in different threads remain the same claim." "No retrieval logic. No UI. No thread creation. No assignment heuristic."

The 2026-04-24 ADR-0124 allowance applies in full: `ThreadId(pub Uuid)`, one nullable `thread_id` column, `Provenance.thread_ids: Vec<ThreadId>`, unchanged schema version, and no behavior (`.docs/decisions/0124-longitudinal-topic-threading.md:25-49`). The ADR's downstream decision is explicitly deferred to v1.4.2 Options A/B/C (`.docs/decisions/0124-longitudinal-topic-threading.md:50-73`). ADR-0105 §1 forward compatibility also applies: known fields keep meaning, unknown additions are tolerated, and initial `provenance_schema_version` is `1` (`.docs/decisions/0105-provenance-as-first-class-output.md:23-70`).

Acceptance is narrow: existing claim rows backfill as `NULL`, default Provenance serializes `thread_ids: []`, two-id roundtrip works, `provenance_schema_version == 1`, and the existing dedup computation stays byte-equivalent for same-content claims with different thread ids.

## 2. Approach

Add `ThreadId` in the Provenance module created by W3-B. The prompt allows `src-tauri/src/abilities/provenance.rs` or equivalent; W3-B's prompt owns `src-tauri/src/abilities/provenance/`, while current code has no `src-tauri/src/abilities` tree yet. Implementation target after W3-B lands: `src-tauri/src/abilities/provenance/mod.rs` or a colocated `ids.rs` re-exported from that module. Do not touch W2 frozen `src-tauri/src/intelligence/provider.rs`.

`ThreadId` shape:

```rust
pub struct ThreadId(pub Uuid);
```

Implement serde as a transparent, lower-case hyphenated UUID string with strict parse-on-deserialize. Implement `JsonSchema` as a string schema with `format = "uuid"`, following the existing MCP-side `JsonSchema` dependency pattern (`src-tauri/src/mcp/main.rs:14-17`, `:43-47`). Prefer manual serde/schema impls over changing `uuid` features, because current `src-tauri/Cargo.toml:33` enables only `uuid = { version = "1", features = ["v4"] }` and `schemars` is optional behind `mcp` (`src-tauri/Cargo.toml:83-90`).

Add `pub thread_ids: Vec<ThreadId>` to the W3-B `Provenance` envelope beside the ADR-0105 fields. It must use `#[serde(default)]` and must not use `skip_serializing_if`, because the acceptance criterion requires default serialization as `[]`. Builder/default construction initializes `Vec::new()`. No production assignment method or heuristic is added; tests may construct non-empty Provenance directly to prove serialization. ADR-0105's builder still owns field-attribution enforcement and finalization (`.docs/decisions/0105-provenance-as-first-class-output.md:305-312`), and composition semantics remain unchanged (`.docs/decisions/0105-provenance-as-first-class-output.md:257-266`).

Add one migration after DOS-7's `intelligence_claims` migration:

```sql
ALTER TABLE intelligence_claims ADD COLUMN thread_id TEXT NULL;
```

Current migration tail is version 125 registered at `src-tauri/src/migrations.rs:588-590`; DOS-7's draft tentatively takes version 126 / file `125_dos_7_claims_consolidation.sql` (`.docs/plans/wave-W3/DOS-7-plan.md:21`). If that holds, DOS-296 takes the next slot: `src-tauri/src/migrations/126_dos_296_claim_thread_id.sql` registered as version 127. If W3-C finalizes a different number, this migration moves to the immediately following number. Existing rows need no `UPDATE`; SQLite adds the column with implicit `NULL`.

Add only tests around dedup, not production dedup code. ADR-0113's formula is `hash(entity_id, claim_type, field_path, normalized_claim_text)` (`.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:187-190`), and ADR-0124 forbids adding `thread_id` (`.docs/decisions/0124-longitudinal-topic-threading.md:44-48`). Once DOS-7 exposes the canonicalization helper or claim proposal path, add a test that passes identical claim content with two different `ThreadId`s and asserts the same `dedup_key`.

End-state alignment: v1.4.0 stores the compatibility seam now, so v1.4.2 can choose Theme entity, derived `thread_id`, or hybrid without rewriting the load-bearing claim table (`.docs/decisions/0124-longitudinal-topic-threading.md:75-88`, `:104-109`). It forecloses using `thread_id` as part of claim identity, and it intentionally does not foreclose a later many-to-many `claim_threads` table.

## 3. Key decisions

`provenance_schema_version` remains `1`. The new field is additive and default-empty, so a bump would violate both the ticket and ADR-0124 (`.docs/decisions/0124-longitudinal-topic-threading.md:46-48`).

`thread_ids` serializes when empty. Pick: `#[serde(default)]` without `skip_serializing_if`; future consumers see an empty list rather than a missing field, matching ADR-0124 consequences (`.docs/decisions/0124-longitudinal-topic-threading.md:124-126`).

No index on `intelligence_claims.thread_id` in v1.4.0. There is no retrieval by thread in this issue, and ADR-0124 says v1.4.2 owns retrieval and assignment (`.docs/decisions/0124-longitudinal-topic-threading.md:111-118`). Adding an index now optimizes a non-existent query and can be revisited when v1.4.2 selects the actual access pattern.

Single nullable column, not `claim_threads`. ADR-0124 says the v1.4.0 single column is sufficient, and a child table is an additive v1.4.2 extension if multiple membership becomes real (`.docs/decisions/0124-longitudinal-topic-threading.md:75-88`).

Manual `ThreadId` serde/schema impls. This keeps the change inside owned Provenance files and avoids a Cargo feature decision that W3-B may already need to solve for the whole envelope.

No direct builder assignment behavior. Default-empty substrate is enough for v1.4.0; v1.4.2 can add an explicit setter or assignment API when semantics exist.

## 4. Security

No new auth or external input path is introduced. `thread_id` is not read from users, MCP callers, or model output in this issue. The only validation surface is serde deserialization of `ThreadId`, which must reject non-UUID strings and avoid accepting arbitrary labels that could later be confused with user-authored theme names.

Cross-tenant or cross-entity exposure risk stays dormant because there is no retrieval by thread and no UI. The plan must not add "all claims in this thread" loaders. Future v1.4.2 retrieval must validate subject scope before using a thread id, but DOS-296 only stores opaque ids.

Thread ids are identifiers, not content. Tests use synthetic UUIDs and generic subjects only, preserving the no-customer-data rule in `CLAUDE.md:16-18`. Logs and migration diagnostics should report only migration counts and column presence, never claim text or provenance JSON.

## 5. Performance

The SQLite migration is a metadata-only `ALTER TABLE ... ADD COLUMN ... NULL` on the claim table after DOS-7 creates it. It does not backfill values, scan provenance blobs, or add indexes. The migration runner applies pending migrations in order and records the version only after successful execution (`src-tauri/src/migrations.rs:1028-1085`).

Runtime overhead is negligible: one empty `Vec` in each Provenance envelope and a `thread_ids: []` JSON field. No hot-path queries, Trust Compiler math, claim invalidation, or projection writes change. Dedup stays keyed on normalized content only, so claim insert/corroboration performance is unchanged.

Future retrieval cost is intentionally not optimized in this issue. If v1.4.2 chooses thread-centric reads, it can add an index on `thread_id` or a `claim_threads` table with a primary key after real query shapes exist.

## 6. Coding standards

Services-only mutations are preserved. DOS-296 adds schema and types; it does not write claims outside DOS-7's `services::claims::commit_claim` path. Current per-entity invalidation expects DOS-7 commits to call `db::claim_invalidation::bump_for_subject` inside the claim transaction (`src-tauri/src/db/claim_invalidation.rs:43-45`, `:180-228`); DOS-296 does not alter that path.

Intelligence Loop check (`CLAUDE.md:7-14`): no signal emission, health-score input, intelligence-context inclusion, briefing callout, or Bayesian feedback hook is added now. The correct answer for all five is "not in v1.4.0; v1.4.2 owns semantics and consumers." That is acceptable here because ADR-0124 defines this as a substrate allowance with no behavior (`.docs/decisions/0124-longitudinal-topic-threading.md:136-137`).

No direct `Utc::now()` or `thread_rng()` is needed. Migration SQL uses no dynamic values. Tests use fixed UUID strings. Clippy budget is unchanged; the PR must pass `cargo clippy -- -D warnings && cargo test && pnpm tsc --noEmit` per `CLAUDE.md:20-24`.

## 7. Integration with parallel wave-mates

W3-C / DOS-7 is the hard dependency. W3-F must not open until `intelligence_claims` exists. Current DOS-7 draft says it may include `thread_id` in the base table (`.docs/plans/wave-W3/DOS-7-plan.md:21-23`, `:85-87`), while the DOS-296 prompt says W3-F's migration adds the column on top. Resolve before coding: either W3-C removes `thread_id` and W3-F owns the ALTER, or W3-F downgrades the migration to a schema assertion only under architect approval. Do not rely on the migration runner's duplicate-column tolerance (`src-tauri/src/migrations.rs:1063-1071`) as the coordination mechanism.

W3-B / DOS-211 creates the Provenance envelope under `src-tauri/src/abilities/provenance/`. DOS-296 should land `ThreadId` and `thread_ids` into that initial envelope when possible so there is no transient Provenance schema without the field. W3-B also owns `SourceAttribution`, trust, warnings, and size-budget behavior; DOS-296 only adds the thread field.

W3-G / DOS-299 and W3-H / DOS-300 are adjacent schema adders. Coordinate migration numbers so the final sequence is DOS-7 base table first, then additive columns in deterministic wave order, with the W3 integration commit resolving any filename/version drift. The wave gate explicitly expects a single integration commit to resolve migration numbering (`.docs/plans/v1.4.0-waves.md:524-533`).

Reviewer: architect-reviewer per the substrate/schema matrix (`.docs/plans/v1.4.0-waves.md:503-508`).

## 8. Failure modes + rollback

If the DOS-296 migration runs before DOS-7, `ALTER TABLE intelligence_claims` fails because the table is absent. That is correct: no schema version is recorded on failure (`src-tauri/src/migrations.rs:1078-1085`), and the fix is migration ordering, not table creation in this PR.

If W3-C already added the column, the runtime may treat duplicate column as benign, but that would hide an ownership violation. CI/review should catch the duplicate before merge. If a local developer hits it, rollback is to remove the duplicate migration entry or rebase on the agreed W3 sequence.

If Provenance deserialization sees old JSON without `thread_ids`, `#[serde(default)]` returns `Vec::new()`. If it sees invalid UUID strings in `thread_ids`, deserialization fails for that Provenance payload rather than accepting malformed thread references.

Rollback for an applied additive migration is restore from the migration safety backup created before pending migrations (`src-tauri/src/migrations.rs:1019-1026`) or leave the unused nullable column in place. No data rewrite is required. W1-B universal write fence is honored because DOS-296 performs no `intelligence.json` writes; if bundled with DOS-7 cutover, it runs inside the same schema-epoch/drain envelope that `write_fence::bump_schema_epoch` and `drain_with_timeout` provide (`src-tauri/src/intelligence/write_fence.rs:120-155`, `:223-238`).

## 9. Test evidence to be produced

Rust tests:
- `thread_id_rejects_invalid_uuid_string`
- `thread_id_json_schema_is_uuid_string`
- `provenance_default_thread_ids_serializes_as_empty_array`
- `provenance_thread_ids_roundtrip_two_ids`
- `provenance_missing_thread_ids_deserializes_to_empty_vec`
- `provenance_schema_version_remains_one_with_thread_ids`
- `intelligence_claims_thread_id_column_is_nullable`
- `dedup_key_ignores_thread_id`

Migration evidence: a schema test against a DB migrated through DOS-7 then DOS-296 that verifies `PRAGMA table_info(intelligence_claims)` includes `thread_id` with nullable TEXT affinity and existing inserted pre-migration rows read `NULL`.

Wave merge-gate artifact: include DOS-296 test output in the W3 `cargo clippy -- -D warnings && cargo test && pnpm tsc --noEmit` bundle. Suite S contribution is a short note that no new write site, retrieval path, or PII-bearing log exists. Suite P contribution is ALTER timing/metadata-only evidence. Suite E contribution is no behavior drift: bundle 1+5 should see identical claim identity and outputs because `thread_id` is unset.

## 10. Open questions

1. Migration ownership conflict: should W3-C remove `thread_id` from its base `intelligence_claims` create-table draft so W3-F can own the additive migration exactly as assigned, or should W3-F treat the DOS-7 base-table inclusion as the implementation and only own Provenance/tests?
2. `JsonSchema` dependency shape: should W3-B make `schemars` available to the core Provenance module, or should `ThreadId`'s schema impl be `cfg(feature = "mcp")`/re-exported through the existing `rmcp::schemars` path?
3. Dedup test placement: will DOS-7 expose a pure `compute_dedup_key(...)` helper that W3-F can test without touching `commit_claim`, or should W3-F add the assertion in DOS-7's claim-service integration tests?
