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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

/// Opaque carrier — service-rendered, not interpreted on the wire.
/// Real shape lives in the trust subsystem; this is the wire surface.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RenderedProvenanceSummary {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
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
#[serde(tag = "status", rename_all = "lowercase", rename_all_fields = "camelCase")]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum MeetingSpineType {
    Customer,
    Internal,
    Partner,
    Personal,
    OneOnOne,
    Project,
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

/// Wire mirror of `LinkedEntity` from `src/types/index.ts`.
/// W2 services map the existing Rust `LinkedEntity` (in `types.rs`) to this shape.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LinkedEntityWire {
    pub id: String,
    pub name: String,
    pub entity_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PillView {
    pub label: String,
    pub tone: PillTone,
}

/// Mirrors `PillTone` from `src/components/ui/Pill.tsx`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PillTone {
    Sage,
    Turmeric,
    Terracotta,
    Larkspur,
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

// ─── Service entry point (W0 stub) ───────────────────────────────────────

/// W0 stub. Actual assembly lands in W2 (DOS-414..DOS-419) — each section's
/// service writes into the matching sub-view-model and the orchestrator (this
/// function) composes them.
pub async fn get_briefing_view_model(_state: &AppState) -> BriefingResult {
    BriefingResult::Loading
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
    fn meeting_spine_type_serializes_kebab_case() {
        let v = MeetingSpineType::OneOnOne;
        let s = serde_json::to_string(&v).unwrap();
        assert_eq!(s, "\"one-on-one\"");
    }

    #[test]
    fn meeting_spine_state_serializes_kebab_case() {
        let v = MeetingSpineState::InProgress;
        let s = serde_json::to_string(&v).unwrap();
        assert_eq!(s, "\"in-progress\"");
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
