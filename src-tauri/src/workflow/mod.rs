//! Workflow definitions
//!
//! Each workflow defines:
//! - Phase 1 prepare script
//! - Phase 2 Claude command
//! - Phase 3 deliver script

pub mod archive;
pub mod reconcile;
pub mod today;
pub mod week;

use crate::types::WorkflowId;

/// Workflow configuration
#[derive(Debug, Clone, Copy)]
pub struct Workflow {
    pub id: WorkflowId,
    pub prepare_script: &'static str,
    pub claude_command: &'static str,
    pub deliver_script: &'static str,
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
            prepare_script: "prepare_archive.py",
            claude_command: "/archive",
            deliver_script: "deliver_archive.py",
        }
    }

    /// Get the prepare script name
    pub fn prepare_script(&self) -> &str {
        self.prepare_script
    }

    /// Get the Claude command to run
    pub fn claude_command(&self) -> &str {
        self.claude_command
    }

    /// Get the deliver script name
    pub fn deliver_script(&self) -> &str {
        self.deliver_script
    }

    /// Get the workflow ID
    pub fn id(&self) -> WorkflowId {
        self.id
    }
}
