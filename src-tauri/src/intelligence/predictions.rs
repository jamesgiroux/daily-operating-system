//! I635: Prep prediction scorecard — compare pre-meeting predictions against outcomes.
//!
//! Extracts risks/wins from `prep_frozen_json` (pre-meeting predictions) and
//! `enriched_captures` (post-meeting outcomes), then classifies each item as:
//! - Confirmed: predicted and happened
//! - NotRaised: predicted but didn't come up
//! - Surprise: happened but wasn't predicted
//!
//! Emits Bayesian feedback signals for confirmed predictions to reward accurate sources.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::db::types::EnrichedCapture;
use crate::db::ActionDb;

/// Complete scorecard comparing pre-meeting predictions against outcomes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PredictionScorecard {
    pub risk_predictions: Vec<PredictionResult>,
    pub win_predictions: Vec<PredictionResult>,
    pub has_data: bool,
}

/// A single prediction result with classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PredictionResult {
    pub text: String,
    pub category: PredictionCategory,
    pub source: Option<String>,
    pub match_text: Option<String>,
}

/// Classification of a prediction against actual outcomes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PredictionCategory {
    /// Predicted and happened.
    Confirmed,
    /// Predicted but didn't come up.
    NotRaised,
    /// Happened but wasn't predicted.
    Surprise,
}

/// Jaccard similarity between two strings based on lowercased word tokens.
fn jaccard_similarity(a: &str, b: &str) -> f64 {
    let set_a: HashSet<String> = a
        .to_lowercase()
        .split_whitespace()
        .map(String::from)
        .collect();
    let set_b: HashSet<String> = b
        .to_lowercase()
        .split_whitespace()
        .map(String::from)
        .collect();
    let intersection = set_a.intersection(&set_b).count() as f64;
    let union_count = set_a.union(&set_b).count() as f64;
    if union_count == 0.0 {
        0.0
    } else {
        intersection / union_count
    }
}

/// Keyword-overlap threshold for matching predictions against outcomes.
///
/// We use a stricter fallback threshold so loosely-related phrases do not
/// appear as confirmed predictions in the Meeting Record.
const MATCH_THRESHOLD: f64 = 0.5;

/// A prep item extracted from frozen prep JSON.
#[derive(Debug, Clone)]
pub struct PrepItem {
    pub text: String,
    pub source: Option<String>,
}

fn normalize_prediction_display_text(text: &str) -> String {
    let trimmed = text.trim();
    if let Some(rest) = trimmed.strip_prefix('[') {
        if let Some(end) = rest.find(']') {
            let tag = &rest[..end];
            if !tag.is_empty()
                && tag
                    .chars()
                    .all(|ch| ch.is_ascii_uppercase() || ch == '_' || ch.is_ascii_digit())
            {
                return rest[end + 1..].trim().to_string();
            }
        }
    }

    trimmed.to_string()
}

/// Compute the prediction scorecard by matching prep items against outcome items.
///
/// For each prep risk/win: best match >= threshold -> Confirmed, else NotRaised.
/// For each outcome risk/win not matched by any prep item -> Surprise.
pub fn compute_scorecard(
    prep_risks: &[PrepItem],
    prep_wins: &[PrepItem],
    outcome_risks: &[String],
    outcome_wins: &[String],
) -> PredictionScorecard {
    let risk_predictions = classify_predictions(prep_risks, outcome_risks);
    let win_predictions = classify_predictions(prep_wins, outcome_wins);

    let has_data = !risk_predictions.is_empty() || !win_predictions.is_empty();

    PredictionScorecard {
        risk_predictions,
        win_predictions,
        has_data,
    }
}

/// Classify prep items against outcomes and add surprise items.
fn classify_predictions(prep_items: &[PrepItem], outcomes: &[String]) -> Vec<PredictionResult> {
    let mut results = Vec::new();
    let mut matched_outcomes: HashSet<usize> = HashSet::new();

    // For each prep item, find best matching outcome
    for item in prep_items {
        let mut best_score = 0.0_f64;
        let mut best_idx: Option<usize> = None;
        let mut best_text: Option<String> = None;

        for (idx, outcome) in outcomes.iter().enumerate() {
            let score = jaccard_similarity(&item.text, outcome);
            if score > best_score {
                best_score = score;
                best_idx = Some(idx);
                best_text = Some(outcome.clone());
            }
        }

        if best_score >= MATCH_THRESHOLD {
            if let Some(idx) = best_idx {
                matched_outcomes.insert(idx);
            }
            results.push(PredictionResult {
                text: normalize_prediction_display_text(&item.text),
                category: PredictionCategory::Confirmed,
                source: item.source.clone(),
                match_text: best_text.map(|text| normalize_prediction_display_text(&text)),
            });
        } else {
            results.push(PredictionResult {
                text: normalize_prediction_display_text(&item.text),
                category: PredictionCategory::NotRaised,
                source: item.source.clone(),
                match_text: None,
            });
        }
    }

    // Outcomes not matched by any prep item are surprises
    for (idx, outcome) in outcomes.iter().enumerate() {
        if !matched_outcomes.contains(&idx) {
            results.push(PredictionResult {
                text: normalize_prediction_display_text(outcome),
                category: PredictionCategory::Surprise,
                source: None,
                match_text: None,
            });
        }
    }

    results
}

/// Extract prep risks from a `FullMeetingPrep` struct (ADR-0101: DB read model).
///
/// Reads from both `risks` (Vec<String>) and `entity_risks` (Vec<IntelRisk>)
/// fields on the struct, avoiding JSON parsing entirely.
pub fn extract_prep_risks_from_struct(prep: &crate::types::FullMeetingPrep) -> Vec<PrepItem> {
    let mut items = Vec::new();

    // Simple string risks
    if let Some(ref risks) = prep.risks {
        for text in risks {
            if !text.is_empty() {
                items.push(PrepItem {
                    text: text.clone(),
                    source: None,
                });
            }
        }
    }

    // Structured entity risks (IntelRisk with text + source)
    if let Some(ref entity_risks) = prep.entity_risks {
        for risk in entity_risks {
            if !risk.text.is_empty() {
                items.push(PrepItem {
                    text: risk.text.clone(),
                    source: risk.source.clone(),
                });
            }
        }
    }

    items
}

/// Extract prep wins from a `FullMeetingPrep` struct (ADR-0101: DB read model).
///
/// Reads from `recent_wins` and `recent_win_sources` fields on the struct,
/// avoiding JSON parsing entirely.
pub fn extract_prep_wins_from_struct(prep: &crate::types::FullMeetingPrep) -> Vec<PrepItem> {
    let mut items = Vec::new();

    if let Some(ref wins) = prep.recent_wins {
        for text in wins {
            if !text.is_empty() {
                items.push(PrepItem {
                    text: text.clone(),
                    source: None,
                });
            }
        }
    }

    // Attach structured win sources by index
    if let Some(ref sources) = prep.recent_win_sources {
        for (i, source) in sources.iter().enumerate() {
            if i < items.len() && !source.label.is_empty() {
                items[i].source = Some(source.label.clone());
            }
        }
    }

    items
}

/// Extract prep risks from frozen prep JSON.
///
/// Looks for both `risks` (Vec<String>) and `entityRisks` (Vec<IntelRisk>)
/// fields in the prep structure.
///
/// NOTE: Prefer `extract_prep_risks_from_struct` when a `FullMeetingPrep` is
/// available (ADR-0101). This JSON-based version is retained for legacy callers
/// and tests that work with raw JSON strings.
pub fn extract_prep_risks(frozen_json: &str) -> Vec<PrepItem> {
    let value: serde_json::Value = match serde_json::from_str(frozen_json) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    // The frozen JSON may have a "prep" wrapper or be the prep directly
    let prep = value.get("prep").unwrap_or(&value);

    let mut items = Vec::new();

    // Simple string risks
    if let Some(risks) = prep.get("risks").and_then(|v| v.as_array()) {
        for risk in risks {
            if let Some(text) = risk.as_str() {
                if !text.is_empty() {
                    items.push(PrepItem {
                        text: normalize_prediction_display_text(text),
                        source: None,
                    });
                }
            }
        }
    }

    // Structured entity risks (IntelRisk with text + source)
    if let Some(entity_risks) = prep.get("entityRisks").and_then(|v| v.as_array()) {
        for risk in entity_risks {
            if let Some(text) = risk.get("text").and_then(|v| v.as_str()) {
                if !text.is_empty() {
                    let source = risk
                        .get("source")
                        .and_then(|v| v.as_str())
                        .map(String::from);
                    items.push(PrepItem {
                        text: normalize_prediction_display_text(text),
                        source,
                    });
                }
            }
        }
    }

    items
}

/// Extract prep wins from frozen prep JSON.
///
/// NOTE: Prefer `extract_prep_wins_from_struct` when a `FullMeetingPrep` is
/// available (ADR-0101). This JSON-based version is retained for legacy callers.
pub fn extract_prep_wins(frozen_json: &str) -> Vec<PrepItem> {
    let value: serde_json::Value = match serde_json::from_str(frozen_json) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let prep = value.get("prep").unwrap_or(&value);

    let mut items = Vec::new();

    if let Some(wins) = prep.get("recentWins").and_then(|v| v.as_array()) {
        for win in wins {
            if let Some(text) = win.as_str() {
                if !text.is_empty() {
                    items.push(PrepItem {
                        text: normalize_prediction_display_text(text),
                        source: None,
                    });
                }
            }
        }
    }

    // Check for structured win sources to attach provenance
    if let Some(sources) = prep.get("recentWinSources").and_then(|v| v.as_array()) {
        // Match win sources by index to the items already extracted
        for (i, source) in sources.iter().enumerate() {
            if i < items.len() {
                if let Some(label) = source.get("label").and_then(|v| v.as_str()) {
                    items[i].source = Some(label.to_string());
                }
            }
        }
    }

    items
}

/// Extract outcome risks/wins from enriched captures.
pub fn extract_outcome_items(captures: &[EnrichedCapture]) -> (Vec<String>, Vec<String>) {
    let mut risks = Vec::new();
    let mut wins = Vec::new();

    for capture in captures {
        match capture.capture_type.as_str() {
            "risk" => risks.push(normalize_prediction_display_text(&capture.content)),
            "win" => wins.push(normalize_prediction_display_text(&capture.content)),
            _ => {}
        }
    }

    (risks, wins)
}

/// Emit Bayesian feedback for confirmed predictions.
///
/// For each confirmed prediction with a source, reward that source
/// by incrementing its alpha (success count) in signal_weights.
pub fn emit_prediction_feedback(db: &ActionDb, scorecard: &PredictionScorecard, meeting_id: &str) {
    let all_predictions = scorecard
        .risk_predictions
        .iter()
        .chain(scorecard.win_predictions.iter());

    for prediction in all_predictions {
        match prediction.category {
            PredictionCategory::Confirmed => {
                if let Some(ref source) = prediction.source {
                    // Reward the source that made a correct prediction
                    if let Err(e) = db.upsert_signal_weight(
                        source,
                        "meeting",
                        "prediction_confirmed",
                        1.0, // alpha_delta: reward correct prediction
                        0.0, // beta_delta: no penalty
                    ) {
                        log::warn!(
                            "Failed to emit prediction feedback for meeting {}: {}",
                            meeting_id,
                            e
                        );
                    }
                }
            }
            PredictionCategory::Surprise => {
                // Log for coverage gap tracking — no penalty emission
                log::info!(
                    "Prediction surprise for meeting {}: {}",
                    meeting_id,
                    prediction.text
                );
            }
            PredictionCategory::NotRaised => {
                // Absence is not evidence — no action needed
            }
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;

    #[test]
    fn test_jaccard_similarity_identical() {
        let score = jaccard_similarity("support ticket concerns", "support ticket concerns");
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_jaccard_similarity_partial_overlap() {
        let score = jaccard_similarity("support ticket concerns", "API support issues discussed");
        // "support" is shared; sets are {support,ticket,concerns} and {api,support,issues,discussed}
        // intersection = 1, union = 6 -> 0.166...
        assert!(score > 0.1);
        assert!(score < 0.5);
    }

    #[test]
    fn test_jaccard_similarity_no_overlap() {
        let score = jaccard_similarity("budget approval", "technical architecture review");
        assert!((score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_jaccard_similarity_empty_strings() {
        assert!((jaccard_similarity("", "")).abs() < f64::EPSILON);
        assert!((jaccard_similarity("hello", "")).abs() < f64::EPSILON);
    }

    #[test]
    fn test_normalize_prediction_display_text_strips_control_tags() {
        assert_eq!(
            normalize_prediction_display_text("[YELLOW] Launch services pricing unresolved"),
            "Launch services pricing unresolved"
        );
        assert_eq!(
            normalize_prediction_display_text("[EXPANSION] Customer volunteered AI collaboration"),
            "Customer volunteered AI collaboration"
        );
    }

    #[test]
    fn test_normalize_prediction_display_text_preserves_plain_text() {
        assert_eq!(
            normalize_prediction_display_text("Security review delayed signature"),
            "Security review delayed signature"
        );
    }

    #[test]
    fn test_compute_scorecard_confirmed() {
        let prep_risks = vec![PrepItem {
            text: "support ticket backlog growing".to_string(),
            source: Some("glean".to_string()),
        }];
        let outcome_risks = vec!["growing support ticket backlog discussed".to_string()];

        let scorecard = compute_scorecard(&prep_risks, &[], &outcome_risks, &[]);

        assert!(scorecard.has_data);
        assert_eq!(scorecard.risk_predictions.len(), 1);
        assert!(matches!(
            scorecard.risk_predictions[0].category,
            PredictionCategory::Confirmed
        ));
        assert!(scorecard.risk_predictions[0].match_text.is_some());
    }

    #[test]
    fn test_compute_scorecard_not_raised() {
        let prep_risks = vec![PrepItem {
            text: "budget approval pending".to_string(),
            source: None,
        }];
        let outcome_risks = vec!["technical integration challenges".to_string()];

        let scorecard = compute_scorecard(&prep_risks, &[], &outcome_risks, &[]);

        // Budget item should be NotRaised, technical item should be Surprise
        assert_eq!(scorecard.risk_predictions.len(), 2);
        let not_raised = scorecard
            .risk_predictions
            .iter()
            .find(|p| p.text == "budget approval pending")
            .unwrap();
        assert!(matches!(not_raised.category, PredictionCategory::NotRaised));

        let surprise = scorecard
            .risk_predictions
            .iter()
            .find(|p| p.text == "technical integration challenges")
            .unwrap();
        assert!(matches!(surprise.category, PredictionCategory::Surprise));
    }

    #[test]
    fn test_compute_scorecard_empty() {
        let scorecard = compute_scorecard(&[], &[], &[], &[]);
        assert!(!scorecard.has_data);
        assert!(scorecard.risk_predictions.is_empty());
        assert!(scorecard.win_predictions.is_empty());
    }

    #[test]
    fn test_extract_prep_risks_simple() {
        let json = r#"{"risks":["budget concerns","timeline slipping"]}"#;
        let items = extract_prep_risks(json);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].text, "budget concerns");
        assert!(items[0].source.is_none());
    }

    #[test]
    fn test_extract_prep_risks_with_prep_wrapper() {
        let json = r#"{"prep":{"risks":["budget concerns"],"entityRisks":[{"text":"API issues","source":"glean"}]}}"#;
        let items = extract_prep_risks(json);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].text, "budget concerns");
        assert_eq!(items[1].text, "API issues");
        assert_eq!(items[1].source.as_deref(), Some("glean"));
    }

    #[test]
    fn test_extract_prep_wins() {
        let json = r#"{"recentWins":["successful onboarding","hit Q3 targets"]}"#;
        let items = extract_prep_wins(json);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].text, "successful onboarding");
    }

    #[test]
    fn test_extract_outcome_items() {
        let captures = vec![
            EnrichedCapture {
                id: "c1".to_string(),
                meeting_id: "m1".to_string(),
                meeting_title: "Test".to_string(),
                account_id: None,
                capture_type: "risk".to_string(),
                content: "API latency issues".to_string(),
                sub_type: None,
                urgency: None,
                impact: None,
                evidence_quote: None,
                speaker: None,
                captured_at: "2026-01-01T00:00:00Z".to_string(),
            },
            EnrichedCapture {
                id: "c2".to_string(),
                meeting_id: "m1".to_string(),
                meeting_title: "Test".to_string(),
                account_id: None,
                capture_type: "win".to_string(),
                content: "Customer praised the new dashboard".to_string(),
                sub_type: None,
                urgency: None,
                impact: None,
                evidence_quote: None,
                speaker: None,
                captured_at: "2026-01-01T00:00:00Z".to_string(),
            },
            EnrichedCapture {
                id: "c3".to_string(),
                meeting_id: "m1".to_string(),
                meeting_title: "Test".to_string(),
                account_id: None,
                capture_type: "decision".to_string(),
                content: "Will schedule follow-up".to_string(),
                sub_type: None,
                urgency: None,
                impact: None,
                evidence_quote: None,
                speaker: None,
                captured_at: "2026-01-01T00:00:00Z".to_string(),
            },
        ];

        let (risks, wins) = extract_outcome_items(&captures);
        assert_eq!(risks.len(), 1);
        assert_eq!(wins.len(), 1);
        assert_eq!(risks[0], "API latency issues");
        assert_eq!(wins[0], "Customer praised the new dashboard");
    }

    #[test]
    fn test_extract_outcome_items_normalizes_legacy_tagged_captures() {
        let captures = vec![
            EnrichedCapture {
                id: "c1".to_string(),
                meeting_id: "m1".to_string(),
                meeting_title: "Test".to_string(),
                account_id: None,
                capture_type: "risk".to_string(),
                content: "[YELLOW] Launch services pricing unresolved".to_string(),
                sub_type: None,
                urgency: Some("yellow".to_string()),
                impact: None,
                evidence_quote: None,
                speaker: None,
                captured_at: "2026-01-01T00:00:00Z".to_string(),
            },
            EnrichedCapture {
                id: "c2".to_string(),
                meeting_id: "m1".to_string(),
                meeting_title: "Test".to_string(),
                account_id: None,
                capture_type: "win".to_string(),
                content: "[EXPANSION] Customer volunteered AI collaboration".to_string(),
                sub_type: Some("expansion".to_string()),
                urgency: None,
                impact: None,
                evidence_quote: None,
                speaker: None,
                captured_at: "2026-01-01T00:00:00Z".to_string(),
            },
        ];

        let (risks, wins) = extract_outcome_items(&captures);
        assert_eq!(risks, vec!["Launch services pricing unresolved"]);
        assert_eq!(wins, vec!["Customer volunteered AI collaboration"]);
    }

    #[test]
    fn test_emit_prediction_feedback_rewards_confirmed_sources_only() {
        let db = test_db();
        let scorecard = PredictionScorecard {
            has_data: true,
            risk_predictions: vec![
                PredictionResult {
                    text: "Security review delays signature".to_string(),
                    category: PredictionCategory::Confirmed,
                    source: Some("prep-risk-source".to_string()),
                    match_text: Some("Security review delayed signature".to_string()),
                },
                PredictionResult {
                    text: "Budget concerns do not surface".to_string(),
                    category: PredictionCategory::NotRaised,
                    source: Some("prep-risk-source".to_string()),
                    match_text: None,
                },
            ],
            win_predictions: vec![
                PredictionResult {
                    text: "Champion expands rollout".to_string(),
                    category: PredictionCategory::Confirmed,
                    source: Some("prep-win-source".to_string()),
                    match_text: Some("Champion approved rollout expansion".to_string()),
                },
                PredictionResult {
                    text: "Unexpected procurement blocker".to_string(),
                    category: PredictionCategory::Surprise,
                    source: None,
                    match_text: None,
                },
            ],
        };

        emit_prediction_feedback(&db, &scorecard, "mtg-feedback");

        let (risk_alpha, risk_beta, risk_updates): (f64, f64, i32) = db
            .conn_ref()
            .query_row(
                "SELECT alpha, beta, update_count
                 FROM signal_weights
                 WHERE source = 'prep-risk-source'
                   AND entity_type = 'meeting'
                   AND signal_type = 'prediction_confirmed'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("query confirmed risk feedback row");
        assert!((risk_alpha - 2.0).abs() < 0.01);
        assert!((risk_beta - 1.0).abs() < 0.01);
        assert_eq!(risk_updates, 1);

        let (win_alpha, win_beta, win_updates): (f64, f64, i32) = db
            .conn_ref()
            .query_row(
                "SELECT alpha, beta, update_count
                 FROM signal_weights
                 WHERE source = 'prep-win-source'
                   AND entity_type = 'meeting'
                   AND signal_type = 'prediction_confirmed'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("query confirmed win feedback row");
        assert!((win_alpha - 2.0).abs() < 0.01);
        assert!((win_beta - 1.0).abs() < 0.01);
        assert_eq!(win_updates, 1);

        let unrelated_rows: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*)
                 FROM signal_weights
                 WHERE source = 'prep-risk-source'
                   AND entity_type = 'meeting'
                   AND signal_type = 'prediction_confirmed'
                   AND beta > 1.0",
                [],
                |row| row.get(0),
            )
            .expect("query unrelated feedback rows");
        assert_eq!(unrelated_rows, 0);
    }

    #[test]
    fn test_extract_prep_risks_from_struct() {
        let prep = crate::types::FullMeetingPrep {
            file_path: String::new(),
            title: "Test".to_string(),
            time_range: String::new(),
            risks: Some(vec![
                "budget concerns".to_string(),
                "timeline slipping".to_string(),
            ]),
            entity_risks: Some(vec![crate::intelligence::IntelRisk {
                text: "API issues".to_string(),
                source: Some("glean".to_string()),
                urgency: "medium".to_string(),
                item_source: None,
                discrepancy: None,

                ..Default::default()

            }]),
            ..Default::default()
        };
        let items = extract_prep_risks_from_struct(&prep);
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].text, "budget concerns");
        assert!(items[0].source.is_none());
        assert_eq!(items[2].text, "API issues");
        assert_eq!(items[2].source.as_deref(), Some("glean"));
    }

    #[test]
    fn test_extract_prep_wins_from_struct() {
        let prep = crate::types::FullMeetingPrep {
            file_path: String::new(),
            title: "Test".to_string(),
            time_range: String::new(),
            recent_wins: Some(vec![
                "successful onboarding".to_string(),
                "hit Q3 targets".to_string(),
            ]),
            recent_win_sources: Some(vec![crate::types::SourceReference {
                label: "glean".to_string(),
                path: None,
                last_updated: None,
            }]),
            ..Default::default()
        };
        let items = extract_prep_wins_from_struct(&prep);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].text, "successful onboarding");
        assert_eq!(items[0].source.as_deref(), Some("glean"));
        assert_eq!(items[1].text, "hit Q3 targets");
        assert!(items[1].source.is_none());
    }
}
