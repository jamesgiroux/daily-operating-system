//! Week workflow definition
//!
//! Three-phase pattern: prepare_week.py → Claude /week → deliver_week.py
//! Runs Monday early AM. Generates week overview, action summary, focus blocks.

use crate::types::WorkflowId;
use crate::workflow::Workflow;

/// Week workflow configuration
pub const WEEK_WORKFLOW: Workflow = Workflow {
    id: WorkflowId::Week,
    prepare_script: "prepare_week.py",
    claude_command: "/week",
    deliver_script: "deliver_week.py",
};
