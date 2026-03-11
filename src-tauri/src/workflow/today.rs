//! Today workflow implementation
//!
//! Per-operation pipeline (ADR-0042, ADR-0049):
//! 1. Rust-native prepare — fetch calendar/emails, classify, write directive
//! 2. Rust-native mechanical delivery — schedule, actions, preps, emails
//! 3. AI enrichment — Claude Code enriches emails + briefing narrative
//!
//! I513: sync_actions_to_db removed — DB is the source of truth for actions.
//! The old JSON→DB sync direction is no longer needed.

use crate::types::WorkflowId;
use crate::workflow::Workflow;

/// The /today workflow configuration
pub const TODAY_WORKFLOW: Workflow = Workflow {
    id: WorkflowId::Today,
    claude_command: "/today",
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_today_workflow_config() {
        assert_eq!(TODAY_WORKFLOW.claude_command, "/today");
    }
}
