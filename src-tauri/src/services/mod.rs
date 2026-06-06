use std::sync::Arc;

use crate::state::AppState;

pub mod account_fact_claims;
pub mod accounts;
pub mod action_claims;
pub mod actions;
pub mod claim_feedback_propagation;
pub mod claim_files;
pub mod claim_receipt;
pub mod claim_review_queue;
pub mod claims;
pub mod claims_backfill;
pub mod commitment_bridge;
pub mod comparator_thresholds;
pub mod composition_layout;
pub mod composition_projection;
pub mod composition_render_orchestrator;
pub mod compositions;
pub mod context;
pub mod correction_artifacts;
pub mod correction_stickiness_eval;
pub mod dashboard;
pub mod derived_state;
pub mod emails;
pub mod enrichment_side_effects;
pub mod entities;
pub mod entity_archive_folders;
pub mod entity_context;
pub mod entity_intelligence;
pub mod entity_linking;
pub mod external_replay;
pub mod fail_improve;
pub mod feedback;
pub mod glean_finalization;
pub mod health_debouncer;
pub mod hygiene;
pub mod integrations;
pub mod intelligence;
pub mod invalidation_jobs;
pub mod linear;
pub mod linear_issue_signals;
pub mod markdown_preview;
pub mod mcp_v2;
pub mod meeting_prep_status;
pub mod meetings;
pub mod meetings_view;
pub mod meetings_writer;
pub mod mutations;
pub mod people;
pub mod projection_signing;
pub mod projects;
pub mod rebuild;
pub mod recommendations;
pub mod replica_refresh;
pub mod reports;
pub mod runtime_evidence_backfill;
pub mod sensitivity;
pub mod settings;
pub mod signals;
pub mod source_asof_backfill;
pub mod source_management_ledger;
pub mod stakeholder_writer;
pub mod success_plans;
pub mod surface_nonce;
pub mod surface_pairing;
pub mod surface_session_keychain;
pub mod temporal;
pub mod threads;
pub mod trust_extraction;
pub mod trust_recompute;
pub mod user_entity;
pub mod version_dispatcher;
pub mod versioning;
pub mod workspace_backfill;
pub mod workspace_ingestion;

#[cfg(test)]
mod tests;

/// Command-facing service boundary for mutation workflows.
///
/// Background processors may still use owned `ActionDb` handles, but should call
/// service-owned mutation functions instead of direct DB mutations.
#[derive(Clone)]
pub struct ServiceLayer {
    state: Arc<AppState>,
}

impl ServiceLayer {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    pub fn state(&self) -> &AppState {
        self.state.as_ref()
    }

    pub fn state_arc(&self) -> Arc<AppState> {
        self.state.clone()
    }
}
