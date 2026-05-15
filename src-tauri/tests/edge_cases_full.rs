#![cfg(feature = "release-gate")]

#[path = "harness/mod.rs"]
mod harness;
#[path = "edge_cases/support.rs"]
mod support;

#[path = "edge_cases/integration/calendar_to_briefing_integration.rs"]
mod calendar_to_briefing_integration;
#[path = "edge_cases/integration/account_meeting_claim_parity_integration.rs"]
mod account_meeting_claim_parity_integration;
#[path = "edge_cases/integration/project_meeting_claim_parity_integration.rs"]
mod project_meeting_claim_parity_integration;
#[path = "edge_cases/integration/transcript_to_commitment_to_work_integration.rs"]
mod transcript_to_commitment_to_work_integration;
#[path = "edge_cases/integration/user_correction_survives_enrichment_rerun_integration.rs"]
mod user_correction_survives_enrichment_rerun_integration;
#[path = "edge_cases/integration/revoked_source_masking_integration.rs"]
mod revoked_source_masking_integration;
#[path = "edge_cases/integration/glean_unavailable_fallback_integration.rs"]
mod glean_unavailable_fallback_integration;
#[path = "edge_cases/integration/activity_log_event_population_integration.rs"]
mod activity_log_event_population_integration;
#[path = "edge_cases/integration/lint_seeded_corpus_integration.rs"]
mod lint_seeded_corpus_integration;
#[path = "edge_cases/integration/old_path_new_ability_parity_integration.rs"]
mod old_path_new_ability_parity_integration;
