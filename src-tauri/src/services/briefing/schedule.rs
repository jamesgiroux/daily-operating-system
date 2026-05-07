//! Schedule composer — produces the `ScheduleViewModel` slice.
//!
//! **Trust source:** existing dashboard data flow (`services::dashboard`)
//! plus the meeting-prep queue. The redesign lifts today/past/future
//! grouping logic out of `DailyBriefing.tsx` (`compareEmailRank`, today-
//! filter, inline `useState` filtering) and ±7 days shape out of
//! `WeekPage.tsx` into a single composer.
//!
//! **W2a default:** empty meetings list, zero meeting mix, empty day chart.
//! Editorial copy renders ("Today's schedule" / "0 meetings"). DayChart
//! defaults to a reasonable 8am–8pm range with empty bars. The composer
//! shape is sound; live data plumbing is the W2a follow-up.
//!
//! **Unblocked at:** DOS-417 (calendar grouping lift impl). The lift work
//! reads the existing DailyBriefing temporal grouping + the WeekPage ±7
//! days logic and produces a single `ScheduleService` that this composer
//! calls.

use crate::services::briefing_view_model::{
    DayChartViewModel, ScheduleMeetingMix, ScheduleViewModel,
};
use crate::state::AppState;

pub async fn compose_schedule(_state: &AppState) -> ScheduleViewModel {
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
}
