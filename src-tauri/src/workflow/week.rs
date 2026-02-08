//! Week workflow definition
//!
//! Three-phase pattern (ADR-0049 Rust-native):
//! Rust prepare_week → Claude /week → Rust deliver_week
//! Runs Monday early AM. Generates week overview, action summary, focus blocks.

use crate::types::WorkflowId;
use crate::workflow::Workflow;

/// Week workflow configuration
pub const WEEK_WORKFLOW: Workflow = Workflow {
    id: WorkflowId::Week,
    claude_command: "/week",
};
