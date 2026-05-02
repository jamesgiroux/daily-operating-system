use std::sync::Arc;

use crate::state::AppState;

pub mod accounts;
pub mod actions;
pub mod claims;
pub mod commitment_bridge;
pub mod context;
pub mod entity_linking;
pub mod dashboard;
pub mod emails;
pub mod entities;
pub mod entity_context;
pub mod feedback;
pub mod health_debouncer;
pub mod hygiene;
pub mod integrations;
pub mod intelligence;
pub mod linear;
pub mod meetings;
pub mod mutations;
pub mod people;
pub mod projects;
pub mod reports;
pub mod settings;
pub mod signals;
pub mod success_plans;
pub mod user_entity;

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
