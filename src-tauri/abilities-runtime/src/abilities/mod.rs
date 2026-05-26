//! Ability runtime modules.

pub mod account_overview;
pub mod claim_receipt;
pub mod claims;
pub mod composition;
pub mod detect_risk_shift;
pub mod entity_intake;
pub mod extractors;
pub mod fallback_projection;
pub mod feedback;
pub mod get_daily_briefing;
pub mod get_daily_readiness;
pub mod get_entity_context;
pub mod get_entity_intelligence;
pub mod list_accounts;
pub mod list_open_loops;
pub mod list_pagination;
pub mod list_people;
pub mod list_projects;
pub mod markdown_preview;
pub mod portfolio_attention;
pub mod prepare_meeting;
pub mod provenance;
pub mod recommendations;
pub mod registry;
pub mod source_management_ledger;
pub mod temporal;
pub mod threads;
pub mod tracer;
pub mod trust;
pub mod workspace_graph;
pub mod workspace_place_document;

pub use claims::{
    metadata_for_claim_type, metadata_for_name, subject_kind_is_canonical_for, CanonicalStatus,
    CanonicalSubjectType, ClaimSentiment, ClaimType, ClaimTypeMetadata, EntityRef, LiteralKind,
    ObjectValue, Polarity, PredicateRef, QualifierSet, StructuredClaim, StructuredClaimStatus,
    UnknownClaimTypeError, CLAIM_TYPE_REGISTRY,
};
pub use fallback_projection::{
    project_composition_for_surface, register_custom_block_schema, AuditCategory, AuditIntent,
    CustomBlockSchema, DiagnosticKind, DiagnosticReason, EditRoute, EditRouteRefusalReason,
    FallbackProjectionContext, ProducerOutputInvalidReason, ProjectedBlock, ProjectedComposition,
    ProjectionDiagnostic, ProjectionError, SurfaceKind, DEFAULT_UNKNOWN_BLOCK_CAP,
    FALLBACK_BANNER_COPY,
};
pub use feedback::{
    feedback_semantics, transition_for_feedback, ClaimFeedbackMetadata, ClaimRenderPolicy,
    ClaimVerificationState, FeedbackAction, RepairAction, TrustEffect,
};
pub use provenance::*;
pub use registry::{
    close_schema_objects, validate_schema_closure, validate_schema_closure_for_ability,
    AbilityCategory, AbilityContext, AbilityDescriptor, AbilityError, AbilityErrorKind,
    AbilityPolicy, AbilityRegistry, AbilityResult, Actor, ActorKind, ComposesEntry,
    ConfirmationProof, SignalPolicy,
};
pub use temporal::*;
pub use threads::ThreadMetadata;
pub use tracer::{AbilityTracer, NoopAbilityTracer, SpanHandle, NOOP_ABILITY_TRACER};
