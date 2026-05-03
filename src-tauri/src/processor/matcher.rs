//! Multi-signal meeting matcher for inbox documents.
//!
//! Scores candidate meetings using title similarity, time proximity,
//! and entity context. Used by the inbox processor to auto-link
//! documents to their corresponding historical meetings.

use chrono::{DateTime, Utc};
use std::collections::HashSet;

/// Minimum score required for automatic matching.
const AUTO_LINK_THRESHOLD: i32 = 100;

/// Maximum theoretical score: title exact (100) + time same-day (80) + entity match (40) = 220.
const MAX_SCORE: f64 = 220.0;

/// A candidate meeting with scoring context.
pub struct MeetingCandidate {
    pub meeting_id: String,
    pub title: String,
    pub start_time: Option<DateTime<Utc>>,
    pub entity_id: Option<String>,
}

/// Result of a successful match.
pub struct MatchResult {
    pub meeting_id: String,
    pub score: i32,
    pub confidence: f64,
}

/// Score a document against candidate meetings and return the best match
/// if it exceeds the auto-link threshold.
///
/// Scoring signals:
/// - Title similarity: exact (100), contains (70), token overlap >0.5 (50), >0.3 (25)
/// - Time proximity: same day (80), within 3 days (50), within 7 days (20)
/// - Entity match: same entity_id (40)
pub fn find_best_match(
    doc_title: &str,
    doc_time: Option<DateTime<Utc>>,
    doc_entity_id: Option<&str>,
    candidates: &[MeetingCandidate],
) -> Option<MatchResult> {
    let mut best: Option<(usize, i32)> = None;

    for (i, candidate) in candidates.iter().enumerate() {
        let mut score = 0i32;

        // Title similarity
        score += score_title_similarity(doc_title, &candidate.title);

        // Time proximity
        if let (Some(doc_t), Some(cand_t)) = (doc_time, candidate.start_time) {
            score += score_time_proximity(doc_t, cand_t);
        }

        // Entity match bonus
        if let (Some(doc_eid), Some(cand_eid)) = (doc_entity_id, candidate.entity_id.as_deref()) {
            if doc_eid == cand_eid {
                score += 40;
            }
        }

        if score >= AUTO_LINK_THRESHOLD
            && best
                .as_ref()
                .is_none_or(|(_, best_score)| score > *best_score)
        {
            best = Some((i, score));
        }
    }

    best.map(|(idx, score)| {
        let confidence = (score as f64 / MAX_SCORE).min(1.0);
        MatchResult {
            meeting_id: candidates[idx].meeting_id.clone(),
            score,
            confidence,
        }
    })
}

/// Score title similarity between document and meeting.
///
/// Same algorithm as quill::matcher and granola::matcher for consistency.
fn score_title_similarity(doc_title: &str, meeting_title: &str) -> i32 {
    let doc_lower = doc_title.to_lowercase();
    let meet_lower = meeting_title.to_lowercase();

    // Empty titles provide no signal
    if doc_lower.is_empty() || meet_lower.is_empty() {
        return 0;
    }

    // Exact match
    if doc_lower == meet_lower {
        return 100;
    }

    // Contains match (one contains the other)
    if doc_lower.contains(&meet_lower) || meet_lower.contains(&doc_lower) {
        return 70;
    }

    // Token overlap (Jaccard similarity)
    let doc_tokens: HashSet<&str> = doc_lower.split_whitespace().collect();
    let meet_tokens: HashSet<&str> = meet_lower.split_whitespace().collect();

    if doc_tokens.is_empty() || meet_tokens.is_empty() {
        return 0;
    }

    let intersection = doc_tokens.intersection(&meet_tokens).count();
    let union = doc_tokens.union(&meet_tokens).count();
    let jaccard = intersection as f64 / union as f64;

    if jaccard > 0.5 {
        50
    } else if jaccard > 0.3 {
        25
    } else {
        0
    }
}

/// Score time proximity between document and meeting.
///
/// Uses hour-based windows appropriate for inbox documents, which may be
/// processed hours or days after the meeting occurred.
fn score_time_proximity(doc_time: DateTime<Utc>, meeting_time: DateTime<Utc>) -> i32 {
    let diff_hours = (doc_time - meeting_time).num_hours().unsigned_abs();

    if diff_hours <= 24 {
        80
    } else if diff_hours <= 72 {
        50
    } else if diff_hours <= 168 {
        // 7 days
        20
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_exact_title_match() {
        assert_eq!(
            score_title_similarity("Weekly Standup", "Weekly Standup"),
            100
        );
    }

    #[test]
    fn test_case_insensitive_title_match() {
        assert_eq!(
            score_title_similarity("weekly standup", "Weekly Standup"),
            100
        );
    }

    #[test]
    fn test_contains_title_match() {
        assert_eq!(
            score_title_similarity("Notes from Weekly Standup", "Weekly Standup"),
            70
        );
    }

    #[test]
    fn test_contains_title_match_reverse() {
        assert_eq!(
            score_title_similarity("Weekly Standup", "Weekly Standup — Q1 Planning"),
            70
        );
    }

    #[test]
    fn test_token_overlap_high() {
        // "Q2 Planning Review" vs "Q2 Strategy Planning"
        // tokens: {q2, planning, review} vs {q2, strategy, planning}
        // intersection: {q2, planning} = 2, union: {q2, planning, review, strategy} = 4
        // jaccard = 0.5 — boundary case, > 0.3 so = 25
        assert_eq!(
            score_title_similarity("Q2 Planning Review", "Q2 Strategy Planning"),
            25
        );
    }

    #[test]
    fn test_token_overlap_strong() {
        // "Acme Weekly Sync" vs "Weekly Sync Acme"
        // tokens identical, jaccard = 1.0
        assert_eq!(
            score_title_similarity("Acme Weekly Sync", "Weekly Sync Acme"),
            50
        );
    }

    #[test]
    fn test_no_title_match() {
        assert_eq!(score_title_similarity("Lunch Order", "Board Meeting"), 0);
    }

    #[test]
    fn test_empty_titles() {
        assert_eq!(score_title_similarity("", ""), 0); // empty titles provide no signal
    }

    #[test]
    fn test_one_empty_title() {
        assert_eq!(score_title_similarity("", "Board Meeting"), 0);
    }

    #[test]
    fn test_time_proximity_same_day() {
        let t1 = Utc.with_ymd_and_hms(2026, 2, 20, 10, 0, 0).unwrap();
        let t2 = Utc.with_ymd_and_hms(2026, 2, 20, 14, 0, 0).unwrap();
        assert_eq!(score_time_proximity(t1, t2), 80);
    }

    #[test]
    fn test_time_proximity_next_day() {
        let t1 = Utc.with_ymd_and_hms(2026, 2, 20, 10, 0, 0).unwrap();
        let t2 = Utc.with_ymd_and_hms(2026, 2, 21, 10, 0, 0).unwrap();
        assert_eq!(score_time_proximity(t1, t2), 80); // 24h exactly
    }

    #[test]
    fn test_time_proximity_3_days() {
        let t1 = Utc.with_ymd_and_hms(2026, 2, 20, 10, 0, 0).unwrap();
        let t2 = Utc.with_ymd_and_hms(2026, 2, 22, 10, 0, 0).unwrap();
        assert_eq!(score_time_proximity(t1, t2), 50);
    }

    #[test]
    fn test_time_proximity_5_days() {
        let t1 = Utc.with_ymd_and_hms(2026, 2, 20, 10, 0, 0).unwrap();
        let t2 = Utc.with_ymd_and_hms(2026, 2, 25, 10, 0, 0).unwrap();
        assert_eq!(score_time_proximity(t1, t2), 20);
    }

    #[test]
    fn test_time_proximity_beyond_week() {
        let t1 = Utc.with_ymd_and_hms(2026, 2, 20, 10, 0, 0).unwrap();
        let t2 = Utc.with_ymd_and_hms(2026, 3, 5, 10, 0, 0).unwrap();
        assert_eq!(score_time_proximity(t1, t2), 0);
    }

    #[test]
    fn test_full_match_all_signals() {
        let candidates = vec![MeetingCandidate {
            meeting_id: "m1".to_string(),
            title: "Weekly Standup".to_string(),
            start_time: Some(Utc.with_ymd_and_hms(2026, 2, 20, 10, 0, 0).unwrap()),
            entity_id: Some("acme".to_string()),
        }];
        let result = find_best_match(
            "Weekly Standup",
            Some(Utc.with_ymd_and_hms(2026, 2, 20, 14, 0, 0).unwrap()),
            Some("acme"),
            &candidates,
        );
        assert!(result.is_some());
        let m = result.unwrap();
        assert_eq!(m.meeting_id, "m1");
        assert_eq!(m.score, 220); // 100 (exact title) + 80 (same day) + 40 (entity)
        assert!((m.confidence - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_title_and_time_without_entity() {
        let candidates = vec![MeetingCandidate {
            meeting_id: "m1".to_string(),
            title: "Weekly Standup".to_string(),
            start_time: Some(Utc.with_ymd_and_hms(2026, 2, 20, 10, 0, 0).unwrap()),
            entity_id: None,
        }];
        let result = find_best_match(
            "Weekly Standup",
            Some(Utc.with_ymd_and_hms(2026, 2, 20, 14, 0, 0).unwrap()),
            None,
            &candidates,
        );
        assert!(result.is_some());
        assert_eq!(result.unwrap().score, 180); // 100 + 80
    }

    #[test]
    fn test_below_threshold() {
        let candidates = vec![MeetingCandidate {
            meeting_id: "m1".to_string(),
            title: "Board Meeting".to_string(),
            start_time: None,
            entity_id: None,
        }];
        let result = find_best_match("Lunch Order", None, None, &candidates);
        assert!(result.is_none());
    }

    #[test]
    fn test_best_match_selected() {
        let candidates = vec![
            MeetingCandidate {
                meeting_id: "m1".to_string(),
                title: "Some Other Meeting".to_string(),
                start_time: Some(Utc.with_ymd_and_hms(2026, 2, 20, 10, 0, 0).unwrap()),
                entity_id: Some("acme".to_string()),
            },
            MeetingCandidate {
                meeting_id: "m2".to_string(),
                title: "Weekly Standup".to_string(),
                start_time: Some(Utc.with_ymd_and_hms(2026, 2, 20, 10, 0, 0).unwrap()),
                entity_id: None,
            },
        ];
        let result = find_best_match(
            "Weekly Standup",
            Some(Utc.with_ymd_and_hms(2026, 2, 20, 10, 0, 0).unwrap()),
            None,
            &candidates,
        );
        assert!(result.is_some());
        assert_eq!(result.unwrap().meeting_id, "m2"); // exact title + time > entity-only
    }

    #[test]
    fn test_empty_candidates() {
        let result = find_best_match("Weekly Standup", None, None, &[]);
        assert!(result.is_none());
    }

    #[test]
    fn test_entity_match_pushes_over_threshold() {
        // Title contains = 70, entity = 40 -> 110 >= 100
        let candidates = vec![MeetingCandidate {
            meeting_id: "m1".to_string(),
            title: "Acme Weekly Standup — Q1".to_string(),
            start_time: None,
            entity_id: Some("acme".to_string()),
        }];
        let result = find_best_match("Acme Weekly Standup", None, Some("acme"), &candidates);
        assert!(result.is_some());
        assert_eq!(result.unwrap().score, 110); // 70 (contains) + 40 (entity)
    }

    #[test]
    fn test_confidence_capped_at_one() {
        // Even if somehow score exceeds MAX_SCORE, confidence is capped at 1.0
        let candidates = vec![MeetingCandidate {
            meeting_id: "m1".to_string(),
            title: "Test".to_string(),
            start_time: Some(Utc.with_ymd_and_hms(2026, 2, 20, 10, 0, 0).unwrap()),
            entity_id: Some("acme".to_string()),
        }];
        let result = find_best_match(
            "Test",
            Some(Utc.with_ymd_and_hms(2026, 2, 20, 10, 0, 0).unwrap()),
            Some("acme"),
            &candidates,
        );
        assert!(result.is_some());
        assert!(result.unwrap().confidence <= 1.0);
    }
}
