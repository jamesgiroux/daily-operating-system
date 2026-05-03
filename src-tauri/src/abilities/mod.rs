//! Ability substrate modules.

pub mod claims;
pub mod provenance;
pub mod registry;

pub use claims::{
    metadata_for_claim_type, metadata_for_name, subject_kind_is_canonical_for,
    CanonicalSubjectType, ClaimType, ClaimTypeMetadata, UnknownClaimTypeError,
    CLAIM_TYPE_REGISTRY,
};
pub use provenance::*;
pub use registry::{
    AbilityCategory, AbilityContext, AbilityDescriptor, AbilityError, AbilityErrorKind,
    AbilityRegistry, AbilityResult, Actor, ConfirmationToken,
};
