// Daily Briefing view-model contract — Rust wire mirror of `src/types/briefing.ts`.
// See `.docs/decisions/0129-briefing-view-model-contract.md` and DOS-413.
//
// W0 lands the contract types and a `Loading` stub. Real assembly ships in W2
// (DOS-414..DOS-419) where each section's service writes into the matching
// sub-view-model.

#![allow(dead_code)] // W0: types declared, used by W2 services.

use serde::{Deserialize, Serialize};

use crate::json_loader::DataFreshness;
use crate::state::AppState;
use crate::types::GoogleAuthStatus;

// ─── Trust + lifecycle mixins ────────────────────────────────────────────

/// Required trust attribution on every fact-bearing view-model type.
/// Mirrors `TrustMixin` in `src/types/briefing.ts`.
///
/// Rust serde does not have row-polymorphism, so this struct is
/// `#[serde(flatten)]`-included into each carrier instead of inherited.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TrustMixin {
    pub trust_band: TrustBandWire,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trust_field_path: Option<String>,
    /// `null` represents "trust scored without a source date" (allowed by the
    /// wire contract). `None` (omitted) = no trust scoring yet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trust_source_date: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rendered_provenance: Option<RenderedProvenanceSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrustBandWire {
    LikelyCurrent,
    UseWithCaution,
    NeedsVerification,
    Unscored,
}

/// Wire mirror of `RenderedProvenanceSummary` from `src/types/index.ts:146`.
/// The frontend reads sub-fields of `value` directly; the Rust side keeps
/// `value` opaque so producers (the trust subsystem in W2/DOS-411) can shape
/// the sources / field_attributions tree without a type churn here.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RenderedProvenanceSummary {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub surface: Option<String>,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleMixin {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correction_state: Option<CorrectionState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CorrectionState {
    None,
    Corrected,
    Contested,
}

// ─── Top-level envelope ───────────────────────────────────────────────────

/// Wire mirror of TS `BriefingLoadState`. Tauri command returns this.
///
/// `Serialize`-only because the `Success` variant nests `DataFreshness` and
/// `GoogleAuthStatus` from upstream modules that don't derive `Deserialize` /
/// `PartialEq`. Mirrors the `DashboardResult` precedent at
/// `services/dashboard.rs:137`. Tests assert wire shape via `serde_json::Value`
/// rather than full round-trip of the envelope.
#[derive(Debug, Clone, Serialize)]
#[allow(clippy::large_enum_variant)]
#[serde(
    tag = "status",
    rename_all = "lowercase",
    rename_all_fields = "camelCase"
)]
pub enum BriefingResult {
    Loading,
    Error {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        detail_message: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        code: Option<BriefingErrorCode>,
        #[serde(skip_serializing_if = "Option::is_none")]
        service: Option<BriefingSectionId>,
    },
    Empty {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        google_auth: Option<GoogleAuthStatus>,
        #[serde(skip_serializing_if = "Option::is_none")]
        checklist_items: Option<Vec<BriefingEmptyChecklistItem>>,
    },
    Success {
        model: BriefingViewModel,
        freshness: DataFreshness,
        #[serde(skip_serializing_if = "Option::is_none")]
        google_auth: Option<GoogleAuthStatus>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BriefingErrorCode {
    ServiceUnavailable,
    DependencyFailed,
    RateLimited,
    Internal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BriefingSectionId {
    Lead,
    Schedule,
    Predictions,
    Moving,
    Watch,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BriefingEmptyChecklistItem {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ChecklistItemStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ChecklistItemStatus {
    Todo,
    Done,
}

// ─── Top-level model ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BriefingViewModel {
    pub date: BriefingDateViewModel,
    pub folio: BriefingFolioViewModel,
    pub day_strip: DayStripViewModel,
    pub lead: LeadViewModel,
    pub schedule: ScheduleViewModel,
    pub predictions: PredictionsViewModel,
    pub moving: MovingViewModel,
    pub watch: WatchViewModel,
}

// ─── Date ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BriefingDateViewModel {
    pub iso_date: String,
    pub display_date: String,
}

// ─── Folio bar ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BriefingFolioViewModel {
    pub label: String,
    pub crumbs: Vec<String>,
    pub date_label: String,
    pub readiness: Vec<ReadinessPair>,
    pub actions: Vec<FolioActionKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReadinessPair {
    pub label: String,
    pub semantic: ReadinessSemantic,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessSemantic {
    Healthy,
    NeedsAttention,
    InProgress,
    Blocked,
    Neutral,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FolioActionKind {
    Refresh,
    Regenerate,
    Archive,
    Discover,
    New,
}

// ─── DayStrip nav ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DayStripViewModel {
    pub prev: DayStripNeighbor,
    pub current: DayStripCurrent,
    pub next: DayStripNeighbor,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DayStripCurrent {
    pub label: String,
    pub iso_date: String,
    pub aria_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DayStripNeighbor {
    pub label: String,
    pub iso_date: String,
    pub preview: String,
    pub href: String,
}

// ─── Lead ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LeadViewModel {
    pub headline: LeadHeadline,
    pub focus_capacity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus_block: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LeadHeadline {
    pub lead: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub punch_line: Option<String>,
}

// ─── Schedule ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleViewModel {
    pub label: String,
    pub heading: String,
    pub count_label: String,
    pub meeting_mix: ScheduleMeetingMix,
    pub summary: String,
    pub day_chart: DayChartViewModel,
    pub meetings: Vec<ScheduleMeeting>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleMeetingMix {
    pub customer: u32,
    pub partner: u32,
    pub internal: u32,
    pub personal: u32,
    pub one_on_one: u32,
    pub cancelled: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleMeeting {
    #[serde(flatten)]
    pub trust: TrustMixin,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
    pub accent_type: MeetingSpineType,
    pub state: MeetingSpineState,
    pub time: MeetingTimeViewModel,
    pub state_tags: Vec<MeetingStateTag>,
    pub title: String,
    pub eyebrow: ScheduleMeetingEyebrow,
    pub context: String,
    pub attendee_summary: String,
    pub intelligence_quality: IntelligenceQualityView,
    pub briefing_action: BriefingActionView,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleMeetingEyebrow {
    pub entity_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relationship: Option<String>,
}

/// Mirrors `MeetingSpineType` from `src/components/dashboard/MeetingSpineItem.tsx`.
///
/// The wire strings derive from `MeetingType` (`src/types/index.ts:22-32`),
/// which uses snake_case (`"one_on_one"`). Single-word variants happen to be
/// lowercase identical; `OneOnOne` is the one that needs an explicit rename.
/// `Personal` is not a valid `MeetingSpineType` per the TS source — the TS
/// type includes `Extract<MeetingType, "customer" | "internal" | "one_on_one">`
/// plus `"partner"` and `"project"` literals.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MeetingSpineType {
    Customer,
    Internal,
    Partner,
    Project,
    #[serde(rename = "one_on_one")]
    OneOnOne,
}

/// Mirrors `MeetingSpineState` from `src/components/dashboard/MeetingSpineItem.tsx`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum MeetingSpineState {
    Past,
    InProgress,
    Upcoming,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MeetingTimeViewModel {
    pub starts_at_iso: String,
    pub ends_at_iso: String,
    pub start_label: String,
    pub duration_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MeetingStateTag {
    Now,
    Upcoming,
    Ended,
    Cancelled,
    Building,
    NoBriefingYet,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IntelligenceQualityView {
    pub level: IntelligenceQualityLevel,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IntelligenceQualityLevel {
    Fresh,
    Ready,
    Developing,
    Sparse,
    Captured,
    NoBriefing,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum BriefingActionView {
    Link { label: String, href: String },
    Create { label: String },
    None,
}

// ─── DayChart ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DayChartViewModel {
    pub range_start_hour: u32,
    pub range_end_hour: u32,
    pub hour_ticks: Vec<DayChartHourTick>,
    pub legend: Vec<DayChartLegendItem>,
    pub bars: Vec<DayChartBarViewModel>,
    pub now_line: Option<DayChartNowLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DayChartNowLine {
    pub label: String,
    pub left_pct: f64,
    pub iso_time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DayChartHourTick {
    pub label: String,
    pub muted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DayChartLegendItem {
    pub kind: DayChartBarKind,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DayChartBarKind {
    Customer,
    Internal,
    Partner,
    Personal,
    OneOnOne,
    Project,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DayChartBarViewModel {
    pub kind: DayChartBarKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<DayChartBarState>,
    pub layout: DayChartBarLayout,
    pub title: String,
    pub time_label: String,
    pub tooltip: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DayChartBarLayout {
    pub left_pct: f64,
    pub width_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DayChartBarState {
    Past,
    Now,
    Upcoming,
    Cancelled,
}

// ─── Predictions ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PredictionsViewModel {
    pub label: String,
    pub count_label: String,
    pub collapsed_label: String,
    pub expand_hint: String,
    pub count: u32,
    /// Service-capped at ≤10 items.
    pub predictions: Vec<PredictionItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PredictionItem {
    #[serde(flatten)]
    pub trust: TrustMixin,
    pub id: String,
    pub text: String,
    pub confidence: ConfidenceView,
    pub ability_source: AbilitySource,
    pub basis_link: BasisLink,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConfidenceView {
    pub value: f64,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AbilitySource {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BasisLink {
    pub label: String,
    pub href: String,
}

// ─── Moving ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MovingViewModel {
    pub label: String,
    pub heading: String,
    pub count_label: String,
    pub summary: String,
    /// ≤3, service-capped.
    pub entities: Vec<MovingEntityViewModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MovingEntityViewModel {
    pub kind: MovingEntityKind,
    pub entity: LinkedEntityWire,
    pub href: String,
    pub state_pill: PillView,
    pub lede: String,
    /// 3-5, service-ordered.
    pub signals: Vec<MovingSignalViewModel>,
    pub provenance_stats: Vec<ProvenanceStatView>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MovingEntityKind {
    Customer,
    Person,
    Project,
    Internal,
    Lifecycle,
}

/// Wire mirror of `LinkedEntity` from `src/types/index.ts:235-253`.
/// W2 services map the existing Rust `LinkedEntity` (in `types.rs`) to this
/// shape. Routing href is on the parent `MovingEntityViewModel.href`, not here
/// — `LinkedEntity` is identity + linkage metadata, not navigation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LinkedEntityWire {
    pub id: String,
    pub name: String,
    pub entity_type: LinkedEntityType,
    /// Per-junction confidence (0.0 – 1.0). Higher = stronger match.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    /// True if this is the primary entity for the meeting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_primary: Option<bool>,
    /// True for low-confidence siblings rendered as muted suggestions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested: Option<bool>,
    /// Deterministic link role from the entity-linking engine. Supersedes
    /// `is_primary` + `suggested` when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<LinkRole>,
    /// Rule identifier that produced this link (e.g. `"P5"`, `"P9"`).
    /// `Some(None)` represents the wire `null` form (rule absent but
    /// explicitly cleared).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applied_rule: Option<Option<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LinkedEntityType {
    Account,
    Project,
    Person,
}

/// Mirrors `LinkRole` from `src/types/index.ts`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LinkRole {
    Primary,
    Related,
    AutoSuggested,
    UserDismissed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PillView {
    pub label: String,
    pub tone: PillTone,
}

/// Mirrors `PillTone` from `src/components/ui/Pill.tsx`. Seven tones —
/// keep this set in sync with the TS source.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PillTone {
    Sage,
    Turmeric,
    Terracotta,
    Larkspur,
    Olive,
    Eucalyptus,
    Neutral,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MovingSignalViewModel {
    #[serde(flatten)]
    pub trust: TrustMixin,
    #[serde(flatten)]
    pub lifecycle: LifecycleMixin,
    pub kind: SignalDotKind,
    pub when: String,
    pub what_segments: Vec<WhatSegment>,
    pub urgency: SignalUrgency,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_action: Option<ThreadAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WhatSegment {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emphasized: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SignalDotKind {
    Meeting,
    Action,
    Email,
    Lifecycle,
    GongCall,
    ZendeskTicket,
    SlackThread,
    LinearIssue,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SignalUrgency {
    Normal,
    Overdue,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ThreadAction {
    pub label: String,
    pub href: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProvenanceStatView {
    #[serde(flatten)]
    pub trust: TrustMixin,
    pub label: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trend: Option<ProvenanceTrend>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProvenanceTrend {
    Up,
    Down,
    Flat,
}

// ─── Watch ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WatchViewModel {
    pub label: String,
    pub heading: String,
    pub count_label: String,
    pub summary: String,
    pub rows: Vec<WatchRowViewModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum WatchRowViewModel {
    SuggestedAction(WatchSuggestedActionRow),
    OpenAction(WatchOpenActionRow),
    Parked(WatchParkedRow),
    Aging(WatchAgingRow),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WatchSuggestedActionRow {
    #[serde(flatten)]
    pub trust: TrustMixin,
    #[serde(flatten)]
    pub lifecycle: LifecycleMixin,
    pub who: String,
    pub what: String,
    pub action_id: String,
    pub selector: InferredActionSelectorViewModel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WatchOpenActionRow {
    #[serde(flatten)]
    pub trust: TrustMixin,
    pub who: String,
    pub what: String,
    pub action_id: String,
    pub check_button_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WatchParkedRow {
    #[serde(flatten)]
    pub trust: TrustMixin,
    pub who: String,
    pub what: String,
    pub parked_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WatchAgingRow {
    #[serde(flatten)]
    pub trust: TrustMixin,
    pub who: String,
    pub what: String,
    pub action_id: String,
    pub age_label: String,
    pub since: String,
    pub options: Vec<WatchAgingOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WatchAgingOption {
    pub id: WatchAgingOptionId,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WatchAgingOptionId {
    Restore,
    Archive,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InferredActionSelectorViewModel {
    pub trigger_label: String,
    pub options: Vec<InferredActionOption>,
    pub selected_option_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InferredActionOption {
    pub id: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<ConfidenceView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub divider: Option<bool>,
}

// ─── Orchestrator (W2b — composes the 5 W2a slices) ─────────────────────

use chrono::Local;

/// Compose the briefing by running all five section composers concurrently
/// and assembling the envelope.
///
/// Each section composer is non-fallible today and returns an empty branch
/// where upstream data isn't wired (per W2a trust-source declarations).
/// `tokio::join!` is used rather than `try_join!` because the composers
/// don't return `Result`. When live data wiring lands (per-ticket follow-
/// ups), composers that can fail will return `Result` and the orchestrator
/// switches to `try_join!` — at which point the orchestrator owns whether a
/// section failure escalates to `BriefingResult::Error` (with `service:
/// BriefingSectionId`) or degrades the section to its empty branch.
///
/// The chrome slices (`date`, `folio`, `day_strip`) are composed inline —
/// they derive from the current date and a static folio config, no
/// per-section composer needed.
pub async fn get_briefing_view_model(state: &AppState) -> BriefingResult {
    let (lead, schedule, predictions, moving, watch) = tokio::join!(
        crate::services::briefing::lead::compose_lead(state),
        crate::services::briefing::schedule::compose_schedule(state),
        crate::services::briefing::predictions::compose_predictions(state),
        crate::services::briefing::moving::compose_moving(state),
        crate::services::briefing::watch::compose_watch(state),
    );

    let now = Local::now();
    let iso_date = now.format("%Y-%m-%d").to_string();
    let display_date = now.format("%A, %B %-d, %Y").to_string();
    let date_label_caps = display_date.to_uppercase();

    let model = BriefingViewModel {
        date: BriefingDateViewModel {
            iso_date: iso_date.clone(),
            display_date: display_date.clone(),
        },
        folio: BriefingFolioViewModel {
            label: "Daily Briefing".to_string(),
            crumbs: vec!["Daily Briefing".to_string()],
            date_label: date_label_caps,
            readiness: vec![],
            actions: vec![FolioActionKind::Refresh],
            status: None,
        },
        day_strip: compose_day_strip(now),
        lead,
        schedule,
        predictions,
        moving,
        watch,
    };

    BriefingResult::Success {
        model,
        freshness: DataFreshness::Unknown,
        google_auth: None,
    }
}

fn compose_day_strip<Tz: chrono::TimeZone>(now: chrono::DateTime<Tz>) -> DayStripViewModel
where
    Tz::Offset: std::fmt::Display,
{
    let yesterday = now.clone() - chrono::Duration::days(1);
    let tomorrow = now.clone() + chrono::Duration::days(1);
    DayStripViewModel {
        prev: DayStripNeighbor {
            label: "Yesterday".to_string(),
            iso_date: yesterday.format("%Y-%m-%d").to_string(),
            preview: String::new(),
            href: format!("/briefing/{}", yesterday.format("%Y-%m-%d")),
        },
        current: DayStripCurrent {
            label: "Today".to_string(),
            iso_date: now.format("%Y-%m-%d").to_string(),
            aria_label: format!("Today, {}", now.format("%A %B %-d %Y")),
        },
        next: DayStripNeighbor {
            label: "Tomorrow".to_string(),
            iso_date: tomorrow.format("%Y-%m-%d").to_string(),
            preview: String::new(),
            href: format!("/briefing/{}", tomorrow.format("%Y-%m-%d")),
        },
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn sample_trust() -> TrustMixin {
        TrustMixin {
            trust_band: TrustBandWire::Unscored,
            trust_field_path: None,
            trust_source_date: None,
            rendered_provenance: None,
        }
    }

    /// Minimal but exhaustive fixture covering every union variant on the
    /// load-state envelope and every enum used in the success model.
    fn sample_success() -> BriefingResult {
        BriefingResult::Success {
            model: BriefingViewModel {
                date: BriefingDateViewModel {
                    iso_date: "2026-04-23".into(),
                    display_date: "Thursday, April 23, 2026".into(),
                },
                folio: BriefingFolioViewModel {
                    label: "Daily Briefing".into(),
                    crumbs: vec!["Daily Briefing".into()],
                    date_label: "THURSDAY, APRIL 23, 2026".into(),
                    readiness: vec![ReadinessPair {
                        label: "3 briefings ready".into(),
                        semantic: ReadinessSemantic::Healthy,
                    }],
                    actions: vec![FolioActionKind::Refresh, FolioActionKind::Regenerate],
                    status: None,
                },
                day_strip: DayStripViewModel {
                    prev: DayStripNeighbor {
                        label: "Yesterday".into(),
                        iso_date: "2026-04-22".into(),
                        preview: "Quiet day".into(),
                        href: "/briefing/2026-04-22".into(),
                    },
                    current: DayStripCurrent {
                        label: "Today".into(),
                        iso_date: "2026-04-23".into(),
                        aria_label: "Today".into(),
                    },
                    next: DayStripNeighbor {
                        label: "Tomorrow".into(),
                        iso_date: "2026-04-24".into(),
                        preview: "Heavy".into(),
                        href: "/briefing/2026-04-24".into(),
                    },
                },
                lead: LeadViewModel {
                    headline: LeadHeadline {
                        lead: "A focused day.".into(),
                        punch_line: None,
                    },
                    focus_capacity: "3 hours of focus block".into(),
                    focus_block: None,
                },
                schedule: ScheduleViewModel {
                    label: "Today".into(),
                    heading: "Today's schedule".into(),
                    count_label: "0 meetings".into(),
                    meeting_mix: ScheduleMeetingMix::default(),
                    summary: "No meetings.".into(),
                    day_chart: DayChartViewModel {
                        range_start_hour: 8,
                        range_end_hour: 20,
                        hour_ticks: vec![],
                        legend: vec![],
                        bars: vec![],
                        now_line: None,
                    },
                    meetings: vec![],
                },
                predictions: PredictionsViewModel {
                    label: "Predictions".into(),
                    count_label: "0 today".into(),
                    collapsed_label: "0 predictions today".into(),
                    expand_hint: "expand".into(),
                    count: 0,
                    predictions: vec![],
                },
                moving: MovingViewModel {
                    label: "Moving".into(),
                    heading: "What's moving".into(),
                    count_label: "0 entities".into(),
                    summary: "Quiet.".into(),
                    entities: vec![],
                },
                watch: WatchViewModel {
                    label: "Watch".into(),
                    heading: "Worth a look".into(),
                    count_label: "0".into(),
                    summary: "Nothing pressing.".into(),
                    rows: vec![],
                },
            },
            freshness: DataFreshness::Unknown,
            google_auth: None,
        }
    }

    /// Populated fixture exercising every WatchRow variant, every
    /// BriefingActionView kind, non-null RenderedProvenanceSummary, mutation
    /// IDs on every claim-bearing row, and a populated MovingEntity with
    /// signals + provenance stats. Closes the test-coverage gap codex flagged
    /// — the empty `sample_success()` fixture left these unexercised.
    #[test]
    fn populated_fixture_round_trips_every_variant() {
        use serde_json::json as j;

        let trust_with_provenance = TrustMixin {
            trust_band: TrustBandWire::LikelyCurrent,
            trust_field_path: Some("moving.entity.health".into()),
            trust_source_date: Some(Some("2026-04-22T18:00:00Z".into())),
            rendered_provenance: Some(RenderedProvenanceSummary {
                surface: Some("dashboard".into()),
                value: j!({
                    "sources": [{"id": "src_1", "label": "CRM"}],
                    "fieldAttributions": {"health": {"sourceIds": ["src_1"]}},
                    "producedAt": "2026-04-22T18:00:00Z",
                }),
            }),
        };

        let watch_rows = vec![
            WatchRowViewModel::SuggestedAction(WatchSuggestedActionRow {
                trust: sample_trust(),
                lifecycle: LifecycleMixin {
                    correction_state: Some(CorrectionState::None),
                },
                who: "Globex".into(),
                what: "Pushing intro to Q3.".into(),
                action_id: "act_sug".into(),
                selector: InferredActionSelectorViewModel {
                    trigger_label: "Snooze to Q3".into(),
                    options: vec![InferredActionOption {
                        id: "snooze_q3".into(),
                        label: "Snooze to Q3".into(),
                        confidence: Some(ConfidenceView {
                            value: 0.82,
                            label: "82%".into(),
                        }),
                        divider: None,
                    }],
                    selected_option_id: "snooze_q3".into(),
                },
            }),
            WatchRowViewModel::OpenAction(WatchOpenActionRow {
                trust: sample_trust(),
                who: "Acme".into(),
                what: "Send pricing appendix.".into(),
                action_id: "act_open".into(),
                check_button_label: "Mark complete".into(),
            }),
            WatchRowViewModel::Parked(WatchParkedRow {
                trust: sample_trust(),
                who: "Internal".into(),
                what: "Tier 3 deck circulating.".into(),
                parked_label: "Parked".into(),
            }),
            WatchRowViewModel::Aging(WatchAgingRow {
                trust: sample_trust(),
                who: "Stark".into(),
                what: "Old support thread.".into(),
                action_id: "act_age".into(),
                age_label: "2w".into(),
                since: "2026-04-09".into(),
                options: vec![
                    WatchAgingOption {
                        id: WatchAgingOptionId::Restore,
                        label: "Restore".into(),
                    },
                    WatchAgingOption {
                        id: WatchAgingOptionId::Archive,
                        label: "Archive".into(),
                    },
                ],
            }),
        ];

        let moving_entity = MovingEntityViewModel {
            kind: MovingEntityKind::Customer,
            entity: LinkedEntityWire {
                id: "ent_1".into(),
                name: "Globex".into(),
                entity_type: LinkedEntityType::Account,
                confidence: Some(0.94),
                is_primary: Some(true),
                suggested: None,
                role: Some(LinkRole::Primary),
                applied_rule: Some(Some("P5".into())),
            },
            href: "/accounts/ent_1".into(),
            state_pill: PillView {
                label: "Renewal ↑".into(),
                tone: PillTone::Olive,
            },
            lede: "Pricing memo went out.".into(),
            signals: vec![MovingSignalViewModel {
                trust: trust_with_provenance.clone(),
                lifecycle: LifecycleMixin {
                    correction_state: Some(CorrectionState::Corrected),
                },
                kind: SignalDotKind::GongCall,
                when: "10:00".into(),
                what_segments: vec![
                    WhatSegment {
                        text: "Call recorded with ".into(),
                        emphasized: None,
                    },
                    WhatSegment {
                        text: "champion".into(),
                        emphasized: Some(true),
                    },
                ],
                urgency: SignalUrgency::Overdue,
                thread_action: Some(ThreadAction {
                    label: "→ thread".into(),
                    href: "/threads/abc".into(),
                }),
            }],
            provenance_stats: vec![ProvenanceStatView {
                trust: sample_trust(),
                label: "Health".into(),
                value: "71 +3".into(),
                trend: Some(ProvenanceTrend::Up),
            }],
        };

        let prediction = PredictionItem {
            trust: sample_trust(),
            id: "pred_1".into(),
            text: "Northwind QBR raises pricing pushback.".into(),
            confidence: ConfidenceView {
                value: 0.72,
                label: "72%".into(),
            },
            ability_source: AbilitySource {
                id: "predict_meeting_friction".into(),
                label: "predict_meeting_friction".into(),
            },
            basis_link: BasisLink {
                label: "basis".into(),
                href: "/predictions/pred_1".into(),
            },
        };

        let mut model = match sample_success() {
            BriefingResult::Success { model, .. } => model,
            _ => unreachable!(),
        };
        model.watch.rows = watch_rows;
        model.moving.entities = vec![moving_entity];
        model.predictions.predictions = vec![prediction];
        model.predictions.count = 1;
        model.schedule.meetings = vec![ScheduleMeeting {
            trust: sample_trust(),
            id: "mtg_1".into(),
            href: Some("/meetings/mtg_1".into()),
            accent_type: MeetingSpineType::OneOnOne,
            state: MeetingSpineState::Upcoming,
            time: MeetingTimeViewModel {
                starts_at_iso: "2026-04-23T10:00:00Z".into(),
                ends_at_iso: "2026-04-23T10:30:00Z".into(),
                start_label: "10:00".into(),
                duration_label: "30m".into(),
            },
            state_tags: vec![MeetingStateTag::Upcoming, MeetingStateTag::NoBriefingYet],
            title: "1:1 with Jen".into(),
            eyebrow: ScheduleMeetingEyebrow {
                entity_name: "Jen Park".into(),
                relationship: Some("Internal".into()),
            },
            context: "Recurring sync.".into(),
            attendee_summary: "You and Jen".into(),
            intelligence_quality: IntelligenceQualityView {
                level: IntelligenceQualityLevel::NoBriefing,
                label: "No briefing".into(),
            },
            briefing_action: BriefingActionView::Create {
                label: "Create briefing".into(),
            },
        }];

        let result = BriefingResult::Success {
            model,
            freshness: DataFreshness::Unknown,
            google_auth: None,
        };

        let parsed: Value = serde_json::from_str(&serde_json::to_string(&result).unwrap()).unwrap();

        // Watch row variants — all 4 with their kind discriminator.
        let rows = parsed["model"]["watch"]["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0]["kind"], "suggestedAction");
        assert_eq!(rows[0]["actionId"], "act_sug");
        assert_eq!(rows[0]["correctionState"], "none");
        assert_eq!(rows[1]["kind"], "openAction");
        assert_eq!(rows[1]["actionId"], "act_open");
        assert_eq!(rows[2]["kind"], "parked");
        assert_eq!(rows[3]["kind"], "aging");
        assert_eq!(rows[3]["actionId"], "act_age");
        assert_eq!(rows[3]["options"][0]["id"], "restore");
        assert_eq!(rows[3]["options"][1]["id"], "archive");

        // Selector confidence on suggested action.
        assert_eq!(
            rows[0]["selector"]["options"][0]["confidence"]["label"],
            "82%"
        );

        // Moving entity — LinkedEntity full shape, hyphenated SignalDot kind,
        // populated provenance.
        let entity = &parsed["model"]["moving"]["entities"][0];
        assert_eq!(entity["kind"], "customer");
        assert_eq!(entity["entity"]["entityType"], "account");
        assert_eq!(entity["entity"]["confidence"], 0.94);
        assert_eq!(entity["entity"]["isPrimary"], true);
        assert_eq!(entity["entity"]["role"], "primary");
        assert_eq!(entity["entity"]["appliedRule"], "P5");
        assert!(
            entity["entity"].get("href").is_none(),
            "LinkedEntity must NOT have href"
        );
        assert_eq!(entity["statePill"]["tone"], "olive");

        let signal = &entity["signals"][0];
        assert_eq!(signal["kind"], "gong-call");
        assert_eq!(signal["urgency"], "overdue");
        assert_eq!(signal["correctionState"], "corrected");
        assert_eq!(signal["whatSegments"][1]["emphasized"], true);
        assert_eq!(signal["threadAction"]["label"], "→ thread");
        // Trust + populated provenance summary flow through.
        assert_eq!(signal["trustBand"], "likely_current");
        assert_eq!(signal["renderedProvenance"]["surface"], "dashboard");
        assert_eq!(
            signal["renderedProvenance"]["value"]["sources"][0]["id"],
            "src_1"
        );

        // Prediction id + ability source.
        let pred = &parsed["model"]["predictions"]["predictions"][0];
        assert_eq!(pred["id"], "pred_1");
        assert_eq!(pred["abilitySource"]["id"], "predict_meeting_friction");

        // Schedule meeting — accentType uses underscore (the C1 fix), state
        // tags include NoBriefingYet (snake_case), BriefingActionView::Create
        // exercises the Create variant.
        let mtg = &parsed["model"]["schedule"]["meetings"][0];
        assert_eq!(mtg["accentType"], "one_on_one");
        assert_eq!(mtg["state"], "upcoming");
        assert_eq!(mtg["stateTags"][1], "no_briefing_yet");
        assert_eq!(mtg["briefingAction"]["kind"], "create");
        assert_eq!(mtg["briefingAction"]["label"], "Create briefing");
    }

    #[test]
    fn serializes_loading() {
        let v = BriefingResult::Loading;
        let parsed: Value = serde_json::from_str(&serde_json::to_string(&v).unwrap()).unwrap();
        assert_eq!(parsed, json!({"status": "loading"}));
    }

    #[test]
    fn serializes_error() {
        let v = BriefingResult::Error {
            message: "Briefing unavailable".into(),
            detail_message: Some("Try again shortly".into()),
            code: Some(BriefingErrorCode::DependencyFailed),
            service: Some(BriefingSectionId::Predictions),
        };
        let parsed: Value = serde_json::from_str(&serde_json::to_string(&v).unwrap()).unwrap();
        assert_eq!(parsed["status"], "error");
        assert_eq!(parsed["message"], "Briefing unavailable");
        assert_eq!(parsed["detailMessage"], "Try again shortly");
        assert_eq!(parsed["code"], "dependency_failed");
        assert_eq!(parsed["service"], "predictions");
    }

    #[test]
    fn serializes_empty_with_checklist() {
        let v = BriefingResult::Empty {
            message: "Connect Google".into(),
            google_auth: None,
            checklist_items: Some(vec![BriefingEmptyChecklistItem {
                label: "Connect Google".into(),
                status: Some(ChecklistItemStatus::Todo),
            }]),
        };
        let parsed: Value = serde_json::from_str(&serde_json::to_string(&v).unwrap()).unwrap();
        assert_eq!(parsed["status"], "empty");
        assert_eq!(parsed["checklistItems"][0]["label"], "Connect Google");
        assert_eq!(parsed["checklistItems"][0]["status"], "todo");
    }

    #[test]
    fn serializes_success_minimal() {
        let v = sample_success();
        let parsed: Value = serde_json::from_str(&serde_json::to_string(&v).unwrap()).unwrap();
        assert_eq!(parsed["status"], "success");
        assert_eq!(parsed["model"]["date"]["isoDate"], "2026-04-23");
        assert_eq!(parsed["model"]["folio"]["label"], "Daily Briefing");
        assert_eq!(parsed["model"]["folio"]["actions"][0], "refresh");
    }

    #[test]
    fn signal_dot_kind_serializes_kebab_case() {
        let v = SignalDotKind::GongCall;
        let s = serde_json::to_string(&v).unwrap();
        assert_eq!(s, "\"gong-call\"");
        let v2: SignalDotKind = serde_json::from_str("\"zendesk-ticket\"").unwrap();
        assert_eq!(v2, SignalDotKind::ZendeskTicket);
    }

    #[test]
    fn meeting_spine_type_one_on_one_uses_underscore() {
        // Wire string must match `MeetingType` source-of-truth at
        // `src/types/index.ts:22-32` which uses `"one_on_one"`.
        let v = MeetingSpineType::OneOnOne;
        assert_eq!(serde_json::to_string(&v).unwrap(), "\"one_on_one\"");
        let v2: MeetingSpineType = serde_json::from_str("\"one_on_one\"").unwrap();
        assert_eq!(v, v2);
    }

    #[test]
    fn meeting_spine_state_serializes_kebab_case() {
        let v = MeetingSpineState::InProgress;
        let s = serde_json::to_string(&v).unwrap();
        assert_eq!(s, "\"in-progress\"");
    }

    /// Parameterized literal-string check across every wire enum. Walks each
    /// variant against the exact TS literal it claims to mirror, so a mismatch
    /// (rename rule wrong, missing variant, typo) fails the closest test.
    /// Class-level coverage — keeps W2 services from flowing wrong strings.
    #[test]
    fn enum_wire_strings_match_ts_source() {
        fn check<T: Serialize>(value: T, expected: &str, what: &str) {
            let s = serde_json::to_string(&value).unwrap();
            assert_eq!(s, format!("\"{}\"", expected), "{} wrong wire string", what);
        }

        // TrustBandWire (snake_case)
        check(
            TrustBandWire::LikelyCurrent,
            "likely_current",
            "TrustBandWire::LikelyCurrent",
        );
        check(
            TrustBandWire::UseWithCaution,
            "use_with_caution",
            "TrustBandWire::UseWithCaution",
        );
        check(
            TrustBandWire::NeedsVerification,
            "needs_verification",
            "TrustBandWire::NeedsVerification",
        );
        check(
            TrustBandWire::Unscored,
            "unscored",
            "TrustBandWire::Unscored",
        );

        // CorrectionState (lowercase)
        check(CorrectionState::None, "none", "CorrectionState::None");
        check(
            CorrectionState::Corrected,
            "corrected",
            "CorrectionState::Corrected",
        );
        check(
            CorrectionState::Contested,
            "contested",
            "CorrectionState::Contested",
        );

        // BriefingErrorCode (snake_case)
        check(
            BriefingErrorCode::ServiceUnavailable,
            "service_unavailable",
            "BriefingErrorCode::ServiceUnavailable",
        );
        check(
            BriefingErrorCode::DependencyFailed,
            "dependency_failed",
            "BriefingErrorCode::DependencyFailed",
        );
        check(
            BriefingErrorCode::RateLimited,
            "rate_limited",
            "BriefingErrorCode::RateLimited",
        );
        check(
            BriefingErrorCode::Internal,
            "internal",
            "BriefingErrorCode::Internal",
        );

        // BriefingSectionId (snake_case)
        check(BriefingSectionId::Lead, "lead", "BriefingSectionId::Lead");
        check(
            BriefingSectionId::Schedule,
            "schedule",
            "BriefingSectionId::Schedule",
        );
        check(
            BriefingSectionId::Predictions,
            "predictions",
            "BriefingSectionId::Predictions",
        );
        check(
            BriefingSectionId::Moving,
            "moving",
            "BriefingSectionId::Moving",
        );
        check(
            BriefingSectionId::Watch,
            "watch",
            "BriefingSectionId::Watch",
        );

        // ChecklistItemStatus (lowercase)
        check(
            ChecklistItemStatus::Todo,
            "todo",
            "ChecklistItemStatus::Todo",
        );
        check(
            ChecklistItemStatus::Done,
            "done",
            "ChecklistItemStatus::Done",
        );

        // ReadinessSemantic (snake_case)
        check(
            ReadinessSemantic::Healthy,
            "healthy",
            "ReadinessSemantic::Healthy",
        );
        check(
            ReadinessSemantic::NeedsAttention,
            "needs_attention",
            "ReadinessSemantic::NeedsAttention",
        );
        check(
            ReadinessSemantic::InProgress,
            "in_progress",
            "ReadinessSemantic::InProgress",
        );
        check(
            ReadinessSemantic::Blocked,
            "blocked",
            "ReadinessSemantic::Blocked",
        );
        check(
            ReadinessSemantic::Neutral,
            "neutral",
            "ReadinessSemantic::Neutral",
        );

        // FolioActionKind (lowercase)
        check(
            FolioActionKind::Refresh,
            "refresh",
            "FolioActionKind::Refresh",
        );
        check(
            FolioActionKind::Regenerate,
            "regenerate",
            "FolioActionKind::Regenerate",
        );
        check(
            FolioActionKind::Archive,
            "archive",
            "FolioActionKind::Archive",
        );
        check(
            FolioActionKind::Discover,
            "discover",
            "FolioActionKind::Discover",
        );
        check(FolioActionKind::New, "new", "FolioActionKind::New");

        // MeetingSpineType — tested separately above (one_on_one needs explicit rename)
        check(
            MeetingSpineType::Customer,
            "customer",
            "MeetingSpineType::Customer",
        );
        check(
            MeetingSpineType::Internal,
            "internal",
            "MeetingSpineType::Internal",
        );
        check(
            MeetingSpineType::Partner,
            "partner",
            "MeetingSpineType::Partner",
        );
        check(
            MeetingSpineType::Project,
            "project",
            "MeetingSpineType::Project",
        );

        // MeetingSpineState (kebab-case)
        check(MeetingSpineState::Past, "past", "MeetingSpineState::Past");
        check(
            MeetingSpineState::InProgress,
            "in-progress",
            "MeetingSpineState::InProgress",
        );
        check(
            MeetingSpineState::Upcoming,
            "upcoming",
            "MeetingSpineState::Upcoming",
        );
        check(
            MeetingSpineState::Cancelled,
            "cancelled",
            "MeetingSpineState::Cancelled",
        );

        // MeetingStateTag (snake_case)
        check(MeetingStateTag::Now, "now", "MeetingStateTag::Now");
        check(
            MeetingStateTag::Upcoming,
            "upcoming",
            "MeetingStateTag::Upcoming",
        );
        check(MeetingStateTag::Ended, "ended", "MeetingStateTag::Ended");
        check(
            MeetingStateTag::Cancelled,
            "cancelled",
            "MeetingStateTag::Cancelled",
        );
        check(
            MeetingStateTag::Building,
            "building",
            "MeetingStateTag::Building",
        );
        check(
            MeetingStateTag::NoBriefingYet,
            "no_briefing_yet",
            "MeetingStateTag::NoBriefingYet",
        );

        // IntelligenceQualityLevel (snake_case)
        check(
            IntelligenceQualityLevel::Fresh,
            "fresh",
            "IntelligenceQualityLevel::Fresh",
        );
        check(
            IntelligenceQualityLevel::Ready,
            "ready",
            "IntelligenceQualityLevel::Ready",
        );
        check(
            IntelligenceQualityLevel::Developing,
            "developing",
            "IntelligenceQualityLevel::Developing",
        );
        check(
            IntelligenceQualityLevel::Sparse,
            "sparse",
            "IntelligenceQualityLevel::Sparse",
        );
        check(
            IntelligenceQualityLevel::Captured,
            "captured",
            "IntelligenceQualityLevel::Captured",
        );
        check(
            IntelligenceQualityLevel::NoBriefing,
            "no_briefing",
            "IntelligenceQualityLevel::NoBriefing",
        );

        // DayChartBarKind (camelCase)
        check(
            DayChartBarKind::Customer,
            "customer",
            "DayChartBarKind::Customer",
        );
        check(
            DayChartBarKind::Internal,
            "internal",
            "DayChartBarKind::Internal",
        );
        check(
            DayChartBarKind::Partner,
            "partner",
            "DayChartBarKind::Partner",
        );
        check(
            DayChartBarKind::Personal,
            "personal",
            "DayChartBarKind::Personal",
        );
        check(
            DayChartBarKind::OneOnOne,
            "oneOnOne",
            "DayChartBarKind::OneOnOne",
        );
        check(
            DayChartBarKind::Project,
            "project",
            "DayChartBarKind::Project",
        );
        check(
            DayChartBarKind::Cancelled,
            "cancelled",
            "DayChartBarKind::Cancelled",
        );

        // DayChartBarState (lowercase)
        check(DayChartBarState::Past, "past", "DayChartBarState::Past");
        check(DayChartBarState::Now, "now", "DayChartBarState::Now");
        check(
            DayChartBarState::Upcoming,
            "upcoming",
            "DayChartBarState::Upcoming",
        );
        check(
            DayChartBarState::Cancelled,
            "cancelled",
            "DayChartBarState::Cancelled",
        );

        // MovingEntityKind (lowercase)
        check(
            MovingEntityKind::Customer,
            "customer",
            "MovingEntityKind::Customer",
        );
        check(
            MovingEntityKind::Person,
            "person",
            "MovingEntityKind::Person",
        );
        check(
            MovingEntityKind::Project,
            "project",
            "MovingEntityKind::Project",
        );
        check(
            MovingEntityKind::Internal,
            "internal",
            "MovingEntityKind::Internal",
        );
        check(
            MovingEntityKind::Lifecycle,
            "lifecycle",
            "MovingEntityKind::Lifecycle",
        );

        // PillTone (lowercase) — 7 tones, must match src/components/ui/Pill.tsx
        check(PillTone::Sage, "sage", "PillTone::Sage");
        check(PillTone::Turmeric, "turmeric", "PillTone::Turmeric");
        check(PillTone::Terracotta, "terracotta", "PillTone::Terracotta");
        check(PillTone::Larkspur, "larkspur", "PillTone::Larkspur");
        check(PillTone::Olive, "olive", "PillTone::Olive");
        check(PillTone::Eucalyptus, "eucalyptus", "PillTone::Eucalyptus");
        check(PillTone::Neutral, "neutral", "PillTone::Neutral");

        // SignalDotKind (kebab-case) — 8 kinds
        check(SignalDotKind::Meeting, "meeting", "SignalDotKind::Meeting");
        check(SignalDotKind::Action, "action", "SignalDotKind::Action");
        check(SignalDotKind::Email, "email", "SignalDotKind::Email");
        check(
            SignalDotKind::Lifecycle,
            "lifecycle",
            "SignalDotKind::Lifecycle",
        );
        check(
            SignalDotKind::GongCall,
            "gong-call",
            "SignalDotKind::GongCall",
        );
        check(
            SignalDotKind::ZendeskTicket,
            "zendesk-ticket",
            "SignalDotKind::ZendeskTicket",
        );
        check(
            SignalDotKind::SlackThread,
            "slack-thread",
            "SignalDotKind::SlackThread",
        );
        check(
            SignalDotKind::LinearIssue,
            "linear-issue",
            "SignalDotKind::LinearIssue",
        );

        // SignalUrgency (lowercase)
        check(SignalUrgency::Normal, "normal", "SignalUrgency::Normal");
        check(SignalUrgency::Overdue, "overdue", "SignalUrgency::Overdue");

        // ProvenanceTrend (lowercase)
        check(ProvenanceTrend::Up, "up", "ProvenanceTrend::Up");
        check(ProvenanceTrend::Down, "down", "ProvenanceTrend::Down");
        check(ProvenanceTrend::Flat, "flat", "ProvenanceTrend::Flat");

        // WatchAgingOptionId (lowercase)
        check(
            WatchAgingOptionId::Restore,
            "restore",
            "WatchAgingOptionId::Restore",
        );
        check(
            WatchAgingOptionId::Archive,
            "archive",
            "WatchAgingOptionId::Archive",
        );
    }

    #[test]
    fn trust_source_date_serializes_three_states() {
        // Wire contract is `string | null` (optional). Three serialized forms.
        let omitted = TrustMixin {
            trust_band: TrustBandWire::Unscored,
            trust_field_path: None,
            trust_source_date: None,
            rendered_provenance: None,
        };
        let parsed: Value =
            serde_json::from_str(&serde_json::to_string(&omitted).unwrap()).unwrap();
        assert!(
            parsed.get("trustSourceDate").is_none(),
            "None should omit field"
        );

        let null_value = TrustMixin {
            trust_band: TrustBandWire::Unscored,
            trust_field_path: None,
            trust_source_date: Some(None),
            rendered_provenance: None,
        };
        let parsed: Value =
            serde_json::from_str(&serde_json::to_string(&null_value).unwrap()).unwrap();
        assert_eq!(
            parsed["trustSourceDate"],
            Value::Null,
            "Some(None) should serialize as null"
        );

        let with_value = TrustMixin {
            trust_band: TrustBandWire::Unscored,
            trust_field_path: None,
            trust_source_date: Some(Some("2026-04-23".into())),
            rendered_provenance: None,
        };
        let parsed: Value =
            serde_json::from_str(&serde_json::to_string(&with_value).unwrap()).unwrap();
        assert_eq!(parsed["trustSourceDate"], "2026-04-23");
    }

    #[test]
    fn watch_row_uses_kind_tag() {
        let v = WatchRowViewModel::Parked(WatchParkedRow {
            trust: sample_trust(),
            who: "Internal".into(),
            what: "Tier 3 deck circulating".into(),
            parked_label: "Parked".into(),
        });
        let s = serde_json::to_string(&v).unwrap();
        let parsed: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed["kind"], "parked");
        assert_eq!(parsed["who"], "Internal");
        let v2: WatchRowViewModel = serde_json::from_str(&s).unwrap();
        assert_eq!(v, v2);
    }

    #[test]
    fn briefing_action_view_uses_kind_tag() {
        let link = BriefingActionView::Link {
            label: "View".into(),
            href: "/foo".into(),
        };
        let parsed: Value = serde_json::from_str(&serde_json::to_string(&link).unwrap()).unwrap();
        assert_eq!(parsed["kind"], "link");

        let none = BriefingActionView::None;
        let parsed: Value = serde_json::from_str(&serde_json::to_string(&none).unwrap()).unwrap();
        assert_eq!(parsed["kind"], "none");
    }

    #[test]
    fn trust_mixin_flattens_into_carrier() {
        let stat = ProvenanceStatView {
            trust: sample_trust(),
            label: "Health".into(),
            value: "71 +3".into(),
            trend: Some(ProvenanceTrend::Up),
        };
        let parsed: Value = serde_json::from_str(&serde_json::to_string(&stat).unwrap()).unwrap();
        assert_eq!(parsed["trustBand"], "unscored");
        assert_eq!(parsed["label"], "Health");
        assert_eq!(parsed["trend"], "up");
        assert!(parsed.get("trust").is_none(), "TrustMixin must flatten");
    }
}
