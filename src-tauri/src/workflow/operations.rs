//! Per-operation pipeline types (ADR-0042)
//!
//! Each atomic operation runs its own pipeline:
//! - Mechanical: prepare → deliver (instant, no AI)
//! - AI-enriched: prepare → enrich → deliver (background)

use serde::{Deserialize, Serialize};

/// An atomic operation in the today pipeline
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    /// Calendar → schedule.json (mechanical)
    Schedule,
    /// Actions → actions.json (mechanical)
    Actions,
    /// Meeting contexts → preps/*.json (mechanical)
    Preps,
    /// Emails → emails.json (AI-enriched, future)
    Emails,
    /// Overview narrative (AI-enriched, future)
    Briefing,
}

impl Operation {
    /// Whether this operation requires AI enrichment
    pub fn needs_ai(&self) -> bool {
        matches!(self, Operation::Emails | Operation::Briefing)
    }
}

impl std::fmt::Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operation::Schedule => write!(f, "schedule"),
            Operation::Actions => write!(f, "actions"),
            Operation::Preps => write!(f, "preps"),
            Operation::Emails => write!(f, "emails"),
            Operation::Briefing => write!(f, "briefing"),
        }
    }
}
