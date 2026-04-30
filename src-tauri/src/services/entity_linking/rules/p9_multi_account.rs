//! P9 — Multiple conflicting P4b/P4c candidates → primary = none, all related.
//! This rule is checked by the phase dispatcher after P4b/P4c produce >1 candidate.
//! It does not run inline; phases.rs handles the multi-candidate case directly.

use crate::db::ActionDb;
use super::super::types::{LinkingContext, RuleOutcome};

pub struct P9MultiAccount;

impl super::super::phases::Rule for P9MultiAccount {
    fn id(&self) -> &'static str { "P9" }

    fn evaluate(
        &self,
        _service_ctx: &crate::services::context::ServiceContext<'_>,
        _ctx: &LinkingContext,
        _db: &ActionDb,
    ) -> Result<RuleOutcome, String> {
        // Always skips inline — the dispatcher handles multi-account detection.
        Ok(RuleOutcome::Skip)
    }
}
