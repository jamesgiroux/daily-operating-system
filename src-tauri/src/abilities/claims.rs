//! Claim anatomy substrate primitives per ADR-0125.
//!
//! Three closed-set primitives gate every `intelligence_claims` row:
//!
//! - `ClaimType` — the canonical taxonomy of what a claim asserts.
//!   Mirrors the ADR-0115 Signal Policy Registry pattern: an enum + a
//!   `const` slice so registry-completeness is compile-time exhaustive
//!   and a non-test `match` proves coverage.
//! - `CanonicalSubjectType` — the subject kinds a claim type is
//!   permitted to attach to. The cross-tenant / wrong-entity bleed
//!   guard: `commit_claim` rejects a `stakeholder_role` on an `Account`
//!   subject because the registry pins it to `Person`.
//! - `ClaimTypeMetadata` — what the registry stores per claim type:
//!   the canonical persisted name, allowed subjects, and default
//!   `TemporalScope` / `ClaimSensitivity`.
//!
//! Render policy (sensitivity ceilings), freshness math (temporal
//! decay), and supersession-by-scope are deliberately out of scope —
//! they consume this metadata in later versions but are not active
//! gates here.

use serde::{Deserialize, Serialize};

use crate::db::claims::{ClaimSensitivity, TemporalScope};

/// Actor classes permitted to write a given claim type. The
/// authorization grain at commit time: a claim type tagged with
/// only `[System]` rejects writes from `User` actors and vice
/// versa. An empty `allowed_actor_classes` slice means
/// "no restriction" — useful during migration windows where the
/// closed actor surface isn't fully defined.
///
/// W4-C `invoke_ability` consumes this for actor-filtered MCP
/// discovery; the substrate exposes the field so authorization
/// is colocated with the claim-type taxonomy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimActorClass {
    /// Real users producing claims via UI (manual dismiss,
    /// stakeholder edit, etc.).
    User,
    /// Backfill / migration / repair / system-maintenance code.
    System,
    /// AI abilities producing claims such as entity context and
    /// meeting brief outputs.
    Agent,
}

impl ClaimActorClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::System => "system",
            Self::Agent => "agent",
        }
    }
}

/// How quickly a claim type's meaning should lose freshness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreshnessDecayClass {
    /// Does not decay; the claim remains meaningful until explicitly changed.
    Static,
    /// Months-scale half-life for slowly changing facts.
    Slow,
    /// Weeks-scale half-life for facts that drift during normal operation.
    Medium,
    /// Days-scale half-life for short-lived preparation or status facts.
    Fast,
    /// Freshness is tied to a source event or explicit expiry.
    EventBound,
}

/// How new same-subject claims interact with existing claims.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitPolicyClass {
    /// Same-meaning claims merge through corroboration.
    Reinforce,
    /// Contradictions branch into separate claims for human resolution.
    Fork,
    /// Newer claims supersede older claims of the same meaning.
    Replace,
}

/// Subject kinds a claim may attach to. Matches the runtime
/// `SubjectRef` set (Account, Meeting, Person, Project, Email).
/// `Global` and `Multi` are deliberately absent: the v1.4.0 spine
/// rejects them at commit per ADR-0091.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanonicalSubjectType {
    Account,
    Meeting,
    Person,
    Project,
    Email,
}

impl CanonicalSubjectType {
    /// Lowercase form matching `SubjectRef.kind`'s normalized JSON.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Account => "account",
            Self::Meeting => "meeting",
            Self::Person => "person",
            Self::Project => "project",
            Self::Email => "email",
        }
    }
}

/// Closed set of claim types. The string form is the canonical persisted
/// name; the enum gives the writer-side closed-match contract.
///
/// Two cohorts coexist:
/// - **Production / lifecycle types** are the names current writers and
///   backfills use today. They cover dismissal, role, and entity-field
///   correction lifecycles.
/// - **Pilot context types** are the v1.4.0 W5 pilot ability outputs
///   (entity context, meeting brief). Pilots set `temporal_scope` and
///   `sensitivity` explicitly per claim; the registry default is the
///   conservative fallback for backfill.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimType {
    // --- Production / lifecycle -------------------------------------
    Risk,
    Win,
    StakeholderRole,
    LinkingDismissed,
    EmailDismissed,
    IntelligenceFieldDismissed,
    FeedbackFieldDismissed,
    TriageSnooze,
    MeetingEntityDismissed,
    AccountFieldCorrection,
    /// Generic dismissed-item lifecycle row written by the
    /// services::claims_backfill m9 path. Subject_ref is supplied
    /// by the caller; the canonical subjects are entity-or-meeting.
    DismissedItem,
    /// Briefing-callout dismissal (legacy `briefing_callouts` table
    /// migration). Subject is whatever the dismissed callout was
    /// attached to — entity or meeting.
    BriefingCalloutDismissed,
    /// Nudge dismissal (legacy `nudges` table migration). Subject
    /// is the entity the nudge targeted.
    NudgeDismissed,
    // --- Pilot context (W5 abilities) -------------------------------
    EntityIdentity,
    EntitySummary,
    EntityCurrentState,
    EntityRisk,
    EntityWin,
    StakeholderEngagement,
    StakeholderAssessment,
    ValueDelivered,
    MeetingReadiness,
    CompanyContext,
    OpenLoop,
    MeetingTopic,
    MeetingEventNote,
    AttendeeContext,
    MeetingChangeMarker,
    SuggestedOutcome,
}

impl ClaimType {
    /// Canonical persisted string. The registry's `name` field is this
    /// value; DB rows store this string in `intelligence_claims.claim_type`.
    pub fn as_str(&self) -> &'static str {
        metadata_for_claim_type(*self).name
    }

    /// Parse the persisted-string form. Unknown strings return `Err` so
    /// `commit_claim` can fail closed before insert.
    pub fn try_from_db_str(s: &str) -> Result<Self, UnknownClaimTypeError> {
        for entry in CLAIM_TYPE_REGISTRY {
            if entry.name == s {
                return Ok(entry.kind);
            }
        }
        Err(UnknownClaimTypeError(s.to_string()))
    }
}

/// Returned when a string does not correspond to any registered claim
/// type. Carried by `services::claims::ClaimError::UnknownClaimType`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownClaimTypeError(pub String);

impl std::fmt::Display for UnknownClaimTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown claim_type: {}", self.0)
    }
}

impl std::error::Error for UnknownClaimTypeError {}

/// Per-claim-type metadata. The registry stores one of these per
/// `ClaimType` variant. Defaults apply when a `ClaimProposal` leaves
/// `temporal_scope` / `sensitivity` at the conservative fallback;
/// pilot abilities override per-claim because dynamic scopes
/// (`PointInTime`, `Trend`) need claim-specific timestamps.
#[derive(Debug, Clone)]
pub struct ClaimTypeMetadata {
    pub kind: ClaimType,
    /// Canonical persisted string. MUST be unique across the registry
    /// (enforced by `claim_type_registry_has_unique_names`).
    pub name: &'static str,
    pub default_temporal_scope: TemporalScope,
    pub default_sensitivity: ClaimSensitivity,
    pub freshness_decay_class: FreshnessDecayClass,
    pub commit_policy_class: CommitPolicyClass,
    /// Subjects this claim type may attach to. `commit_claim` rejects
    /// rows whose `subject_ref.kind` is not in this slice.
    pub canonical_subject_types: &'static [CanonicalSubjectType],
    /// Actor classes permitted to write this claim type. Empty slice
    /// means no restriction — useful during the W3 substrate window
    /// where the closed actor surface is still being defined.
    /// W4-C `invoke_ability` consumes this for actor-filtered MCP
    /// discovery.
    pub allowed_actor_classes: &'static [ClaimActorClass],
}

/// Compile-time exhaustive lookup. Adding a new `ClaimType` variant
/// without a match arm is a build error; the registry slice used by
/// persisted-name traversal is checked separately in tests.
pub const fn metadata_for_claim_type(kind: ClaimType) -> &'static ClaimTypeMetadata {
    match kind {
        ClaimType::Risk => &ClaimTypeMetadata {
            kind: ClaimType::Risk,
            name: "risk",
            default_temporal_scope: TemporalScope::State,
            default_sensitivity: ClaimSensitivity::Internal,
            freshness_decay_class: FreshnessDecayClass::Medium,
            commit_policy_class: CommitPolicyClass::Fork,
            canonical_subject_types: SUBJECTS_ANY_ENTITY,
            allowed_actor_classes: ACTORS_ANY,
        },
        ClaimType::Win => &ClaimTypeMetadata {
            kind: ClaimType::Win,
            name: "win",
            default_temporal_scope: TemporalScope::State,
            default_sensitivity: ClaimSensitivity::Internal,
            freshness_decay_class: FreshnessDecayClass::Slow,
            commit_policy_class: CommitPolicyClass::Reinforce,
            canonical_subject_types: SUBJECTS_ANY_ENTITY,
            allowed_actor_classes: ACTORS_ANY,
        },
        ClaimType::StakeholderRole => &ClaimTypeMetadata {
            kind: ClaimType::StakeholderRole,
            name: "stakeholder_role",
            default_temporal_scope: TemporalScope::State,
            default_sensitivity: ClaimSensitivity::Internal,
            freshness_decay_class: FreshnessDecayClass::Slow,
            commit_policy_class: CommitPolicyClass::Replace,
            canonical_subject_types: SUBJECTS_PERSON,
            allowed_actor_classes: ACTORS_USER_OR_SYSTEM,
        },
        ClaimType::LinkingDismissed => &ClaimTypeMetadata {
            kind: ClaimType::LinkingDismissed,
            name: "linking_dismissed",
            default_temporal_scope: TemporalScope::State,
            default_sensitivity: ClaimSensitivity::Internal,
            freshness_decay_class: FreshnessDecayClass::Static,
            commit_policy_class: CommitPolicyClass::Reinforce,
            canonical_subject_types: SUBJECTS_LINKING_DISMISSED,
            allowed_actor_classes: ACTORS_USER_OR_SYSTEM,
        },
        ClaimType::EmailDismissed => &ClaimTypeMetadata {
            kind: ClaimType::EmailDismissed,
            name: "email_dismissed",
            default_temporal_scope: TemporalScope::State,
            default_sensitivity: ClaimSensitivity::Internal,
            freshness_decay_class: FreshnessDecayClass::Static,
            commit_policy_class: CommitPolicyClass::Reinforce,
            canonical_subject_types: SUBJECTS_EMAIL,
            allowed_actor_classes: ACTORS_USER_OR_SYSTEM,
        },
        ClaimType::IntelligenceFieldDismissed => &ClaimTypeMetadata {
            kind: ClaimType::IntelligenceFieldDismissed,
            name: "intelligence_field_dismissed",
            default_temporal_scope: TemporalScope::State,
            default_sensitivity: ClaimSensitivity::Internal,
            freshness_decay_class: FreshnessDecayClass::Static,
            commit_policy_class: CommitPolicyClass::Reinforce,
            canonical_subject_types: SUBJECTS_ENTITY_OR_MEETING,
            allowed_actor_classes: ACTORS_USER_OR_SYSTEM,
        },
        ClaimType::FeedbackFieldDismissed => &ClaimTypeMetadata {
            kind: ClaimType::FeedbackFieldDismissed,
            name: "feedback_field_dismissed",
            default_temporal_scope: TemporalScope::State,
            default_sensitivity: ClaimSensitivity::Internal,
            freshness_decay_class: FreshnessDecayClass::Static,
            commit_policy_class: CommitPolicyClass::Reinforce,
            canonical_subject_types: SUBJECTS_ENTITY_OR_MEETING,
            allowed_actor_classes: ACTORS_USER_OR_SYSTEM,
        },
        ClaimType::TriageSnooze => &ClaimTypeMetadata {
            kind: ClaimType::TriageSnooze,
            name: "triage_snooze",
            default_temporal_scope: TemporalScope::State,
            default_sensitivity: ClaimSensitivity::Internal,
            freshness_decay_class: FreshnessDecayClass::EventBound,
            commit_policy_class: CommitPolicyClass::Replace,
            canonical_subject_types: SUBJECTS_TRIAGE,
            allowed_actor_classes: ACTORS_USER_OR_SYSTEM,
        },
        ClaimType::MeetingEntityDismissed => &ClaimTypeMetadata {
            kind: ClaimType::MeetingEntityDismissed,
            name: "meeting_entity_dismissed",
            default_temporal_scope: TemporalScope::State,
            default_sensitivity: ClaimSensitivity::Internal,
            freshness_decay_class: FreshnessDecayClass::Static,
            commit_policy_class: CommitPolicyClass::Reinforce,
            canonical_subject_types: SUBJECTS_MEETING,
            allowed_actor_classes: ACTORS_USER_OR_SYSTEM,
        },
        ClaimType::AccountFieldCorrection => &ClaimTypeMetadata {
            kind: ClaimType::AccountFieldCorrection,
            name: "account_field_correction",
            default_temporal_scope: TemporalScope::State,
            default_sensitivity: ClaimSensitivity::Internal,
            freshness_decay_class: FreshnessDecayClass::Static,
            commit_policy_class: CommitPolicyClass::Reinforce,
            canonical_subject_types: SUBJECTS_ACCOUNT,
            allowed_actor_classes: ACTORS_USER_OR_SYSTEM,
        },
        ClaimType::DismissedItem => &ClaimTypeMetadata {
            kind: ClaimType::DismissedItem,
            name: "dismissed_item",
            default_temporal_scope: TemporalScope::State,
            default_sensitivity: ClaimSensitivity::Internal,
            freshness_decay_class: FreshnessDecayClass::Static,
            commit_policy_class: CommitPolicyClass::Reinforce,
            canonical_subject_types: SUBJECTS_TRIAGE,
            allowed_actor_classes: ACTORS_SYSTEM,
        },
        ClaimType::BriefingCalloutDismissed => &ClaimTypeMetadata {
            kind: ClaimType::BriefingCalloutDismissed,
            name: "briefing_callout_dismissed",
            default_temporal_scope: TemporalScope::State,
            default_sensitivity: ClaimSensitivity::Internal,
            freshness_decay_class: FreshnessDecayClass::Static,
            commit_policy_class: CommitPolicyClass::Reinforce,
            canonical_subject_types: SUBJECTS_ENTITY_OR_MEETING,
            allowed_actor_classes: ACTORS_SYSTEM,
        },
        ClaimType::NudgeDismissed => &ClaimTypeMetadata {
            kind: ClaimType::NudgeDismissed,
            name: "nudge_dismissed",
            default_temporal_scope: TemporalScope::State,
            default_sensitivity: ClaimSensitivity::Internal,
            freshness_decay_class: FreshnessDecayClass::Static,
            commit_policy_class: CommitPolicyClass::Reinforce,
            canonical_subject_types: SUBJECTS_ENTITY_OR_MEETING,
            allowed_actor_classes: ACTORS_SYSTEM,
        },
        ClaimType::EntityIdentity => &ClaimTypeMetadata {
            kind: ClaimType::EntityIdentity,
            name: "entity_identity",
            default_temporal_scope: TemporalScope::State,
            default_sensitivity: ClaimSensitivity::Internal,
            freshness_decay_class: FreshnessDecayClass::Slow,
            commit_policy_class: CommitPolicyClass::Replace,
            canonical_subject_types: SUBJECTS_ANY_ENTITY,
            allowed_actor_classes: ACTORS_AGENT,
        },
        ClaimType::EntitySummary => &ClaimTypeMetadata {
            kind: ClaimType::EntitySummary,
            name: "entity_summary",
            default_temporal_scope: TemporalScope::State,
            default_sensitivity: ClaimSensitivity::Internal,
            freshness_decay_class: FreshnessDecayClass::Slow,
            commit_policy_class: CommitPolicyClass::Replace,
            canonical_subject_types: SUBJECTS_ANY_ENTITY,
            allowed_actor_classes: ACTORS_AGENT,
        },
        ClaimType::EntityCurrentState => &ClaimTypeMetadata {
            kind: ClaimType::EntityCurrentState,
            name: "entity_current_state",
            default_temporal_scope: TemporalScope::State,
            default_sensitivity: ClaimSensitivity::Internal,
            freshness_decay_class: FreshnessDecayClass::Slow,
            commit_policy_class: CommitPolicyClass::Replace,
            canonical_subject_types: SUBJECTS_ANY_ENTITY,
            allowed_actor_classes: ACTORS_AGENT,
        },
        ClaimType::EntityRisk => &ClaimTypeMetadata {
            kind: ClaimType::EntityRisk,
            name: "entity_risk",
            default_temporal_scope: TemporalScope::State,
            default_sensitivity: ClaimSensitivity::Internal,
            freshness_decay_class: FreshnessDecayClass::Medium,
            commit_policy_class: CommitPolicyClass::Fork,
            canonical_subject_types: SUBJECTS_ANY_ENTITY,
            allowed_actor_classes: ACTORS_AGENT,
        },
        ClaimType::EntityWin => &ClaimTypeMetadata {
            kind: ClaimType::EntityWin,
            name: "entity_win",
            default_temporal_scope: TemporalScope::State,
            default_sensitivity: ClaimSensitivity::Internal,
            freshness_decay_class: FreshnessDecayClass::Slow,
            commit_policy_class: CommitPolicyClass::Reinforce,
            canonical_subject_types: SUBJECTS_ANY_ENTITY,
            allowed_actor_classes: ACTORS_AGENT,
        },
        ClaimType::StakeholderEngagement => &ClaimTypeMetadata {
            kind: ClaimType::StakeholderEngagement,
            name: "stakeholder_engagement",
            default_temporal_scope: TemporalScope::State,
            default_sensitivity: ClaimSensitivity::Internal,
            freshness_decay_class: FreshnessDecayClass::Medium,
            commit_policy_class: CommitPolicyClass::Reinforce,
            canonical_subject_types: SUBJECTS_PERSON,
            allowed_actor_classes: ACTORS_AGENT,
        },
        ClaimType::StakeholderAssessment => &ClaimTypeMetadata {
            kind: ClaimType::StakeholderAssessment,
            name: "stakeholder_assessment",
            default_temporal_scope: TemporalScope::State,
            default_sensitivity: ClaimSensitivity::Confidential,
            freshness_decay_class: FreshnessDecayClass::Medium,
            commit_policy_class: CommitPolicyClass::Reinforce,
            canonical_subject_types: SUBJECTS_PERSON,
            allowed_actor_classes: ACTORS_AGENT,
        },
        ClaimType::ValueDelivered => &ClaimTypeMetadata {
            kind: ClaimType::ValueDelivered,
            name: "value_delivered",
            default_temporal_scope: TemporalScope::State,
            default_sensitivity: ClaimSensitivity::Internal,
            freshness_decay_class: FreshnessDecayClass::Slow,
            commit_policy_class: CommitPolicyClass::Reinforce,
            canonical_subject_types: SUBJECTS_ANY_ENTITY,
            allowed_actor_classes: ACTORS_AGENT,
        },
        ClaimType::MeetingReadiness => &ClaimTypeMetadata {
            kind: ClaimType::MeetingReadiness,
            name: "meeting_readiness",
            default_temporal_scope: TemporalScope::State,
            default_sensitivity: ClaimSensitivity::Internal,
            freshness_decay_class: FreshnessDecayClass::Fast,
            commit_policy_class: CommitPolicyClass::Replace,
            canonical_subject_types: SUBJECTS_MEETING,
            allowed_actor_classes: ACTORS_AGENT,
        },
        ClaimType::CompanyContext => &ClaimTypeMetadata {
            kind: ClaimType::CompanyContext,
            name: "company_context",
            default_temporal_scope: TemporalScope::State,
            default_sensitivity: ClaimSensitivity::Internal,
            freshness_decay_class: FreshnessDecayClass::Slow,
            commit_policy_class: CommitPolicyClass::Reinforce,
            canonical_subject_types: SUBJECTS_ACCOUNT,
            allowed_actor_classes: ACTORS_AGENT,
        },
        ClaimType::OpenLoop => &ClaimTypeMetadata {
            kind: ClaimType::OpenLoop,
            name: "open_loop",
            default_temporal_scope: TemporalScope::State,
            default_sensitivity: ClaimSensitivity::Internal,
            freshness_decay_class: FreshnessDecayClass::Medium,
            commit_policy_class: CommitPolicyClass::Reinforce,
            canonical_subject_types: SUBJECTS_ENTITY_OR_MEETING,
            allowed_actor_classes: ACTORS_AGENT,
        },
        ClaimType::MeetingTopic => &ClaimTypeMetadata {
            kind: ClaimType::MeetingTopic,
            name: "meeting_topic",
            default_temporal_scope: TemporalScope::State,
            default_sensitivity: ClaimSensitivity::Internal,
            freshness_decay_class: FreshnessDecayClass::EventBound,
            commit_policy_class: CommitPolicyClass::Reinforce,
            canonical_subject_types: SUBJECTS_MEETING,
            allowed_actor_classes: ACTORS_AGENT,
        },
        ClaimType::MeetingEventNote => &ClaimTypeMetadata {
            kind: ClaimType::MeetingEventNote,
            name: "meeting_event_note",
            default_temporal_scope: TemporalScope::PointInTime,
            default_sensitivity: ClaimSensitivity::Internal,
            freshness_decay_class: FreshnessDecayClass::EventBound,
            commit_policy_class: CommitPolicyClass::Reinforce,
            canonical_subject_types: SUBJECTS_MEETING,
            allowed_actor_classes: ACTORS_AGENT,
        },
        ClaimType::AttendeeContext => &ClaimTypeMetadata {
            kind: ClaimType::AttendeeContext,
            name: "attendee_context",
            default_temporal_scope: TemporalScope::State,
            default_sensitivity: ClaimSensitivity::Internal,
            freshness_decay_class: FreshnessDecayClass::EventBound,
            commit_policy_class: CommitPolicyClass::Reinforce,
            canonical_subject_types: SUBJECTS_PERSON,
            allowed_actor_classes: ACTORS_AGENT,
        },
        ClaimType::MeetingChangeMarker => &ClaimTypeMetadata {
            kind: ClaimType::MeetingChangeMarker,
            name: "meeting_change_marker",
            default_temporal_scope: TemporalScope::PointInTime,
            default_sensitivity: ClaimSensitivity::Internal,
            freshness_decay_class: FreshnessDecayClass::EventBound,
            commit_policy_class: CommitPolicyClass::Reinforce,
            canonical_subject_types: SUBJECTS_MEETING,
            allowed_actor_classes: ACTORS_AGENT,
        },
        ClaimType::SuggestedOutcome => &ClaimTypeMetadata {
            kind: ClaimType::SuggestedOutcome,
            name: "suggested_outcome",
            default_temporal_scope: TemporalScope::State,
            default_sensitivity: ClaimSensitivity::Internal,
            freshness_decay_class: FreshnessDecayClass::EventBound,
            commit_policy_class: CommitPolicyClass::Replace,
            canonical_subject_types: SUBJECTS_MEETING,
            allowed_actor_classes: ACTORS_AGENT,
        },
    }
}

/// Common subject-type slices to keep registry rows compact.
const SUBJECTS_ACCOUNT: &[CanonicalSubjectType] = &[CanonicalSubjectType::Account];
const SUBJECTS_PERSON: &[CanonicalSubjectType] = &[CanonicalSubjectType::Person];
const SUBJECTS_MEETING: &[CanonicalSubjectType] = &[CanonicalSubjectType::Meeting];
const SUBJECTS_EMAIL: &[CanonicalSubjectType] = &[CanonicalSubjectType::Email];
const SUBJECTS_ANY_ENTITY: &[CanonicalSubjectType] = &[
    CanonicalSubjectType::Account,
    CanonicalSubjectType::Project,
    CanonicalSubjectType::Person,
];
const SUBJECTS_ENTITY_OR_MEETING: &[CanonicalSubjectType] = &[
    CanonicalSubjectType::Account,
    CanonicalSubjectType::Project,
    CanonicalSubjectType::Person,
    CanonicalSubjectType::Meeting,
];
/// linking_dismissed must accept Email — `manual_dismiss` for
/// owner_type=Email shadow-writes claim_type='linking_dismissed'
/// on the Email subject. Distinct from the entity-or-meeting set
/// because Email is a dismissable owner here but not for
/// `meeting_entity_dismissed` (which only attaches to Meeting).
const SUBJECTS_LINKING_DISMISSED: &[CanonicalSubjectType] = &[
    CanonicalSubjectType::Account,
    CanonicalSubjectType::Project,
    CanonicalSubjectType::Person,
    CanonicalSubjectType::Meeting,
    CanonicalSubjectType::Email,
];
const SUBJECTS_TRIAGE: &[CanonicalSubjectType] = &[
    CanonicalSubjectType::Account,
    CanonicalSubjectType::Project,
    CanonicalSubjectType::Person,
    CanonicalSubjectType::Meeting,
    CanonicalSubjectType::Email,
];

/// Actor-class slices for the W3 substrate window. The system+user
/// pairing covers all current dismissal lifecycle types (legacy
/// backfill writes as system; runtime user-driven dismissals write
/// as user). Pilot context claim types are agent-only since they're
/// produced by AI abilities.
const ACTORS_ANY: &[ClaimActorClass] = &[
    ClaimActorClass::User,
    ClaimActorClass::System,
    ClaimActorClass::Agent,
];
const ACTORS_USER_OR_SYSTEM: &[ClaimActorClass] =
    &[ClaimActorClass::User, ClaimActorClass::System];
const ACTORS_AGENT: &[ClaimActorClass] = &[ClaimActorClass::Agent];
const ACTORS_SYSTEM: &[ClaimActorClass] = &[ClaimActorClass::System];

/// Closed registry of claim types for name-based traversal paths.
/// `metadata_for_claim_type` is independently exhaustive; this slice
/// backs persisted-name parsing and registry-order drift tests.
pub const CLAIM_TYPE_REGISTRY: &[ClaimTypeMetadata] = &[
    // --- Production / lifecycle -------------------------------------
    ClaimTypeMetadata {
        kind: ClaimType::Risk,
        name: "risk",
        default_temporal_scope: TemporalScope::State,
        default_sensitivity: ClaimSensitivity::Internal,
        freshness_decay_class: FreshnessDecayClass::Medium,
        commit_policy_class: CommitPolicyClass::Fork,
        canonical_subject_types: SUBJECTS_ANY_ENTITY,
        allowed_actor_classes: ACTORS_ANY,
    },
    ClaimTypeMetadata {
        kind: ClaimType::Win,
        name: "win",
        default_temporal_scope: TemporalScope::State,
        default_sensitivity: ClaimSensitivity::Internal,
        freshness_decay_class: FreshnessDecayClass::Slow,
        commit_policy_class: CommitPolicyClass::Reinforce,
        canonical_subject_types: SUBJECTS_ANY_ENTITY,
        allowed_actor_classes: ACTORS_ANY,
    },
    ClaimTypeMetadata {
        kind: ClaimType::StakeholderRole,
        name: "stakeholder_role",
        default_temporal_scope: TemporalScope::State,
        default_sensitivity: ClaimSensitivity::Internal,
        freshness_decay_class: FreshnessDecayClass::Slow,
        commit_policy_class: CommitPolicyClass::Replace,
        canonical_subject_types: SUBJECTS_PERSON,
        allowed_actor_classes: ACTORS_USER_OR_SYSTEM,
    },
    ClaimTypeMetadata {
        kind: ClaimType::LinkingDismissed,
        name: "linking_dismissed",
        default_temporal_scope: TemporalScope::State,
        default_sensitivity: ClaimSensitivity::Internal,
        freshness_decay_class: FreshnessDecayClass::Static,
        commit_policy_class: CommitPolicyClass::Reinforce,
        // Email is a dismissable owner here (manual_dismiss for
        // owner_type=Email writes this) — distinct from the
        // entity-or-meeting set used by other lifecycle types.
        canonical_subject_types: SUBJECTS_LINKING_DISMISSED,
        allowed_actor_classes: ACTORS_USER_OR_SYSTEM,
    },
    ClaimTypeMetadata {
        kind: ClaimType::EmailDismissed,
        name: "email_dismissed",
        default_temporal_scope: TemporalScope::State,
        default_sensitivity: ClaimSensitivity::Internal,
        freshness_decay_class: FreshnessDecayClass::Static,
        commit_policy_class: CommitPolicyClass::Reinforce,
        canonical_subject_types: SUBJECTS_EMAIL,
        allowed_actor_classes: ACTORS_USER_OR_SYSTEM,
    },
    ClaimTypeMetadata {
        kind: ClaimType::IntelligenceFieldDismissed,
        name: "intelligence_field_dismissed",
        default_temporal_scope: TemporalScope::State,
        default_sensitivity: ClaimSensitivity::Internal,
        freshness_decay_class: FreshnessDecayClass::Static,
        commit_policy_class: CommitPolicyClass::Reinforce,
        canonical_subject_types: SUBJECTS_ENTITY_OR_MEETING,
        allowed_actor_classes: ACTORS_USER_OR_SYSTEM,
    },
    ClaimTypeMetadata {
        kind: ClaimType::FeedbackFieldDismissed,
        name: "feedback_field_dismissed",
        default_temporal_scope: TemporalScope::State,
        default_sensitivity: ClaimSensitivity::Internal,
        freshness_decay_class: FreshnessDecayClass::Static,
        commit_policy_class: CommitPolicyClass::Reinforce,
        canonical_subject_types: SUBJECTS_ENTITY_OR_MEETING,
        allowed_actor_classes: ACTORS_USER_OR_SYSTEM,
    },
    ClaimTypeMetadata {
        kind: ClaimType::TriageSnooze,
        name: "triage_snooze",
        default_temporal_scope: TemporalScope::State,
        default_sensitivity: ClaimSensitivity::Internal,
        freshness_decay_class: FreshnessDecayClass::EventBound,
        commit_policy_class: CommitPolicyClass::Replace,
        canonical_subject_types: SUBJECTS_TRIAGE,
        allowed_actor_classes: ACTORS_USER_OR_SYSTEM,
    },
    ClaimTypeMetadata {
        kind: ClaimType::MeetingEntityDismissed,
        name: "meeting_entity_dismissed",
        default_temporal_scope: TemporalScope::State,
        default_sensitivity: ClaimSensitivity::Internal,
        freshness_decay_class: FreshnessDecayClass::Static,
        commit_policy_class: CommitPolicyClass::Reinforce,
        canonical_subject_types: SUBJECTS_MEETING,
        allowed_actor_classes: ACTORS_USER_OR_SYSTEM,
    },
    ClaimTypeMetadata {
        kind: ClaimType::AccountFieldCorrection,
        name: "account_field_correction",
        default_temporal_scope: TemporalScope::State,
        default_sensitivity: ClaimSensitivity::Internal,
        freshness_decay_class: FreshnessDecayClass::Static,
        commit_policy_class: CommitPolicyClass::Reinforce,
        canonical_subject_types: SUBJECTS_ACCOUNT,
        allowed_actor_classes: ACTORS_USER_OR_SYSTEM,
    },
    // --- Backfill-only (legacy migration write paths) ---------------
    ClaimTypeMetadata {
        kind: ClaimType::DismissedItem,
        name: "dismissed_item",
        default_temporal_scope: TemporalScope::State,
        default_sensitivity: ClaimSensitivity::Internal,
        freshness_decay_class: FreshnessDecayClass::Static,
        commit_policy_class: CommitPolicyClass::Reinforce,
        // Subject_ref is supplied by the m9 backfill caller and may
        // be any entity or meeting; matching the runtime triage set
        // (incl. Email) keeps backfill insertion permissible.
        canonical_subject_types: SUBJECTS_TRIAGE,
        allowed_actor_classes: ACTORS_SYSTEM,
    },
    ClaimTypeMetadata {
        kind: ClaimType::BriefingCalloutDismissed,
        name: "briefing_callout_dismissed",
        default_temporal_scope: TemporalScope::State,
        default_sensitivity: ClaimSensitivity::Internal,
        freshness_decay_class: FreshnessDecayClass::Static,
        commit_policy_class: CommitPolicyClass::Reinforce,
        canonical_subject_types: SUBJECTS_ENTITY_OR_MEETING,
        allowed_actor_classes: ACTORS_SYSTEM,
    },
    ClaimTypeMetadata {
        kind: ClaimType::NudgeDismissed,
        name: "nudge_dismissed",
        default_temporal_scope: TemporalScope::State,
        default_sensitivity: ClaimSensitivity::Internal,
        freshness_decay_class: FreshnessDecayClass::Static,
        commit_policy_class: CommitPolicyClass::Reinforce,
        canonical_subject_types: SUBJECTS_ENTITY_OR_MEETING,
        allowed_actor_classes: ACTORS_SYSTEM,
    },
    // --- Pilot context (W5 abilities) -------------------------------
    ClaimTypeMetadata {
        kind: ClaimType::EntityIdentity,
        name: "entity_identity",
        default_temporal_scope: TemporalScope::State,
        default_sensitivity: ClaimSensitivity::Internal,
        freshness_decay_class: FreshnessDecayClass::Slow,
        commit_policy_class: CommitPolicyClass::Replace,
        canonical_subject_types: SUBJECTS_ANY_ENTITY,
        allowed_actor_classes: ACTORS_AGENT,
    },
    ClaimTypeMetadata {
        kind: ClaimType::EntitySummary,
        name: "entity_summary",
        default_temporal_scope: TemporalScope::State,
        default_sensitivity: ClaimSensitivity::Internal,
        freshness_decay_class: FreshnessDecayClass::Slow,
        commit_policy_class: CommitPolicyClass::Replace,
        canonical_subject_types: SUBJECTS_ANY_ENTITY,
        allowed_actor_classes: ACTORS_AGENT,
    },
    ClaimTypeMetadata {
        kind: ClaimType::EntityCurrentState,
        name: "entity_current_state",
        default_temporal_scope: TemporalScope::State,
        default_sensitivity: ClaimSensitivity::Internal,
        freshness_decay_class: FreshnessDecayClass::Slow,
        commit_policy_class: CommitPolicyClass::Replace,
        canonical_subject_types: SUBJECTS_ANY_ENTITY,
        allowed_actor_classes: ACTORS_AGENT,
    },
    ClaimTypeMetadata {
        kind: ClaimType::EntityRisk,
        name: "entity_risk",
        default_temporal_scope: TemporalScope::State,
        default_sensitivity: ClaimSensitivity::Internal,
        freshness_decay_class: FreshnessDecayClass::Medium,
        commit_policy_class: CommitPolicyClass::Fork,
        canonical_subject_types: SUBJECTS_ANY_ENTITY,
        allowed_actor_classes: ACTORS_AGENT,
    },
    ClaimTypeMetadata {
        kind: ClaimType::EntityWin,
        name: "entity_win",
        default_temporal_scope: TemporalScope::State,
        default_sensitivity: ClaimSensitivity::Internal,
        freshness_decay_class: FreshnessDecayClass::Slow,
        commit_policy_class: CommitPolicyClass::Reinforce,
        canonical_subject_types: SUBJECTS_ANY_ENTITY,
        allowed_actor_classes: ACTORS_AGENT,
    },
    ClaimTypeMetadata {
        kind: ClaimType::StakeholderEngagement,
        name: "stakeholder_engagement",
        default_temporal_scope: TemporalScope::State,
        default_sensitivity: ClaimSensitivity::Internal,
        freshness_decay_class: FreshnessDecayClass::Medium,
        commit_policy_class: CommitPolicyClass::Reinforce,
        canonical_subject_types: SUBJECTS_PERSON,
        allowed_actor_classes: ACTORS_AGENT,
    },
    ClaimTypeMetadata {
        kind: ClaimType::StakeholderAssessment,
        name: "stakeholder_assessment",
        default_temporal_scope: TemporalScope::State,
        default_sensitivity: ClaimSensitivity::Confidential,
        freshness_decay_class: FreshnessDecayClass::Medium,
        commit_policy_class: CommitPolicyClass::Reinforce,
        canonical_subject_types: SUBJECTS_PERSON,
        allowed_actor_classes: ACTORS_AGENT,
    },
    ClaimTypeMetadata {
        kind: ClaimType::ValueDelivered,
        name: "value_delivered",
        default_temporal_scope: TemporalScope::State,
        default_sensitivity: ClaimSensitivity::Internal,
        freshness_decay_class: FreshnessDecayClass::Slow,
        commit_policy_class: CommitPolicyClass::Reinforce,
        canonical_subject_types: SUBJECTS_ANY_ENTITY,
        allowed_actor_classes: ACTORS_AGENT,
    },
    ClaimTypeMetadata {
        kind: ClaimType::MeetingReadiness,
        name: "meeting_readiness",
        default_temporal_scope: TemporalScope::State,
        default_sensitivity: ClaimSensitivity::Internal,
        freshness_decay_class: FreshnessDecayClass::Fast,
        commit_policy_class: CommitPolicyClass::Replace,
        canonical_subject_types: SUBJECTS_MEETING,
        allowed_actor_classes: ACTORS_AGENT,
    },
    ClaimTypeMetadata {
        kind: ClaimType::CompanyContext,
        name: "company_context",
        default_temporal_scope: TemporalScope::State,
        default_sensitivity: ClaimSensitivity::Internal,
        freshness_decay_class: FreshnessDecayClass::Slow,
        commit_policy_class: CommitPolicyClass::Reinforce,
        canonical_subject_types: SUBJECTS_ACCOUNT,
        allowed_actor_classes: ACTORS_AGENT,
    },
    ClaimTypeMetadata {
        kind: ClaimType::OpenLoop,
        name: "open_loop",
        default_temporal_scope: TemporalScope::State,
        default_sensitivity: ClaimSensitivity::Internal,
        freshness_decay_class: FreshnessDecayClass::Medium,
        commit_policy_class: CommitPolicyClass::Reinforce,
        canonical_subject_types: SUBJECTS_ENTITY_OR_MEETING,
        allowed_actor_classes: ACTORS_AGENT,
    },
    ClaimTypeMetadata {
        kind: ClaimType::MeetingTopic,
        name: "meeting_topic",
        default_temporal_scope: TemporalScope::State,
        default_sensitivity: ClaimSensitivity::Internal,
        freshness_decay_class: FreshnessDecayClass::EventBound,
        commit_policy_class: CommitPolicyClass::Reinforce,
        canonical_subject_types: SUBJECTS_MEETING,
        allowed_actor_classes: ACTORS_AGENT,
    },
    ClaimTypeMetadata {
        kind: ClaimType::MeetingEventNote,
        name: "meeting_event_note",
        // PointInTime per ADR-0125 — pilots supply occurred_at via the
        // ClaimProposal; the registry default is overridden per claim.
        default_temporal_scope: TemporalScope::PointInTime,
        default_sensitivity: ClaimSensitivity::Internal,
        freshness_decay_class: FreshnessDecayClass::EventBound,
        commit_policy_class: CommitPolicyClass::Reinforce,
        canonical_subject_types: SUBJECTS_MEETING,
        allowed_actor_classes: ACTORS_AGENT,
    },
    ClaimTypeMetadata {
        kind: ClaimType::AttendeeContext,
        name: "attendee_context",
        default_temporal_scope: TemporalScope::State,
        default_sensitivity: ClaimSensitivity::Internal,
        freshness_decay_class: FreshnessDecayClass::EventBound,
        commit_policy_class: CommitPolicyClass::Reinforce,
        canonical_subject_types: SUBJECTS_PERSON,
        allowed_actor_classes: ACTORS_AGENT,
    },
    ClaimTypeMetadata {
        kind: ClaimType::MeetingChangeMarker,
        name: "meeting_change_marker",
        default_temporal_scope: TemporalScope::PointInTime,
        default_sensitivity: ClaimSensitivity::Internal,
        freshness_decay_class: FreshnessDecayClass::EventBound,
        commit_policy_class: CommitPolicyClass::Reinforce,
        canonical_subject_types: SUBJECTS_MEETING,
        allowed_actor_classes: ACTORS_AGENT,
    },
    ClaimTypeMetadata {
        kind: ClaimType::SuggestedOutcome,
        name: "suggested_outcome",
        default_temporal_scope: TemporalScope::State,
        default_sensitivity: ClaimSensitivity::Internal,
        freshness_decay_class: FreshnessDecayClass::EventBound,
        commit_policy_class: CommitPolicyClass::Replace,
        canonical_subject_types: SUBJECTS_MEETING,
        allowed_actor_classes: ACTORS_AGENT,
    },
];

/// Look up a metadata row by canonical persisted name. Returns `None`
/// for unknown strings; `commit_claim` maps that to
/// `ClaimError::UnknownClaimType`.
pub fn metadata_for_name(name: &str) -> Option<&'static ClaimTypeMetadata> {
    CLAIM_TYPE_REGISTRY.iter().find(|m| m.name == name)
}

/// True when a claim of `kind` is permitted on a subject of `subject_kind`
/// (lowercase string). Used by `commit_claim` as the cross-subject bleed
/// guard. Unknown subject_kind returns false (fail closed).
pub fn subject_kind_is_canonical_for(kind: ClaimType, subject_kind: &str) -> bool {
    let meta = metadata_for_claim_type(kind);
    meta.canonical_subject_types
        .iter()
        .any(|s| s.as_str() == subject_kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expected_freshness_decay_class(kind: ClaimType) -> FreshnessDecayClass {
        match kind {
            ClaimType::LinkingDismissed
            | ClaimType::EmailDismissed
            | ClaimType::IntelligenceFieldDismissed
            | ClaimType::FeedbackFieldDismissed
            | ClaimType::MeetingEntityDismissed
            | ClaimType::AccountFieldCorrection
            | ClaimType::DismissedItem
            | ClaimType::BriefingCalloutDismissed
            | ClaimType::NudgeDismissed => FreshnessDecayClass::Static,
            ClaimType::Win
            | ClaimType::StakeholderRole
            | ClaimType::EntityIdentity
            | ClaimType::EntitySummary
            | ClaimType::EntityCurrentState
            | ClaimType::EntityWin
            | ClaimType::ValueDelivered
            | ClaimType::CompanyContext => FreshnessDecayClass::Slow,
            ClaimType::Risk
            | ClaimType::EntityRisk
            | ClaimType::StakeholderEngagement
            | ClaimType::StakeholderAssessment
            | ClaimType::OpenLoop => FreshnessDecayClass::Medium,
            ClaimType::MeetingReadiness => FreshnessDecayClass::Fast,
            ClaimType::TriageSnooze
            | ClaimType::MeetingTopic
            | ClaimType::MeetingEventNote
            | ClaimType::AttendeeContext
            | ClaimType::MeetingChangeMarker
            | ClaimType::SuggestedOutcome => FreshnessDecayClass::EventBound,
        }
    }

    fn expected_commit_policy_class(kind: ClaimType) -> CommitPolicyClass {
        match kind {
            ClaimType::Risk | ClaimType::EntityRisk => CommitPolicyClass::Fork,
            ClaimType::TriageSnooze
            | ClaimType::StakeholderRole
            | ClaimType::EntityIdentity
            | ClaimType::EntitySummary
            | ClaimType::EntityCurrentState
            | ClaimType::MeetingReadiness
            | ClaimType::SuggestedOutcome => CommitPolicyClass::Replace,
            ClaimType::Win
            | ClaimType::LinkingDismissed
            | ClaimType::EmailDismissed
            | ClaimType::IntelligenceFieldDismissed
            | ClaimType::FeedbackFieldDismissed
            | ClaimType::MeetingEntityDismissed
            | ClaimType::AccountFieldCorrection
            | ClaimType::DismissedItem
            | ClaimType::BriefingCalloutDismissed
            | ClaimType::NudgeDismissed
            | ClaimType::EntityWin
            | ClaimType::StakeholderEngagement
            | ClaimType::StakeholderAssessment
            | ClaimType::ValueDelivered
            | ClaimType::CompanyContext
            | ClaimType::OpenLoop
            | ClaimType::MeetingTopic
            | ClaimType::MeetingEventNote
            | ClaimType::AttendeeContext
            | ClaimType::MeetingChangeMarker => CommitPolicyClass::Reinforce,
        }
    }

    #[test]
    fn claim_type_registry_has_unique_names() {
        let mut seen = std::collections::HashSet::new();
        for entry in CLAIM_TYPE_REGISTRY {
            assert!(
                seen.insert(entry.name),
                "duplicate registry name: {}",
                entry.name
            );
        }
    }

    #[test]
    fn claim_type_registry_indices_align_with_enum() {
        // Drift guard for callers that traverse CLAIM_TYPE_REGISTRY
        // as a slice. The direct enum lookup above is independent,
        // but name parsing still depends on every registry row
        // carrying the expected kind/name pair.
        let cases = [
            (ClaimType::Risk, "risk"),
            (ClaimType::Win, "win"),
            (ClaimType::StakeholderRole, "stakeholder_role"),
            (ClaimType::LinkingDismissed, "linking_dismissed"),
            (ClaimType::EmailDismissed, "email_dismissed"),
            (
                ClaimType::IntelligenceFieldDismissed,
                "intelligence_field_dismissed",
            ),
            (ClaimType::FeedbackFieldDismissed, "feedback_field_dismissed"),
            (ClaimType::TriageSnooze, "triage_snooze"),
            (ClaimType::MeetingEntityDismissed, "meeting_entity_dismissed"),
            (ClaimType::AccountFieldCorrection, "account_field_correction"),
            (ClaimType::DismissedItem, "dismissed_item"),
            (ClaimType::BriefingCalloutDismissed, "briefing_callout_dismissed"),
            (ClaimType::NudgeDismissed, "nudge_dismissed"),
            (ClaimType::EntityIdentity, "entity_identity"),
            (ClaimType::EntitySummary, "entity_summary"),
            (ClaimType::EntityCurrentState, "entity_current_state"),
            (ClaimType::EntityRisk, "entity_risk"),
            (ClaimType::EntityWin, "entity_win"),
            (ClaimType::StakeholderEngagement, "stakeholder_engagement"),
            (ClaimType::StakeholderAssessment, "stakeholder_assessment"),
            (ClaimType::ValueDelivered, "value_delivered"),
            (ClaimType::MeetingReadiness, "meeting_readiness"),
            (ClaimType::CompanyContext, "company_context"),
            (ClaimType::OpenLoop, "open_loop"),
            (ClaimType::MeetingTopic, "meeting_topic"),
            (ClaimType::MeetingEventNote, "meeting_event_note"),
            (ClaimType::AttendeeContext, "attendee_context"),
            (ClaimType::MeetingChangeMarker, "meeting_change_marker"),
            (ClaimType::SuggestedOutcome, "suggested_outcome"),
        ];
        for (idx, (kind, expected)) in cases.iter().copied().enumerate() {
            let m = &CLAIM_TYPE_REGISTRY[idx];
            assert_eq!(m.kind, kind);
            assert_eq!(m.name, expected, "registry slice mismatch for {kind:?}");
        }
        assert_eq!(
            cases.len(),
            CLAIM_TYPE_REGISTRY.len(),
            "registry size diverged from coverage list"
        );
    }

    #[test]
    fn try_from_db_str_roundtrip() {
        for entry in CLAIM_TYPE_REGISTRY {
            let parsed = ClaimType::try_from_db_str(entry.name).unwrap();
            assert_eq!(parsed, entry.kind);
            assert_eq!(parsed.as_str(), entry.name);
        }
    }

    #[test]
    fn try_from_db_str_rejects_unknown() {
        let err = ClaimType::try_from_db_str("not_a_real_type").unwrap_err();
        assert_eq!(err.0, "not_a_real_type");
    }

    #[test]
    fn serde_claim_type_rejects_unknown_string() {
        let parsed: Result<ClaimType, _> = serde_json::from_str("\"not_a_real_type\"");
        assert!(parsed.is_err());
    }

    #[test]
    fn subject_kind_canonical_check_accepts_registered_subject() {
        assert!(subject_kind_is_canonical_for(
            ClaimType::StakeholderRole,
            "person"
        ));
        assert!(subject_kind_is_canonical_for(
            ClaimType::AccountFieldCorrection,
            "account"
        ));
        assert!(subject_kind_is_canonical_for(
            ClaimType::EmailDismissed,
            "email"
        ));
    }

    #[test]
    fn linking_dismissed_accepts_email_subject() {
        // Regression: manual_dismiss for owner_type=Email
        // shadow-writes claim_type='linking_dismissed' on the Email
        // subject. The registry must permit this or the new
        // commit_claim canonical-subject guard rejects every email
        // link dismissal.
        assert!(subject_kind_is_canonical_for(
            ClaimType::LinkingDismissed,
            "email"
        ));
        // Other linking_dismissed targets stay valid.
        for k in ["account", "project", "person", "meeting"] {
            assert!(
                subject_kind_is_canonical_for(ClaimType::LinkingDismissed, k),
                "linking_dismissed must permit subject {k}"
            );
        }
    }

    #[test]
    fn backfill_claim_types_are_registered() {
        // The migration 130/131 backfill paths and the
        // claims_backfill m9 hot path write these claim_type
        // strings. They must round-trip through the registry or
        // ClaimType::try_from_db_str rejects rows the migrations
        // already inserted.
        for name in [
            "dismissed_item",
            "briefing_callout_dismissed",
            "nudge_dismissed",
        ] {
            assert!(
                ClaimType::try_from_db_str(name).is_ok(),
                "backfill claim_type {name} missing from registry"
            );
        }
    }

    #[test]
    fn allowed_actor_classes_partition_substrate_correctly() {
        // Pilot context types are agent-only; lifecycle dismissals
        // accept user or system; legacy backfill is system-only.
        // The partition is the W4-C authorization gate input —
        // a test pins the shape so accidental widening doesn't
        // grant agents permission to write dismissal lifecycle
        // rows or vice-versa.
        let agent_only = [
            ClaimType::EntityIdentity,
            ClaimType::EntitySummary,
            ClaimType::EntityCurrentState,
            ClaimType::EntityRisk,
            ClaimType::EntityWin,
            ClaimType::StakeholderEngagement,
            ClaimType::StakeholderAssessment,
            ClaimType::ValueDelivered,
            ClaimType::MeetingReadiness,
            ClaimType::CompanyContext,
            ClaimType::OpenLoop,
            ClaimType::MeetingTopic,
            ClaimType::MeetingEventNote,
            ClaimType::AttendeeContext,
            ClaimType::MeetingChangeMarker,
            ClaimType::SuggestedOutcome,
        ];
        for kind in agent_only {
            let actors = metadata_for_claim_type(kind).allowed_actor_classes;
            assert_eq!(actors.len(), 1, "{kind:?} should be agent-only");
            assert_eq!(actors[0], ClaimActorClass::Agent);
        }

        let system_only = [
            ClaimType::DismissedItem,
            ClaimType::BriefingCalloutDismissed,
            ClaimType::NudgeDismissed,
        ];
        for kind in system_only {
            let actors = metadata_for_claim_type(kind).allowed_actor_classes;
            assert_eq!(actors.len(), 1, "{kind:?} should be system-only");
            assert_eq!(actors[0], ClaimActorClass::System);
        }

        let user_or_system = [
            ClaimType::StakeholderRole,
            ClaimType::LinkingDismissed,
            ClaimType::EmailDismissed,
            ClaimType::IntelligenceFieldDismissed,
            ClaimType::FeedbackFieldDismissed,
            ClaimType::TriageSnooze,
            ClaimType::MeetingEntityDismissed,
            ClaimType::AccountFieldCorrection,
        ];
        for kind in user_or_system {
            let actors = metadata_for_claim_type(kind).allowed_actor_classes;
            assert_eq!(actors.len(), 2, "{kind:?} should be user-or-system");
            assert!(actors.contains(&ClaimActorClass::User));
            assert!(actors.contains(&ClaimActorClass::System));
        }
    }

    #[test]
    fn freshness_decay_class_partition_makes_sense() {
        for entry in CLAIM_TYPE_REGISTRY {
            assert_eq!(
                entry.freshness_decay_class,
                expected_freshness_decay_class(entry.kind),
                "{:?} freshness decay class drifted",
                entry.kind
            );
        }
    }

    #[test]
    fn commit_policy_class_partition_makes_sense() {
        for entry in CLAIM_TYPE_REGISTRY {
            assert_eq!(
                entry.commit_policy_class,
                expected_commit_policy_class(entry.kind),
                "{:?} commit policy class drifted",
                entry.kind
            );
        }
    }

    #[test]
    fn metadata_completeness() {
        let mut has_non_static_freshness = false;
        let mut has_non_reinforce_policy = false;

        for entry in CLAIM_TYPE_REGISTRY {
            assert_eq!(
                entry.freshness_decay_class,
                expected_freshness_decay_class(entry.kind),
                "{:?} must have explicit freshness metadata",
                entry.kind
            );
            assert_eq!(
                entry.commit_policy_class,
                expected_commit_policy_class(entry.kind),
                "{:?} must have explicit commit policy metadata",
                entry.kind
            );
            has_non_static_freshness |= entry.freshness_decay_class != FreshnessDecayClass::Static;
            has_non_reinforce_policy |= entry.commit_policy_class != CommitPolicyClass::Reinforce;
        }

        assert!(
            has_non_static_freshness,
            "registry must not be filled with default freshness metadata"
        );
        assert!(
            has_non_reinforce_policy,
            "registry must not be filled with default commit policy metadata"
        );
    }

    #[test]
    fn subject_kind_canonical_check_rejects_off_subject() {
        // Cross-subject bleed guard: stakeholder_role on an account
        // is rejected because the registry pins it to person only.
        assert!(!subject_kind_is_canonical_for(
            ClaimType::StakeholderRole,
            "account"
        ));
        // Unknown subject_kind fails closed.
        assert!(!subject_kind_is_canonical_for(
            ClaimType::Risk,
            "globaaaal"
        ));
    }

    #[test]
    fn registry_never_includes_global_subject_in_spine() {
        // ADR-0091 spine restriction: no v1.4.0 claim type may be
        // committed on a Global subject. The CanonicalSubjectType
        // enum doesn't have a Global variant, so this is structural —
        // but assert it explicitly so future enum widening doesn't
        // silently lose the restriction.
        for entry in CLAIM_TYPE_REGISTRY {
            for s in entry.canonical_subject_types {
                let label = s.as_str();
                assert_ne!(label, "global", "registry must not allow Global subject");
                assert_ne!(label, "multi", "registry must not allow Multi subject");
            }
        }
    }

    #[test]
    fn defaults_are_conservative_state_internal() {
        // ADR-0125: every claim type defaults to State + Internal
        // unless a pilot specifically needs PointInTime semantics.
        // Two pilot types (meeting_event_note, meeting_change_marker)
        // legitimately default to PointInTime; everything else is
        // State. Sensitivity defaults to Internal except where
        // claim content carries personal context (stakeholder_assessment
        // → Confidential).
        for entry in CLAIM_TYPE_REGISTRY {
            match entry.kind {
                ClaimType::MeetingEventNote | ClaimType::MeetingChangeMarker => {
                    assert!(matches!(
                        entry.default_temporal_scope,
                        TemporalScope::PointInTime
                    ));
                }
                _ => assert!(matches!(entry.default_temporal_scope, TemporalScope::State)),
            }
            match entry.kind {
                ClaimType::StakeholderAssessment => {
                    assert!(matches!(
                        entry.default_sensitivity,
                        ClaimSensitivity::Confidential
                    ));
                }
                _ => assert!(matches!(
                    entry.default_sensitivity,
                    ClaimSensitivity::Internal
                )),
            }
        }
    }
}
