//! Phase dispatcher and Rule trait.
//!
//! The four-phase engine:
//!   Phase 1 — Suppress      (are we allowed to link this owner at all?)
//!   Phase 2 — Record facts  (create person stubs, write meeting_attendees)
//!   Phase 3 — Select primary (deterministic rule table — first Matched wins)
//!   Phase 4 — Cascade       (related chips, stakeholder queue, tier mapping)
//!
//! Rules are structs that implement `Rule`. The dispatcher in Phase 3 iterates
//! the rule list in fixed order and stops at the first `RuleOutcome::Matched`.
//! Every outcome (Matched or Skip) is recorded in entity_linking_evaluations.

use crate::db::ActionDb;

use super::types::{LinkingContext, RuleOutcome};

/// A single deterministic linking rule.
///
/// Implement this for each Px rule (P1UserOverride, P2ThreadInheritance, …).
/// The `evaluate` method must be read-only with respect to the DB — no writes.
/// Writes happen in the phase dispatcher after the winning rule is identified.
pub trait Rule: Send + Sync {
    /// Stable rule identifier stored in entity_linking_evaluations.rule_id.
    /// Use the canonical name from the spec, e.g. "P4a", "P2".
    fn id(&self) -> &'static str;

    fn evaluate(&self, ctx: &LinkingContext, db: &ActionDb) -> RuleOutcome;
}

/// TODO(Lane-C): Build the ordered rule list and run the four-phase engine.
pub fn run_phases(
    _ctx: &LinkingContext,
    _db: &ActionDb,
) -> Result<super::types::LinkOutcome, String> {
    unimplemented!("Lane C: phase dispatcher")
}
