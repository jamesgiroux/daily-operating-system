use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Owner (the thing being linked — a meeting, email, or email thread)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnerType {
    Meeting,
    Email,
    EmailThread,
}

impl OwnerType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Meeting => "meeting",
            Self::Email => "email",
            Self::EmailThread => "email_thread",
        }
    }
}

impl TryFrom<&str> for OwnerType {
    type Error = String;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "meeting" => Ok(Self::Meeting),
            "email" => Ok(Self::Email),
            "email_thread" => Ok(Self::EmailThread),
            other => Err(format!("unknown owner_type: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnerRef {
    pub owner_type: OwnerType,
    pub owner_id: String,
}

// ---------------------------------------------------------------------------
// Entity reference
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRef {
    pub entity_id: String,
    pub entity_type: String, // "account" | "person" | "project"
}

// ---------------------------------------------------------------------------
// Link role — mirrors the DB CHECK constraint on linked_entities_raw.role
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkRole {
    Primary,
    Related,
    AutoSuggested,
}

impl LinkRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Primary => "primary",
            Self::Related => "related",
            Self::AutoSuggested => "auto_suggested",
        }
    }
}

// ---------------------------------------------------------------------------
// Tier — coarse classification of the meeting/email shape (C5)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkTier {
    Entity,  // account or partner primary
    Person,  // person primary
    Minimal, // personal / 1:1 internal with no account evidence
    Skip,    // broadcast / suppressed — do not enrich
}

// ---------------------------------------------------------------------------
// Participants — input facts for Phase 2 and Phase 3
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParticipantRole {
    From,
    To,
    Cc,
    ReplyTo,
    Attendee,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Participant {
    pub email: String,
    pub name: Option<String>,
    pub role: ParticipantRole,
    /// Resolved person_id if find_or_create_person has been called for this email.
    pub person_id: Option<String>,
    /// Domain extracted from email (e.g. "acme.com"). Cached here so rules
    /// don't repeat the extraction on every candidate check.
    pub domain: Option<String>,
}

// ---------------------------------------------------------------------------
// Linking context — the input to evaluate()
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkingContext {
    pub owner: OwnerRef,
    pub participants: Vec<Participant>,
    /// Meeting title or email subject. Used by P5 (title/subject evidence).
    pub title: Option<String>,
    /// Raw attendee/recipient count, which may be higher than participants.len()
    /// for large events where the adapter caps the participant list.
    pub attendee_count: usize,
    /// Email thread_id — populated by the email adapter for P2 inheritance.
    pub thread_id: Option<String>,
    /// Calendar series master event id — populated by the calendar adapter for P3.
    pub series_id: Option<String>,
    /// Snapshot of entity_graph_version.version read at the start of evaluate().
    /// Stored in entity_linking_evaluations for staleness detection.
    pub graph_version: i64,
    /// User's own domains (from app config). Used by rules to classify
    /// participants as internal or external.
    pub user_domains: Vec<String>,
}

impl LinkingContext {
    /// True if the email address belongs to the user's own organisation.
    pub fn is_internal_email(&self, email: &str) -> bool {
        let Some(domain) = email.rsplit_once('@').map(|(_, d)| d) else {
            return false;
        };
        self.user_domains
            .iter()
            .any(|ud| ud.eq_ignore_ascii_case(domain))
    }

    pub fn internal_participants(&self) -> impl Iterator<Item = &Participant> {
        self.participants
            .iter()
            .filter(|p| self.is_internal_email(&p.email))
    }

    pub fn external_participants(&self) -> impl Iterator<Item = &Participant> {
        self.participants
            .iter()
            .filter(|p| !self.is_internal_email(&p.email))
    }

    /// The From-role participant (email surface only).
    pub fn from_participant(&self) -> Option<&Participant> {
        self.participants
            .iter()
            .find(|p| p.role == ParticipantRole::From)
    }

    /// True when there are exactly 2 participants (1:1 shape).
    pub fn is_one_on_one(&self) -> bool {
        self.participants.len() == 2
    }
}

// ---------------------------------------------------------------------------
// Rule output — produced by each Rule::evaluate() call
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub entity: EntityRef,
    pub role: LinkRole,
    pub confidence: f64,
    pub rule_id: String,
    /// Full evidence blob — matched text, rejected candidates, parent email id,
    /// domain candidates, phase outputs (ADR-0105 / Codex finding 14).
    pub evidence: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleOutcome {
    /// This rule matched and produced a candidate. Phase 3 stops here.
    Matched(Candidate),
    /// This rule did not apply. Continue to the next rule.
    Skip,
}

// ---------------------------------------------------------------------------
// Link outcome — the output of evaluate()
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkOutcome {
    pub owner: OwnerRef,
    pub primary: Option<EntityRef>,
    pub related: Vec<EntityRef>,
    pub tier: LinkTier,
    /// Rule id that produced the primary, e.g. "P4a". None when primary = none.
    pub applied_rule: Option<String>,
}

// ---------------------------------------------------------------------------
// Trigger — what caused this evaluation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Trigger {
    CalendarPoll,
    CalendarResolverSweep,
    CalendarUserEdit,
    EmailFetch,
    EmailThreadUpdate,
    EmailUserEdit,
    AccountRelinked,
    EntityGraphChange,
}

impl Trigger {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CalendarPoll => "CalendarPoll",
            Self::CalendarResolverSweep => "CalendarResolverSweep",
            Self::CalendarUserEdit => "CalendarUserEdit",
            Self::EmailFetch => "EmailFetch",
            Self::EmailThreadUpdate => "EmailThreadUpdate",
            Self::EmailUserEdit => "EmailUserEdit",
            Self::AccountRelinked => "AccountRelinked",
            Self::EntityGraphChange => "EntityGraphChange",
        }
    }
}
