pub mod prompts;
pub mod synthesis;

use dailyos_abilities_macro::ability;

pub use synthesis::{
    draft_claims_for_publish, AttendeeContext, BriefSubjectRef, BriefTemporalScope, ChangeMarker,
    ClaimDraft, EvidenceSource, MeetingAttendee, MeetingBrief, MeetingBriefContext, MeetingSummary,
    OpenLoop, PrepareMeetingInput, SuggestedOutcome, Topic,
};

use crate::abilities::{AbilityContext, AbilityResult};

#[ability(
    name = "prepare_meeting",
    category = Transform,
    version = "0.1.0",
    schema_version = 1,
    allowed_actors = [User, Agent, System],
    allowed_modes = [Live, Simulate, Evaluate],
    requires_confirmation = false,
    may_publish = false,
    composes = [{ id = "get_entity_context", ability = "get_entity_context", optional = false }],
    experimental = false,
    signal_policy = { emits_on_output_change = [], coalesce = false }
)]
pub async fn prepare_meeting(
    ctx: &AbilityContext<'_>,
    input: PrepareMeetingInput,
) -> AbilityResult<MeetingBrief> {
    synthesis::build_meeting_brief(ctx, input).await
}
