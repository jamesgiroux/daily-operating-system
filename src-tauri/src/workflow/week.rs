//! Week workflow definition
//!
//! Per-operation pipeline (I94, matches today pattern):
//! Rust prepare_week → Rust deliver_week → enrich_week (AI, fault-tolerant)
//! Runs Monday early AM. Generates week overview, action summary, focus blocks.

use crate::types::WorkflowId;
use crate::workflow::Workflow;

/// Week workflow configuration
pub const WEEK_WORKFLOW: Workflow = Workflow {
    id: WorkflowId::Week,
    claude_command: "/week",
};
