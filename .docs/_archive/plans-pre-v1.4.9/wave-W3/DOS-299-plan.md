# Implementation Plan: DOS-299

## Revision history

- v1 (2026-05-01) - initial L0 draft.

## 1. Contract restated

DOS-299 promotes the already-shipped `ItemSource.sourced_at` evidence timestamp into first-class claim provenance and trust inputs. It is not a new provenance model. Load-bearing ticket lines: "promote the existing `ItemSource.sourced_at`", "Backfill is a non-goal. That was wrong.", "`unknown_timestamp_penalty` - NOT a separate factor.", and "Trust Compiler scores using 5 canonical inputs + 1 composer-local helper."

Current code confirms the premise: `ItemSource` already has `source`, `confidence`, `sourced_at`, and `reference` at `src-tauri/src/intelligence/io.rs:29-41`; missing source confidence defaults to `0.5` at `src-tauri/src/intelligence/io.rs:1227-1230`; reconciliation prompts require `itemSource.sourcedAt` at `src-tauri/src/intelligence/dimension_prompts.rs:753-764`; runtime suppression already compares `sourced_at > dismissed_at` at `src-tauri/src/db/intelligence_feedback.rs:64-99` and is called from `src-tauri/src/intel_queue.rs:2375-2402`.

The 2026-04-24 ADR amendments apply in full: ADR-0105 §A-D makes `source_asof` must-populate-when-knowable; §E makes backfill mandatory; §F adds semantic bounds and operator-visible warnings/quarantine; ADR-0107 adds `DataSource::LegacyUnattributed` at confidence `0.5`; ADR-0114 R1.4 freezes five canonical scoring factors. Linear comment risk also applies: the compiler must distinguish evidence time, observation/ingestion time, and claim row creation time.

## 2. Approach

Extend W3-B's `src-tauri/src/abilities/provenance/` once DOS-211 lands. Add a source-time helper module under that directory, e.g. `source_time.rs`, containing `parse_source_timestamp`, `SourceTimestampStatus::{Accepted, Implausible, Malformed, Missing}`, `SourceTimestampImplausibleReason`, and builder glue that sets `SourceAttribution.source_asof`. `SourceAttribution` itself comes from DOS-211; ADR-0105 defines the field at `.docs/decisions/0105-provenance-as-first-class-output.md:165-170`.

Builder algorithm: for each source, try `itemSource.sourcedAt`; then cited Glean document `lastModified` / `createdAt`; then source-class inference where already structured; otherwise set `source_asof = None` and append `ProvenanceWarning::SourceTimestampUnknown { source_index, fallback }`. The current Glean path stamps synthetic manifest/document times at `src-tauri/src/intelligence/glean_provider.rs:304`, `:500`, `:638`, and fact-promotion `observed_at` at `:1394-1413`; DOS-299 normalizes these call sites to carry upstream document timestamps into provenance where available, while preserving ingestion `observed_at` as ingestion time.

Backfill lands with DOS-7's consolidation migration. DOS-7 currently plans `intelligence_claims.source_asof` in its base table and migration slot after `src-tauri/src/migrations.rs:588-590`; DOS-299 supplies the Rust backfill routine and audit/quarantine SQL for source-time semantics. For each migrated legacy claim, inspect the source `IntelligenceJson` item: parseable and within bounds writes `source_asof`; parseable-but-implausible writes `source_asof` plus `SourceTimestampImplausible`; malformed writes a quarantine row and blocks migration completion; missing `itemSource` writes `data_source = LegacyUnattributed`, `confidence = 0.5`, `source_asof = NULL`, and `SourceTimestampUnknown`.

Freshness input prep changes in the Trust Compiler extractor owned by DOS-5/W3-I after DOS-299 merges. Extend `FreshnessContext` with `timestamp_known: bool`, compute freshness age from `source_asof` when accepted, otherwise from `observed_at + STALENESS_PADDING`, otherwise `MAX_AGE`, and apply `unknown_timestamp_penalty` inside `freshness_weight`. Add a factor-count assertion that the Trust Compiler passes exactly five canonical factors plus composer-local `subject_fit_confidence`.

End-state alignment: this preserves the legacy suppression/timeliness signal while making evidence age queryable on `intelligence_claims`. It forecloses treating row creation time or fresh ingestion as fresh evidence.

## 3. Key decisions

Fallback chain is fixed: `itemSource.sourcedAt` -> Glean document `lastModified` / `createdAt` -> source-class structured date -> `SourceTimestampUnknown { source_index, fallback }`. The user-facing assignment explicitly requires the shorter builder chain of itemSource, Glean document, then warning; source-class inference remains ADR-0105-compatible but must not delay those required paths.

Quarantine table shape: create a DOS-299-owned table in DOS-7's migration, e.g. `source_asof_backfill_quarantine(id TEXT PRIMARY KEY, claim_source TEXT NOT NULL, legacy_entity_id TEXT NOT NULL, legacy_field_path TEXT NOT NULL, legacy_item_hash TEXT, raw_sourced_at TEXT, reason TEXT NOT NULL, created_at TEXT NOT NULL, remediation_status TEXT NOT NULL DEFAULT 'pending')`. Do not store claim text or raw source excerpts. The migration gate requires zero pending malformed rows; implausible-but-parseable rows are loaded with warnings and included in audit counts.

Bounds semantics split parsing from scoring. `Malformed` covers unparseable, timezone-less, `< 2015-01-01`, and `> now + 5 years`; it quarantines and halts. `Implausible` covers before entity origin and `> now + 30 days`; it lifts the parsed timestamp for traceability and emits `SourceTimestampImplausible`, but trust input prep must treat it as unknown/observed-time fallback rather than granting freshness.

Config defaults live in the scoring config owned by ADR-0114 R1.6: app-support `config/scoring.toml` with compiled defaults when no file exists. DOS-299 adds `staleness_padding_days = 30` and `unknown_timestamp_penalty = 0.6`; no hot reload in v1.4.0.

Line-number drift is acknowledged. The ticket cites older line numbers for `effective_confidence`, `dimension_prompts`, `is_suppressed`, and `glean_provider`; implementation should use current lines named in §1-2 and keep a grep audit artifact.

## 4. Security

Timestamp parsing is untrusted input because LLM output and legacy JSON can contain malformed or hostile strings. Parse strictly as RFC3339/ISO8601 with timezone, bound with ADR-0105 §F, and never let malformed strings reach SQL as dynamic SQL. Quarantine rows store identifiers and error classes only, not customer text.

Cross-tenant/subject bleed risk is in backfill joins. The backfill must attach source timestamps only to the legacy item being migrated into that claim, using DOS-7's canonical subject/field/item hash, not fuzzy text matching across entities.

`LegacyUnattributed` is backfill-only per ADR-0107. Add or reuse a CI lint so new production writers outside migration/backfill code cannot write `DataSource::LegacyUnattributed`; otherwise new claims could launder unknown-source evidence into the substrate.

Glean document timestamps are metadata, not authorization proof. DOS-299 may copy timestamps from documents already selected/cited by the existing Glean flow, but must not broaden Glean reads or log inaccessible document content.

## 5. Performance

Hot paths touched: ProvenanceBuilder finalization, claim backfill, and trust input extraction. Builder work is O(number of sources) with only local parsing and no network calls. Cache parsed timestamps inside the builder per source to avoid repeated parse work across field attributions.

The backfill streams `IntelligenceJson` rows and migrated claim candidates; it must not materialize every entity blob in memory. Index the quarantine/audit lookup by `reason` and `remediation_status`; the steady-state `intelligence_claims.source_asof` read path should use DOS-7's default claim indexes rather than a standalone freshness index unless DOS-5 profiling shows one is needed.

Trust prep adds one boolean and one multiplication when timestamp is unknown. That is negligible relative to DB extraction and factor aggregation. The expensive part is migration parsing; target audit output includes total legacy items, with-itemSource count, accepted, implausible, malformed quarantined, missing itemSource, and coverage percentage. Coverage must be >=95% for legacy claims with populated `ItemSource.sourced_at`.

## 6. Coding standards

Services-only mutations: migration/backfill writes route through DOS-7's migration/cutover path and `services/claims.rs` where applicable; builder code constructs provenance but does not write DB rows. Do not edit `src-tauri/src/services/context.rs` or W2 provider seams.

Intelligence Loop check: `source_asof` feeds trust freshness and suppression/tombstone resurrection, not direct signals; it can influence health only through Trust Compiler consumers; it belongs in prep/intel context only as provenance metadata; briefing callouts should consume claim/trust state, not raw timestamp heuristics; user feedback remains claim-level.

No new direct `Utc::now()` in services or abilities. Use `ServiceContext.clock` for observed/created timestamps and pass `now` explicitly into timestamp-bound checks. Existing `Utc::now()` stamps in source-attribution write paths must be audited: `src-tauri/src/intelligence/io.rs:1702` and `:1766` are real `sourcedAt` stamps; Glean manifest/observed-at stamps at `glean_provider.rs:304`, `:500`, `:638`, `:1395` need normalization or documented ingestion-only status.

No customer data in fixtures. Tests use generic entities and timestamps. Clippy budget remains `cargo clippy -- -D warnings`.

## 7. Integration with parallel wave-mates

W3-B/DOS-211 owns `src-tauri/src/abilities/provenance/` and the `SourceAttribution` shape. DOS-299 extends that module after W3-B lands; if both branch concurrently, W3-B creates the envelope and DOS-299 owns only source-time helpers and warning population.

W3-C/DOS-7 owns `intelligence_claims`, migration numbering, and the atomic consolidation migration. DOS-299 must land together with DOS-7 or be merged into its migration hook because the source-time backfill is part of consolidation, not a later cleanup.

W3-I/DOS-5 consumes `FreshnessContext { timestamp_known }`, five canonical factors, and the factor-count assertion. DOS-299 defines the input semantics; DOS-5 owns score computation. W3-H/DOS-300 adds `temporal_scope`; freshness should not infer claim temporality beyond that registry. DOS-218/DOS-219 fixtures must include one populated `source_asof` and one `SourceTimestampUnknown`.

## 8. Failure modes + rollback

If malformed timestamps are found, the backfill writes quarantine rows and the migration halts before recording completion. Operators remediate or exclude via the quarantine workflow, then rerun. Implausible rows do not halt but appear in audit counts and provenance warnings.

If backfill coverage is below 95% for legacy claims with populated `ItemSource.sourced_at`, stop and escalate rather than relaxing silently; the wave plan explicitly calls out this risk at `.docs/plans/v1.4.0-waves.md:742`.

If builder fallback fails in live enrichment, it still writes `SourceTimestampUnknown` and uses conservative freshness fallback; it must not drop otherwise valid claims. If DOS-5 reads older rows without `timestamp_known`, default to false and apply the penalty.

Rollback follows DOS-7's migration rollback posture: restore from pre-migration backup before completion; after completion, do not delete legacy JSON until projection parity passes. W1-B write fence is honored by running under the DOS-7 drain/schema-epoch cutover, so stale workers cannot overwrite backfilled claim state.

## 9. Test evidence to be produced

Unit tests: `parse_source_timestamp_accepts_rfc3339_with_timezone`, `parse_source_timestamp_rejects_timezone_less`, `parse_source_timestamp_rejects_before_2015`, `parse_source_timestamp_rejects_far_future`, `parse_source_timestamp_marks_near_future_implausible`, `parse_source_timestamp_marks_before_entity_origin_implausible`.

Builder/backfill tests: `provenance_builder_lifts_item_source_sourced_at`, `provenance_builder_falls_back_to_glean_last_modified`, `provenance_builder_emits_source_timestamp_unknown`, `backfill_lifts_item_source_sourced_at_to_source_asof`, `backfill_implausible_lifts_and_warns`, `backfill_malformed_quarantines_and_halts`, `backfill_missing_item_source_uses_legacy_unattributed_confidence_05`, `backfill_source_asof_coverage_at_least_95_percent`.

Trust/suppression tests: `freshness_context_timestamp_unknown_applies_penalty`, `freshness_context_implausible_uses_observed_fallback`, `trust_compiler_uses_five_canonical_factors`, `item_level_confidence_parity_legacy_twenty_items`, `is_suppressed_newer_source_asof_resurrects_existing_risk_or_win`.

Wave merge-gate artifact: attach the source-time audit grep, migration audit counts, quarantine count, coverage percentage, and commands `cargo clippy -- -D warnings && cargo test && pnpm tsc --noEmit`. Suite S contribution is quarantine/PII/lint evidence; Suite P is streaming backfill plus builder O(source count); Suite E is fixture coverage for known and unknown source timestamps.

## 10. Open questions

1. Confirm exact owner for the Rust backfill hook: DOS-299 as a module called by DOS-7's migration, or merged directly into DOS-7's cutover code.
2. Confirm Glean metadata availability: current `glean_provider.rs` mostly preserves synthetic `SourceManifestEntry.modified_at`; if Glean cited documents do not expose `lastModified` / `createdAt` in the existing response path, does DOS-299 add that capture or emit `SourceTimestampUnknown` until DOS-218/DOS-219 fixtures exercise it?
3. Confirm whether `SourceTimestampImplausible` freshness fallback should be encoded as `timestamp_known = false` or as a richer freshness input status. The factor count stays five either way.
