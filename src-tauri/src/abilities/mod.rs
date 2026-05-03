//! Ability substrate modules.

pub mod claims;
pub mod feedback;
pub mod provenance;
pub mod registry;
pub mod threads;

pub use claims::{
    metadata_for_claim_type, metadata_for_name, subject_kind_is_canonical_for,
    CanonicalSubjectType, ClaimType, ClaimTypeMetadata, UnknownClaimTypeError,
    CLAIM_TYPE_REGISTRY,
};
pub use feedback::{
    feedback_semantics, transition_for_feedback, ClaimFeedbackMetadata, ClaimRenderPolicy,
    ClaimVerificationState, FeedbackAction, RepairAction, TrustEffect,
};
pub use provenance::*;
pub use registry::{
    AbilityCategory, AbilityContext, AbilityDescriptor, AbilityError, AbilityErrorKind,
    AbilityRegistry, AbilityResult, Actor, ConfirmationToken,
};
pub use threads::{create_thread, ThreadMetadata};
