//! Schedule composer — produces the `ScheduleViewModel` slice.
//!
//! **Trust source:** existing dashboard data flow (`services::dashboard`).
//! Calls `get_dashboard_data` internally and reshapes the meeting list
//! into the redesign's `ScheduleMeeting` view-model shape. The full
//! DOS-417 calendar lift (today/past/future grouping out of
//! `DailyBriefing.tsx` plus ±7 days out of `WeekPage.tsx`) is a follow-up;
//! this composer ships the minimum viable real-data flow first.
//!
//! **W2a default:** if `get_dashboard_data` returns Empty/Error, the
//! composer falls back to the empty-branch shape (zero meetings, default
//! day chart). Trust band is `Unscored` until DOS-320 trust UI wires
//! into briefing meetings (DOS-427, W4).
//!
//! **Unblocked at:** DOS-417 fully fleshes the temporal grouping + day
//! chart bars + editorial summary. This composer's MVP version ships the
//! shape now so the redesign surface renders real meetings end-to-end.

use crate::services::briefing_view_model::{
    BriefingActionView, DayChartViewModel, IntelligenceQualityLevel,
    IntelligenceQualityView, MeetingSpineState, MeetingSpineType, MeetingStateTag,
    MeetingTimeViewModel, ScheduleMeeting, ScheduleMeetingEyebrow, ScheduleMeetingMix,
    ScheduleViewModel, TrustBandWire, TrustMixin,
};
use crate::services::dashboard::{get_dashboard_data, DashboardResult};
use crate::state::AppState;
use crate::types::{Meeting, MeetingType};

pub async fn compose_schedule(state: &AppState) -> ScheduleViewModel {
    let result = get_dashboard_data(state).await;
    let meetings: Vec<Meeting> = match result {
        DashboardResult::Success { data, .. } => data.meetings,
        // Empty / Error from upstream → empty-branch fallback. The envelope
        // owns whether to surface Error to the user; per-section just
        // degrades gracefully.
        _ => vec![],
    };

    let mix = compute_meeting_mix(&meetings);
    let count = meetings.len();
    let count_label = format_count_label(count);
    let summary = format_summary(count, &mix);
    let view_meetings: Vec<ScheduleMeeting> =
        meetings.into_iter().map(map_meeting).collect();

    ScheduleViewModel {
        label: "Today".to_string(),
        heading: "Today's schedule".to_string(),
        count_label,
        meeting_mix: mix,
        summary,
        day_chart: DayChartViewModel {
            range_start_hour: 8,
            range_end_hour: 20,
            hour_ticks: vec![],
            legend: vec![],
            bars: vec![],
            now_line: None,
        },
        meetings: view_meetings,
    }
}

fn compute_meeting_mix(meetings: &[Meeting]) -> ScheduleMeetingMix {
    let mut mix = ScheduleMeetingMix::default();
    for m in meetings {
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
    if segments.is_empty() {
        return format!("{} meetings today.", count);
    }
    format!("{}.", segments.join(" · "))
}

fn map_meeting(m: Meeting) -> ScheduleMeeting {
    let accent_type = map_meeting_type(&m.meeting_type);
    let state = if m.is_current.unwrap_or(false) {
        MeetingSpineState::InProgress
    } else {
        // Without temporal grouping yet (DOS-417 follow-up), default to
        // Upcoming. The full lift will compute past/upcoming from start_iso.
        MeetingSpineState::Upcoming
    };
    let starts_at_iso = m.start_iso.clone().unwrap_or_default();
    let title = m.title.clone();
    ScheduleMeeting {
        trust: TrustMixin {
            trust_band: TrustBandWire::Unscored,
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
        context: m.prep.as_ref().and_then(|p| p.context.clone()).unwrap_or_default(),
        attendee_summary: String::new(),
        intelligence_quality: IntelligenceQualityView {
            level: if m.prep.is_some() {
                IntelligenceQualityLevel::Ready
            } else {
                IntelligenceQualityLevel::NoBriefing
            },
            label: if m.prep.is_some() { "Ready" } else { "No briefing" }.to_string(),
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
        // Personal has no MeetingSpineType — fall through to Internal so the
        // accent paints neutrally. DOS-417 may introduce a Personal variant
        // if the design calls for it.
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
    // Best-effort: peel a primary entity from the prep stakeholders list, or
    // fall back to empty. Full mapping is DOS-417 follow-up.
    m.prep
        .as_ref()
        .and_then(|p| p.stakeholders.as_ref())
        .and_then(|s| s.first())
        .map(|stake| stake.name.clone())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn empty_branch_fixture() -> ScheduleViewModel {
        ScheduleViewModel {
            label: "Today".to_string(),
            heading: "Today's schedule".to_string(),
            count_label: "0 meetings".to_string(),
            meeting_mix: ScheduleMeetingMix::default(),
            summary: "No meetings today.".to_string(),
            day_chart: DayChartViewModel {
                range_start_hour: 8,
                range_end_hour: 20,
                hour_ticks: vec![],
                legend: vec![],
                bars: vec![],
                now_line: None,
            },
            meetings: vec![],
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
    }

    #[test]
    fn schedule_serializes_to_camel_case_wire_shape() {
        let vm = empty_branch_fixture();
        let s = serde_json::to_string(&vm).expect("serialize");
        let parsed: Value = serde_json::from_str(&s).expect("parse");
        assert_eq!(parsed["countLabel"], "0 meetings");
        assert_eq!(parsed["meetingMix"]["oneOnOne"], 0);
        assert_eq!(parsed["dayChart"]["rangeStartHour"], 8);
        assert!(parsed["dayChart"]["nowLine"].is_null());
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
            time: "10:00".to_string(),
            end_time: None,
            start_iso: Some("2026-04-23T10:00:00Z".to_string()),
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

    #[test]
    fn count_label_pluralizes_correctly() {
        assert_eq!(format_count_label(0), "0 meetings");
        assert_eq!(format_count_label(1), "1 meeting");
        assert_eq!(format_count_label(5), "5 meetings");
    }

    #[test]
    fn meeting_mix_groups_qbr_and_training_with_customer() {
        let meetings = vec![
            make_meeting("a", MeetingType::Customer, false),
            make_meeting("b", MeetingType::Qbr, false),
            make_meeting("c", MeetingType::Training, false),
            make_meeting("d", MeetingType::OneOnOne, false),
            make_meeting("e", MeetingType::TeamSync, false),
            make_meeting("f", MeetingType::Partnership, false),
            make_meeting("g", MeetingType::External, false),
            make_meeting("h", MeetingType::Personal, false),
        ];
        let mix = compute_meeting_mix(&meetings);
        assert_eq!(mix.customer, 3, "Customer + Qbr + Training fold to customer");
        assert_eq!(mix.internal, 1, "TeamSync folds to internal");
        assert_eq!(mix.one_on_one, 1);
        assert_eq!(mix.partner, 2, "Partnership + External fold to partner");
        assert_eq!(mix.personal, 1);
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
        assert_eq!(map_meeting_type(&MeetingType::Customer), MeetingSpineType::Customer);
        assert_eq!(map_meeting_type(&MeetingType::Qbr), MeetingSpineType::Customer);
        assert_eq!(map_meeting_type(&MeetingType::OneOnOne), MeetingSpineType::OneOnOne);
        assert_eq!(map_meeting_type(&MeetingType::Partnership), MeetingSpineType::Partner);
        assert_eq!(map_meeting_type(&MeetingType::AllHands), MeetingSpineType::Internal);
        assert_eq!(map_meeting_type(&MeetingType::Personal), MeetingSpineType::Internal);
    }

    #[test]
    fn meeting_with_prep_renders_link_action_and_ready_quality() {
        let m = make_meeting("m1", MeetingType::Customer, true);
        let view = map_meeting(m);
        assert_eq!(view.id, "m1");
        assert_eq!(view.intelligence_quality.level, IntelligenceQualityLevel::Ready);
        assert_eq!(view.eyebrow.entity_name, "Globex");
        assert!(matches!(view.briefing_action, BriefingActionView::Link { .. }));
        assert!(!view.state_tags.contains(&MeetingStateTag::NoBriefingYet));
    }

    #[test]
    fn meeting_without_prep_renders_create_action_and_no_briefing_tag() {
        let m = make_meeting("m2", MeetingType::Internal, false);
        let view = map_meeting(m);
        assert_eq!(view.intelligence_quality.level, IntelligenceQualityLevel::NoBriefing);
        assert!(matches!(view.briefing_action, BriefingActionView::Create { .. }));
        assert!(view.state_tags.contains(&MeetingStateTag::NoBriefingYet));
    }

    #[test]
    fn meeting_in_progress_renders_now_state_tag() {
        let mut m = make_meeting("m3", MeetingType::Customer, true);
        m.is_current = Some(true);
        let view = map_meeting(m);
        assert_eq!(view.state, MeetingSpineState::InProgress);
        assert!(view.state_tags.contains(&MeetingStateTag::Now));
    }
}
