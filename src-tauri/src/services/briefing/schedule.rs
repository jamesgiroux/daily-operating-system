//! Schedule composer - produces the `ScheduleViewModel` slice.
//!
//! Trust source: existing dashboard data flow (`services::dashboard`). The
//! composer calls `get_dashboard_data` and reshapes the meeting list from the
//! Google Calendar ingestion pipeline into the redesign schedule view-model.
//! Meeting trust comes from active meeting-level claims when a scored source exists.

use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::BTreeMap;

use chrono::{
    DateTime, Datelike, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Timelike,
    Utc, Weekday,
};
use serde::{Deserialize, Serialize};

use crate::abilities::claims::ClaimType;
use crate::abilities::provenance::trust::claim_trust_band_from_score;
use crate::abilities::trust::TrustBand;
use crate::db::claims::IntelligenceClaim;
use crate::db::ActionDb;
use crate::services::briefing_view_model::{
    BriefingActionView, DayChartBarKind, DayChartBarLayout, DayChartBarState, DayChartBarViewModel,
    DayChartHourTick, DayChartLegendItem, DayChartNowLine, DayChartViewModel,
    IntelligenceQualityLevel, IntelligenceQualityView, MeetingSpineState, MeetingSpineType,
    MeetingStateTag, MeetingTimeViewModel, ScheduleMeeting, ScheduleMeetingEyebrow,
    ScheduleMeetingMix, ScheduleViewModel, TrustBandWire, TrustMixin,
};
use crate::services::claims::load_claims_active;
use crate::services::dashboard::{get_dashboard_data, DashboardResult};
use crate::state::AppState;
use crate::types::{Meeting, MeetingType, OverlayStatus, TimelineMeeting};

const RANGE_START_HOUR: u32 = 8;
const RANGE_END_HOUR: u32 = 20;
const ESTIMATED_DURATION_MINUTES: i64 = 45;
const MIN_BAR_WIDTH_PCT: f64 = 1.25;

thread_local! {
    static SCHEDULE_TRUST_BANDS: RefCell<BTreeMap<String, TrustBandWire>> = const { RefCell::new(BTreeMap::new()) };
}

pub async fn compose_schedule(state: &AppState) -> ScheduleViewModel {
    let result = get_dashboard_data(state).await;
    let meetings: Vec<Meeting> = match result {
        DashboardResult::Success { data, .. } => data.meetings,
        _ => vec![],
    };
    let trust_bands = if meetings.is_empty() {
        BTreeMap::new()
    } else {
        match state.with_db_read(|db| load_schedule_trust_bands(&meetings, db)) {
            Ok(trust_bands) => trust_bands,
            Err(error) => {
                log::warn!("schedule: failed to load meeting trust bands: {error}");
                BTreeMap::new()
            }
        }
    };

    let ctx = state.live_service_context();
    with_schedule_trust_bands(trust_bands, || {
        compose_schedule_from_meetings(meetings, ctx.clock.now())
    })
}

fn compose_schedule_from_meetings(meetings: Vec<Meeting>, now: DateTime<Utc>) -> ScheduleViewModel {
    let mix = compute_meeting_mix(&meetings);
    let count = meetings.len();
    let count_label = format_count_label(count);
    let summary = format_summary(count, &mix);

    let mut analyzed: Vec<(usize, Meeting, TemporalInfo)> = meetings
        .into_iter()
        .enumerate()
        .map(|(idx, meeting)| {
            let temporal = analyze_meeting_time(&meeting, now);
            (idx, meeting, temporal)
        })
        .collect();

    analyzed.sort_by(|(left_idx, _, left), (right_idx, _, right)| {
        match (left.start, right.start) {
            (Some(a), Some(b)) => a.cmp(&b).then_with(|| left_idx.cmp(right_idx)),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => left_idx.cmp(right_idx),
        }
    });

    let view_meetings: Vec<ScheduleMeeting> = analyzed
        .iter()
        .map(|(_, meeting, temporal)| {
            let mut view = map_meeting(meeting.clone());
            apply_temporal_fields(&mut view, meeting, temporal);
            view
        })
        .collect();

    ScheduleViewModel {
        label: "Today".to_string(),
        heading: "Today's schedule".to_string(),
        count_label,
        meeting_mix: mix,
        summary,
        day_chart: build_day_chart(&analyzed, now),
        meetings: view_meetings,
    }
}

pub async fn compose_week_schedule_shape(
    state: &AppState,
    days_before: i64,
    days_after: i64,
    now: DateTime<Utc>,
) -> Result<WeekScheduleShape, String> {
    let timeline =
        crate::services::meetings::get_meeting_timeline(state, Some(days_before), Some(days_after))
            .await?;
    Ok(compose_week_schedule_shape_from_timeline(
        timeline,
        days_before,
        days_after,
        now,
    ))
}

pub fn compose_week_schedule_shape_from_timeline(
    timeline: Vec<TimelineMeeting>,
    days_before: i64,
    days_after: i64,
    now: DateTime<Utc>,
) -> WeekScheduleShape {
    let now_local = now.with_timezone(&Local);
    let today = now_local.date_naive();
    let start_date = today - Duration::days(days_before);
    let end_date = today + Duration::days(days_after);

    let mut filtered = Vec::new();
    for meeting in timeline {
        if let Some(local_start) = parse_timeline_local_start(&meeting.start_time) {
            let date = local_start.date_naive();
            if date >= start_date && date <= end_date {
                filtered.push(meeting);
            }
        }
    }

    let shape_days = derive_shape_from_timeline(&filtered, today);

    WeekScheduleShape {
        week_meta: compute_week_meta(today),
        shape_epigraph: compute_shape_epigraph(&shape_days),
        shape_days,
        timeline_groups: group_timeline_by_date(&filtered, today),
        readiness_stats: compute_week_readiness_stats(&filtered, now, false),
        folio_readiness_stats: compute_week_readiness_stats(&filtered, now, true),
        future_meeting_count: filtered
            .iter()
            .filter(|m| parse_timeline_utc_start(&m.start_time).is_some_and(|start| start > now))
            .count(),
        timeline: filtered,
    }
}

#[derive(Debug, Clone)]
struct TemporalInfo {
    start: Option<DateTime<Utc>>,
    end_real: Option<DateTime<Utc>>,
    end_for_math: Option<DateTime<Utc>>,
    state: MeetingSpineState,
}

fn analyze_meeting_time(meeting: &Meeting, now: DateTime<Utc>) -> TemporalInfo {
    let start = parse_rfc3339_utc(meeting.start_iso.as_deref());
    let end_real = start.and_then(|start| derive_end_time(start, meeting.end_time.as_deref()));
    let end_for_math = start
        .map(|start| end_real.unwrap_or(start + Duration::minutes(ESTIMATED_DURATION_MINUTES)));
    let state = classify_temporal_state(meeting, start, end_for_math, now);

    TemporalInfo {
        start,
        end_real,
        end_for_math,
        state,
    }
}

fn classify_temporal_state(
    meeting: &Meeting,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> MeetingSpineState {
    if meeting.overlay_status == Some(OverlayStatus::Cancelled) {
        return MeetingSpineState::Cancelled;
    }

    match (start, end) {
        (Some(start), Some(end)) if start <= now && now < end => MeetingSpineState::InProgress,
        (Some(_), Some(end)) if end <= now => MeetingSpineState::Past,
        (Some(start), _) if now < start => MeetingSpineState::Upcoming,
        _ => MeetingSpineState::Upcoming,
    }
}

fn apply_temporal_fields(view: &mut ScheduleMeeting, meeting: &Meeting, temporal: &TemporalInfo) {
    view.state = temporal.state.clone();
    view.time = build_time_view_model(meeting, temporal);
    view.state_tags = build_state_tags(&temporal.state, meeting);
}

fn build_time_view_model(meeting: &Meeting, temporal: &TemporalInfo) -> MeetingTimeViewModel {
    let starts_at_iso = temporal
        .start
        .map(|_| meeting.start_iso.clone().unwrap_or_default())
        .unwrap_or_default();
    let ends_at_iso = temporal
        .end_real
        .map(|end| end.to_rfc3339())
        .unwrap_or_default();
    let start_label = temporal
        .start
        .map(format_time_label)
        .unwrap_or_else(|| meeting.time.clone());
    let duration_label = temporal
        .end_real
        .and_then(|end| temporal.start.map(|start| (start, end)))
        .map(|(start, end)| format_duration_label((end - start).num_minutes()))
        .unwrap_or_default();

    MeetingTimeViewModel {
        starts_at_iso,
        ends_at_iso,
        start_label,
        duration_label,
    }
}

fn parse_rfc3339_utc(value: Option<&str>) -> Option<DateTime<Utc>> {
    value
        .and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
        .map(|dt| dt.with_timezone(&Utc))
}

fn derive_end_time(start: DateTime<Utc>, end_time: Option<&str>) -> Option<DateTime<Utc>> {
    let raw = end_time?.trim();
    if raw.is_empty() {
        return None;
    }

    if let Ok(end) = DateTime::parse_from_rfc3339(raw) {
        let end = end.with_timezone(&Utc);
        return (end > start).then_some(end);
    }

    if let Some(end) = parse_naive_datetime_as_local(raw) {
        let end = end.with_timezone(&Utc);
        return (end > start).then_some(end);
    }

    let start_local = start.with_timezone(&Local);
    let end_time = parse_display_time(raw)?;
    let end_naive = start_local.date_naive().and_time(end_time);
    let end_local = localize_naive(end_naive)?;
    let mut end = end_local.with_timezone(&Utc);
    if end <= start && end_time <= start_local.time() {
        end += Duration::days(1);
    }
    (end > start).then_some(end)
}

fn parse_naive_datetime_as_local(value: &str) -> Option<DateTime<Local>> {
    ["%Y-%m-%dT%H:%M:%S", "%Y-%m-%d %H:%M:%S"]
        .iter()
        .find_map(|fmt| NaiveDateTime::parse_from_str(value, fmt).ok())
        .and_then(localize_naive)
}

fn parse_display_time(value: &str) -> Option<NaiveTime> {
    let upper = value.trim().to_uppercase();
    ["%-I:%M %p", "%I:%M %p", "%H:%M"]
        .iter()
        .find_map(|fmt| NaiveTime::parse_from_str(&upper, fmt).ok())
}

fn localize_naive(naive: NaiveDateTime) -> Option<DateTime<Local>> {
    Local
        .from_local_datetime(&naive)
        .single()
        .or_else(|| Local.from_local_datetime(&naive).earliest())
}

fn format_time_label(dt: DateTime<Utc>) -> String {
    dt.with_timezone(&Local).format("%-I:%M %p").to_string()
}

fn format_duration_label(minutes: i64) -> String {
    if minutes < 60 {
        return format!("{}m", minutes.max(0));
    }
    let hours = minutes / 60;
    let rem = minutes % 60;
    if rem > 0 {
        format!("{}h {}m", hours, rem)
    } else {
        format!("{}h", hours)
    }
}

fn compute_meeting_mix(meetings: &[Meeting]) -> ScheduleMeetingMix {
    let mut mix = ScheduleMeetingMix::default();
    for m in meetings {
        if m.overlay_status == Some(OverlayStatus::Cancelled) {
            mix.cancelled += 1;
            continue;
        }

        match m.meeting_type {
            MeetingType::Customer | MeetingType::Qbr | MeetingType::Training => {
                mix.customer += 1;
            }
            MeetingType::Internal | MeetingType::TeamSync | MeetingType::AllHands => {
                mix.internal += 1;
            }
            MeetingType::OneOnOne => mix.one_on_one += 1,
            MeetingType::Partnership | MeetingType::External => mix.partner += 1,
            MeetingType::Personal => mix.personal += 1,
        }
    }
    mix
}

fn format_count_label(count: usize) -> String {
    match count {
        0 => "0 meetings".to_string(),
        1 => "1 meeting".to_string(),
        n => format!("{} meetings", n),
    }
}

fn format_summary(count: usize, mix: &ScheduleMeetingMix) -> String {
    if count == 0 {
        return "No meetings today.".to_string();
    }
    let mut segments: Vec<String> = Vec::new();
    if mix.customer > 0 {
        segments.push(format!("{} customer", mix.customer));
    }
    if mix.internal > 0 {
        segments.push(format!("{} internal", mix.internal));
    }
    if mix.one_on_one > 0 {
        segments.push(format!("{} 1:1", mix.one_on_one));
    }
    if mix.partner > 0 {
        segments.push(format!("{} partner", mix.partner));
    }
    if mix.personal > 0 {
        segments.push(format!("{} personal", mix.personal));
    }
    if mix.cancelled > 0 {
        segments.push(format!("{} cancelled", mix.cancelled));
    }
    if segments.is_empty() {
        return format!("{} meetings today.", count);
    }
    format!("{}.", segments.join(" · "))
}

fn with_schedule_trust_bands<T>(
    trust_bands: BTreeMap<String, TrustBandWire>,
    f: impl FnOnce() -> T,
) -> T {
    SCHEDULE_TRUST_BANDS.with(|context| {
        let previous = context.replace(trust_bands);
        let result = f();
        context.replace(previous);
        result
    })
}

#[derive(Debug, Clone)]
struct MeetingTrustClaim {
    claim_type: String,
    trust_score: Option<f64>,
}

impl From<&IntelligenceClaim> for MeetingTrustClaim {
    fn from(claim: &IntelligenceClaim) -> Self {
        Self {
            claim_type: claim.claim_type.clone(),
            trust_score: claim.trust_score,
        }
    }
}

fn load_schedule_trust_bands(
    meetings: &[Meeting],
    db: &ActionDb,
) -> Result<BTreeMap<String, TrustBandWire>, String> {
    let mut trust_bands = BTreeMap::new();
    for meeting in meetings
        .iter()
        .filter(|meeting| !meeting.id.trim().is_empty())
    {
        let subject_ref = serde_json::json!({
            "kind": "meeting",
            "id": meeting.id.as_str(),
        })
        .to_string();
        let claims = load_claims_active(db, &subject_ref, None).map_err(|e| e.to_string())?;
        let claims: Vec<MeetingTrustClaim> = claims.iter().map(MeetingTrustClaim::from).collect();
        if let Some(trust) = select_meeting_trust(&claims) {
            trust_bands.insert(meeting.id.clone(), trust);
        }
    }
    Ok(trust_bands)
}

fn related_meeting_trust_claim_types() -> [&'static str; 4] {
    [
        ClaimType::MeetingTopic.as_str(),
        ClaimType::MeetingChangeMarker.as_str(),
        ClaimType::SuggestedOutcome.as_str(),
        ClaimType::OpenLoop.as_str(),
    ]
}

fn select_meeting_trust(claims: &[MeetingTrustClaim]) -> Option<TrustBandWire> {
    let readiness_type = ClaimType::MeetingReadiness.as_str();
    if claims
        .iter()
        .any(|claim| claim.claim_type == readiness_type)
    {
        return claims
            .iter()
            .filter(|claim| claim.claim_type == readiness_type)
            .find_map(trust_band_for_claim);
    }

    let related_types = related_meeting_trust_claim_types();
    claims
        .iter()
        .filter(|claim| related_types.contains(&claim.claim_type.as_str()))
        .filter_map(|claim| claim_trust_band(claim).map(|band| (claim, band)))
        .fold(None, |selected, (claim, band)| match selected {
            Some((selected_claim, selected_band))
                if trust_caution_rank(selected_band) <= trust_caution_rank(band) =>
            {
                Some((selected_claim, selected_band))
            }
            _ => Some((claim, band)),
        })
        .map(|(_, band)| trust_band_wire(band))
}

fn load_trust_band_for_meeting(meeting: &Meeting) -> TrustBandWire {
    SCHEDULE_TRUST_BANDS
        .with(|bands| bands.borrow().get(&meeting.id).cloned())
        .unwrap_or(TrustBandWire::Unscored)
}

fn trust_band_for_claim(claim: &MeetingTrustClaim) -> Option<TrustBandWire> {
    claim_trust_band(claim).map(trust_band_wire)
}

fn claim_trust_band(claim: &MeetingTrustClaim) -> Option<TrustBand> {
    match claim_trust_band_from_score(claim.trust_score) {
        TrustBand::LikelyCurrent => Some(TrustBand::LikelyCurrent),
        TrustBand::UseWithCaution => Some(TrustBand::UseWithCaution),
        TrustBand::NeedsVerification => Some(TrustBand::NeedsVerification),
        TrustBand::Unscored => None,
    }
}

fn trust_caution_rank(band: TrustBand) -> u8 {
    match band {
        TrustBand::NeedsVerification => 0,
        TrustBand::UseWithCaution => 1,
        TrustBand::LikelyCurrent => 2,
        TrustBand::Unscored => 3,
    }
}

fn trust_band_wire(band: TrustBand) -> TrustBandWire {
    match band {
        TrustBand::LikelyCurrent => TrustBandWire::LikelyCurrent,
        TrustBand::UseWithCaution => TrustBandWire::UseWithCaution,
        TrustBand::NeedsVerification => TrustBandWire::NeedsVerification,
        TrustBand::Unscored => TrustBandWire::Unscored,
    }
}

fn map_meeting(m: Meeting) -> ScheduleMeeting {
    let accent_type = map_meeting_type(&m.meeting_type);
    let state = if m.overlay_status == Some(OverlayStatus::Cancelled) {
        MeetingSpineState::Cancelled
    } else if m.is_current.unwrap_or(false) {
        MeetingSpineState::InProgress
    } else {
        MeetingSpineState::Upcoming
    };
    let starts_at_iso = m.start_iso.clone().unwrap_or_default();
    let title = m.title.clone();
    ScheduleMeeting {
        trust: TrustMixin {
            trust_band: load_trust_band_for_meeting(&m),
            trust_field_path: None,
            trust_source_date: None,
            rendered_provenance: None,
        },
        id: m.id.clone(),
        href: Some(format!("/meetings/{}", m.id)),
        accent_type,
        state: state.clone(),
        time: MeetingTimeViewModel {
            starts_at_iso,
            ends_at_iso: String::new(),
            start_label: m.time.clone(),
            duration_label: String::new(),
        },
        state_tags: build_state_tags(&state, &m),
        title,
        eyebrow: ScheduleMeetingEyebrow {
            entity_name: extract_entity_name(&m),
            relationship: None,
        },
        context: m
            .prep
            .as_ref()
            .and_then(|p| p.context.clone())
            .unwrap_or_default(),
        attendee_summary: String::new(),
        intelligence_quality: IntelligenceQualityView {
            level: if m.prep.is_some() {
                IntelligenceQualityLevel::Ready
            } else {
                IntelligenceQualityLevel::NoBriefing
            },
            label: if m.prep.is_some() {
                "Ready"
            } else {
                "No briefing"
            }
            .to_string(),
        },
        briefing_action: if m.prep.is_some() {
            BriefingActionView::Link {
                label: "Open briefing".to_string(),
                href: format!("/meetings/{}", m.id),
            }
        } else {
            BriefingActionView::Create {
                label: "Create briefing".to_string(),
            }
        },
    }
}

fn map_meeting_type(ty: &MeetingType) -> MeetingSpineType {
    match ty {
        MeetingType::Customer | MeetingType::Qbr | MeetingType::Training => {
            MeetingSpineType::Customer
        }
        MeetingType::Internal | MeetingType::TeamSync | MeetingType::AllHands => {
            MeetingSpineType::Internal
        }
        MeetingType::OneOnOne => MeetingSpineType::OneOnOne,
        MeetingType::Partnership | MeetingType::External => MeetingSpineType::Partner,
        MeetingType::Personal => MeetingSpineType::Internal,
    }
}

fn build_state_tags(state: &MeetingSpineState, m: &Meeting) -> Vec<MeetingStateTag> {
    let mut tags = Vec::new();
    match state {
        MeetingSpineState::InProgress => tags.push(MeetingStateTag::Now),
        MeetingSpineState::Upcoming => tags.push(MeetingStateTag::Upcoming),
        MeetingSpineState::Past => tags.push(MeetingStateTag::Ended),
        MeetingSpineState::Cancelled => tags.push(MeetingStateTag::Cancelled),
    }
    if m.prep.is_none() {
        tags.push(MeetingStateTag::NoBriefingYet);
    }
    tags
}

fn extract_entity_name(m: &Meeting) -> String {
    m.prep
        .as_ref()
        .and_then(|p| p.stakeholders.as_ref())
        .and_then(|s| s.first())
        .map(|stake| stake.name.clone())
        .unwrap_or_default()
}

fn build_day_chart(
    analyzed: &[(usize, Meeting, TemporalInfo)],
    now: DateTime<Utc>,
) -> DayChartViewModel {
    DayChartViewModel {
        range_start_hour: RANGE_START_HOUR,
        range_end_hour: RANGE_END_HOUR,
        hour_ticks: build_hour_ticks(),
        legend: build_day_chart_legend(analyzed),
        bars: build_day_chart_bars(analyzed, now),
        now_line: build_now_line(now),
    }
}

fn build_hour_ticks() -> Vec<DayChartHourTick> {
    (RANGE_START_HOUR..=RANGE_END_HOUR)
        .step_by(2)
        .map(|hour| DayChartHourTick {
            label: format_hour_tick(hour),
            muted: false,
        })
        .collect()
}

fn format_hour_tick(hour: u32) -> String {
    match hour {
        0 => "12 AM".to_string(),
        1..=11 => format!("{} AM", hour),
        12 => "12 PM".to_string(),
        _ => format!("{} PM", hour - 12),
    }
}

fn build_day_chart_legend(analyzed: &[(usize, Meeting, TemporalInfo)]) -> Vec<DayChartLegendItem> {
    let present: Vec<DayChartBarKind> = analyzed
        .iter()
        .filter(|(_, _, temporal)| temporal.start.is_some())
        .map(|(_, meeting, _)| map_day_chart_kind(meeting))
        .fold(Vec::new(), |mut kinds, kind| {
            if !kinds.contains(&kind) {
                kinds.push(kind);
            }
            kinds
        });

    canonical_day_chart_kinds()
        .into_iter()
        .filter(|kind| present.contains(kind))
        .map(|kind| DayChartLegendItem {
            label: day_chart_kind_label(&kind).to_string(),
            kind,
        })
        .collect()
}

fn build_day_chart_bars(
    analyzed: &[(usize, Meeting, TemporalInfo)],
    now: DateTime<Utc>,
) -> Vec<DayChartBarViewModel> {
    let schedule_date = now.with_timezone(&Local).date_naive();
    analyzed
        .iter()
        .filter_map(|(_, meeting, temporal)| build_day_chart_bar(meeting, temporal, schedule_date))
        .collect()
}

fn build_day_chart_bar(
    meeting: &Meeting,
    temporal: &TemporalInfo,
    schedule_date: NaiveDate,
) -> Option<DayChartBarViewModel> {
    let start = temporal.start?;
    let end = temporal.end_for_math?;
    let total_minutes = i64::from(RANGE_END_HOUR - RANGE_START_HOUR) * 60;
    let range_start = i64::from(RANGE_START_HOUR) * 60;
    let start_offset = local_minutes_from_schedule_midnight(start, schedule_date) - range_start;
    let end_offset = local_minutes_from_schedule_midnight(end, schedule_date) - range_start;

    if end_offset <= 0 || start_offset >= total_minutes {
        return None;
    }

    let visible_start = start_offset.clamp(0, total_minutes);
    let visible_end = end_offset.clamp(0, total_minutes);
    if visible_end <= visible_start {
        return None;
    }

    let left_pct = (visible_start as f64 / total_minutes as f64 * 100.0).clamp(0.0, 100.0);
    let true_width_pct =
        ((visible_end - visible_start) as f64 / total_minutes as f64 * 100.0).clamp(0.0, 100.0);
    let width_pct = true_width_pct.max(MIN_BAR_WIDTH_PCT).min(100.0 - left_pct);
    let time_label = build_chart_time_label(temporal);
    let tooltip = build_chart_tooltip(&meeting.title, &time_label, temporal);

    Some(DayChartBarViewModel {
        kind: map_day_chart_kind(meeting),
        state: Some(map_day_chart_state(&temporal.state)),
        layout: DayChartBarLayout {
            left_pct,
            width_pct,
        },
        title: meeting.title.clone(),
        time_label,
        tooltip,
    })
}

fn local_minutes_from_schedule_midnight(dt: DateTime<Utc>, schedule_date: NaiveDate) -> i64 {
    let local = dt.with_timezone(&Local);
    let day_delta = local
        .date_naive()
        .signed_duration_since(schedule_date)
        .num_days();
    day_delta * 24 * 60 + i64::from(local.hour()) * 60 + i64::from(local.minute())
}

fn build_chart_time_label(temporal: &TemporalInfo) -> String {
    let Some(start) = temporal.start else {
        return String::new();
    };
    let start_label = format_time_label(start);
    if let Some(end) = temporal.end_real {
        format!("{} - {}", start_label, format_time_label(end))
    } else {
        start_label
    }
}

fn build_chart_tooltip(title: &str, time_label: &str, temporal: &TemporalInfo) -> String {
    let duration = temporal.end_real.and_then(|end| {
        temporal
            .start
            .map(|start| format_duration_label((end - start).num_minutes()))
    });
    match (time_label.is_empty(), duration) {
        (true, _) => title.to_string(),
        (false, Some(duration)) => format!("{} · {} · {}", title, time_label, duration),
        (false, None) => format!("{} · {}", title, time_label),
    }
}

fn map_day_chart_kind(meeting: &Meeting) -> DayChartBarKind {
    if meeting.overlay_status == Some(OverlayStatus::Cancelled) {
        return DayChartBarKind::Cancelled;
    }

    match meeting.meeting_type {
        MeetingType::Customer | MeetingType::Qbr | MeetingType::Training => {
            DayChartBarKind::Customer
        }
        MeetingType::Internal | MeetingType::TeamSync | MeetingType::AllHands => {
            DayChartBarKind::Internal
        }
        MeetingType::OneOnOne => DayChartBarKind::OneOnOne,
        MeetingType::Partnership | MeetingType::External => DayChartBarKind::Partner,
        MeetingType::Personal => DayChartBarKind::Personal,
    }
}

fn map_day_chart_state(state: &MeetingSpineState) -> DayChartBarState {
    match state {
        MeetingSpineState::Past => DayChartBarState::Past,
        MeetingSpineState::InProgress => DayChartBarState::Now,
        MeetingSpineState::Upcoming => DayChartBarState::Upcoming,
        MeetingSpineState::Cancelled => DayChartBarState::Cancelled,
    }
}

fn canonical_day_chart_kinds() -> Vec<DayChartBarKind> {
    vec![
        DayChartBarKind::Customer,
        DayChartBarKind::Partner,
        DayChartBarKind::Internal,
        DayChartBarKind::OneOnOne,
        DayChartBarKind::Personal,
        DayChartBarKind::Project,
        DayChartBarKind::Cancelled,
    ]
}

fn day_chart_kind_label(kind: &DayChartBarKind) -> &'static str {
    match kind {
        DayChartBarKind::Customer => "Customer",
        DayChartBarKind::Partner => "Partner",
        DayChartBarKind::Internal => "Internal",
        DayChartBarKind::OneOnOne => "1:1",
        DayChartBarKind::Personal => "Personal",
        DayChartBarKind::Project => "Project",
        DayChartBarKind::Cancelled => "Cancelled",
    }
}

fn build_now_line(now: DateTime<Utc>) -> Option<DayChartNowLine> {
    let now_local = now.with_timezone(&Local);
    let minutes = now_local.hour() * 60 + now_local.minute();
    let range_start = RANGE_START_HOUR * 60;
    let range_end = RANGE_END_HOUR * 60;
    if minutes < range_start || minutes > range_end {
        return None;
    }
    let total_minutes = (RANGE_END_HOUR - RANGE_START_HOUR) * 60;
    Some(DayChartNowLine {
        label: "Now".to_string(),
        left_pct: f64::from(minutes - range_start) / f64::from(total_minutes) * 100.0,
        iso_time: now.to_rfc3339(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekScheduleShape {
    pub week_meta: WeekMeta,
    pub shape_days: Vec<WeekDayShape>,
    pub shape_epigraph: String,
    pub timeline_groups: WeekTimelineGroups,
    pub readiness_stats: Vec<WeekReadinessStat>,
    pub folio_readiness_stats: Vec<WeekReadinessStat>,
    pub future_meeting_count: usize,
    pub timeline: Vec<TimelineMeeting>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WeekMeta {
    pub week_number: String,
    pub date_range: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WeekDayShape {
    pub day_name: String,
    pub date: String,
    pub meeting_count: usize,
    pub meeting_minutes: u32,
    pub density: String,
    pub meetings: Vec<serde_json::Value>,
    pub available_blocks: Vec<serde_json::Value>,
    pub is_today: bool,
    pub is_past: bool,
    pub is_heavy: bool,
    pub bar_value: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekTimelineGroups {
    pub earlier_past: Vec<WeekTimelineGroup>,
    pub recent_past: Vec<WeekTimelineGroup>,
    pub today: Vec<WeekTimelineGroup>,
    pub future: Vec<WeekTimelineGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekTimelineGroup {
    pub date_key: String,
    pub label: String,
    pub meetings: Vec<TimelineMeeting>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WeekReadinessStat {
    pub label: String,
    pub color: WeekReadinessColor,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WeekReadinessColor {
    Sage,
    Terracotta,
}

fn compute_week_meta(today: NaiveDate) -> WeekMeta {
    let monday = monday_for(today);
    let friday = monday + Duration::days(4);
    WeekMeta {
        week_number: monday.iso_week().week().to_string(),
        date_range: format!(
            "{} - {}",
            format_month_day(monday),
            format_month_day(friday)
        ),
    }
}

fn derive_shape_from_timeline(timeline: &[TimelineMeeting], today: NaiveDate) -> Vec<WeekDayShape> {
    let monday = monday_for(today);
    (0..5)
        .map(|idx| {
            let date = monday + Duration::days(idx);
            let day_meetings: Vec<&TimelineMeeting> = timeline
                .iter()
                .filter(|m| {
                    parse_timeline_local_start(&m.start_time)
                        .is_some_and(|start| start.date_naive() == date)
                })
                .collect();
            let meeting_count = day_meetings.len();
            let meeting_minutes: u32 = day_meetings
                .iter()
                .map(|meeting| timeline_meeting_minutes(meeting))
                .sum();
            let density = match meeting_count {
                n if n >= 5 => "packed",
                n if n >= 4 => "heavy",
                2..=3 => "moderate",
                _ => "light",
            }
            .to_string();
            let is_today = date == today;
            let is_past = date < today;
            let is_heavy = meeting_count >= 5 || meeting_minutes >= 360;

            WeekDayShape {
                day_name: weekday_name(date.weekday()).to_string(),
                date: date_key(date),
                meeting_count,
                meeting_minutes,
                density,
                meetings: vec![],
                available_blocks: vec![],
                is_today,
                is_past,
                is_heavy,
                bar_value: meeting_minutes.min(480),
            }
        })
        .collect()
}

fn compute_shape_epigraph(day_shapes: &[WeekDayShape]) -> String {
    if day_shapes.is_empty() {
        return String::new();
    }

    let mut sorted = day_shapes.to_vec();
    sorted.sort_by(|a, b| b.meeting_minutes.cmp(&a.meeting_minutes));
    let busiest = &sorted[0];
    let lightest = &sorted[sorted.len() - 1];

    let split = day_shapes.len().div_ceil(2);
    let front_load: u32 = day_shapes[..split].iter().map(|d| d.meeting_minutes).sum();
    let back_load: u32 = day_shapes[split..].iter().map(|d| d.meeting_minutes).sum();

    let shape = if f64::from(front_load) > f64::from(back_load) * 1.5 {
        "Front-loaded"
    } else if f64::from(back_load) > f64::from(front_load) * 1.5 {
        "Back-loaded"
    } else {
        "Balanced"
    };

    if lightest.meeting_count <= 1 {
        format!(
            "{}. {} is the crux \u{2014} Clear {} for recovery.",
            shape, busiest.day_name, lightest.day_name
        )
    } else {
        format!("{}. {} is the crux.", shape, busiest.day_name)
    }
}

fn group_timeline_by_date(timeline: &[TimelineMeeting], today: NaiveDate) -> WeekTimelineGroups {
    let mut by_date: BTreeMap<NaiveDate, Vec<TimelineMeeting>> = BTreeMap::new();
    for meeting in timeline {
        if let Some(start) = parse_timeline_local_start(&meeting.start_time) {
            by_date
                .entry(start.date_naive())
                .or_default()
                .push(meeting.clone());
        }
    }

    let mut groups = WeekTimelineGroups::default();
    for (date, meetings) in by_date {
        let diff_days = date.signed_duration_since(today).num_days();
        let group = WeekTimelineGroup {
            date_key: date_key(date),
            label: timeline_group_label(date, diff_days),
            meetings,
        };

        if diff_days < -2 {
            groups.earlier_past.push(group);
        } else if diff_days < 0 {
            groups.recent_past.push(group);
        } else if diff_days == 0 {
            groups.today.push(group);
        } else {
            groups.future.push(group);
        }
    }

    groups
}

fn timeline_group_label(date: NaiveDate, diff_days: i64) -> String {
    match diff_days {
        0 => "Today".to_string(),
        -1 => "Yesterday".to_string(),
        1 => "Tomorrow".to_string(),
        n if n < 0 => format!("{} days ago - {}", n.abs(), date.format("%A, %b %-d")),
        _ => date.format("%A, %b %-d").to_string(),
    }
}

fn compute_week_readiness_stats(
    timeline: &[TimelineMeeting],
    now: DateTime<Utc>,
    folio: bool,
) -> Vec<WeekReadinessStat> {
    let future: Vec<&TimelineMeeting> = timeline
        .iter()
        .filter(|m| parse_timeline_utc_start(&m.start_time).is_some_and(|start| start > now))
        .collect();
    if future.is_empty() {
        return vec![];
    }

    let ready = future.iter().filter(|m| m.has_prep).count();
    let total = future.len();
    let needs_prep = total - ready;
    let mut stats = Vec::new();
    if folio && ready == total {
        stats.push(WeekReadinessStat {
            label: format!("{}/{} ready", total, total),
            color: WeekReadinessColor::Sage,
        });
    } else {
        stats.push(WeekReadinessStat {
            label: format!("{} ready", ready),
            color: WeekReadinessColor::Sage,
        });
    }
    if needs_prep > 0 {
        stats.push(WeekReadinessStat {
            label: format!("{} building", needs_prep),
            color: WeekReadinessColor::Terracotta,
        });
    }
    stats
}

fn timeline_meeting_minutes(meeting: &TimelineMeeting) -> u32 {
    let Some(start) = parse_timeline_utc_start(&meeting.start_time) else {
        return ESTIMATED_DURATION_MINUTES as u32;
    };
    let Some(end) = meeting
        .end_time
        .as_deref()
        .and_then(parse_timeline_utc_start)
    else {
        return ESTIMATED_DURATION_MINUTES as u32;
    };
    let minutes = (end - start).num_minutes();
    if minutes > 0 {
        minutes as u32
    } else {
        ESTIMATED_DURATION_MINUTES as u32
    }
}

fn parse_timeline_utc_start(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .ok()
        .or_else(|| parse_naive_datetime_as_local(value).map(|dt| dt.with_timezone(&Utc)))
}

fn parse_timeline_local_start(value: &str) -> Option<DateTime<Local>> {
    parse_timeline_utc_start(value).map(|dt| dt.with_timezone(&Local))
}

fn monday_for(date: NaiveDate) -> NaiveDate {
    let days_from_monday = i64::from(date.weekday().num_days_from_monday());
    date - Duration::days(days_from_monday)
}

fn weekday_name(weekday: Weekday) -> &'static str {
    match weekday {
        Weekday::Mon => "Monday",
        Weekday::Tue => "Tuesday",
        Weekday::Wed => "Wednesday",
        Weekday::Thu => "Thursday",
        Weekday::Fri => "Friday",
        Weekday::Sat => "Saturday",
        Weekday::Sun => "Sunday",
    }
}

fn format_month_day(date: NaiveDate) -> String {
    date.format("%b %-d").to_string()
}

fn date_key(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-04-23T14:30:00-04:00")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn empty_branch_fixture() -> ScheduleViewModel {
        compose_schedule_from_meetings(vec![], now())
    }

    fn make_meeting(id: &str, ty: MeetingType, has_prep: bool) -> Meeting {
        let prep = if has_prep {
            Some(crate::types::MeetingPrep {
                context: Some("Quarterly check-in".to_string()),
                stakeholders: Some(vec![crate::types::Stakeholder {
                    name: "Globex".to_string(),
                    role: None,
                    focus: None,
                }]),
                ..Default::default()
            })
        } else {
            None
        };
        Meeting {
            id: id.to_string(),
            calendar_event_id: None,
            time: "10:00 AM".to_string(),
            end_time: Some("2026-04-23T11:00:00-04:00".to_string()),
            start_iso: Some("2026-04-23T10:00:00-04:00".to_string()),
            title: format!("Meeting {}", id),
            meeting_type: ty,
            prep,
            is_current: None,
            prep_file: None,
            has_prep,
            overlay_status: None,
            prep_reviewed: None,
            linked_entities: None,
            suggested_unarchive_account_id: None,
            intelligence_quality: None,
            calendar_attendees: None,
            calendar_description: None,
        }
    }

    fn make_trust_claim(claim_type: &'static str, trust_score: Option<f64>) -> MeetingTrustClaim {
        MeetingTrustClaim {
            claim_type: claim_type.to_string(),
            trust_score,
        }
    }

    fn trust_bands_for_claims(
        meeting_id: &str,
        claims: Vec<MeetingTrustClaim>,
    ) -> BTreeMap<String, TrustBandWire> {
        select_meeting_trust(&claims)
            .map(|trust| BTreeMap::from([(meeting_id.to_string(), trust)]))
            .unwrap_or_default()
    }

    fn make_timeline_meeting(
        id: &str,
        start: &str,
        end: Option<&str>,
        has_prep: bool,
    ) -> TimelineMeeting {
        TimelineMeeting {
            id: id.to_string(),
            title: format!("Meeting {}", id),
            start_time: start.to_string(),
            end_time: end.map(str::to_string),
            meeting_type: "customer".to_string(),
            intelligence_quality: None,
            has_outcomes: false,
            outcome_summary: None,
            entities: vec![],
            has_new_signals: false,
            prior_meeting_id: None,
            follow_up_count: None,
            has_prep,
        }
    }

    #[test]
    fn schedule_empty_branch_zero_meetings_with_default_day_chart() {
        let vm = empty_branch_fixture();
        assert!(vm.meetings.is_empty());
        assert_eq!(vm.meeting_mix, ScheduleMeetingMix::default());
        assert_eq!(vm.day_chart.range_start_hour, 8);
        assert_eq!(vm.day_chart.range_end_hour, 20);
        assert!(vm.day_chart.bars.is_empty());
        assert_eq!(
            vm.day_chart
                .hour_ticks
                .iter()
                .map(|t| t.label.as_str())
                .collect::<Vec<_>>(),
            vec!["8 AM", "10 AM", "12 PM", "2 PM", "4 PM", "6 PM", "8 PM"]
        );
    }

    #[test]
    fn schedule_serializes_to_camel_case_wire_shape() {
        let vm = empty_branch_fixture();
        let s = serde_json::to_string(&vm).expect("serialize");
        let parsed: Value = serde_json::from_str(&s).expect("parse");
        assert_eq!(parsed["countLabel"], "0 meetings");
        assert_eq!(parsed["meetingMix"]["oneOnOne"], 0);
        assert_eq!(parsed["dayChart"]["rangeStartHour"], 8);
        assert!(parsed["dayChart"]["nowLine"]["leftPct"].is_number());
    }

    #[test]
    fn count_label_pluralizes_correctly() {
        assert_eq!(format_count_label(0), "0 meetings");
        assert_eq!(format_count_label(1), "1 meeting");
        assert_eq!(format_count_label(5), "5 meetings");
    }

    #[test]
    fn meeting_mix_groups_qbr_and_training_with_customer() {
        let mut cancelled = make_meeting("i", MeetingType::Customer, false);
        cancelled.overlay_status = Some(OverlayStatus::Cancelled);
        let meetings = vec![
            make_meeting("a", MeetingType::Customer, false),
            make_meeting("b", MeetingType::Qbr, false),
            make_meeting("c", MeetingType::Training, false),
            make_meeting("d", MeetingType::OneOnOne, false),
            make_meeting("e", MeetingType::TeamSync, false),
            make_meeting("f", MeetingType::Partnership, false),
            make_meeting("g", MeetingType::External, false),
            make_meeting("h", MeetingType::Personal, false),
            cancelled,
        ];
        let mix = compute_meeting_mix(&meetings);
        assert_eq!(
            mix.customer, 3,
            "Customer + Qbr + Training fold to customer"
        );
        assert_eq!(mix.internal, 1, "TeamSync folds to internal");
        assert_eq!(mix.one_on_one, 1);
        assert_eq!(mix.partner, 2, "Partnership + External fold to partner");
        assert_eq!(mix.personal, 1);
        assert_eq!(mix.cancelled, 1);
    }

    #[test]
    fn summary_omits_zero_segments() {
        let mix = ScheduleMeetingMix {
            customer: 2,
            internal: 0,
            one_on_one: 1,
            partner: 0,
            personal: 0,
            cancelled: 0,
        };
        let summary = format_summary(3, &mix);
        assert_eq!(summary, "2 customer · 1 1:1.");
    }

    #[test]
    fn summary_handles_zero_meetings() {
        assert_eq!(
            format_summary(0, &ScheduleMeetingMix::default()),
            "No meetings today."
        );
    }

    #[test]
    fn meeting_type_maps_to_spine_type() {
        assert_eq!(
            map_meeting_type(&MeetingType::Customer),
            MeetingSpineType::Customer
        );
        assert_eq!(
            map_meeting_type(&MeetingType::Qbr),
            MeetingSpineType::Customer
        );
        assert_eq!(
            map_meeting_type(&MeetingType::OneOnOne),
            MeetingSpineType::OneOnOne
        );
        assert_eq!(
            map_meeting_type(&MeetingType::Partnership),
            MeetingSpineType::Partner
        );
        assert_eq!(
            map_meeting_type(&MeetingType::AllHands),
            MeetingSpineType::Internal
        );
        assert_eq!(
            map_meeting_type(&MeetingType::Personal),
            MeetingSpineType::Internal
        );
    }

    #[test]
    fn meeting_with_prep_renders_link_action_and_ready_quality() {
        let m = make_meeting("m1", MeetingType::Customer, true);
        let view = map_meeting(m);
        assert_eq!(view.id, "m1");
        assert_eq!(
            view.intelligence_quality.level,
            IntelligenceQualityLevel::Ready
        );
        assert_eq!(view.eyebrow.entity_name, "Globex");
        assert!(matches!(
            view.briefing_action,
            BriefingActionView::Link { .. }
        ));
        assert!(!view.state_tags.contains(&MeetingStateTag::NoBriefingYet));
    }

    #[test]
    fn meeting_without_prep_renders_create_action_and_no_briefing_tag() {
        let m = make_meeting("m2", MeetingType::Internal, false);
        let view = map_meeting(m);
        assert_eq!(
            view.intelligence_quality.level,
            IntelligenceQualityLevel::NoBriefing
        );
        assert!(matches!(
            view.briefing_action,
            BriefingActionView::Create { .. }
        ));
        assert!(view.state_tags.contains(&MeetingStateTag::NoBriefingYet));
    }

    #[test]
    fn meeting_trust_defaults_to_unscored_without_claim() {
        let m = make_meeting("m-unscored", MeetingType::Customer, true);
        let view = map_meeting(m);

        assert_eq!(view.trust.trust_band, TrustBandWire::Unscored);
        assert_eq!(view.trust.trust_field_path, None);
        assert_eq!(view.trust.trust_source_date, None);
    }

    #[test]
    fn meeting_trust_uses_meeting_readiness_claim_score() {
        let m = make_meeting("m-readiness", MeetingType::Customer, true);
        let trust_bands = trust_bands_for_claims(
            "m-readiness",
            vec![make_trust_claim(
                ClaimType::MeetingReadiness.as_str(),
                Some(0.82),
            )],
        );
        let view = with_schedule_trust_bands(trust_bands, || map_meeting(m));

        assert_eq!(view.trust.trust_band, TrustBandWire::LikelyCurrent);
    }

    #[test]
    fn meeting_trust_falls_back_to_related_meeting_claims() {
        let trust = select_meeting_trust(&[make_trust_claim(
            ClaimType::MeetingChangeMarker.as_str(),
            Some(0.62),
        )]);

        assert_eq!(trust, Some(TrustBandWire::UseWithCaution));
    }

    #[test]
    fn meeting_trust_ignores_unscored_when_scored_related_claim_exists() {
        let trust = select_meeting_trust(&[
            make_trust_claim(ClaimType::MeetingTopic.as_str(), None),
            make_trust_claim(ClaimType::OpenLoop.as_str(), Some(0.41)),
        ]);

        assert_eq!(trust, Some(TrustBandWire::NeedsVerification));
    }

    #[test]
    fn schedule_serializes_scored_trust_band_to_camel_case_wire_shape() {
        let m = make_meeting("m-serialized", MeetingType::Customer, true);
        let trust_bands = trust_bands_for_claims(
            "m-serialized",
            vec![make_trust_claim(
                ClaimType::MeetingReadiness.as_str(),
                Some(0.62),
            )],
        );
        let view = with_schedule_trust_bands(trust_bands, || map_meeting(m));
        let parsed: Value = serde_json::to_value(&view).expect("serialize meeting");

        assert_eq!(parsed["trustBand"], "use_with_caution");
        assert!(parsed.get("trust_band").is_none());
    }

    #[test]
    fn meeting_in_progress_renders_now_state_tag() {
        let mut m = make_meeting("m3", MeetingType::Customer, true);
        m.is_current = Some(true);
        let view = map_meeting(m);
        assert_eq!(view.state, MeetingSpineState::InProgress);
        assert!(view.state_tags.contains(&MeetingStateTag::Now));
    }

    #[test]
    fn temporal_classification_covers_past_now_upcoming_and_cancelled() {
        let mut past = make_meeting("past", MeetingType::Internal, false);
        past.start_iso = Some("2026-04-23T12:00:00-04:00".to_string());
        past.end_time = Some("2026-04-23T13:00:00-04:00".to_string());

        let mut current = make_meeting("current", MeetingType::Customer, false);
        current.start_iso = Some("2026-04-23T14:00:00-04:00".to_string());
        current.end_time = Some("2026-04-23T15:00:00-04:00".to_string());

        let mut upcoming = make_meeting("upcoming", MeetingType::OneOnOne, false);
        upcoming.start_iso = Some("2026-04-23T16:00:00-04:00".to_string());
        upcoming.end_time = Some("2026-04-23T17:00:00-04:00".to_string());

        let mut cancelled = make_meeting("cancelled", MeetingType::Partnership, false);
        cancelled.start_iso = Some("2026-04-23T14:00:00-04:00".to_string());
        cancelled.end_time = Some("2026-04-23T15:00:00-04:00".to_string());
        cancelled.overlay_status = Some(OverlayStatus::Cancelled);

        let vm = compose_schedule_from_meetings(vec![upcoming, current, cancelled, past], now());
        let states: Vec<(&str, MeetingSpineState)> = vm
            .meetings
            .iter()
            .map(|m| (m.id.as_str(), m.state.clone()))
            .collect();

        assert_eq!(
            states,
            vec![
                ("past", MeetingSpineState::Past),
                ("current", MeetingSpineState::InProgress),
                ("cancelled", MeetingSpineState::Cancelled),
                ("upcoming", MeetingSpineState::Upcoming),
            ]
        );
        assert!(vm.meetings[1].state_tags.contains(&MeetingStateTag::Now));
        assert!(vm.meetings[2]
            .state_tags
            .contains(&MeetingStateTag::Cancelled));
    }

    #[test]
    fn temporal_boundaries_are_inclusive_at_start_and_exclusive_at_end() {
        let mut at_start = make_meeting("at-start", MeetingType::Customer, false);
        at_start.start_iso = Some("2026-04-23T14:30:00-04:00".to_string());
        at_start.end_time = Some("2026-04-23T15:00:00-04:00".to_string());

        let mut just_past = make_meeting("just-past", MeetingType::Customer, false);
        just_past.start_iso = Some("2026-04-23T14:00:00-04:00".to_string());
        just_past.end_time = Some("2026-04-23T14:30:00-04:00".to_string());

        let mut just_future = make_meeting("just-future", MeetingType::Customer, false);
        just_future.start_iso = Some("2026-04-23T14:30:01-04:00".to_string());
        just_future.end_time = Some("2026-04-23T15:00:00-04:00".to_string());

        let vm = compose_schedule_from_meetings(vec![just_future, just_past, at_start], now());
        assert_eq!(vm.meetings[0].state, MeetingSpineState::Past);
        assert_eq!(vm.meetings[1].state, MeetingSpineState::InProgress);
        assert_eq!(vm.meetings[2].state, MeetingSpineState::Upcoming);
    }

    #[test]
    fn invalid_start_stays_renderable_without_chart_bar() {
        let mut invalid = make_meeting("invalid", MeetingType::Customer, false);
        invalid.start_iso = Some("not-a-date".to_string());
        let vm = compose_schedule_from_meetings(vec![invalid], now());

        assert_eq!(vm.meetings.len(), 1);
        assert_eq!(vm.meetings[0].state, MeetingSpineState::Upcoming);
        assert_eq!(vm.meetings[0].time.starts_at_iso, "");
        assert!(vm.day_chart.bars.is_empty());
    }

    #[test]
    fn duration_labels_use_real_end_and_skip_estimated_fallback() {
        let mut long = make_meeting("long", MeetingType::Customer, false);
        long.start_iso = Some("2026-04-23T09:00:00-04:00".to_string());
        long.end_time = Some("10:30 AM".to_string());

        let mut estimated = make_meeting("estimated", MeetingType::Internal, false);
        estimated.start_iso = Some("2026-04-23T11:00:00-04:00".to_string());
        estimated.end_time = None;

        let vm = compose_schedule_from_meetings(vec![long, estimated], now());
        assert_eq!(vm.meetings[0].time.duration_label, "1h 30m");
        assert!(vm.meetings[0].time.ends_at_iso.ends_with("+00:00"));
        assert_eq!(vm.meetings[1].time.duration_label, "");
        assert_eq!(vm.meetings[1].time.ends_at_iso, "");
    }

    #[test]
    fn day_chart_layout_legend_and_now_line_are_stable() {
        let mut customer = make_meeting("customer", MeetingType::Customer, false);
        customer.start_iso = Some("2026-04-23T09:00:00-04:00".to_string());
        customer.end_time = Some("2026-04-23T10:00:00-04:00".to_string());

        let mut one_on_one = make_meeting("one-on-one", MeetingType::OneOnOne, false);
        one_on_one.start_iso = Some("2026-04-23T14:15:00-04:00".to_string());
        one_on_one.end_time = None;

        let vm = compose_schedule_from_meetings(vec![one_on_one, customer], now());
        assert_eq!(vm.day_chart.legend[0].kind, DayChartBarKind::Customer);
        assert_eq!(vm.day_chart.legend[1].kind, DayChartBarKind::OneOnOne);
        assert_eq!(vm.day_chart.bars.len(), 2);

        let first = &vm.day_chart.bars[0];
        assert_eq!(first.kind, DayChartBarKind::Customer);
        assert!((first.layout.left_pct - 8.333).abs() < 0.01);
        assert!((first.layout.width_pct - 8.333).abs() < 0.01);

        let second = &vm.day_chart.bars[1];
        assert_eq!(second.state, Some(DayChartBarState::Now));
        assert!((second.layout.left_pct - 52.083).abs() < 0.01);
        assert!((second.layout.width_pct - 6.25).abs() < 0.01);

        let now_line = vm.day_chart.now_line.expect("now line");
        assert_eq!(now_line.label, "Now");
        assert!((now_line.left_pct - 54.166).abs() < 0.01);
    }

    #[test]
    fn now_line_is_omitted_outside_chart_range() {
        let late_now = DateTime::parse_from_rfc3339("2026-04-23T21:00:00-04:00")
            .unwrap()
            .with_timezone(&Utc);
        let vm = compose_schedule_from_meetings(vec![], late_now);
        assert!(vm.day_chart.now_line.is_none());
    }

    #[test]
    fn week_shape_groups_by_local_date_and_buckets() {
        let now = DateTime::parse_from_rfc3339("2026-04-23T12:00:00-04:00")
            .unwrap()
            .with_timezone(&Utc);
        let timeline = vec![
            make_timeline_meeting(
                "utc-boundary",
                "2026-04-23T01:30:00Z",
                Some("2026-04-23T02:00:00Z"),
                true,
            ),
            make_timeline_meeting(
                "yesterday",
                "2026-04-22T10:00:00-04:00",
                Some("2026-04-22T11:00:00-04:00"),
                true,
            ),
            make_timeline_meeting(
                "today",
                "2026-04-23T10:00:00-04:00",
                Some("2026-04-23T11:00:00-04:00"),
                false,
            ),
            make_timeline_meeting(
                "future",
                "2026-04-24T10:00:00-04:00",
                Some("2026-04-24T11:00:00-04:00"),
                false,
            ),
            make_timeline_meeting(
                "earlier",
                "2026-04-19T10:00:00-04:00",
                Some("2026-04-19T11:00:00-04:00"),
                true,
            ),
        ];
        let shape = compose_week_schedule_shape_from_timeline(timeline, 7, 7, now);

        assert_eq!(
            shape.timeline_groups.earlier_past[0].label,
            "4 days ago - Sunday, Apr 19"
        );
        assert_eq!(shape.timeline_groups.recent_past[0].date_key, "2026-04-22");
        assert_eq!(shape.timeline_groups.recent_past[0].meetings.len(), 2);
        assert_eq!(shape.timeline_groups.today[0].label, "Today");
        assert_eq!(shape.timeline_groups.future[0].label, "Tomorrow");
    }

    #[test]
    fn week_shape_density_thresholds_and_minutes_match_current_shape() {
        let now = DateTime::parse_from_rfc3339("2026-04-23T12:00:00-04:00")
            .unwrap()
            .with_timezone(&Utc);
        let mut timeline = Vec::new();
        for i in 0..5 {
            timeline.push(make_timeline_meeting(
                &format!("mon-{}", i),
                &format!("2026-04-20T{:02}:00:00-04:00", i + 8),
                Some(&format!("2026-04-20T{:02}:30:00-04:00", i + 8)),
                true,
            ));
        }
        for i in 0..4 {
            timeline.push(make_timeline_meeting(
                &format!("tue-{}", i),
                &format!("2026-04-21T{:02}:00:00-04:00", i + 8),
                None,
                true,
            ));
        }
        timeline.push(make_timeline_meeting(
            "wed-1",
            "2026-04-22T09:00:00-04:00",
            Some("2026-04-22T10:00:00-04:00"),
            true,
        ));
        timeline.push(make_timeline_meeting(
            "wed-2",
            "2026-04-22T11:00:00-04:00",
            Some("2026-04-22T12:00:00-04:00"),
            true,
        ));

        let shape = compose_week_schedule_shape_from_timeline(timeline, 7, 7, now);
        assert_eq!(shape.shape_days[0].density, "packed");
        assert_eq!(shape.shape_days[0].meeting_minutes, 150);
        assert_eq!(shape.shape_days[1].density, "heavy");
        assert_eq!(shape.shape_days[1].meeting_minutes, 180);
        assert_eq!(shape.shape_days[2].density, "moderate");
        assert_eq!(shape.shape_days[3].density, "light");
        assert_eq!(shape.shape_days[4].density, "light");
        assert!(shape.shape_epigraph.contains("Tuesday is the crux"));
    }

    #[test]
    fn week_shape_filters_to_plus_minus_boundaries() {
        let now = DateTime::parse_from_rfc3339("2026-04-23T12:00:00-04:00")
            .unwrap()
            .with_timezone(&Utc);
        let timeline = vec![
            make_timeline_meeting("inside-before", "2026-04-16T09:00:00-04:00", None, true),
            make_timeline_meeting("inside-after", "2026-04-30T09:00:00-04:00", None, true),
            make_timeline_meeting("outside-before", "2026-04-15T09:00:00-04:00", None, true),
            make_timeline_meeting("outside-after", "2026-05-01T09:00:00-04:00", None, true),
        ];
        let shape = compose_week_schedule_shape_from_timeline(timeline, 7, 7, now);
        assert_eq!(shape.timeline.len(), 2);
        assert_eq!(shape.timeline[0].id, "inside-before");
        assert_eq!(shape.timeline[1].id, "inside-after");
    }
}
