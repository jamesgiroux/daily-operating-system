//! Meeting correlation: match Quill meetings to DailyOS calendar events.
//!
//! Uses a multi-signal scoring algorithm combining title similarity,
//! time proximity, and participant overlap to find the best match.

use chrono::{DateTime, Utc};
use std::collections::HashSet;

use super::client::QuillMeeting;

/// Result of matching a DailyOS meeting against Quill candidates.
#[derive(Debug, Clone)]
pub struct MatchResult {
    pub quill_meeting_id: String,
    /// Normalized confidence score (0.0–1.0).
    pub confidence: f64,
    /// Raw score (sum of all signal scores).
    pub score: u32,
    /// Whether the score meets the auto-match threshold (>=100).
    pub matched: bool,
}

/// Minimum score required for automatic matching.
const MATCH_THRESHOLD: u32 = 100;

/// Maximum theoretical score (title exact + time within 5 min + 3 participants).
const MAX_SCORE: u32 = 100 + 80 + 60;

/// Maximum participant overlap score.
const MAX_PARTICIPANT_SCORE: u32 = 60;

/// Score per matching participant email.
const PARTICIPANT_MATCH_SCORE: u32 = 20;

/// Find the best matching Quill meeting for a DailyOS calendar event.
///
/// Returns `None` if no candidate scores at or above the threshold.
pub fn match_meeting(
    title: &str,
    start_time: &DateTime<Utc>,
    attendees: &[String],
    quill_meetings: &[QuillMeeting],
) -> Option<MatchResult> {
    let mut best: Option<MatchResult> = None;

    for qm in quill_meetings {
        let t_score = title_score(title, &qm.title);

        let tm_score = qm
            .start_time
            .as_deref()
            .and_then(|t| t.parse::<DateTime<Utc>>().ok())
            .map(|qt| time_proximity_score(start_time, &qt))
            .unwrap_or(0);

        let quill_emails: Vec<String> = qm
            .participants
            .iter()
            .filter_map(|p| p.email.as_deref().map(|e| e.to_lowercase()))
            .collect();
        let p_score = participant_overlap_score(attendees, &quill_emails);

        let score = t_score + tm_score + p_score;
        let confidence = (score as f64) / (MAX_SCORE as f64);

        if score >= MATCH_THRESHOLD {
            let candidate = MatchResult {
                quill_meeting_id: qm.id.clone(),
                confidence: confidence.min(1.0),
                score,
                matched: true,
            };

            if best.as_ref().is_none_or(|b| score > b.score) {
                best = Some(candidate);
            }
        }
    }

    best
}

/// Score title similarity between two meeting titles.
fn title_score(a: &str, b: &str) -> u32 {
    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();

    if a_lower == b_lower {
        return 100;
    }
    if a_lower.contains(&b_lower) || b_lower.contains(&a_lower) {
        return 70;
    }
    if title_token_overlap(&a_lower, &b_lower) > 0.5 {
        return 50;
    }
    0
}

/// Jaccard similarity on word tokens.
fn title_token_overlap(a: &str, b: &str) -> f64 {
    let tokens_a: HashSet<&str> = a.split_whitespace().collect();
    let tokens_b: HashSet<&str> = b.split_whitespace().collect();

    if tokens_a.is_empty() && tokens_b.is_empty() {
        return 0.0;
    }

    let intersection = tokens_a.intersection(&tokens_b).count();
    let union = tokens_a.union(&tokens_b).count();

    if union == 0 {
        return 0.0;
    }

    intersection as f64 / union as f64
}

/// Score based on time proximity between meeting start times.
fn time_proximity_score(a: &DateTime<Utc>, b: &DateTime<Utc>) -> u32 {
    let diff_minutes = (*a - *b).num_minutes().unsigned_abs();

    if diff_minutes <= 5 {
        80
    } else if diff_minutes <= 15 {
        60
    } else if diff_minutes <= 30 {
        30
    } else {
        0
    }
}

/// Score based on participant email overlap.
fn participant_overlap_score(a: &[String], b: &[String]) -> u32 {
    let set_a: HashSet<String> = a.iter().map(|e| e.to_lowercase()).collect();
    let set_b: HashSet<String> = b.iter().map(|e| e.to_lowercase()).collect();

    let matches = set_a.intersection(&set_b).count() as u32;
    (matches * PARTICIPANT_MATCH_SCORE).min(MAX_PARTICIPANT_SCORE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quill::client::{QuillMeeting, QuillParticipant};
    use chrono::TimeZone;

    fn make_quill_meeting(
        id: &str,
        title: &str,
        start: Option<&str>,
        participants: Vec<(&str, &str)>,
    ) -> QuillMeeting {
        QuillMeeting {
            id: id.to_string(),
            title: title.to_string(),
            start_time: start.map(String::from),
            end_time: None,
            participants: participants
                .into_iter()
                .map(|(name, email)| QuillParticipant {
                    name: Some(name.to_string()),
                    email: Some(email.to_string()),
                })
                .collect(),
            has_transcript: true,
        }
    }

    #[test]
    fn test_exact_title_match() {
        let start = Utc.with_ymd_and_hms(2026, 2, 17, 14, 0, 0).unwrap();
        let quill = vec![make_quill_meeting(
            "q1",
            "Acme Weekly Sync",
            Some("2026-02-17T14:00:00Z"),
            vec![],
        )];

        let result = match_meeting("Acme Weekly Sync", &start, &[], &quill);
        assert!(result.is_some());
        let m = result.unwrap();
        assert!(m.matched);
        assert_eq!(m.quill_meeting_id, "q1");
        assert_eq!(m.score, 180);
    }

    #[test]
    fn test_case_insensitive_title_match() {
        let start = Utc.with_ymd_and_hms(2026, 2, 17, 14, 0, 0).unwrap();
        let quill = vec![make_quill_meeting(
            "q1",
            "acme weekly sync",
            Some("2026-02-17T14:00:00Z"),
            vec![],
        )];

        let result = match_meeting("Acme Weekly Sync", &start, &[], &quill);
        assert!(result.is_some());
        assert_eq!(result.unwrap().score, 180);
    }

    #[test]
    fn test_contains_title_match() {
        let start = Utc.with_ymd_and_hms(2026, 2, 17, 14, 0, 0).unwrap();
        let quill = vec![make_quill_meeting(
            "q1",
            "Acme Weekly Sync - Q1 Planning",
            Some("2026-02-17T14:02:00Z"),
            vec![],
        )];

        let result = match_meeting("Acme Weekly Sync", &start, &[], &quill);
        assert!(result.is_some());
        assert_eq!(result.unwrap().score, 150);
    }

    #[test]
    fn test_token_overlap_match() {
        let start = Utc.with_ymd_and_hms(2026, 2, 17, 14, 0, 0).unwrap();
        let quill = vec![make_quill_meeting(
            "q1",
            "Weekly Sync Acme Corp",
            Some("2026-02-17T14:00:00Z"),
            vec![],
        )];

        let result = match_meeting("Acme Weekly Sync", &start, &[], &quill);
        assert!(result.is_some());
        assert_eq!(result.unwrap().score, 130);
    }

    #[test]
    fn test_time_and_participants_compensate_title_mismatch() {
        let start = Utc.with_ymd_and_hms(2026, 2, 17, 14, 0, 0).unwrap();
        let quill = vec![make_quill_meeting(
            "q1",
            "Completely Different Title",
            Some("2026-02-17T14:03:00Z"),
            vec![
                ("Alice", "alice@acme.com"),
                ("Bob", "bob@acme.com"),
                ("Carol", "carol@acme.com"),
            ],
        )];

        let attendees: Vec<String> = vec![
            "alice@acme.com".to_string(),
            "bob@acme.com".to_string(),
            "carol@acme.com".to_string(),
        ];

        let result = match_meeting("Acme QBR", &start, &attendees, &quill);
        assert!(result.is_some());
        assert_eq!(result.unwrap().score, 140);
    }

    #[test]
    fn test_below_threshold_returns_none() {
        let start = Utc.with_ymd_and_hms(2026, 2, 17, 14, 0, 0).unwrap();
        let quill = vec![make_quill_meeting(
            "q1",
            "Completely Different Meeting",
            Some("2026-02-17T16:00:00Z"),
            vec![],
        )];

        let result = match_meeting("Acme Weekly Sync", &start, &[], &quill);
        assert!(result.is_none());
    }

    #[test]
    fn test_best_match_selected() {
        let start = Utc.with_ymd_and_hms(2026, 2, 17, 14, 0, 0).unwrap();
        let quill = vec![
            make_quill_meeting(
                "q1",
                "Some Other Meeting",
                Some("2026-02-17T14:00:00Z"),
                vec![
                    ("Alice", "alice@acme.com"),
                    ("Bob", "bob@acme.com"),
                    ("Carol", "carol@acme.com"),
                ],
            ),
            make_quill_meeting(
                "q2",
                "Acme Weekly Sync",
                Some("2026-02-17T14:00:00Z"),
                vec![],
            ),
        ];

        let result = match_meeting("Acme Weekly Sync", &start, &[], &quill);
        assert!(result.is_some());
        assert_eq!(result.unwrap().quill_meeting_id, "q2");
    }

    #[test]
    fn test_empty_quill_meetings() {
        let start = Utc.with_ymd_and_hms(2026, 2, 17, 14, 0, 0).unwrap();
        let result = match_meeting("Acme QBR", &start, &[], &[]);
        assert!(result.is_none());
    }

    #[test]
    fn test_empty_titles() {
        let start = Utc.with_ymd_and_hms(2026, 2, 17, 14, 0, 0).unwrap();
        let quill = vec![make_quill_meeting(
            "q1",
            "",
            Some("2026-02-17T14:00:00Z"),
            vec![],
        )];
        let result = match_meeting("", &start, &[], &quill);
        assert!(result.is_some());
    }

    #[test]
    fn test_time_15_min_proximity() {
        let start = Utc.with_ymd_and_hms(2026, 2, 17, 14, 0, 0).unwrap();
        let quill = vec![make_quill_meeting(
            "q1",
            "Acme Weekly Sync",
            Some("2026-02-17T14:12:00Z"),
            vec![],
        )];

        let result = match_meeting("Acme Weekly Sync", &start, &[], &quill);
        assert!(result.is_some());
        assert_eq!(result.unwrap().score, 160);
    }

    #[test]
    fn test_time_30_min_proximity() {
        let start = Utc.with_ymd_and_hms(2026, 2, 17, 14, 0, 0).unwrap();
        let quill = vec![make_quill_meeting(
            "q1",
            "Acme Weekly Sync",
            Some("2026-02-17T14:25:00Z"),
            vec![],
        )];

        let result = match_meeting("Acme Weekly Sync", &start, &[], &quill);
        assert!(result.is_some());
        assert_eq!(result.unwrap().score, 130);
    }

    #[test]
    fn test_participant_capped_at_max() {
        let start = Utc.with_ymd_and_hms(2026, 2, 17, 14, 0, 0).unwrap();
        let quill = vec![make_quill_meeting(
            "q1",
            "Acme QBR",
            Some("2026-02-17T14:00:00Z"),
            vec![
                ("A", "a@test.com"),
                ("B", "b@test.com"),
                ("C", "c@test.com"),
                ("D", "d@test.com"),
                ("E", "e@test.com"),
            ],
        )];

        let attendees: Vec<String> = vec![
            "a@test.com", "b@test.com", "c@test.com", "d@test.com", "e@test.com",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        let result = match_meeting("Acme QBR", &start, &attendees, &quill);
        assert!(result.is_some());
        assert_eq!(result.unwrap().score, 240);
    }

    #[test]
    fn test_no_quill_start_time() {
        let start = Utc.with_ymd_and_hms(2026, 2, 17, 14, 0, 0).unwrap();
        let quill = vec![make_quill_meeting("q1", "Acme Weekly Sync", None, vec![])];

        let result = match_meeting("Acme Weekly Sync", &start, &[], &quill);
        assert!(result.is_some());
        assert_eq!(result.unwrap().score, 100);
    }

    #[test]
    fn test_title_token_overlap_values() {
        assert!(title_token_overlap("acme weekly sync", "weekly sync acme corp") > 0.5);
        assert!(title_token_overlap("acme qbr", "globex standup") < 0.5);
        assert_eq!(title_token_overlap("", ""), 0.0);
        assert_eq!(title_token_overlap("hello", "hello"), 1.0);
    }

    #[test]
    fn test_time_proximity_boundaries() {
        let base = Utc.with_ymd_and_hms(2026, 2, 17, 14, 0, 0).unwrap();

        assert_eq!(time_proximity_score(&base, &base), 80);
        assert_eq!(
            time_proximity_score(
                &base,
                &Utc.with_ymd_and_hms(2026, 2, 17, 14, 5, 0).unwrap()
            ),
            80
        );
        assert_eq!(
            time_proximity_score(
                &base,
                &Utc.with_ymd_and_hms(2026, 2, 17, 14, 6, 0).unwrap()
            ),
            60
        );
        assert_eq!(
            time_proximity_score(
                &base,
                &Utc.with_ymd_and_hms(2026, 2, 17, 14, 15, 0).unwrap()
            ),
            60
        );
        assert_eq!(
            time_proximity_score(
                &base,
                &Utc.with_ymd_and_hms(2026, 2, 17, 14, 16, 0).unwrap()
            ),
            30
        );
        assert_eq!(
            time_proximity_score(
                &base,
                &Utc.with_ymd_and_hms(2026, 2, 17, 14, 30, 0).unwrap()
            ),
            30
        );
        assert_eq!(
            time_proximity_score(
                &base,
                &Utc.with_ymd_and_hms(2026, 2, 17, 14, 31, 0).unwrap()
            ),
            0
        );
    }

    #[test]
    fn test_participant_overlap() {
        let a = vec!["alice@acme.com".to_string(), "bob@acme.com".to_string()];
        let b = vec!["alice@acme.com".to_string(), "carol@acme.com".to_string()];
        assert_eq!(participant_overlap_score(&a, &b), 20);
    }

    #[test]
    fn test_participant_case_insensitive() {
        let a = vec!["Alice@Acme.com".to_string()];
        let b = vec!["alice@acme.com".to_string()];
        assert_eq!(participant_overlap_score(&a, &b), 20);
    }
}
