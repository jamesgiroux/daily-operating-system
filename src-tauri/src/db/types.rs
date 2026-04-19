//! Shared type definitions for the database layer.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

fn default_email_signal_source() -> String {
    "email_enrichment".to_string()
}

/// Errors specific to database operations.
#[derive(Debug, Error)]
pub enum DbError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("Home directory not found")]
    HomeDirNotFound,

    #[error("Failed to create database directory: {0}")]
    CreateDir(std::io::Error),

    #[error("Schema migration failed: {0}")]
    Migration(String),

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("Encryption key missing: database at {db_path} is encrypted but the Keychain entry was not found")]
    KeyMissing { db_path: String },
}

/// A row from the `actions` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbAction {
    pub id: String,
    pub title: String,
    pub priority: i32,
    pub status: String,
    pub created_at: String,
    pub due_date: Option<String>,
    pub completed_at: Option<String>,
    pub account_id: Option<String>,
    pub project_id: Option<String>,
    pub source_type: Option<String>,
    pub source_id: Option<String>,
    pub source_label: Option<String>,
    pub context: Option<String>,
    pub waiting_on: Option<String>,
    pub updated_at: String,
    pub person_id: Option<String>,
    pub account_name: Option<String>,
    /// Next upcoming meeting title for the action's account (I342).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_meeting_title: Option<String>,
    /// Next upcoming meeting start time for the action's account (I342).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_meeting_start: Option<String>,
    /// Whether this action requires a decision (DOS-17).
    #[serde(default)]
    pub needs_decision: bool,
    /// Who owns the decision (DOS-17).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision_owner: Option<String>,
    /// What's at stake if the decision is delayed (DOS-17).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision_stakes: Option<String>,
    /// Linear issue identifier (e.g. "DOS-42") when pushed to Linear (DOS-52).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linear_identifier: Option<String>,
    /// Linear issue URL when pushed to Linear (DOS-52).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linear_url: Option<String>,
}

/// Account classification: customer, internal org, or partner (I382).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum AccountType {
    #[default]
    Customer,
    Internal,
    Partner,
}

impl AccountType {
    /// Parse from the TEXT column stored in SQLite.
    pub fn from_db(s: &str) -> Self {
        match s {
            "internal" => AccountType::Internal,
            "partner" => AccountType::Partner,
            _ => AccountType::Customer,
        }
    }

    /// Value to store in SQLite TEXT column.
    pub fn as_db_str(&self) -> &'static str {
        match self {
            AccountType::Customer => "customer",
            AccountType::Internal => "internal",
            AccountType::Partner => "partner",
        }
    }

    pub fn is_internal(&self) -> bool {
        matches!(self, AccountType::Internal)
    }
}

impl std::fmt::Display for AccountType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_db_str())
    }
}

/// A row from the `accounts` table.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DbAccount {
    pub id: String,
    pub name: String,
    pub lifecycle: Option<String>,
    pub arr: Option<f64>,
    pub health: Option<String>,
    pub contract_start: Option<String>,
    pub contract_end: Option<String>,
    pub nps: Option<i32>,
    pub tracker_path: Option<String>,
    pub parent_id: Option<String>,
    pub account_type: AccountType,
    pub updated_at: String,
    pub archived: bool,
    /// JSON array of auto-extracted keywords for entity resolution (I305).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<String>,
    /// UTC timestamp when keywords were last extracted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords_extracted_at: Option<String>,
    /// JSON metadata for preset-driven custom fields (I311).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>,
    /// I646 C3: Separate commercial opportunity stage (migration 076).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commercial_stage: Option<String>,
    /// I644: ARR range low bound.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arr_range_low: Option<f64>,
    /// I644: ARR range high bound.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arr_range_high: Option<f64>,
    /// I644: Renewal likelihood (0.0–1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub renewal_likelihood: Option<f64>,
    /// I644: Source for renewal_likelihood.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub renewal_likelihood_source: Option<String>,
    /// I644: When renewal_likelihood was last updated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub renewal_likelihood_updated_at: Option<String>,
    /// I644: Renewal model (e.g., "annual", "multi_year").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub renewal_model: Option<String>,
    /// I644: Renewal pricing method (e.g., "flat", "usage_based").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub renewal_pricing_method: Option<String>,
    /// I644: Support tier (e.g., "premium", "standard", "basic").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub support_tier: Option<String>,
    /// I644: Source for support_tier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub support_tier_source: Option<String>,
    /// I644: When support_tier was last updated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub support_tier_updated_at: Option<String>,
    /// I644: Number of active subscriptions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_subscription_count: Option<i32>,
    /// I644: Growth potential score (0.0–1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub growth_potential_score: Option<f64>,
    /// I644: Source for growth_potential_score.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub growth_potential_score_source: Option<String>,
    /// I644: ICP fit score (0.0–1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icp_fit_score: Option<f64>,
    /// I644: Source for icp_fit_score.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icp_fit_score_source: Option<String>,
    /// I644: Primary product name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_product: Option<String>,
    /// I644: Customer status (e.g., "active", "at_risk", "churned").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_status: Option<String>,
    /// I644: Source for customer_status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_status_source: Option<String>,
    /// I644: When customer_status was last updated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_status_updated_at: Option<String>,
    /// I644: Company overview JSON (promoted from dashboard.json).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company_overview: Option<String>,
    /// I644: Strategic programs JSON array (promoted from dashboard.json).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategic_programs: Option<String>,
    /// I644: Free-text notes (promoted from dashboard.json).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// DOS-110: User's manual health sentiment assessment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_health_sentiment: Option<String>,
    /// DOS-110: When the user last set their health sentiment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sentiment_set_at: Option<String>,
}

/// Parameters for writing a source reference row (I644).
#[derive(Debug, Clone)]
pub struct AccountSourceRef<'a> {
    pub account_id: &'a str,
    pub field: &'a str,
    pub source_system: &'a str,
    pub source_kind: &'a str,
    pub source_value: Option<&'a str>,
    pub observed_at: &'a str,
    pub reference_id: Option<&'a str>,
}

/// Provenance metadata for a tracked account field.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbAccountFieldProvenance {
    pub field: String,
    pub source: String,
    pub updated_at: Option<String>,
}

/// A logged automatic lifecycle change for an account.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbLifecycleChange {
    pub id: i64,
    pub account_id: String,
    pub previous_lifecycle: Option<String>,
    pub new_lifecycle: String,
    pub previous_stage: Option<String>,
    pub new_stage: Option<String>,
    pub previous_contract_end: Option<String>,
    pub new_contract_end: Option<String>,
    pub source: String,
    pub confidence: f64,
    pub evidence: Option<String>,
    pub health_score_before: Option<f64>,
    pub health_score_after: Option<f64>,
    pub user_response: String,
    pub response_notes: Option<String>,
    pub created_at: String,
    pub reviewed_at: Option<String>,
}

/// A discovered product or entitlement for an account.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbAccountProduct {
    pub id: i64,
    pub account_id: String,
    pub name: String,
    pub category: Option<String>,
    pub status: String,
    pub arr_portion: Option<f64>,
    pub source: String,
    pub confidence: f64,
    pub notes: Option<String>,
    /// I651: Product classification type (e.g., "cms", "analytics")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_type: Option<String>,
    /// I651: Product tier (e.g., "enhanced", "standard", "basic")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<String>,
    /// I651: Billing terms (e.g., "annual", "monthly", "multi_year")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_terms: Option<String>,
    /// I651: Annual recurring revenue for this product
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arr: Option<f64>,
    /// I651: When this product classification was last verified from Glean
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_verified_at: Option<String>,
    /// I651: Source system for product data (e.g., "salesforce", "glean")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_source: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// A person's stakeholder roles across all their linked accounts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonAccountRole {
    pub account_id: String,
    pub account_name: String,
    pub role: String,
    pub data_source: String,
}

/// A row from `account_stakeholders`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbAccountTeamMember {
    pub account_id: String,
    pub person_id: String,
    pub person_name: String,
    pub person_email: String,
    /// Comma-separated roles from account_stakeholder_roles (I652).
    /// Backward-compatible: callers using `.role.contains("champion")` still work.
    pub role: String,
    pub created_at: String,
}

/// A single role assignment with its provenance (I652).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StakeholderRole {
    pub role: String,
    pub data_source: String,
}

/// Full stakeholder data with data_source for the DB-first read model (I652).
/// Returns ALL stakeholders (user-confirmed + Glean-suggested + Google-sourced)
/// plus linked people from entity_members.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbStakeholderFull {
    pub person_id: String,
    pub person_name: String,
    pub person_email: Option<String>,
    pub organization: Option<String>,
    /// Job title from people table.
    pub person_role: Option<String>,
    /// Comma-separated roles from account_stakeholder_roles.
    /// Backward-compatible field — use `roles` for typed access.
    pub stakeholder_role: String,
    /// Typed multi-role assignments with per-role provenance (I652).
    pub roles: Vec<StakeholderRole>,
    /// Provenance: 'user', 'glean', 'google'.
    pub data_source: String,
    /// Engagement level: strong_advocate, engaged, neutral, disengaged, unknown (I652).
    pub engagement: Option<String>,
    /// Provenance for engagement value (I652).
    pub data_source_engagement: Option<String>,
    /// Free-text assessment of the person's stance (I652).
    pub assessment: Option<String>,
    /// Provenance for assessment value (I652).
    pub data_source_assessment: Option<String>,
    pub last_seen_in_glean: Option<String>,
    pub created_at: String,
    pub linkedin_url: Option<String>,
    pub photo_url: Option<String>,
    pub meeting_count: Option<i64>,
    pub last_seen: Option<String>,
}

/// A pending stakeholder suggestion from the `stakeholder_suggestions` table (I652 phase 2).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StakeholderSuggestionRow {
    pub id: i64,
    pub account_id: String,
    pub person_id: Option<String>,
    pub suggested_name: Option<String>,
    pub suggested_email: Option<String>,
    pub suggested_role: Option<String>,
    pub suggested_engagement: Option<String>,
    pub source: String,
    pub status: String,
    pub created_at: String,
}

/// A row from `account_team_import_notes`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbAccountTeamImportNote {
    pub id: i64,
    pub account_id: String,
    pub legacy_field: String,
    pub legacy_value: String,
    pub note: String,
    pub created_at: String,
}

/// Aggregated signals for a parent account's children (I114).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParentAggregate {
    pub bu_count: usize,
    pub total_arr: Option<f64>,
    pub worst_health: Option<String>,
    pub nearest_renewal: Option<String>,
}

/// Aggregated signals for a parent project's children (I388).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectParentAggregate {
    pub child_count: usize,
    pub active_count: usize,
    pub on_hold_count: usize,
    pub completed_count: usize,
    pub nearest_target_date: Option<String>,
}

/// A row from the `meetings` table (joined with `meeting_prep` and `meeting_transcripts`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbMeeting {
    pub id: String,
    pub title: String,
    pub meeting_type: String,
    pub start_time: String,
    pub end_time: Option<String>,
    pub attendees: Option<String>,
    pub notes_path: Option<String>,
    pub summary: Option<String>,
    pub created_at: String,
    pub calendar_event_id: Option<String>,
    /// Calendar event description (I185). Plumbed from Google Calendar API.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Enriched prep context JSON (I181). Only populated by get_meeting_by_id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prep_context_json: Option<String>,
    /// User-authored agenda items (JSON array).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agenda_json: Option<String>,
    /// User-authored notes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_notes: Option<String>,
    /// Frozen prep JSON captured during archive.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prep_frozen_json: Option<String>,
    /// UTC timestamp when prep was frozen.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prep_frozen_at: Option<String>,
    /// Absolute path to immutable prep snapshot JSON.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prep_snapshot_path: Option<String>,
    /// SHA-256 hash of snapshot payload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prep_snapshot_hash: Option<String>,
    /// Absolute path to transcript destination.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcript_path: Option<String>,
    /// UTC timestamp when transcript was processed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcript_processed_at: Option<String>,
    /// Intelligence lifecycle state (ADR-0081).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intelligence_state: Option<String>,
    /// Intelligence quality level (ADR-0081).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intelligence_quality: Option<String>,
    /// UTC timestamp of last enrichment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_enriched_at: Option<String>,
    /// Number of signals associated with this meeting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal_count: Option<i32>,
    /// Whether new signals have arrived since last view.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_new_signals: Option<i32>,
    /// UTC timestamp of last user view.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_viewed_at: Option<String>,
}

pub struct EnsureMeetingHistoryInput<'a> {
    pub id: &'a str,
    pub title: &'a str,
    pub meeting_type: &'a str,
    pub start_time: &'a str,
    pub end_time: Option<&'a str>,
    pub calendar_event_id: Option<&'a str>,
    pub attendees: Option<&'a str>,
    pub description: Option<&'a str>,
}

/// Outcome of syncing a meeting into the meetings table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeetingSyncOutcome {
    /// Meeting was newly inserted.
    New,
    /// Meeting already existed but title or start_time changed.
    Changed,
    /// Meeting already existed and nothing meaningful changed.
    Unchanged,
}

/// A row from the `processing_log` table.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbProcessingLog {
    pub id: String,
    pub filename: String,
    pub source_path: String,
    pub destination_path: Option<String>,
    pub classification: String,
    pub status: String,
    pub processed_at: Option<String>,
    pub error_message: Option<String>,
    pub created_at: String,
}

/// A row from the `captures` table (post-meeting wins/risks).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbCapture {
    pub id: String,
    pub meeting_id: String,
    pub meeting_title: String,
    pub account_id: Option<String>,
    pub project_id: Option<String>,
    pub capture_type: String,
    pub content: String,
    pub captured_at: String,
}

/// Email-derived intelligence signal linked to an entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbEmailSignal {
    pub id: i64,
    pub email_id: String,
    pub sender_email: Option<String>,
    pub person_id: Option<String>,
    pub entity_id: String,
    pub entity_type: String,
    pub signal_type: String,
    pub signal_text: String,
    pub confidence: Option<f64>,
    pub sentiment: Option<String>,
    pub urgency: Option<String>,
    pub detected_at: String,
    #[serde(default = "default_email_signal_source")]
    pub source: String,
}

/// A row from the `emails` table (I368).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbEmail {
    pub email_id: String,
    pub thread_id: Option<String>,
    pub sender_email: Option<String>,
    pub sender_name: Option<String>,
    pub subject: Option<String>,
    pub snippet: Option<String>,
    pub priority: Option<String>,
    pub is_unread: bool,
    pub received_at: Option<String>,
    pub enrichment_state: String,
    pub enrichment_attempts: i32,
    pub last_enrichment_at: Option<String>,
    /// UTC timestamp when email was last enriched (I652: Gate 0 deduplication).
    pub enriched_at: Option<String>,
    pub last_seen_at: Option<String>,
    pub resolved_at: Option<String>,
    pub entity_id: Option<String>,
    pub entity_type: Option<String>,
    pub contextual_summary: Option<String>,
    pub sentiment: Option<String>,
    pub urgency: Option<String>,
    pub user_is_last_sender: bool,
    pub last_sender_email: Option<String>,
    pub message_count: i32,
    pub created_at: String,
    pub updated_at: String,
    /// Relevance score from scoring pipeline (I395).
    pub relevance_score: Option<f64>,
    /// Human-readable score reason (I395).
    pub score_reason: Option<String>,
    /// When this email was pinned by user for triage sort boost (I579).
    pub pinned_at: Option<String>,
    /// JSON array of extracted commitments (I580).
    pub commitments: Option<String>,
    /// JSON array of extracted questions (I580).
    pub questions: Option<String>,
    /// DOS-242: when true, hide this email from inbox / Records / signal
    /// emission. Set during upsert by `should_suppress_email`. Flip back
    /// to false via `unsuppress_email` rescue command.
    #[serde(default)]
    pub is_noise: bool,
}

/// Email sync statistics for the frontend sync status indicator (I373).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailSyncStats {
    pub last_fetch_at: Option<String>,
    /// DOS-31: Last time the Gmail fetch itself completed successfully,
    /// independent of whether enrichment succeeded. Used to distinguish
    /// "fetch is healthy but enrichment is stuck" from "we can't reach Gmail".
    pub last_successful_fetch_at: Option<String>,
    pub total: i32,
    pub enriched: i32,
    pub pending: i32,
    pub failed: i32,
}

/// Stakeholder relationship signals computed from meeting history and account data (I43).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StakeholderSignals {
    /// Number of meetings in the last 30 days
    pub meeting_frequency_30d: i32,
    /// Number of meetings in the last 90 days
    pub meeting_frequency_90d: i32,
    /// ISO timestamp of the most recent meeting
    pub last_meeting: Option<String>,
    /// ISO timestamp of last account contact (updated_at from accounts table)
    pub last_contact: Option<String>,
    /// Relationship temperature: "hot", "warm", "cool", "cold"
    pub temperature: String,
    /// Trend: "increasing", "stable", "decreasing"
    pub trend: String,
}

/// A row from the `people` table (I51).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbPerson {
    pub id: String,
    pub email: String,
    pub name: String,
    pub organization: Option<String>,
    pub role: Option<String>,
    pub relationship: String, // "internal" | "external" | "unknown"
    pub notes: Option<String>,
    pub tracker_path: Option<String>,
    pub last_seen: Option<String>,
    pub first_seen: Option<String>,
    pub meeting_count: i32,
    pub updated_at: String,
    pub archived: bool,
    // Clay enrichment fields (I228)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linkedin_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub twitter_handle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub photo_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_history: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company_industry: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company_size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company_hq: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_enriched_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enrichment_sources: Option<String>,
}

/// Person-level relationship signals (parallel to StakeholderSignals).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonSignals {
    pub meeting_frequency_30d: i32,
    pub meeting_frequency_90d: i32,
    pub last_meeting: Option<String>,
    pub temperature: String,
    pub trend: String,
}

/// Person with pre-computed signals for list pages (I106).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonListItem {
    pub id: String,
    pub email: String,
    pub name: String,
    pub organization: Option<String>,
    pub role: Option<String>,
    pub relationship: String,
    pub notes: Option<String>,
    pub tracker_path: Option<String>,
    pub last_seen: Option<String>,
    pub first_seen: Option<String>,
    pub meeting_count: i32,
    pub updated_at: String,
    pub archived: bool,
    pub temperature: String,
    pub trend: String,
    /// Comma-separated names of linked accounts (from account_stakeholders).
    pub account_names: Option<String>,
    /// Days since last past meeting (None if never met).
    pub days_since_last_meeting: Option<i64>,
}

/// A row from the `projects` table (I50).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DbProject {
    pub id: String,
    pub name: String,
    pub status: String,
    pub milestone: Option<String>,
    pub owner: Option<String>,
    pub target_date: Option<String>,
    pub tracker_path: Option<String>,
    pub parent_id: Option<String>,
    pub updated_at: String,
    pub archived: bool,
    /// JSON array of auto-extracted keywords for entity resolution (I305).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<String>,
    /// UTC timestamp when keywords were last extracted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords_extracted_at: Option<String>,
    /// JSON metadata for preset-driven custom fields (I311).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>,
    /// I644: Project description (promoted from dashboard.json).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// I644: Project milestones JSON array (promoted from dashboard.json).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub milestones: Option<String>,
    /// I644: Free-text notes (promoted from dashboard.json).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// Activity signals for a project (parallel to StakeholderSignals).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSignals {
    pub meeting_frequency_30d: i32,
    pub meeting_frequency_90d: i32,
    pub last_meeting: Option<String>,
    pub days_until_target: Option<i64>,
    pub open_action_count: i32,
    pub temperature: String,
    pub trend: String,
}

/// A row from the `content_index` table (I124).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbContentFile {
    pub id: String,
    pub entity_id: String,
    pub entity_type: String,
    pub filename: String,
    pub relative_path: String,
    pub absolute_path: String,
    pub format: String,
    pub file_size: i64,
    pub modified_at: String,
    pub indexed_at: String,
    pub extracted_at: Option<String>,
    pub summary: Option<String>,
    pub embeddings_generated_at: Option<String>,
    pub content_type: String,
    pub priority: i32,
}

/// A row from `content_embeddings`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbContentEmbedding {
    pub id: String,
    pub content_file_id: String,
    pub chunk_index: i32,
    pub chunk_text: String,
    pub embedding: Vec<u8>,
    pub created_at: String,
}

/// A row from `chat_sessions`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbChatSession {
    pub id: String,
    pub entity_id: Option<String>,
    pub entity_type: Option<String>,
    pub session_start: String,
    pub session_end: Option<String>,
    pub turn_count: i32,
    pub last_message: Option<String>,
    pub created_at: String,
}

/// A row from `chat_turns`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbChatTurn {
    pub id: String,
    pub session_id: String,
    pub turn_index: i32,
    pub role: String,
    pub content: String,
    pub timestamp: String,
}

/// A lifecycle event for an account (I143 — renewal tracking).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbAccountEvent {
    pub id: i64,
    pub account_id: String,
    pub event_type: String,
    pub event_date: String,
    pub arr_impact: Option<f64>,
    pub notes: Option<String>,
    pub created_at: String,
}

/// A row from the `quill_sync_state` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbQuillSyncState {
    pub id: String,
    pub meeting_id: String,
    pub quill_meeting_id: Option<String>,
    pub state: String,
    pub attempts: i32,
    pub max_attempts: i32,
    pub next_attempt_at: Option<String>,
    pub last_attempt_at: Option<String>,
    pub completed_at: Option<String>,
    pub error_message: Option<String>,
    pub match_confidence: Option<f64>,
    pub transcript_path: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default = "default_quill_source")]
    pub source: String,
}

pub(crate) fn default_quill_source() -> String {
    "quill".to_string()
}

/// Row mapper for quill_sync_state SELECT queries (15 columns including source).
pub(crate) fn map_sync_row(row: &rusqlite::Row) -> rusqlite::Result<DbQuillSyncState> {
    Ok(DbQuillSyncState {
        id: row.get(0)?,
        meeting_id: row.get(1)?,
        quill_meeting_id: row.get(2)?,
        state: row.get(3)?,
        attempts: row.get(4)?,
        max_attempts: row.get(5)?,
        next_attempt_at: row.get(6)?,
        last_attempt_at: row.get(7)?,
        completed_at: row.get(8)?,
        error_message: row.get(9)?,
        match_confidence: row.get(10)?,
        transcript_path: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
        source: row
            .get::<_, String>(14)
            .unwrap_or_else(|_| "quill".to_string()),
    })
}

/// Compute relationship temperature from last meeting date.
pub(crate) fn compute_temperature(last_meeting_iso: &str) -> String {
    let days = days_since_iso(last_meeting_iso);
    match days {
        Some(d) if d < 7 => "hot".to_string(),
        Some(d) if d < 30 => "warm".to_string(),
        Some(d) if d < 60 => "cool".to_string(),
        _ => "cold".to_string(),
    }
}

/// Compute meeting trend from 30d vs 90d frequency.
pub(crate) fn compute_trend(count_30d: i32, count_90d: i32) -> String {
    if count_90d == 0 {
        return "stable".to_string();
    }
    // Expected 30d count is ~1/3 of 90d count (even distribution)
    let expected_30d = count_90d as f64 / 3.0;
    let actual_30d = count_30d as f64;

    if actual_30d > expected_30d * 1.3 {
        "increasing".to_string()
    } else if actual_30d < expected_30d * 0.7 {
        "decreasing".to_string()
    } else {
        "stable".to_string()
    }
}

/// Parse an ISO datetime string and return days since that date.
pub fn days_since_iso(iso: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(iso)
        .or_else(|_| {
            chrono::DateTime::parse_from_rfc3339(&format!("{}+00:00", iso.trim_end_matches('Z')))
        })
        .ok()
        .map(|dt| (Utc::now() - dt.with_timezone(&Utc)).num_days())
}

/// Result of merging two accounts (I198).
#[derive(Debug, serde::Serialize)]
pub struct MergeResult {
    pub actions_moved: usize,
    pub meetings_moved: usize,
    pub people_moved: usize,
    pub events_moved: usize,
    pub children_moved: usize,
}

// ─── I555: Captures metadata + interaction dynamics ──────────────────────────

/// Enriched transcript capture with metadata (I555).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptCapture {
    pub capture_type: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub urgency: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub impact: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_quote: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speaker: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
}

/// Per-meeting interaction dynamics (I555).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InteractionDynamics {
    pub meeting_id: String,
    pub talk_balance_customer_pct: Option<i32>,
    pub talk_balance_internal_pct: Option<i32>,
    #[serde(default)]
    pub speaker_sentiments: Vec<SpeakerSentiment>,
    pub question_density: Option<String>,
    pub decision_maker_active: Option<String>,
    pub forward_looking: Option<String>,
    #[serde(default)]
    pub monologue_risk: bool,
    #[serde(default)]
    pub competitor_mentions: Vec<CompetitorMention>,
    #[serde(default)]
    pub escalation_language: Vec<EscalationQuote>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeakerSentiment {
    pub name: String,
    pub sentiment: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompetitorMention {
    pub competitor: String,
    pub context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EscalationQuote {
    pub quote: String,
    pub speaker: String,
}

/// Per-meeting champion health assessment (I555).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChampionHealthAssessment {
    pub meeting_id: String,
    pub champion_name: Option<String>,
    pub champion_status: String,
    pub champion_evidence: Option<String>,
    pub champion_risk: Option<String>,
}

/// Per-meeting stakeholder role change (I555).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleChange {
    pub id: String,
    pub meeting_id: String,
    pub person_name: String,
    pub old_status: Option<String>,
    pub new_status: Option<String>,
    pub evidence_quote: Option<String>,
}

/// Enriched capture for frontend display (I555/I558).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrichedCapture {
    pub id: String,
    pub meeting_id: String,
    pub meeting_title: String,
    pub account_id: Option<String>,
    pub capture_type: String,
    pub content: String,
    pub sub_type: Option<String>,
    pub urgency: Option<String>,
    pub impact: Option<String>,
    pub evidence_quote: Option<String>,
    pub speaker: Option<String>,
    pub captured_at: String,
}

/// Post-meeting intelligence bundle (I558).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingPostIntelligence {
    pub interaction_dynamics: Option<InteractionDynamics>,
    pub champion_health: Option<ChampionHealthAssessment>,
    pub role_changes: Vec<RoleChange>,
    pub enriched_captures: Vec<EnrichedCapture>,
}

/// A single action item in a continuity thread (I637).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadAction {
    pub title: String,
    pub date: Option<String>,
    pub is_overdue: bool,
}

/// Health score delta between two points in time (I637).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthDelta {
    pub previous: f64,
    pub current: f64,
}

/// Meeting-to-meeting continuity thread: what changed between two meetings
/// with the same entity (I637).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinuityThread {
    pub previous_meeting_date: Option<String>,
    pub previous_meeting_title: Option<String>,
    pub entity_name: Option<String>,
    pub actions_completed: Vec<ThreadAction>,
    pub actions_open: Vec<ThreadAction>,
    pub health_delta: Option<HealthDelta>,
    pub new_attendees: Vec<String>,
    pub is_first_meeting: bool,
}

/// A source reference for a tracked account field (I644).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbAccountSourceRef {
    pub id: String,
    pub account_id: String,
    pub field: String,
    pub source_system: String,
    pub source_kind: String,
    pub source_value: Option<String>,
    pub observed_at: String,
}

/// A row from the `entity_feedback_events` table (I645).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedbackEvent {
    pub id: i64,
    pub entity_id: String,
    pub entity_type: String,
    pub field_key: String,
    pub item_key: Option<String>,
    pub feedback_type: String,
    pub source_system: Option<String>,
    pub source_kind: Option<String>,
    pub previous_value: Option<String>,
    pub corrected_value: Option<String>,
    pub reason: Option<String>,
    pub created_at: String,
}

/// A row from the `suppression_tombstones` table (I645).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuppressionTombstone {
    pub id: i64,
    pub entity_id: String,
    pub field_key: String,
    pub item_key: Option<String>,
    pub item_hash: Option<String>,
    pub dismissed_at: String,
    pub source_scope: Option<String>,
    pub expires_at: Option<String>,
    pub superseded_by_evidence_after: Option<String>,
}

/// Technical footprint, adoption, and service-delivery data for an account (I649).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbAccountTechnicalFootprint {
    pub account_id: String,
    pub integrations_json: Option<String>,
    pub usage_tier: Option<String>,
    pub adoption_score: Option<f64>,
    pub active_users: Option<i64>,
    pub support_tier: Option<String>,
    pub csat_score: Option<f64>,
    pub open_tickets: i64,
    pub services_stage: Option<String>,
    pub source: String,
    pub sourced_at: String,
    pub updated_at: String,
}
