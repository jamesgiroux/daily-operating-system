#![cfg(feature = "release-gate")]

#[path = "harness/mod.rs"]
mod harness;
#[path = "edge_cases/support.rs"]
mod support;

#[path = "edge_cases/dos289_stale_current_regression.rs"]
mod dos289_stale_current_regression;
#[path = "edge_cases/dos290_cross_surface_regression.rs"]
mod dos290_cross_surface_regression;
#[path = "edge_cases/dos291_ambiguous_identity_regression.rs"]
mod dos291_ambiguous_identity_regression;
#[path = "edge_cases/dos292_source_lifecycle_regression.rs"]
mod dos292_source_lifecycle_regression;
#[path = "edge_cases/dos293_sync_refresh_regression.rs"]
mod dos293_sync_refresh_regression;

#[path = "edge_cases/unit/dos282_derive_commitment_id_determinism.rs"]
mod dos282_derive_commitment_id_determinism;
#[path = "edge_cases/unit/dos282_owner_resolution.rs"]
mod dos282_owner_resolution;
#[path = "edge_cases/unit/dos282_claim_canonicalization.rs"]
mod dos282_claim_canonicalization;
#[path = "edge_cases/unit/dos282_trust_factor_math.rs"]
mod dos282_trust_factor_math;
#[path = "edge_cases/unit/dos282_tombstone_pregate.rs"]
mod dos282_tombstone_pregate;
#[path = "edge_cases/unit/dos282_provenance_field_attribution.rs"]
mod dos282_provenance_field_attribution;
#[path = "edge_cases/unit/dos282_ability_category_enforcement.rs"]
mod dos282_ability_category_enforcement;
#[path = "edge_cases/unit/dos282_source_taxonomy_parsing.rs"]
mod dos282_source_taxonomy_parsing;
#[path = "edge_cases/unit/dos282_stale_claim_source_age_vs_index_age.rs"]
mod dos282_stale_claim_source_age_vs_index_age;
#[path = "edge_cases/unit/dos282_duplicate_open_loop_commitment_collapse.rs"]
mod dos282_duplicate_open_loop_commitment_collapse;
