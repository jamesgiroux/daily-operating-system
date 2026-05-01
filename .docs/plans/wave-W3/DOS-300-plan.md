# Implementation Plan: DOS-300

## Revision history

- v1 (2026-05-01) - initial L0 draft.

## 1. Contract restated

DOS-300 ships the ADR-0125 claim-anatomy substrate primitives: `temporal_scope`, `sensitivity`, and a closed claim-type registry for `intelligence_claims`. Load-bearing ticket lines: "`TemporalScope` enum" with `PointInTime`, `State`, and `Trend`; "`ClaimSensitivity` enum: `Public`, `Internal`, `Confidential`, `UserOnly`"; "`ClaimTypeRegistry` - compile-time exhaustive `const &[ClaimTypeMetadata]` slice"; "Pattern mirrors ADR-0115 Signal Policy Registry exactly"; and "`ClaimTypeMetadata.default_temporal_scope` and `default_sensitivity` are looked up at claim-write time when the row leaves either field at default."

The 2026-04-24 Claim Anatomy Review applies exactly for sections 11, 12, and 15: temporal scope is a spine schema allowance because DOS-10 will otherwise decay PointInTime claims incorrectly (`.docs/plans/claim-anatomy-review-2026-04-24.md:61-68`); sensitivity moves surface eligibility onto the row (`:69-73`); and claim-type taxonomy becomes a spine primitive with an ADR-0115-style registry (`:87-91`). The review maps those three dimensions to DOS-300 (`:120-122`) and explicitly defers DOS-10 temporal consultation and DOS-214 render enforcement to v1.4.1 (`:126-130`).

This is substrate-only. Render-time sensitivity gates, freshness math using `temporal_scope`, supersession semantics by scope, and automatic sensitivity inference beyond registry defaults stay out of this PR per ADR-0125 non-goals (`.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:162-171`).

## 2. Approach

Add the claim anatomy types in `src-tauri/src/abilities/claims.rs` after W3-A creates `src-tauri/src/abilities/` (`.docs/plans/v1.4.0-waves.md:466-470`). If W3-A has not landed when coding starts, create only the claims module and coordinate the `pub mod claims;` export with W3-A rather than forking an ability registry. `TemporalScope` stores `PointInTime { occurred_at: DateTime<Utc> }`, `State`, and `Trend { window_start, window_end }` per ADR-0125 lines 23-45. `ClaimSensitivity` stores `Public`, `Internal`, `Confidential`, `UserOnly` per lines 58-76. Derive `Serialize`, `Deserialize`, `Clone`, `Debug`, `PartialEq`, `Eq`; derive `JsonSchema` behind the existing optional `mcp`/`schemars` feature (`src-tauri/Cargo.toml:83-90`).

DB encoding: `sensitivity` is lowercase snake_case text. `temporal_scope` accepts the raw SQL default `state` for `State`; non-state variants are serialized as tagged JSON text so timestamps stay attached to the row. Add `TemporalScope::to_db_value` / `from_db_value` and `ClaimSensitivity::to_db_value` / `from_db_value`; do not depend on bare serde for the SQLite column because ADR-0125 pins `DEFAULT 'state'` (`.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:48-54`).

Add schema after DOS-7 creates `intelligence_claims`. Current migration tail is version 125 (`src-tauri/src/migrations.rs:575-590`); DOS-7 tentatively claims the next slot in its plan (`.docs/plans/wave-W3/DOS-7-plan.md:21`). Preferred DOS-300 files after DOS-7: `src-tauri/src/migrations/12X_dos_300_claim_temporal_scope.sql` with `ALTER TABLE intelligence_claims ADD COLUMN temporal_scope TEXT NOT NULL DEFAULT 'state';`, and `src-tauri/src/migrations/12Y_dos_300_claim_sensitivity.sql` with `ALTER TABLE intelligence_claims ADD COLUMN sensitivity TEXT NOT NULL DEFAULT 'internal';`. Use one column per migration because the current runner treats duplicate-column errors as benign and records the migration after `execute_batch` failure (`src-tauri/src/migrations.rs:1030-1084`); a two-ALTER batch could skip the second column if only the first already exists.

Define `ClaimType` as the canonical Rust enum and make `claim_type` strings a DB serialization detail. Define `ClaimTypeMetadata` exactly with ADR-0125 fields: `name`, `default_temporal_scope`, `default_sensitivity`, `freshness_decay_class`, `commit_policy_class`, `allowed_actor_classes`, and `canonical_subject_types` (`.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:95-121`). Expose `CLAIM_TYPE_REGISTRY: &[ClaimTypeMetadata]` plus `metadata_for_claim_type(ClaimType)` and `ClaimType::try_from_db_str`.

Initial registry entries cover DOS-218 and DOS-219 outputs, seeded from the ticket output shapes and legacy fields: `entity_identity`, `entity_summary`, `entity_current_state`, `entity_risk`, `entity_win`, `stakeholder_role`, `stakeholder_engagement`, `stakeholder_assessment`, `value_delivered`, `meeting_readiness`, `company_context`, `open_loop`, `meeting_topic`, `meeting_event_note`, `attendee_context`, `meeting_change_marker`, and `suggested_outcome`. These map to DOS-218 `EntityContext` fields (`identity`, `state`, `relationships`, `recent_events`, `open_loops`, `trajectory`) and DOS-219 `MeetingBrief` fields (`topics`, `attendee_context`, `open_loops`, `what_changed_since_last`, `suggested_outcomes`) from the Linear bodies. Current legacy anchors are `IntelligenceJson` fields at `src-tauri/src/intelligence/io.rs:932-996`, stakeholder/value/readiness structs at `:1313-1454`, and meeting prep fields at `src-tauri/src/types.rs:1691-1781`.

Wire DOS-7 `services/claims.rs::commit_claim` to normalize defaults before insert. `ClaimProposal` should carry `Option<TemporalScope>` and `Option<ClaimSensitivity>` so omission is distinguishable from an explicit `State` or `Internal`. Production pilot abilities DOS-218 and DOS-219 must set both fields explicitly; registry default substitution is for backfill/legacy callers only, matching the prompt coordination note. `commit_claim` also validates `ClaimType` against `canonical_subject_types` before calling `db::claim_invalidation::bump_for_subject`, whose spine restriction says no v1.4.0 registered claim type may include `Global` (`src-tauri/src/db/claim_invalidation.rs:51-56`, `:152-158`).

End-state alignment: this keeps the claim row as the durable memory substrate DOS-7 describes (`.docs/plans/wave-W3/DOS-7-plan.md:9-27`), prevents free-form taxonomy drift before pilots ship, and forecloses post-spine migrations for the three ADR-0125 dimensions.

## 3. Key decisions

Use a `ClaimType` enum even though ADR-0125 describes strings in the table. ADR-0115 learned that non-test exhaustiveness must be a normal-build `const` match, not only `#[cfg(test)]` (`.docs/decisions/0115-signal-granularity-audit.md:249-267`). Strings alone cannot be exhaustively matched; the enum provides the closed set while `name` remains the canonical persisted string.

Mirror ADR-0115's revised pattern, not the older slice-only sketch. ADR-0115 originally named `src-tauri/src/signals/policy_registry.rs` as a const-slice registry (`.docs/decisions/0115-signal-granularity-audit.md:61-74`), then revised to a function form for variants with data (`:269`). Current `src-tauri/src/signals/` has no `policy_registry.rs` implementation, so DOS-300 copies the ADR pattern directly: const registry for inspectability plus a non-test exhaustive match for build-time coverage.

Keep DB defaults conservative and narrow: `temporal_scope = 'state'`, `sensitivity = 'internal'`. ADR-0125 says `State` preserves current implicit behavior (`.docs/decisions/0125-claim-anatomy-temporal-sensitivity-typeregistry.md:54`) and `Internal` is conservative for internal-system sources (`:79-87`). No claim should become `Public` by omission.

Do not make render policy or freshness policy active here. `FreshnessDecayClass` and `CommitPolicyClass` are metadata fields for later consumers; DOS-10 and DOS-214 consume them later. The registry can store classes now without changing trust math or surface filtering.

Do not silently synthesize timestamps for dynamic temporal scopes. `PointInTime` needs an `occurred_at`; `Trend` needs a window. Claim authors must supply those values for DOS-219 meeting-event and trend claims. See section 10 for the ADR mismatch if reviewers expect dynamic defaults to be represented directly in `ClaimTypeMetadata.default_temporal_scope`.

## 4. Security

The new security surface is not access control yet; it is data classification and subject eligibility. `sensitivity` must fail closed to `internal`, and `ClaimSensitivity::from_db_value` rejects unknown strings rather than downgrading to `Public`. Later render layers can safely enforce ceilings because the row now has a durable tier.

`canonical_subject_types` is a cross-tenant/subject-bleed guard. DOS-218 comments require hostile fixtures for same-domain accounts and wrong-entity evidence, and DOS-219 comments make wrong-account meeting prep the dangerous path. `commit_claim` must reject a `stakeholder_role` on an `Account` if the registry says `Person`, and must reject all `Global` subject claim types in v1.4.0 per the existing invalidation spine restriction.

No PII or claim text should appear in registry lint failures or migration logs. Error messages can include claim type name, subject type, and enum variant; they must not include customer content. This matches DOS-7's non-content audit posture (`.docs/plans/wave-W3/DOS-7-plan.md:51-57`).

## 5. Performance

Runtime cost is one metadata lookup per `commit_claim`. With ~15 initial entries, linear scan is acceptable, but `metadata_for_claim_type(ClaimType)` should be a match so generated code is O(1)-like and compile-time exhaustive. String parsing from DB only happens at boundaries/backfill.

The two `ALTER TABLE ADD COLUMN ... DEFAULT ...` migrations are metadata-only in SQLite for constant defaults, but they still take the writer lane and must run in the DOS-7 cutover window under the W1-B fence. Current write-fence primitives capture and recheck `schema_epoch` before stale `intelligence.json` writes (`src-tauri/src/intelligence/write_fence.rs:1-23`, `:67-109`).

No new index is required in v1.4.0. Default reads continue to use DOS-7's planned `(subject_ref, claim_state, surfacing_state, claim_type)` shape (`.docs/plans/wave-W3/DOS-7-plan.md:61-65`). Add a sensitivity index only when v1.4.1 render gates prove a query need.

## 6. Coding standards

Services-only mutations hold: Transform abilities produce proposals; only `services/claims.rs::commit_claim` writes `intelligence_claims`, matching ADR-0113's propose/commit split (`.docs/decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md:92-101`) and `CLAUDE.md:16`.

Intelligence Loop check (`CLAUDE.md:7-14`): temporal scope feeds future freshness and supersession; sensitivity feeds future render ceilings; claim type feeds dedup, commit policy, rendering, feedback, and source/agent trust. No new signal emission is required merely for adding the columns, but claim writes still go through DOS-7 invalidation in the same transaction.

No direct `Utc::now()` or `thread_rng()` in `abilities/claims.rs` or `services/claims.rs`. Dynamic times come from the `ClaimProposal`, `SourceAttribution`, meeting event time, or `ServiceContext.clock` in the writer. Tests use fixed RFC3339 timestamps. Fixtures use generic accounts/people only per `CLAUDE.md:18`.

Clippy budget: no `unwrap()` in parsing/registry lookup paths; parsing errors should carry typed context. `schemars` use stays feature-gated so non-MCP builds do not require optional deps.

## 7. Integration with parallel wave-mates

W3-C / DOS-7 is the hard dependency. DOS-7 creates `intelligence_claims` and `services/claims.rs`; DOS-300 adds or verifies the two anatomy columns and then wires metadata lookup into `commit_claim`. The current DOS-7 L0 plan already lists `temporal_scope` and `sensitivity` in the base schema (`.docs/plans/wave-W3/DOS-7-plan.md:23`) and says W3-H owns registry enforcement if W3-C includes the columns (`:85`). Before coding, resolve whether W3-C removes those two columns from its create-table or DOS-300 treats the schema piece as already landed and owns validation/default semantics only.

W3-A owns `src-tauri/src/abilities/registry.rs` and the `abilities/` module creation. W3-B owns `src-tauri/src/abilities/provenance/`; DOS-300 should not edit provenance except through typed claim proposals. W3-F owns `thread_id`; W3-G owns `source_asof`; DOS-300 reads both only to choose appropriate registry metadata and temporal scopes.

DOS-218 and DOS-219 are downstream pilots. Their Linear bodies require explicit `temporal_scope` and `sensitivity` on every emitted claim, not implicit DB defaults. DOS-219 specifically tests `PointInTime` for meeting-event claims and `State` for elevated renewal risk. W3-H provides the registry and helpers; pilots own per-claim assignment.

Migration numbering is resolved in the W3 integration train. Current tail is version 125; DOS-7 tentatively uses the next version; W3-F and W3-H both need additive claim-column migrations; the wave doc requires a single integration commit to resolve migration ordering (`.docs/plans/v1.4.0-waves.md:524-532`).

## 8. Failure modes + rollback

If DOS-300 runs before DOS-7, the ALTER fails because `intelligence_claims` does not exist. That is correct; the PR must be sequenced after W3-C. If one column exists and the other does not, a two-column migration is unsafe under the current duplicate-column tolerance, so use one-column migrations or a Rust preflight hook.

If registry validation rejects a claim type during backfill, fail closed and quarantine/report the unknown type rather than inserting an unregistered row. If production code attempts an unknown type, it should fail before DB write through `ClaimType::try_from_db_str` or the typed `ClaimType` API.

Rollback before successful migration version recording is the existing migration backup path (`src-tauri/src/migrations.rs:1019-1026`). After success, rollback is a forward fix: keep the columns, tighten registry metadata, and re-run affected backfill/commit tests. Dropping columns is not worth the risk on the load-bearing claim table.

W1-B universal write fence is honored through DOS-7's cutover: schema epoch bump, drain, backfill/ALTER, reconcile, requeue. Stale legacy file writes are rejected by `FenceCycle` and `fenced_write_intelligence_json` (`src-tauri/src/intelligence/write_fence.rs:67-109`, `:112-120`).

## 9. Test evidence to be produced

Rust tests: `temporal_scope_json_roundtrip_point_in_time`, `temporal_scope_json_roundtrip_state`, `temporal_scope_json_roundtrip_trend`, `temporal_scope_db_default_state_parses`, `claim_sensitivity_json_roundtrip_all_variants`, `claim_sensitivity_db_values_are_lowercase`, `claim_type_registry_has_unique_names`, `claim_type_registry_covers_all_claim_type_variants`, `claim_type_registry_rejects_global_subject_in_spine`, `claim_type_registry_exhaustiveness` (integration target required by the ticket), `commit_claim_substitutes_registry_default_sensitivity_when_omitted`, `commit_claim_substitutes_registry_default_temporal_scope_when_omitted`, `commit_claim_preserves_explicit_point_in_time_scope`, and `commit_claim_rejects_unknown_claim_type`.

Migration tests: `dos300_temporal_scope_column_default_state`, `dos300_sensitivity_column_default_internal`, and `dos300_column_migrations_are_single_alter_statements` to guard against the existing duplicate-column/multi-ALTER footgun.

Pilot contract tests handed to W5: DOS-218 fixture asserts every emitted claim has explicit scope/sensitivity; DOS-219 fixture asserts at least five claim types include correct `PointInTime` vs `State` assignment, plus an internal-only talking point never becomes public by default.

Wave merge-gate artifact: include `cargo test --test claim_type_registry_exhaustiveness`, `cargo clippy -- -D warnings && cargo test && pnpm tsc --noEmit` (`CLAUDE.md:20-24`), and the W3 Suite S/P/E notes. Suite S contribution: sensitivity defaults and subject-type rejection. Suite P contribution: metadata lookup is not measurable against claim write cost; no new read index. Suite E contribution: bundles 1 and 5 can assert stable registered claim types for entity context and meeting prep.

## 10. Open questions

1. DOS-7 schema ownership conflict: should W3-C remove `temporal_scope` and `sensitivity` from its create-table so DOS-300 adds them, or should DOS-300 accept W3-C's base columns and focus on registry/default enforcement?
2. ADR-0125 says `ClaimTypeMetadata.default_temporal_scope: TemporalScope`, but `PointInTime` and `Trend` need claim-specific timestamps/windows and cannot be safely synthesized from a const registry. Should the implementation keep exact ADR shape and require explicit dynamic scopes, or amend metadata to a default policy such as `PointInTimeFromObservedAt` / `TrendFromEvidenceWindow`?
3. Are the proposed initial registry names acceptable canonical strings, or should DOS-218/DOS-219 owners provide final claim-type names before DOS-300 lands?
4. Should `ClaimSensitivity::Confidential` be used for stakeholder personal context in the initial registry, or should all pilot defaults remain `Internal` until DOS-214 render policy defines surface ceilings?
