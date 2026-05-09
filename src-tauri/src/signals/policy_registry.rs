//! Registry-backed signal policy selection.

use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SignalType {
    AbilityOutputChanged {
        ability_name: String,
        output_id: String,
    },
    AccountCreated,
    AccountDomainsUpdated,
    AccountEventRecorded,
    AccountMerged,
    AccountRisk,
    AccountUpdated,
    ActionCluster,
    ChampionEngagementConfirmed,
    ChampionRisk,
    ClaimAsserted,
    ClaimContradiction,
    ClaimFeedbackRecorded,
    ClaimRetracted,
    ClaimSuperseded,
    ClaimTrustChanged,
    ClaimVerificationStateChanged,
    CoAttendance,
    CommitmentCaptured,
    CommitmentReceived,
    CompanyChange,
    CompetitorMentioned,
    ContractSigned,
    EmailCadenceDrop,
    EmailCadenceDropDismissed,
    EmailItemDismissed,
    EmailReceived,
    EmailSignalDismissed,
    EngagementWarning,
    EnrichmentComplete,
    EnrichmentStale,
    EntityArchived,
    EntityCreated,
    EntityEnriched,
    EntityIntelligenceUpdated,
    EntityResolved,
    EntityResolution,
    EntityRestored,
    EntityUpdated,
    EscalationDetected,
    GleanChampionDeparted,
    GleanContactDiscovered,
    GleanOrgChange,
    HealthChange,
    IntelligenceAnnotated,
    IntelligenceConfirmed,
    IntelligenceCorrected,
    IntelligenceCurated,
    IntelligenceRefreshed,
    IntelligenceRejected,
    MeetingFrequency,
    MeetingFrequencyDrop,
    NegativeSentiment,
    ObjectiveCompleted,
    ObjectiveCreated,
    ObjectiveUpdated,
    PersonCreated,
    PersonProfileUpdated,
    PersonUpdated,
    PredictionConfirmed,
    PreMeetingContext,
    PrepInvalidated,
    ProactiveActionCluster,
    ProactiveEmailSpike,
    ProactiveMeetingLoad,
    ProactiveNoContact,
    ProactivePrepGap,
    ProactiveRelationshipDrift,
    ProactiveRenewalGap,
    ProactiveStaleChampion,
    ProfileDiscovered,
    ProfileEnriched,
    ProfileUpdate,
    ProfileUpdated,
    ProjectHealthWarning,
    ReadModelMaterialized,
    RegulatoryGapDetected,
    RegulatoryRequirementDetected,
    RelationshipGraphChanged,
    RelationshipInferred,
    RenewalAtRisk,
    RenewalDataUpdated,
    RenewalProximity,
    RenewalRiskEscalation,
    RenewalStageUpdated,
    SourceRestricted,
    SourceRevoked,
    SourceWithdrawn,
    StakeholderChange,
    StakeholderDisengagement,
    StakeholderUnverified,
    StakeholderVerified,
    StakeholdersChanged,
    StakeholdersUpdated,
    SupportHealthUpdated,
    TechnicalFootprintUpdated,
    ThreadPosition,
    TitleChange,
    TranscriptOutcomes,
    TranscriptSentiment,
    UserCorrection,
    UserCorrectionSubmitted,
    Legacy {
        name: String,
    },
}

impl SignalType {
    pub fn from_name(name: &str) -> Self {
        match name {
            "ability_output_changed" | "AbilityOutputChanged" => Self::AbilityOutputChanged {
                ability_name: String::new(),
                output_id: String::new(),
            },
            "account_created" => Self::AccountCreated,
            "account_domains_updated" => Self::AccountDomainsUpdated,
            "account_event_recorded" => Self::AccountEventRecorded,
            "account_merged" => Self::AccountMerged,
            "account_risk" => Self::AccountRisk,
            "account_updated" => Self::AccountUpdated,
            "action_cluster" => Self::ActionCluster,
            "champion_engagement_confirmed" => Self::ChampionEngagementConfirmed,
            "champion_risk" => Self::ChampionRisk,
            "claim_asserted" => Self::ClaimAsserted,
            "claim_contradiction" => Self::ClaimContradiction,
            "claim_feedback_recorded" => Self::ClaimFeedbackRecorded,
            "claim_retracted" => Self::ClaimRetracted,
            "claim_superseded" => Self::ClaimSuperseded,
            "claim_trust_changed" | "ClaimTrustChanged" => Self::ClaimTrustChanged,
            "claim_verification_state_changed" => Self::ClaimVerificationStateChanged,
            "co_attendance" => Self::CoAttendance,
            "commitment_captured" => Self::CommitmentCaptured,
            "commitment_received" => Self::CommitmentReceived,
            "company_change" => Self::CompanyChange,
            "competitor_mentioned" => Self::CompetitorMentioned,
            "contract_signed" => Self::ContractSigned,
            "email_cadence_drop" => Self::EmailCadenceDrop,
            "email_cadence_drop_dismissed" => Self::EmailCadenceDropDismissed,
            "email_item_dismissed" => Self::EmailItemDismissed,
            "email_received" => Self::EmailReceived,
            "email_signal_dismissed" => Self::EmailSignalDismissed,
            "engagement_warning" => Self::EngagementWarning,
            "enrichment_complete" => Self::EnrichmentComplete,
            "enrichment_stale" => Self::EnrichmentStale,
            "entity_archived" => Self::EntityArchived,
            "entity_created" => Self::EntityCreated,
            "entity_enriched" => Self::EntityEnriched,
            "entity_intelligence_updated" => Self::EntityIntelligenceUpdated,
            "entity_resolved" => Self::EntityResolved,
            "entity_resolution" => Self::EntityResolution,
            "entity_restored" => Self::EntityRestored,
            "entity_updated" | "EntityUpdated" => Self::EntityUpdated,
            "escalation_detected" => Self::EscalationDetected,
            "glean_champion_departed" => Self::GleanChampionDeparted,
            "glean_contact_discovered" => Self::GleanContactDiscovered,
            "glean_org_change" => Self::GleanOrgChange,
            "health_change" => Self::HealthChange,
            "intelligence_annotated" => Self::IntelligenceAnnotated,
            "intelligence_confirmed" => Self::IntelligenceConfirmed,
            "intelligence_corrected" => Self::IntelligenceCorrected,
            "intelligence_curated" => Self::IntelligenceCurated,
            "intelligence_refreshed" => Self::IntelligenceRefreshed,
            "intelligence_rejected" => Self::IntelligenceRejected,
            "meeting_frequency" => Self::MeetingFrequency,
            "meeting_frequency_drop" => Self::MeetingFrequencyDrop,
            "negative_sentiment" => Self::NegativeSentiment,
            "objective_completed" => Self::ObjectiveCompleted,
            "objective_created" => Self::ObjectiveCreated,
            "objective_updated" => Self::ObjectiveUpdated,
            "person_created" => Self::PersonCreated,
            "person_profile_updated" => Self::PersonProfileUpdated,
            "person_updated" => Self::PersonUpdated,
            "prediction_confirmed" => Self::PredictionConfirmed,
            "pre_meeting_context" => Self::PreMeetingContext,
            "prep_invalidated" => Self::PrepInvalidated,
            "proactive_action_cluster" => Self::ProactiveActionCluster,
            "proactive_email_spike" => Self::ProactiveEmailSpike,
            "proactive_meeting_load" => Self::ProactiveMeetingLoad,
            "proactive_no_contact" => Self::ProactiveNoContact,
            "proactive_prep_gap" => Self::ProactivePrepGap,
            "proactive_relationship_drift" => Self::ProactiveRelationshipDrift,
            "proactive_renewal_gap" => Self::ProactiveRenewalGap,
            "proactive_stale_champion" => Self::ProactiveStaleChampion,
            "profile_discovered" => Self::ProfileDiscovered,
            "profile_enriched" => Self::ProfileEnriched,
            "profile_update" => Self::ProfileUpdate,
            "profile_updated" => Self::ProfileUpdated,
            "project_health_warning" => Self::ProjectHealthWarning,
            "read_model_materialized" => Self::ReadModelMaterialized,
            "regulatory_gap_detected" => Self::RegulatoryGapDetected,
            "regulatory_requirement_detected" => Self::RegulatoryRequirementDetected,
            "relationship_graph_changed" => Self::RelationshipGraphChanged,
            "relationship_inferred" => Self::RelationshipInferred,
            "renewal_at_risk" => Self::RenewalAtRisk,
            "renewal_data_updated" => Self::RenewalDataUpdated,
            "renewal_proximity" => Self::RenewalProximity,
            "renewal_risk_escalation" => Self::RenewalRiskEscalation,
            "renewal_stage_updated" => Self::RenewalStageUpdated,
            "source_restricted" => Self::SourceRestricted,
            "source_revoked" => Self::SourceRevoked,
            "source_withdrawn" => Self::SourceWithdrawn,
            "stakeholder_change" => Self::StakeholderChange,
            "stakeholder_disengagement" => Self::StakeholderDisengagement,
            "stakeholder_unverified" => Self::StakeholderUnverified,
            "stakeholder_verified" => Self::StakeholderVerified,
            "stakeholders_changed" => Self::StakeholdersChanged,
            "stakeholders_updated" => Self::StakeholdersUpdated,
            "support_health_updated" => Self::SupportHealthUpdated,
            "technical_footprint_updated" => Self::TechnicalFootprintUpdated,
            "thread_position" => Self::ThreadPosition,
            "title_change" => Self::TitleChange,
            "transcript_outcomes" => Self::TranscriptOutcomes,
            "transcript_sentiment" => Self::TranscriptSentiment,
            "user_correction" => Self::UserCorrection,
            "user_correction_submitted" => Self::UserCorrectionSubmitted,
            other => Self::Legacy {
                name: other.to_string(),
            },
        }
    }

    pub fn canonical_name(&self) -> &str {
        match self {
            Self::AbilityOutputChanged { .. } => "ability_output_changed",
            Self::AccountCreated => "account_created",
            Self::AccountDomainsUpdated => "account_domains_updated",
            Self::AccountEventRecorded => "account_event_recorded",
            Self::AccountMerged => "account_merged",
            Self::AccountRisk => "account_risk",
            Self::AccountUpdated => "account_updated",
            Self::ActionCluster => "action_cluster",
            Self::ChampionEngagementConfirmed => "champion_engagement_confirmed",
            Self::ChampionRisk => "champion_risk",
            Self::ClaimAsserted => "claim_asserted",
            Self::ClaimContradiction => "claim_contradiction",
            Self::ClaimFeedbackRecorded => "claim_feedback_recorded",
            Self::ClaimRetracted => "claim_retracted",
            Self::ClaimSuperseded => "claim_superseded",
            Self::ClaimTrustChanged => "claim_trust_changed",
            Self::ClaimVerificationStateChanged => "claim_verification_state_changed",
            Self::CoAttendance => "co_attendance",
            Self::CommitmentCaptured => "commitment_captured",
            Self::CommitmentReceived => "commitment_received",
            Self::CompanyChange => "company_change",
            Self::CompetitorMentioned => "competitor_mentioned",
            Self::ContractSigned => "contract_signed",
            Self::EmailCadenceDrop => "email_cadence_drop",
            Self::EmailCadenceDropDismissed => "email_cadence_drop_dismissed",
            Self::EmailItemDismissed => "email_item_dismissed",
            Self::EmailReceived => "email_received",
            Self::EmailSignalDismissed => "email_signal_dismissed",
            Self::EngagementWarning => "engagement_warning",
            Self::EnrichmentComplete => "enrichment_complete",
            Self::EnrichmentStale => "enrichment_stale",
            Self::EntityArchived => "entity_archived",
            Self::EntityCreated => "entity_created",
            Self::EntityEnriched => "entity_enriched",
            Self::EntityIntelligenceUpdated => "entity_intelligence_updated",
            Self::EntityResolved => "entity_resolved",
            Self::EntityResolution => "entity_resolution",
            Self::EntityRestored => "entity_restored",
            Self::EntityUpdated => "entity_updated",
            Self::EscalationDetected => "escalation_detected",
            Self::GleanChampionDeparted => "glean_champion_departed",
            Self::GleanContactDiscovered => "glean_contact_discovered",
            Self::GleanOrgChange => "glean_org_change",
            Self::HealthChange => "health_change",
            Self::IntelligenceAnnotated => "intelligence_annotated",
            Self::IntelligenceConfirmed => "intelligence_confirmed",
            Self::IntelligenceCorrected => "intelligence_corrected",
            Self::IntelligenceCurated => "intelligence_curated",
            Self::IntelligenceRefreshed => "intelligence_refreshed",
            Self::IntelligenceRejected => "intelligence_rejected",
            Self::MeetingFrequency => "meeting_frequency",
            Self::MeetingFrequencyDrop => "meeting_frequency_drop",
            Self::NegativeSentiment => "negative_sentiment",
            Self::ObjectiveCompleted => "objective_completed",
            Self::ObjectiveCreated => "objective_created",
            Self::ObjectiveUpdated => "objective_updated",
            Self::PersonCreated => "person_created",
            Self::PersonProfileUpdated => "person_profile_updated",
            Self::PersonUpdated => "person_updated",
            Self::PredictionConfirmed => "prediction_confirmed",
            Self::PreMeetingContext => "pre_meeting_context",
            Self::PrepInvalidated => "prep_invalidated",
            Self::ProactiveActionCluster => "proactive_action_cluster",
            Self::ProactiveEmailSpike => "proactive_email_spike",
            Self::ProactiveMeetingLoad => "proactive_meeting_load",
            Self::ProactiveNoContact => "proactive_no_contact",
            Self::ProactivePrepGap => "proactive_prep_gap",
            Self::ProactiveRelationshipDrift => "proactive_relationship_drift",
            Self::ProactiveRenewalGap => "proactive_renewal_gap",
            Self::ProactiveStaleChampion => "proactive_stale_champion",
            Self::ProfileDiscovered => "profile_discovered",
            Self::ProfileEnriched => "profile_enriched",
            Self::ProfileUpdate => "profile_update",
            Self::ProfileUpdated => "profile_updated",
            Self::ProjectHealthWarning => "project_health_warning",
            Self::ReadModelMaterialized => "read_model_materialized",
            Self::RegulatoryGapDetected => "regulatory_gap_detected",
            Self::RegulatoryRequirementDetected => "regulatory_requirement_detected",
            Self::RelationshipGraphChanged => "relationship_graph_changed",
            Self::RelationshipInferred => "relationship_inferred",
            Self::RenewalAtRisk => "renewal_at_risk",
            Self::RenewalDataUpdated => "renewal_data_updated",
            Self::RenewalProximity => "renewal_proximity",
            Self::RenewalRiskEscalation => "renewal_risk_escalation",
            Self::RenewalStageUpdated => "renewal_stage_updated",
            Self::SourceRestricted => "source_restricted",
            Self::SourceRevoked => "source_revoked",
            Self::SourceWithdrawn => "source_withdrawn",
            Self::StakeholderChange => "stakeholder_change",
            Self::StakeholderDisengagement => "stakeholder_disengagement",
            Self::StakeholderUnverified => "stakeholder_unverified",
            Self::StakeholderVerified => "stakeholder_verified",
            Self::StakeholdersChanged => "stakeholders_changed",
            Self::StakeholdersUpdated => "stakeholders_updated",
            Self::SupportHealthUpdated => "support_health_updated",
            Self::TechnicalFootprintUpdated => "technical_footprint_updated",
            Self::ThreadPosition => "thread_position",
            Self::TitleChange => "title_change",
            Self::TranscriptOutcomes => "transcript_outcomes",
            Self::TranscriptSentiment => "transcript_sentiment",
            Self::UserCorrection => "user_correction",
            Self::UserCorrectionSubmitted => "user_correction_submitted",
            Self::Legacy { name } => name.as_str(),
        }
    }

    pub fn is_claim_rate_limited(&self) -> bool {
        matches!(self, Self::ClaimTrustChanged)
    }

    pub fn uses_emit_path_coalescing(&self) -> bool {
        matches!(
            self,
            Self::EntityUpdated | Self::ClaimTrustChanged | Self::AbilityOutputChanged { .. }
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurabilityClass {
    DurablePropagation,
    CoalescedDurablePropagation,
    DurableLocalAudit,
    PropagateAndHeal,
    EphemeralNonBus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalRole {
    Observation,
    Invalidation,
    UserFeedback,
    ReadModelMaterialized,
    Local,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionModeBehavior {
    PersistInLive,
    CaptureInEvaluateAndSimulate,
    NonBusOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropagationPolicy {
    Local,
    PropagateSync { await_completion: bool },
    PropagateAsync { coalesce: Option<CoalescingPolicy> },
    PropagateAndHeal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoalescingPolicy {
    EntitySignal { window: Duration },
    SubjectAbilityInput { window: Duration },
    SourceVersion { window: Duration },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetResolver {
    None,
    Entity,
    ClaimSubject,
    AbilityOutput,
    MeetingPrep,
    ReadModel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryClass {
    None,
    Invalidation,
    UserVisible,
    Healing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaleMarkerBehavior {
    None,
    MarkAffectedOutputsStale,
    DeadLetterMarksStale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadPrivacy {
    NoPayload,
    NonPiiMetadata,
    UserAuthoredText,
    RedactedRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalEmissionChannel {
    ServiceFacade,
    Infrastructure,
    ActiveTransaction,
    PropagationDerived,
    FixtureSeed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelEligibility {
    AnyBus,
    ServiceOnly,
    ActiveTransactionOnly,
    PropagationOnly,
    FixtureOnly,
    NonBus,
}

impl ChannelEligibility {
    pub fn allows(self, channel: SignalEmissionChannel) -> bool {
        match self {
            Self::AnyBus => true,
            Self::ServiceOnly => matches!(channel, SignalEmissionChannel::ServiceFacade),
            Self::ActiveTransactionOnly => {
                matches!(channel, SignalEmissionChannel::ActiveTransaction)
            }
            Self::PropagationOnly => matches!(channel, SignalEmissionChannel::PropagationDerived),
            Self::FixtureOnly => matches!(channel, SignalEmissionChannel::FixtureSeed),
            Self::NonBus => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignalPolicy {
    pub durability: DurabilityClass,
    pub role: SignalRole,
    pub execution_mode: ExecutionModeBehavior,
    pub propagation: PropagationPolicy,
    pub target_resolver: TargetResolver,
    pub retry_class: RetryClass,
    pub stale_marker: StaleMarkerBehavior,
    pub await_timeout: Option<Duration>,
    pub payload_privacy: PayloadPrivacy,
    pub channel_eligibility: ChannelEligibility,
}

const COALESCE_ENTITY_500MS: CoalescingPolicy = CoalescingPolicy::EntitySignal {
    window: Duration::from_millis(500),
};
pub fn policy_for(signal: &SignalType) -> SignalPolicy {
    use SignalType::*;

    match signal {
        ClaimAsserted
        | ClaimSuperseded
        | ClaimRetracted
        | ClaimContradiction
        | ClaimFeedbackRecorded
        | ClaimVerificationStateChanged => durable_claim_policy(signal),
        ClaimTrustChanged => coalesced_claim_trust_policy(),
        UserCorrection
        | UserCorrectionSubmitted
        | IntelligenceCorrected
        | IntelligenceRejected
        | IntelligenceConfirmed
        | IntelligenceAnnotated
        | IntelligenceCurated
        | EmailSignalDismissed
        | EmailItemDismissed => user_feedback_policy(signal),
        SourceRevoked | SourceRestricted | SourceWithdrawn | GleanChampionDeparted => {
            propagate_and_heal_policy()
        }
        AbilityOutputChanged { .. } => ability_output_policy(),
        ReadModelMaterialized | PrepInvalidated | IntelligenceRefreshed | EnrichmentComplete => {
            read_model_materialized_policy()
        }
        AccountCreated
        | AccountDomainsUpdated
        | AccountEventRecorded
        | AccountMerged
        | AccountRisk
        | AccountUpdated
        | ActionCluster
        | ChampionRisk
        | CoAttendance
        | CommitmentCaptured
        | CommitmentReceived
        | CompanyChange
        | CompetitorMentioned
        | ContractSigned
        | EmailReceived
        | EngagementWarning
        | EnrichmentStale
        | EntityArchived
        | EntityCreated
        | EntityEnriched
        | EntityIntelligenceUpdated
        | EntityResolved
        | EntityResolution
        | EntityRestored
        | EntityUpdated
        | EscalationDetected
        | GleanOrgChange
        | HealthChange
        | MeetingFrequency
        | MeetingFrequencyDrop
        | NegativeSentiment
        | ObjectiveCompleted
        | ObjectiveCreated
        | ObjectiveUpdated
        | PersonCreated
        | PersonProfileUpdated
        | PersonUpdated
        | PreMeetingContext
        | ProactiveActionCluster
        | ProactiveEmailSpike
        | ProactiveMeetingLoad
        | ProactiveNoContact
        | ProactivePrepGap
        | ProactiveRelationshipDrift
        | ProactiveRenewalGap
        | ProactiveStaleChampion
        | ProfileDiscovered
        | ProfileEnriched
        | ProfileUpdate
        | ProfileUpdated
        | ProjectHealthWarning
        | RegulatoryGapDetected
        | RegulatoryRequirementDetected
        | RelationshipGraphChanged
        | RelationshipInferred
        | RenewalAtRisk
        | RenewalDataUpdated
        | RenewalProximity
        | RenewalRiskEscalation
        | RenewalStageUpdated
        | StakeholderChange
        | StakeholderDisengagement
        | StakeholderUnverified
        | StakeholderVerified
        | StakeholdersChanged
        | StakeholdersUpdated
        | SupportHealthUpdated
        | TechnicalFootprintUpdated
        | ThreadPosition
        | TitleChange
        | TranscriptOutcomes
        | TranscriptSentiment
        | ChampionEngagementConfirmed
        | PredictionConfirmed
        | EmailCadenceDrop
        | EmailCadenceDropDismissed
        | GleanContactDiscovered => coalesced_invalidation_policy(),
        Legacy { name } => legacy_policy(name),
    }
}

pub fn policy_for_name(name: &str) -> SignalPolicy {
    let signal = SignalType::from_name(name);
    policy_for(&signal)
}

fn durable_claim_policy(signal: &SignalType) -> SignalPolicy {
    let await_completion = matches!(
        signal,
        SignalType::ClaimRetracted | SignalType::ClaimContradiction
    );
    SignalPolicy {
        durability: DurabilityClass::DurablePropagation,
        role: SignalRole::Invalidation,
        execution_mode: ExecutionModeBehavior::PersistInLive,
        propagation: PropagationPolicy::PropagateSync { await_completion },
        target_resolver: TargetResolver::ClaimSubject,
        retry_class: RetryClass::UserVisible,
        stale_marker: StaleMarkerBehavior::DeadLetterMarksStale,
        await_timeout: await_completion.then(|| Duration::from_millis(500)),
        payload_privacy: PayloadPrivacy::NonPiiMetadata,
        channel_eligibility: ChannelEligibility::AnyBus,
    }
}

fn coalesced_claim_trust_policy() -> SignalPolicy {
    SignalPolicy {
        durability: DurabilityClass::CoalescedDurablePropagation,
        role: SignalRole::Invalidation,
        execution_mode: ExecutionModeBehavior::PersistInLive,
        propagation: PropagationPolicy::PropagateAsync {
            coalesce: Some(COALESCE_ENTITY_500MS),
        },
        target_resolver: TargetResolver::ClaimSubject,
        retry_class: RetryClass::Invalidation,
        stale_marker: StaleMarkerBehavior::DeadLetterMarksStale,
        await_timeout: None,
        payload_privacy: PayloadPrivacy::NonPiiMetadata,
        channel_eligibility: ChannelEligibility::AnyBus,
    }
}

fn user_feedback_policy(signal: &SignalType) -> SignalPolicy {
    let await_completion = matches!(
        signal,
        SignalType::UserCorrection | SignalType::UserCorrectionSubmitted
    );
    SignalPolicy {
        durability: DurabilityClass::DurablePropagation,
        role: SignalRole::UserFeedback,
        execution_mode: ExecutionModeBehavior::PersistInLive,
        propagation: PropagationPolicy::PropagateSync { await_completion },
        target_resolver: TargetResolver::Entity,
        retry_class: RetryClass::UserVisible,
        stale_marker: StaleMarkerBehavior::DeadLetterMarksStale,
        await_timeout: await_completion.then(|| Duration::from_millis(500)),
        payload_privacy: PayloadPrivacy::UserAuthoredText,
        channel_eligibility: ChannelEligibility::AnyBus,
    }
}

fn propagate_and_heal_policy() -> SignalPolicy {
    SignalPolicy {
        durability: DurabilityClass::PropagateAndHeal,
        role: SignalRole::Invalidation,
        execution_mode: ExecutionModeBehavior::PersistInLive,
        propagation: PropagationPolicy::PropagateAndHeal,
        target_resolver: TargetResolver::Entity,
        retry_class: RetryClass::Healing,
        stale_marker: StaleMarkerBehavior::DeadLetterMarksStale,
        await_timeout: None,
        payload_privacy: PayloadPrivacy::NonPiiMetadata,
        channel_eligibility: ChannelEligibility::AnyBus,
    }
}

fn ability_output_policy() -> SignalPolicy {
    SignalPolicy {
        durability: DurabilityClass::CoalescedDurablePropagation,
        role: SignalRole::Invalidation,
        execution_mode: ExecutionModeBehavior::PersistInLive,
        propagation: PropagationPolicy::PropagateAsync {
            coalesce: Some(COALESCE_ENTITY_500MS),
        },
        target_resolver: TargetResolver::AbilityOutput,
        retry_class: RetryClass::Invalidation,
        stale_marker: StaleMarkerBehavior::DeadLetterMarksStale,
        await_timeout: None,
        payload_privacy: PayloadPrivacy::NonPiiMetadata,
        channel_eligibility: ChannelEligibility::AnyBus,
    }
}

fn read_model_materialized_policy() -> SignalPolicy {
    SignalPolicy {
        durability: DurabilityClass::DurableLocalAudit,
        role: SignalRole::ReadModelMaterialized,
        execution_mode: ExecutionModeBehavior::PersistInLive,
        propagation: PropagationPolicy::Local,
        target_resolver: TargetResolver::ReadModel,
        retry_class: RetryClass::None,
        stale_marker: StaleMarkerBehavior::None,
        await_timeout: None,
        payload_privacy: PayloadPrivacy::NonPiiMetadata,
        channel_eligibility: ChannelEligibility::AnyBus,
    }
}

fn coalesced_invalidation_policy() -> SignalPolicy {
    SignalPolicy {
        durability: DurabilityClass::CoalescedDurablePropagation,
        role: SignalRole::Invalidation,
        execution_mode: ExecutionModeBehavior::PersistInLive,
        propagation: PropagationPolicy::PropagateAsync {
            coalesce: Some(COALESCE_ENTITY_500MS),
        },
        target_resolver: TargetResolver::Entity,
        retry_class: RetryClass::Invalidation,
        stale_marker: StaleMarkerBehavior::DeadLetterMarksStale,
        await_timeout: None,
        payload_privacy: PayloadPrivacy::NonPiiMetadata,
        channel_eligibility: ChannelEligibility::AnyBus,
    }
}

fn local_observation_policy() -> SignalPolicy {
    SignalPolicy {
        durability: DurabilityClass::DurableLocalAudit,
        role: SignalRole::Observation,
        execution_mode: ExecutionModeBehavior::PersistInLive,
        propagation: PropagationPolicy::Local,
        target_resolver: TargetResolver::None,
        retry_class: RetryClass::None,
        stale_marker: StaleMarkerBehavior::None,
        await_timeout: None,
        payload_privacy: PayloadPrivacy::NonPiiMetadata,
        channel_eligibility: ChannelEligibility::AnyBus,
    }
}

fn legacy_policy(name: &str) -> SignalPolicy {
    if name.starts_with("claim_") || name.contains("correction") || name.contains("feedback") {
        return user_feedback_policy(&SignalType::UserCorrectionSubmitted);
    }

    if name.contains("revoked") || name.contains("withdrawn") || name.contains("contradiction") {
        return propagate_and_heal_policy();
    }

    if name.contains("updated")
        || name.contains("changed")
        || name.contains("detected")
        || name.contains("risk")
        || name.contains("warning")
        || name.contains("archived")
        || name.contains("created")
        || name.contains("completed")
        || name.contains("captured")
        || name.contains("received")
    {
        return coalesced_invalidation_policy();
    }

    local_observation_policy()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claim_transitions_are_guaranteed_delivery() {
        let policy = policy_for(&SignalType::ClaimAsserted);
        assert_eq!(policy.durability, DurabilityClass::DurablePropagation);
        assert_eq!(policy.role, SignalRole::Invalidation);
        assert!(matches!(
            policy.propagation,
            PropagationPolicy::PropagateSync {
                await_completion: false
            }
        ));
    }

    #[test]
    fn user_correction_allows_bounded_await() {
        let policy = policy_for(&SignalType::UserCorrection);
        assert_eq!(policy.role, SignalRole::UserFeedback);
        assert_eq!(policy.await_timeout, Some(Duration::from_millis(500)));
        assert!(matches!(
            policy.propagation,
            PropagationPolicy::PropagateSync {
                await_completion: true
            }
        ));
    }

    #[test]
    fn read_model_notifications_are_local() {
        let policy = policy_for_name("read_model_materialized");
        assert_eq!(policy.role, SignalRole::ReadModelMaterialized);
        assert_eq!(policy.propagation, PropagationPolicy::Local);
    }

    #[test]
    fn dos237_named_signals_use_500ms_async_coalescing() {
        for name in [
            "EntityUpdated",
            "ClaimTrustChanged",
            "AbilityOutputChanged",
            "entity_updated",
            "claim_trust_changed",
            "ability_output_changed",
        ] {
            let policy = policy_for_name(name);
            assert!(matches!(
                policy.propagation,
                PropagationPolicy::PropagateAsync {
                    coalesce: Some(CoalescingPolicy::EntitySignal { window })
                } if window == Duration::from_millis(500)
            ));
        }
    }
}
