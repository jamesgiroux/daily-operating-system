//! Today workflow implementation
//!
//! Three-phase workflow for daily briefing:
//! 1. prepare_today.py - Fetch calendar, emails, generate directive
//! 2. /today - Claude enriches with AI synthesis
//! 3. deliver_today.py - Write final files to _today/

use crate::types::WorkflowId;
use crate::workflow::Workflow;

/// The /today workflow configuration
pub const TODAY_WORKFLOW: Workflow = Workflow {
    id: WorkflowId::Today,
    prepare_script: "prepare_today.py",
    claude_command: "/today",
    deliver_script: "deliver_today.py",
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_today_workflow_config() {
        assert_eq!(TODAY_WORKFLOW.prepare_script, "prepare_today.py");
        assert_eq!(TODAY_WORKFLOW.claude_command, "/today");
        assert_eq!(TODAY_WORKFLOW.deliver_script, "deliver_today.py");
    }
}
