//! `get_entity_intelligence` Read ability + `EntityIntelligenceEnvelope` DTO.
//!
//! Read-side composition over existing claim/proposal/open-loop substrate.
//! Per `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md` §5.1.
//!
//! Coexists with `get_entity_context`. Consumed by W2 entity-detail blocks
//! (Account/Project/Person Detail) and the W3 briefing ability.

pub mod contracts;
pub mod producer;

pub use contracts::{
    CandidateSetRef, ContextDepth, Cursor, CursorState, EmptyReason, EntityFact,
    EntityIntelligenceEnvelope, EntityIntelligenceInput, EntityKind, EnvelopeProvenance,
    EnvelopeProvenanceSource, EnvelopeSection, EnvelopeTrustSummary, ExclusionReason, Freshness,
    HealthStory, HealthStoryRow, InclusionReason, MetadataProposal, NormalizedSubject,
    OpenLoopWithReceipt, Paginated, ProvenanceRef, ReceiptTargetRef, RecordEntry,
    RelationshipEdge, RelationshipParticipant, RelationshipTruncation, RelationshipsBundle,
    SectionState, SubjectScope, ThreadSummary, Touchpoint, TouchpointBundle, TouchpointKind,
    ENVELOPE_SCHEMA_VERSION, ENVELOPE_SCHEMA_VERSION_V1, ENVELOPE_SCHEMA_VERSION_V2,
};

use dailyos_abilities_macro::ability;

use crate::abilities::{AbilityContext, AbilityResult};

#[ability(
    name = "get_entity_intelligence",
    category = Read,
    version = "0.1.0",
    schema_version = 2,
    allowed_actors = [User, Agent, System, SurfaceClient, McpClient],
    allowed_modes = [Live, Simulate, Evaluate],
    requires_confirmation = false,
    may_publish = false,
    required_scopes = ["read.entity_intelligence"],
    mcp_exposure = Invocable,
    composes = [
        { id = "get_entity_context", ability = "get_entity_context", optional = false },
        { id = "list_open_loops", ability = "list_open_loops", optional = false }
    ],
    experimental = false,
    signal_policy = { emits_on_output_change = [], coalesce = false }
)]
pub async fn get_entity_intelligence(
    ctx: &AbilityContext<'_>,
    input: EntityIntelligenceInput,
) -> AbilityResult<EntityIntelligenceEnvelope> {
    producer::build_entity_intelligence(ctx, input).await
}
