# W6-A DOS-282 Edge-Case Regression Coverage Matrix

Date: 2026-05-15

This is the locked W6-A coverage artifact from `.docs/plans/v1.4.1-waves/W6-A-L0-packet.md` §4.2. It maps each migrated ability path to an existing bundle, sibling-owned W6 bundle, or W6-A meta fixture slot.

## Fast Regression Target

`cargo test --test edge_cases_fast --features release-gate -- --test-threads 4`

Includes:

- `src-tauri/tests/edge_cases/dos289_stale_current_regression.rs`
- `src-tauri/tests/edge_cases/dos290_cross_surface_regression.rs`
- `src-tauri/tests/edge_cases/dos291_ambiguous_identity_regression.rs`
- `src-tauri/tests/edge_cases/dos292_source_lifecycle_regression.rs`
- `src-tauri/tests/edge_cases/dos293_sync_refresh_regression.rs`
- `src-tauri/tests/edge_cases/unit/dos282_derive_commitment_id_determinism.rs`
- `src-tauri/tests/edge_cases/unit/dos282_owner_resolution.rs`
- `src-tauri/tests/edge_cases/unit/dos282_claim_canonicalization.rs`
- `src-tauri/tests/edge_cases/unit/dos282_trust_factor_math.rs`
- `src-tauri/tests/edge_cases/unit/dos282_tombstone_pregate.rs`
- `src-tauri/tests/edge_cases/unit/dos282_provenance_field_attribution.rs`
- `src-tauri/tests/edge_cases/unit/dos282_ability_category_enforcement.rs`
- `src-tauri/tests/edge_cases/unit/dos282_source_taxonomy_parsing.rs`
- `src-tauri/tests/edge_cases/unit/dos282_stale_claim_source_age_vs_index_age.rs`
- `src-tauri/tests/edge_cases/unit/dos282_duplicate_open_loop_commitment_collapse.rs`

## Full Integration Target

`cargo test --test edge_cases_full --features release-gate -- --test-threads 4`

Includes:

- `src-tauri/tests/edge_cases/integration/calendar_to_briefing_integration.rs`
- `src-tauri/tests/edge_cases/integration/account_meeting_claim_parity_integration.rs`
- `src-tauri/tests/edge_cases/integration/project_meeting_claim_parity_integration.rs`
- `src-tauri/tests/edge_cases/integration/transcript_to_commitment_to_work_integration.rs`
- `src-tauri/tests/edge_cases/integration/user_correction_survives_enrichment_rerun_integration.rs`
- `src-tauri/tests/edge_cases/integration/revoked_source_masking_integration.rs`
- `src-tauri/tests/edge_cases/integration/glean_unavailable_fallback_integration.rs`
- `src-tauri/tests/edge_cases/integration/activity_log_event_population_integration.rs`
- `src-tauri/tests/edge_cases/integration/lint_seeded_corpus_integration.rs`
- `src-tauri/tests/edge_cases/integration/old_path_new_ability_parity_integration.rs`

## 5-Path Ability Fixture Matrix

| ability | happy | empty | stale | revoked-source | contradiction |
| --- | --- | --- | --- | --- | --- |
| `get_entity_context` | bundle-1 | W6-A-meta-1 (zero claims for subject) | bundle-11, bundle-14 (W6-B) | bundle-12, bundle-17 (W6-E) | bundle-2, bundle-6, bundle-14 (W6-B) |
| `prepare_meeting` | bundle-5, bundle-13 | W6-A-meta-2 (meeting with zero context) | bundle-11, bundle-14 (W6-B) | bundle-12, bundle-17 (W6-E) | bundle-14 (W6-B), bundle-15 (W6-C) |
| `get_daily_readiness` | W6-A-meta-3 (typical day) | W6-A-meta-4 (no meetings) | bundle-11, bundle-14 (W6-B) | bundle-12, bundle-17 (W6-E) | bundle-14 (W6-B) |
| `detect_risk_shift` | W6-A-meta-5 (degrading account) | N/A - happy path with zero signals doubles as empty | W6-A-meta-6 (stale champion silence) | bundle-12, bundle-17 (W6-E) | W6-A-meta-7 (contradicting risk signals) |
| `list_open_loops` / `extract_commitments` | bundle-9 | W6-A-meta-8 (empty workspace) | W6-A-meta-9 (commitments past TTL) | bundle-17 (W6-E) | W6-A-meta-10 (transcript vs email conflict) |

## Sibling Bundle Cross-Links

- bundle-14: W6-B / DOS-289 stale-current contradiction.
- bundle-15: W6-C / DOS-290 cross-surface consistency.
- bundle-16: W6-D / DOS-291 ambiguous identity.
- bundle-17: W6-E / DOS-292 source lifecycle and actor provenance.
- bundle-18: W6-F / DOS-293 sync refresh and concurrency.

## W6-A Meta Fixture Slots

The W6-A meta slots are reserved for minimum-viable harness fixtures under `src-tauri/tests/fixtures/W6-A-meta-N/`. They are named in the matrix so the acceptance artifact is stable even before release-gate discovery supports non-hyphenated bundle directories.
