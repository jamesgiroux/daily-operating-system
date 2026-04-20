pub mod p1_user_override;
pub mod p2_thread_inheritance;
pub mod p3_series_inheritance;
pub mod p4a_one_on_one;
pub mod p4b_group_shared;
pub mod p4c_sender_domain;
pub mod p5_title_evidence;
pub mod p6_internal_internal;
pub mod p7_internal_external;
pub mod p8_external_external;
pub mod p9_multi_account;
pub mod p10_shared_inbox;
pub mod p11_fallback;

use super::phases::Rule;
use super::types::Candidate;

/// Build the ordered rule list for Phase 3.
///
/// Rules run in this fixed order; the first Matched terminates.
/// P5TitleEvidence is constructed with knowledge of whether P4
/// domain evidence fired (passed from the dispatcher).
pub fn ordered_rules(p4_entity_id: Option<String>) -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(p1_user_override::P1UserOverride),
        Box::new(p2_thread_inheritance::P2ThreadInheritance),
        Box::new(p3_series_inheritance::P3SeriesInheritance),
        Box::new(p4a_one_on_one::P4aOneOnOne),
        Box::new(p4b_group_shared::P4bGroupShared),
        Box::new(p4c_sender_domain::P4cSenderDomain),
        Box::new(p5_title_evidence::P5TitleEvidence { p4_entity_id }),
        Box::new(p6_internal_internal::P6InternalInternal),
        Box::new(p7_internal_external::P7InternalExternal),
        Box::new(p8_external_external::P8ExternalExternal),
        Box::new(p9_multi_account::P9MultiAccount),
        Box::new(p10_shared_inbox::P10SharedInbox),
        Box::new(p11_fallback::P11Fallback),
    ]
}

/// True when the candidate represents a "no primary" sentinel
/// (P8 or P11 return empty entity_id to signal this).
pub fn is_no_primary_sentinel(c: &Candidate) -> bool {
    c.entity.entity_id.is_empty()
}
