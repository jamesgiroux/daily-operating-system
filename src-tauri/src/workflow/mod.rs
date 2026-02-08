//! Workflow definitions
//!
//! Each workflow defines its Claude command for AI enrichment.
//! Prepare and deliver phases are Rust-native (ADR-0049).

pub mod archive;
pub mod deliver;
pub mod impact_rollup;
pub mod operations;
pub mod reconcile;
pub mod today;
pub mod week;

use crate::types::WorkflowId;

/// Workflow configuration
#[derive(Debug, Clone, Copy)]
pub struct Workflow {
    pub id: WorkflowId,
    pub claude_command: &'static str,
}

impl Workflow {
    /// Create a workflow from its ID
    pub fn from_id(id: WorkflowId) -> Self {
        match id {
            WorkflowId::Today => today::TODAY_WORKFLOW,
            WorkflowId::Archive => Self::archive(),
            WorkflowId::Week => week::WEEK_WORKFLOW,
            // InboxBatch is handled directly by the executor, not via three-phase
            WorkflowId::InboxBatch => unreachable!("InboxBatch uses direct processor calls"),
        }
    }

    /// Archive workflow (simplified - just file moves, minimal AI)
    const fn archive() -> Self {
        Self {
            id: WorkflowId::Archive,
            claude_command: "/archive",
        }
    }

    /// Get the Claude command to run
    pub fn claude_command(&self) -> &str {
        self.claude_command
    }

    /// Get the workflow ID
    pub fn id(&self) -> WorkflowId {
        self.id
    }
}
