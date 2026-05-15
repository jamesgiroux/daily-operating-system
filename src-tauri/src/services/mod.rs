use std::sync::Arc;

use crate::state::AppState;

pub mod accounts;
pub mod actions;
pub mod claims;
pub mod claims_backfill;
pub mod commitment_bridge;
pub mod comparator_thresholds;
pub mod compositions;
pub mod context;
pub mod dashboard;
pub mod derived_state;
pub mod emails;
pub mod entities;
pub mod entity_context;
pub mod entity_linking;
pub mod external_replay;
pub mod fail_improve;
pub mod feedback;
pub mod health_debouncer;
pub mod hygiene;
pub mod integrations;
pub mod intelligence;
pub mod invalidation_jobs;
pub mod linear;
pub mod meetings;
pub mod mutations;
pub mod people;
pub mod projects;
pub mod reports;
pub mod sensitivity;
pub mod settings;
pub mod signals;
pub mod source_asof_backfill;
pub mod stakeholder_writer;
pub mod success_plans;
pub mod surface_pairing;
pub mod temporal;
pub mod threads;
pub mod trust_extraction;
pub mod user_entity;
pub mod versioning;

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
