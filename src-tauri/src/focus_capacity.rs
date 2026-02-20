use chrono::{
    DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Timelike,
};
use chrono_tz::Tz;

use crate::types::{Meeting, MeetingType, OverlayStatus, TimeBlock};

const PRE_BUFFER_MINUTES: i64 = 10;
const POST_BUFFER_MINUTES: i64 = 10;
const MIN_AVAILABLE_BLOCK_MINUTES: i64 = 30;
const MIN_DEEP_WORK_BLOCK_MINUTES: i64 = 60;

#[derive(Debug, Clone)]
pub enum FocusCapacitySource {
    Live,
    BriefingFallback,
}

impl FocusCapacitySource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Live => "live",
            Self::BriefingFallback => "briefing_fallback",
        }
    }
}

#[derive(Debug, Clone)]
pub struct FocusCapacityInput {
    pub meetings: Vec<Meeting>,
    pub source: FocusCapacitySource,
    pub timezone: Tz,
    pub work_hours_start: u8,
    pub work_hours_end: u8,
    pub day_date: NaiveDate,
}

#[derive(Debug, Clone)]
pub struct FocusCapacityResult {
    pub meeting_count: u32,
    pub meeting_minutes: u32,
    pub available_minutes: u32,
    pub deep_work_minutes: u32,
    pub available_blocks: Vec<TimeBlock>,
    pub deep_work_blocks: Vec<TimeBlock>,
    pub source: FocusCapacitySource,
    pub warnings: Vec<String>,
}

/// Resolve a local date + hour to a timezone-aware DateTime, handling DST gaps.
///
/// During a spring-forward gap, `earliest()` returns `None`. We fall back to
/// `latest()` (the post-transition instant), and as a last resort use UTC.
fn resolve_local_datetime(tz: &Tz, date: NaiveDate, hour: u32) -> DateTime<Tz> {
    // Fast path: unambiguous local time.
    if let Some(dt) = tz
        .with_ymd_and_hms(date.year(), date.month(), date.day(), hour, 0, 0)
        .single()
    {
        return dt;
    }

    let naive = NaiveDateTime::new(
        date,
        NaiveTime::from_hms_opt(hour, 0, 0).unwrap_or(NaiveTime::MIN),
    );

    if let Some(dt) = tz.from_local_datetime(&naive).earliest() {
        return dt;
    }

    // DST spring-forward gap: local time doesn't exist. Use latest (post-transition).
    if let Some(dt) = tz.from_local_datetime(&naive).latest() {
        log::warn!(
            "DST gap detected for {} {:02}:00 in {}; using post-transition time",
            date,
            hour,
            tz
        );
        return dt;
    }

    // Absolute fallback: interpret as UTC and convert.
    log::warn!(
        "Could not resolve local datetime {} {:02}:00 in {}; falling back to UTC",
        date,
        hour,
        tz
    );
    chrono::Utc
        .with_ymd_and_hms(date.year(), date.month(), date.day(), hour, 0, 0)
        .single()
        .expect("UTC datetime is always unambiguous")
        .with_timezone(tz)
}

pub fn compute_focus_capacity(input: FocusCapacityInput) -> FocusCapacityResult {
    let workday_start = resolve_local_datetime(
        &input.timezone,
        input.day_date,
        input.work_hours_start as u32,
    );
    let workday_end = resolve_local_datetime(
        &input.timezone,
        input.day_date,
        input.work_hours_end as u32,
    );

    let mut raw_intervals: Vec<(DateTime<Tz>, DateTime<Tz>)> = Vec::new();
    let mut buffered_intervals: Vec<(DateTime<Tz>, DateTime<Tz>)> = Vec::new();

    for meeting in &input.meetings {
        if should_exclude_meeting(meeting) {
            continue;
        }

        if let Some((start, end)) = parse_meeting_interval(meeting, input.day_date, &input.timezone)
        {
            // Ignore meetings outside this local day.
            if end <= workday_start || start >= workday_end {
                continue;
            }

            let clipped_start = start.max(workday_start);
            let clipped_end = end.min(workday_end);
            if clipped_end > clipped_start {
                raw_intervals.push((clipped_start, clipped_end));
            }

            let buffered_start = (start - Duration::minutes(PRE_BUFFER_MINUTES)).max(workday_start);
            let buffered_end = (end + Duration::minutes(POST_BUFFER_MINUTES)).min(workday_end);
            if buffered_end > buffered_start {
                buffered_intervals.push((buffered_start, buffered_end));
            }
        }
    }

    let merged_buffered = merge_intervals(buffered_intervals);
    let available_blocks = compute_available_blocks(&merged_buffered, workday_start, workday_end);
    let deep_work_blocks: Vec<TimeBlock> = available_blocks
        .iter()
        .filter(|b| b.duration_minutes as i64 >= MIN_DEEP_WORK_BLOCK_MINUTES)
        .cloned()
        .collect();

    let meeting_minutes: u32 = raw_intervals
        .iter()
        .map(|(start, end)| (end.signed_duration_since(*start).num_minutes().max(0)) as u32)
        .sum();
    let available_minutes: u32 = available_blocks.iter().map(|b| b.duration_minutes).sum();
    let deep_work_minutes: u32 = deep_work_blocks.iter().map(|b| b.duration_minutes).sum();

    let mut warnings = Vec::new();
    if matches!(input.source, FocusCapacitySource::BriefingFallback) {
        warnings.push(
            "Live calendar unavailable. Capacity is estimated from briefing schedule and may be stale.".to_string(),
        );
    }

    FocusCapacityResult {
        meeting_count: raw_intervals.len() as u32,
        meeting_minutes,
        available_minutes,
        deep_work_minutes,
        available_blocks,
        deep_work_blocks,
        source: input.source,
        warnings,
    }
}

fn should_exclude_meeting(meeting: &Meeting) -> bool {
    if meeting.meeting_type == MeetingType::Personal {
        return true;
    }
    if meeting.overlay_status == Some(OverlayStatus::Cancelled) {
        return true;
    }

    let is_all_day = meeting.time.len() == 10 && meeting.time.chars().nth(4) == Some('-');
    is_all_day
}

fn parse_meeting_interval(
    meeting: &Meeting,
    day_date: NaiveDate,
    timezone: &Tz,
) -> Option<(DateTime<Tz>, DateTime<Tz>)> {
    let start = parse_datetime_with_fallback(
        meeting.start_iso.as_deref().unwrap_or(&meeting.time),
        day_date,
        timezone,
    )?;

    let end_raw = meeting.end_time.as_deref()?;
    let end = parse_datetime_with_fallback(end_raw, day_date, timezone)?;

    if end <= start {
        return None;
    }

    Some((start, end))
}

fn parse_datetime_with_fallback(
    value: &str,
    day_date: NaiveDate,
    timezone: &Tz,
) -> Option<DateTime<Tz>> {
    if value.contains('T') {
        if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
            return Some(dt.with_timezone(timezone));
        }
        if let Ok(naive) = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S") {
            return timezone.from_local_datetime(&naive).earliest();
        }
        if let Ok(naive) = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M") {
            return timezone.from_local_datetime(&naive).earliest();
        }
    }

    parse_display_time(value, day_date, timezone)
}

fn parse_display_time(value: &str, day_date: NaiveDate, timezone: &Tz) -> Option<DateTime<Tz>> {
    let formats = ["%-I:%M %p", "%I:%M %p", "%H:%M", "%H:%M:%S"];
    for fmt in formats {
        if let Ok(time) = NaiveTime::parse_from_str(value.trim(), fmt) {
            let naive = NaiveDateTime::new(day_date, time);
            if let Some(dt) = timezone.from_local_datetime(&naive).earliest() {
                return Some(dt);
            }
        }
    }
    None
}

fn merge_intervals(
    mut intervals: Vec<(DateTime<Tz>, DateTime<Tz>)>,
) -> Vec<(DateTime<Tz>, DateTime<Tz>)> {
    if intervals.is_empty() {
        return Vec::new();
    }

    intervals.sort_by_key(|(start, _)| *start);
    let mut merged: Vec<(DateTime<Tz>, DateTime<Tz>)> = Vec::new();

    for (start, end) in intervals {
        if let Some((_, current_end)) = merged.last_mut() {
            if start <= *current_end {
                if end > *current_end {
                    *current_end = end;
                }
                continue;
            }
        }
        merged.push((start, end));
    }

    merged
}

fn compute_available_blocks(
    busy: &[(DateTime<Tz>, DateTime<Tz>)],
    workday_start: DateTime<Tz>,
    workday_end: DateTime<Tz>,
) -> Vec<TimeBlock> {
    let mut blocks = Vec::new();
    let mut cursor = workday_start;

    for (start, end) in busy {
        if *start > cursor {
            let minutes = start.signed_duration_since(cursor).num_minutes();
            if minutes >= MIN_AVAILABLE_BLOCK_MINUTES {
                blocks.push(make_block(cursor, *start, minutes as u32));
            }
        }
        if *end > cursor {
            cursor = *end;
        }
    }

    if workday_end > cursor {
        let minutes = workday_end.signed_duration_since(cursor).num_minutes();
        if minutes >= MIN_AVAILABLE_BLOCK_MINUTES {
            blocks.push(make_block(cursor, workday_end, minutes as u32));
        }
    }

    blocks
}

fn make_block(start: DateTime<Tz>, end: DateTime<Tz>, duration_minutes: u32) -> TimeBlock {
    let suggested_use = if duration_minutes as i64 >= MIN_DEEP_WORK_BLOCK_MINUTES {
        Some("Deep Work".to_string())
    } else if start.hour() < 12 {
        Some("Priority Follow-up".to_string())
    } else {
        Some("Admin / Follow-up".to_string())
    };

    TimeBlock {
        day: "Today".to_string(),
        start: start.to_rfc3339(),
        end: end.to_rfc3339(),
        duration_minutes,
        suggested_use,
        action_id: None,
        meeting_id: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Meeting;

    fn meeting(time: &str, end_time: &str) -> Meeting {
        Meeting {
            id: "m1".to_string(),
            calendar_event_id: None,
            time: time.to_string(),
            end_time: Some(end_time.to_string()),
            start_iso: if time.contains('T') {
                Some(time.to_string())
            } else {
                None
            },
            title: "Meeting".to_string(),
            meeting_type: MeetingType::Customer,
            prep: None,
            is_current: None,
            prep_file: None,
            has_prep: false,
            overlay_status: None,
            prep_reviewed: None,
            linked_entities: None,
            suggested_unarchive_account_id: None,
            intelligence_quality: None,
        }
    }

    fn make_input(meetings: Vec<Meeting>) -> FocusCapacityInput {
        FocusCapacityInput {
            meetings,
            source: FocusCapacitySource::Live,
            timezone: chrono_tz::America::New_York,
            work_hours_start: 8,
            work_hours_end: 18,
            day_date: NaiveDate::from_ymd_opt(2026, 2, 12).unwrap(),
        }
    }

    #[test]
    fn packed_day_has_low_availability() {
        let meetings = vec![
            meeting("2026-02-12T08:30:00-05:00", "2026-02-12T10:30:00-05:00"),
            meeting("2026-02-12T11:00:00-05:00", "2026-02-12T13:00:00-05:00"),
            meeting("2026-02-12T14:00:00-05:00", "2026-02-12T17:00:00-05:00"),
        ];

        let result = compute_focus_capacity(make_input(meetings));
        assert!(result.available_minutes < 180);
        assert!(result.meeting_minutes >= 420);
    }

    #[test]
    fn buffers_merge_tight_gaps() {
        let meetings = vec![
            meeting("2026-02-12T09:00:00-05:00", "2026-02-12T10:00:00-05:00"),
            meeting("2026-02-12T10:15:00-05:00", "2026-02-12T11:00:00-05:00"),
        ];

        let result = compute_focus_capacity(make_input(meetings));
        // 15 minute natural gap disappears due to 10m pre/post buffers.
        assert!(result
            .available_blocks
            .iter()
            .all(|b| b.duration_minutes >= MIN_AVAILABLE_BLOCK_MINUTES as u32));
    }

    #[test]
    fn deep_work_threshold_applies_at_sixty_minutes() {
        let meetings = vec![meeting(
            "2026-02-12T12:00:00-05:00",
            "2026-02-12T14:00:00-05:00",
        )];
        let result = compute_focus_capacity(make_input(meetings));
        assert!(result
            .deep_work_blocks
            .iter()
            .any(|b| b.duration_minutes >= 60));
    }

    #[test]
    fn work_hours_clipping_applies() {
        let meetings = vec![meeting(
            "2026-02-12T06:30:00-05:00",
            "2026-02-12T08:30:00-05:00",
        )];
        let result = compute_focus_capacity(make_input(meetings));
        // 8:00-8:30 clipped into workday
        assert_eq!(result.meeting_minutes, 30);
    }

    #[test]
    fn fallback_sets_warning_and_source() {
        let mut input = make_input(Vec::new());
        input.source = FocusCapacitySource::BriefingFallback;
        let result = compute_focus_capacity(input);

        assert_eq!(result.source.as_str(), "briefing_fallback");
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn parses_display_time_fallback() {
        let mut m = meeting("9:00 AM", "10:00 AM");
        m.start_iso = None;
        let result = compute_focus_capacity(make_input(vec![m]));
        assert_eq!(result.meeting_minutes, 60);
    }
}
