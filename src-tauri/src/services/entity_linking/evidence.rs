//! Evidence JSON builders for entity_linking_evaluations.
//!
//! Each builder produces the evidence blob stored alongside a rule outcome.
//! Load-bearing fields per eng review (plan-eng-review comment, point 11):
//!   matched_text, rejected_candidates, parent_email_id, rule_input_fingerprint
//! These are present as JSON keys in v1; promote to typed columns if audit
//! queries get slow.

// TODO(Lane-C): implement evidence builders.
